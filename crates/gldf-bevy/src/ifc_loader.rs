//! IFC Geometry Loader Plugin for Bevy
//!
//! Loads IFC tessellated geometry (from IFCTRIANGULATEDFACESET) into Bevy meshes.
//! Uses the same localStorage bridge pattern as the L3D loader.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::log;

/// Storage key for IFC geometry data
#[cfg(target_arch = "wasm32")]
pub const IFC_GEOMETRY_KEY: &str = "ifc_geometry";
#[cfg(target_arch = "wasm32")]
pub const IFC_TIMESTAMP_KEY: &str = "ifc_timestamp";

/// Plugin for loading IFC geometry into the Bevy scene
pub struct IfcLoaderPlugin;

impl Plugin for IfcLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(IfcSceneData::default())
            .insert_resource(IfcTimestamp::default())
            .add_systems(Startup, setup_ifc_model)
            .add_systems(Update, poll_ifc_changes);
    }
}

/// Component to mark IFC model entities for cleanup
#[derive(Component)]
pub struct IfcModel;

/// IFC geometry data from localStorage (matches ImportedGeometry format)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IfcGeometry {
    /// Vertex positions (x, y, z) in meters - IFC coordinate system
    pub vertices: Vec<(f64, f64, f64)>,
    /// Triangle indices (0-based)
    pub triangles: Vec<(u32, u32, u32)>,
}

/// Resource to store the current IFC scene data
#[derive(Resource, Default)]
pub struct IfcSceneData {
    /// The IFC geometry data
    pub geometry: Option<IfcGeometry>,
    /// Variant name for display
    pub variant_name: Option<String>,
    /// Flag to indicate data needs reload
    pub needs_reload: bool,
}

/// Resource to track localStorage timestamp for hot-reload
#[derive(Resource, Default)]
pub struct IfcTimestamp(pub String);

/// Load IFC geometry from localStorage (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn load_ifc_geometry_from_storage() -> Option<IfcGeometry> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let geometry_json = storage.get_item(IFC_GEOMETRY_KEY).ok()??;
    log(&format!(
        "[Bevy IFC] Loading geometry from storage, len: {}",
        geometry_json.len()
    ));
    let parsed: Option<IfcGeometry> = serde_json::from_str(&geometry_json).ok();
    if let Some(ref g) = parsed {
        log(&format!(
            "[Bevy IFC] Parsed: {} vertices, {} triangles",
            g.vertices.len(),
            g.triangles.len()
        ));
    }
    parsed
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_ifc_geometry_from_storage() -> Option<IfcGeometry> {
    None
}

/// Get IFC timestamp from localStorage
#[cfg(target_arch = "wasm32")]
pub fn get_ifc_timestamp() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item(IFC_TIMESTAMP_KEY).ok()?
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_ifc_timestamp() -> Option<String> {
    None
}

/// Create a Bevy mesh from IFC geometry
fn create_ifc_mesh(geometry: &IfcGeometry) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());

    // Convert (f64, f64, f64) to [f32; 3] with coordinate system conversion
    // IFC uses Z-up, Bevy uses Y-up: swap Y and Z, negate new Z
    let positions: Vec<[f32; 3]> = geometry
        .vertices
        .iter()
        .map(|(x, y, z)| [*x as f32, *z as f32, -(*y as f32)])
        .collect();

    // Calculate normals from triangles
    let mut normals = vec![[0.0f32; 3]; positions.len()];
    for (i0, i1, i2) in &geometry.triangles {
        let idx0 = *i0 as usize;
        let idx1 = *i1 as usize;
        let idx2 = *i2 as usize;

        if idx0 < positions.len() && idx1 < positions.len() && idx2 < positions.len() {
            let p0 = Vec3::from(positions[idx0]);
            let p1 = Vec3::from(positions[idx1]);
            let p2 = Vec3::from(positions[idx2]);

            let edge1 = p1 - p0;
            let edge2 = p2 - p0;
            let normal = edge1.cross(edge2).normalize_or_zero();

            // Accumulate normals for smooth shading
            for idx in [idx0, idx1, idx2] {
                normals[idx][0] += normal.x;
                normals[idx][1] += normal.y;
                normals[idx][2] += normal.z;
            }
        }
    }

    // Normalize accumulated normals
    for normal in &mut normals {
        let len = (normal[0].powi(2) + normal[1].powi(2) + normal[2].powi(2)).sqrt();
        if len > 0.0 {
            normal[0] /= len;
            normal[1] /= len;
            normal[2] /= len;
        } else {
            *normal = [0.0, 1.0, 0.0]; // Default up
        }
    }

    // Convert triangle indices - IFC uses 1-based, we use 0-based
    let indices: Vec<u32> = geometry
        .triangles
        .iter()
        .flat_map(|(i0, i1, i2)| {
            // Check if indices are 1-based (common in IFC/STEP)
            let offset = if geometry
                .triangles
                .iter()
                .any(|(a, b, c)| *a == 0 || *b == 0 || *c == 0)
            {
                0 // Already 0-based
            } else {
                1 // 1-based, subtract 1
            };
            [
                i0.saturating_sub(offset),
                i1.saturating_sub(offset),
                i2.saturating_sub(offset),
            ]
        })
        .collect();

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

/// Initial setup - check for IFC geometry on startup
fn setup_ifc_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scene_data: ResMut<IfcSceneData>,
) {
    // Try to load IFC geometry from storage
    if let Some(geometry) = load_ifc_geometry_from_storage() {
        log(&format!(
            "[Bevy IFC] Initial load: {} vertices",
            geometry.vertices.len()
        ));
        spawn_ifc_model(&mut commands, &mut meshes, &mut materials, &geometry);
        scene_data.geometry = Some(geometry);
    }
}

/// Poll localStorage for IFC geometry changes
#[allow(unused_variables, unused_mut)]
fn poll_ifc_changes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scene_data: ResMut<IfcSceneData>,
    mut last_timestamp: ResMut<IfcTimestamp>,
    existing_models: Query<Entity, With<IfcModel>>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(new_timestamp) = get_ifc_timestamp() {
            if new_timestamp != last_timestamp.0 {
                log(&format!(
                    "[Bevy IFC] Timestamp changed: {} -> {}",
                    last_timestamp.0, new_timestamp
                ));

                // Remove existing IFC models
                for entity in existing_models.iter() {
                    commands.entity(entity).despawn();
                }

                // Load new geometry
                if let Some(geometry) = load_ifc_geometry_from_storage() {
                    log(&format!(
                        "[Bevy IFC] Loaded: {} vertices, {} triangles",
                        geometry.vertices.len(),
                        geometry.triangles.len()
                    ));
                    spawn_ifc_model(&mut commands, &mut meshes, &mut materials, &geometry);
                    scene_data.geometry = Some(geometry);
                }

                last_timestamp.0 = new_timestamp;
            }
        }
    }
}

/// Spawn IFC model entities
fn spawn_ifc_model(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    geometry: &IfcGeometry,
) {
    if geometry.vertices.is_empty() {
        log("[Bevy IFC] No vertices to render");
        return;
    }

    let mesh = create_ifc_mesh(geometry);
    let mesh_handle = meshes.add(mesh);

    // Use a light gray metallic material for IFC geometry
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.7, 0.7, 0.75),
        metallic: 0.3,
        perceptual_roughness: 0.5,
        ..default()
    });

    // Calculate bounding box center for positioning
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for (x, y, z) in &geometry.vertices {
        let pos = Vec3::new(*x as f32, *z as f32, -(*y as f32));
        min = min.min(pos);
        max = max.max(pos);
    }
    let center = (min + max) / 2.0;
    let size = max - min;

    log(&format!(
        "[Bevy IFC] Spawning mesh at center {:?}, size {:?}",
        center, size
    ));

    // Spawn the mesh centered in the scene
    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material),
        Transform::from_translation(-center), // Center the model
        IfcModel,
    ));

    // Add a point light above the model for visibility
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.98, 0.95),
            intensity: 50000.0,
            range: 20.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, size.y.max(2.0) + 1.0, 0.0),
        IfcModel,
    ));
}
