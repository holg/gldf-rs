//! L3D Model Loader Plugin for Bevy
//!
//! Loads L3D 3D models (OBJ format) into Bevy meshes with proper transformations.
//! Extracts Light Emitting Surfaces (LES) and places photometric lights at those positions.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

use crate::{log, EmitterConfig, GldfSceneData, SceneSettings};

/// Plugin for loading L3D models into the Bevy scene
pub struct L3dLoaderPlugin;

impl Plugin for L3dLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_l3d_model)
            .add_systems(Update, update_l3d_model);
    }
}

/// Component to mark L3D model entities
#[derive(Component)]
pub struct L3dModel;

/// Component to mark lights spawned from L3D light emitting objects
#[derive(Component)]
pub struct L3dLight;

/// Shape of the light emitting surface
#[derive(Debug, Clone)]
enum EmitterShape {
    /// Rectangle with (width, height) in meters
    Rectangle { width: f32, height: f32 },
    /// Circle with diameter in meters
    Circle { diameter: f32 },
    /// Unknown/default - small circle
    Unknown,
}

/// Extracted light emitting object info with accumulated world transform
struct LightEmitter {
    /// World position (accumulated from parent joints)
    world_position: Vec3,
    /// World rotation (accumulated from parent joints)
    world_rotation: Vec3,
    /// Shape of the emitting surface
    shape: EmitterShape,
    /// Name for debugging
    name: String,
}

/// Simple OBJ parser result
struct ObjData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

/// Parse a simple OBJ file
fn parse_obj(content: &str) -> Option<ObjData> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut temp_positions = Vec::new();
    let mut temp_normals = Vec::new();
    let mut temp_uvs = Vec::new();
    let mut indices = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "v" if parts.len() >= 4 => {
                let x: f32 = parts[1].parse().unwrap_or(0.0);
                let y: f32 = parts[2].parse().unwrap_or(0.0);
                let z: f32 = parts[3].parse().unwrap_or(0.0);
                temp_positions.push([x, y, z]);
            }
            "vn" if parts.len() >= 4 => {
                let x: f32 = parts[1].parse().unwrap_or(0.0);
                let y: f32 = parts[2].parse().unwrap_or(0.0);
                let z: f32 = parts[3].parse().unwrap_or(1.0);
                temp_normals.push([x, y, z]);
            }
            "vt" if parts.len() >= 3 => {
                let u: f32 = parts[1].parse().unwrap_or(0.0);
                let v: f32 = parts[2].parse().unwrap_or(0.0);
                temp_uvs.push([u, v]);
            }
            "f" if parts.len() >= 4 => {
                // Parse face - can be v, v/vt, v/vt/vn, or v//vn
                let mut face_indices = Vec::new();

                for part in parts.iter().skip(1) {
                    let vertex_parts: Vec<&str> = part.split('/').collect();

                    let pos_idx: usize = vertex_parts[0].parse::<usize>().unwrap_or(1) - 1;
                    let uv_idx: Option<usize> = vertex_parts
                        .get(1)
                        .and_then(|s| {
                            if s.is_empty() {
                                None
                            } else {
                                s.parse::<usize>().ok()
                            }
                        })
                        .map(|i| i - 1);
                    let norm_idx: Option<usize> = vertex_parts
                        .get(2)
                        .and_then(|s| s.parse::<usize>().ok())
                        .map(|i| i - 1);

                    let vertex_idx = positions.len() as u32;

                    // Get position
                    let pos = temp_positions
                        .get(pos_idx)
                        .copied()
                        .unwrap_or([0.0, 0.0, 0.0]);
                    positions.push(pos);

                    // Get UV
                    let uv = uv_idx
                        .and_then(|i| temp_uvs.get(i))
                        .copied()
                        .unwrap_or([0.0, 0.0]);
                    uvs.push(uv);

                    // Get normal
                    let norm = norm_idx
                        .and_then(|i| temp_normals.get(i))
                        .copied()
                        .unwrap_or([0.0, 1.0, 0.0]);
                    normals.push(norm);

                    face_indices.push(vertex_idx);
                }

                // Triangulate the face (fan triangulation)
                for i in 1..face_indices.len() - 1 {
                    indices.push(face_indices[0]);
                    indices.push(face_indices[i]);
                    indices.push(face_indices[i + 1]);
                }
            }
            _ => {}
        }
    }

    if positions.is_empty() {
        return None;
    }

    // Generate normals if none were provided
    if normals.iter().all(|n| *n == [0.0, 1.0, 0.0]) {
        // Compute flat normals
        for i in (0..indices.len()).step_by(3) {
            if i + 2 < indices.len() {
                let i0 = indices[i] as usize;
                let i1 = indices[i + 1] as usize;
                let i2 = indices[i + 2] as usize;

                let p0 = Vec3::from(positions[i0]);
                let p1 = Vec3::from(positions[i1]);
                let p2 = Vec3::from(positions[i2]);

                let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();

                normals[i0] = normal.to_array();
                normals[i1] = normal.to_array();
                normals[i2] = normal.to_array();
            }
        }
    }

    Some(ObjData {
        positions,
        normals,
        uvs,
        indices,
    })
}

/// Create a Bevy mesh from OBJ data
fn create_mesh_from_obj(obj: &ObjData) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, obj.positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, obj.normals.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, obj.uvs.clone());
    mesh.insert_indices(Indices::U32(obj.indices.clone()));

    mesh
}

/// Setup L3D model on startup
fn setup_l3d_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    scene_data: Res<GldfSceneData>,
    settings: Res<SceneSettings>,
) {
    log(&format!(
        "[L3D] setup_l3d_model called, has L3D: {}, has LDT: {}, emitters: {}",
        scene_data.l3d_data.is_some(),
        scene_data.ldt_data.is_some(),
        scene_data.emitter_config.len()
    ));
    spawn_l3d_model(
        &mut commands,
        &mut meshes,
        &mut materials,
        &scene_data,
        &settings,
    );
}

/// Update L3D model when data changes
fn update_l3d_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    scene_data: Res<GldfSceneData>,
    settings: Res<SceneSettings>,
    existing_models: Query<Entity, With<L3dModel>>,
    existing_lights: Query<Entity, With<L3dLight>>,
) {
    if !scene_data.is_changed() {
        return;
    }
    log(&format!(
        "[L3D] update_l3d_model triggered, has L3D: {}, emitters: {}",
        scene_data.l3d_data.is_some(),
        scene_data.emitter_config.len()
    ));

    // Remove existing models
    for entity in existing_models.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Remove existing L3D lights
    for entity in existing_lights.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Spawn new model
    spawn_l3d_model(
        &mut commands,
        &mut meshes,
        &mut materials,
        &scene_data,
        &settings,
    );
}

/// Spawn L3D model entities and lights at light emitting surfaces
fn spawn_l3d_model(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    scene_data: &GldfSceneData,
    settings: &SceneSettings,
) {
    let Some(l3d_bytes) = &scene_data.l3d_data else {
        log("[L3D] No L3D data to load");
        return;
    };

    log(&format!(
        "[L3D] Loading L3D file, {} bytes",
        l3d_bytes.len()
    ));

    // Parse L3D file
    let l3d = l3d_rs::from_buffer(l3d_bytes);

    log(&format!(
        "[L3D] Parsed L3D: {} parts, {} assets",
        l3d.model.parts.len(),
        l3d.file.assets.len()
    ));

    if l3d.model.parts.is_empty() {
        log("[L3D] No parts in L3D model");
        return;
    }

    // Default material for luminaire body
    let luminaire_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.9),
        metallic: 0.3,
        perceptual_roughness: 0.5,
        ..default()
    });
    log(&format!(
        "[L3D] Luminaire will be placed at: ({:.2}, {:.2}, {:.2})",
        settings.room_width / 2.0,
        settings.attachment_height(),
        settings.room_length / 2.0
    ));

    // Rotation matrix: L3D uses Z-up, Bevy uses Y-up
    let z_to_y_rotation = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);

    // Position luminaire at proper height based on scene type
    // For Room scenes, place luminaire below the ceiling
    // The L3D model coordinates have Z=0 at the mounting surface (ceiling)
    // Use a larger offset to ensure visibility
    let effective_height = settings.attachment_height() - 0.1; // 10cm below ceiling
    let mounting_offset = Vec3::new(
        settings.room_width / 2.0,
        effective_height,
        settings.room_length / 2.0,
    );
    log(&format!(
        "[L3D] Mounting offset: ({:.2}, {:.2}, {:.2}), attachment_height: {:.2}",
        mounting_offset.x, mounting_offset.y, mounting_offset.z,
        settings.attachment_height()
    ));

    // Parse the structure XML to get light emitting objects
    let mut light_emitters: Vec<LightEmitter> = Vec::new();
    if let Ok(luminaire) = l3d_rs::Luminaire::from_xml(&l3d.file.structure) {
        // Start with identity matrix
        let identity = Mat4::IDENTITY;
        extract_light_emitters(&luminaire.structure.geometry, &mut light_emitters, identity);
        log(&format!(
            "[L3D] Found {} light emitting objects",
            light_emitters.len()
        ));
    } else {
        log("[L3D] Failed to parse structure XML for light emitting objects");
    }

    // Load each part
    let mut loaded_count = 0;
    for part in &l3d.model.parts {
        // Find the OBJ asset
        let Some(asset) = l3d.file.assets.iter().find(|a| a.name == part.path) else {
            log(&format!("[L3D] Part asset not found: {}", part.path));
            continue;
        };

        // Parse OBJ content
        let Ok(obj_content) = std::str::from_utf8(&asset.content) else {
            log(&format!(
                "[L3D] Failed to parse asset as UTF-8: {}",
                part.path
            ));
            continue;
        };

        let Some(obj_data) = parse_obj(obj_content) else {
            log(&format!("[L3D] Failed to parse OBJ: {}", part.path));
            continue;
        };

        log(&format!(
            "[L3D] Loading part: {} ({} vertices, {} indices)",
            part.path,
            obj_data.positions.len(),
            obj_data.indices.len()
        ));

        // Create mesh
        let mesh = create_mesh_from_obj(&obj_data);
        let mesh_handle = meshes.add(mesh);

        // Convert transformation matrix (column-major [f32; 16])
        let part_mat = Mat4::from_cols(
            Vec4::new(part.mat[0], part.mat[1], part.mat[2], part.mat[3]),
            Vec4::new(part.mat[4], part.mat[5], part.mat[6], part.mat[7]),
            Vec4::new(part.mat[8], part.mat[9], part.mat[10], part.mat[11]),
            Vec4::new(part.mat[12], part.mat[13], part.mat[14], part.mat[15]),
        );

        // Extract translation, rotation, scale
        let (scale, rotation, translation) = part_mat.to_scale_rotation_translation();

        // Apply Z-to-Y rotation
        let final_rotation = z_to_y_rotation * rotation;
        let final_translation = z_to_y_rotation * translation;

        commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(luminaire_material.clone()),
            Transform {
                translation: final_translation + mounting_offset,
                rotation: final_rotation,
                scale,
            },
            L3dModel,
        ));
        loaded_count += 1;
    }

    log(&format!("[L3D] Loaded {} model parts", loaded_count));

    // Spawn lights at light emitting object positions
    spawn_lights_from_emitters(
        commands,
        meshes,
        materials,
        &light_emitters,
        &mounting_offset,
        &z_to_y_rotation,
        settings,
        &scene_data.emitter_config,
    );
}

/// Convert l3d_rs Mat4 ([f32; 16] column-major) to Bevy Mat4
fn l3d_mat_to_bevy(m: &l3d_rs::Mat4) -> Mat4 {
    Mat4::from_cols_array(m)
}

/// Recursively extract light emitting objects from L3D geometry using matrix transforms
fn extract_light_emitters(
    geometry: &l3d_rs::Geometry,
    emitters: &mut Vec<LightEmitter>,
    parent_matrix: Mat4,
) {
    // Build transform for this geometry (same as l3d_rs does)
    let geo_matrix_raw = l3d_rs::build_transform(&geometry.position, &geometry.rotation);
    let geo_matrix = l3d_mat_to_bevy(&geo_matrix_raw);
    // Multiply with parent to get cumulative transform
    let current_matrix = parent_matrix * geo_matrix;

    // Extract position from the matrix (translation is in the last column)
    let current_pos = Vec3::new(
        current_matrix.w_axis.x,
        current_matrix.w_axis.y,
        current_matrix.w_axis.z,
    );

    log(&format!(
        "[L3D] Checking geometry '{}' -> world({:.3},{:.3},{:.3})",
        geometry.part_name, current_pos.x, current_pos.y, current_pos.z
    ));

    // Check for light emitting objects in this geometry
    if let Some(leo) = &geometry.light_emitting_objects {
        log(&format!(
            "[L3D] Found LightEmittingObjects with {} objects",
            leo.objects().len()
        ));
        for obj in leo.objects() {
            let (px, py, pz): (f32, f32, f32) = obj.position();
            let (rx, ry, rz): (f32, f32, f32) = obj.rotation();

            // Build transform for the LEO
            let leo_pos = l3d_rs::Vec3f {
                x: px,
                y: py,
                z: pz,
            };
            let leo_rot = l3d_rs::Vec3f {
                x: rx,
                y: ry,
                z: rz,
            };
            let leo_matrix_raw = l3d_rs::build_transform(&leo_pos, &leo_rot);
            let leo_matrix = l3d_mat_to_bevy(&leo_matrix_raw);

            // Combine with parent transform
            let final_matrix = current_matrix * leo_matrix;
            let world_pos = Vec3::new(
                final_matrix.w_axis.x,
                final_matrix.w_axis.y,
                final_matrix.w_axis.z,
            );

            // Extract rotation (simplified - just use the LEO rotation for now)
            let world_rot = Vec3::new(rx, ry, rz);

            // Get shape from rectangle or circle
            let shape = if let Some(rect) = obj.rectangle() {
                let (w, h) = rect.size();
                EmitterShape::Rectangle {
                    width: w as f32,
                    height: h as f32,
                }
            } else if let Some(circle) = obj.circle() {
                EmitterShape::Circle {
                    diameter: circle.diameter() as f32,
                }
            } else {
                EmitterShape::Unknown
            };

            log(&format!(
                "[L3D] *** EMITTER '{}': world({:.3},{:.3},{:.3}), shape={:?}",
                obj.part_name(),
                world_pos.x,
                world_pos.y,
                world_pos.z,
                shape
            ));

            emitters.push(LightEmitter {
                world_position: world_pos,
                world_rotation: world_rot,
                shape,
                name: obj.part_name().to_string(),
            });
        }
    }

    // Recurse into joints if present
    if let Some(joints) = &geometry.joints {
        log(&format!(
            "[L3D] Found {} joints to recurse into",
            joints.joint.len()
        ));
        for joint in &joints.joint {
            // Build transform for the joint
            let joint_matrix_raw = l3d_rs::build_transform(&joint.position, &joint.rotation);
            let joint_matrix = l3d_mat_to_bevy(&joint_matrix_raw);
            let accumulated_matrix = current_matrix * joint_matrix;

            let acc_pos = Vec3::new(
                accumulated_matrix.w_axis.x,
                accumulated_matrix.w_axis.y,
                accumulated_matrix.w_axis.z,
            );
            log(&format!(
                "[L3D] Joint '{}' -> accumulated({:.3},{:.3},{:.3})",
                joint.part_name, acc_pos.x, acc_pos.y, acc_pos.z
            ));

            for child_geom in &joint.geometries.geometry {
                extract_light_emitters(child_geom, emitters, accumulated_matrix);
            }
        }
    }
}

/// Spawn point lights at the light emitting object positions
#[allow(clippy::too_many_arguments)]
fn spawn_lights_from_emitters(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    emitters: &[LightEmitter],
    mounting_offset: &Vec3,
    z_to_y_rotation: &Quat,
    settings: &SceneSettings,
    emitter_configs: &[EmitterConfig],
) {
    if emitters.is_empty() {
        log("[L3D] No light emitters found in L3D structure");
        return;
    }

    // Get default light properties from LDT data
    let (default_color, default_flux) = if let Some(ref ldt) = settings.ldt_data {
        let lamp = ldt.lamp_sets.first();
        let color_temp = lamp
            .map(|l| parse_color_temperature(&l.color_appearance))
            .unwrap_or(4000.0);
        let flux = ldt.total_luminous_flux() as f32;
        let lor = (ldt.light_output_ratio / 100.0) as f32;
        (kelvin_to_rgb(color_temp), flux * lor)
    } else {
        (Color::srgb(1.0, 0.95, 0.9), 1000.0) // Default warm white, 1000 lm
    };

    log(&format!(
        "[L3D] Spawning {} lights, default flux: {:.0} lm, {} emitter configs",
        emitters.len(),
        default_flux,
        emitter_configs.len()
    ));

    // Log all emitter names from L3D
    for e in emitters {
        log(&format!("[L3D] L3D emitter name: '{}'", e.name));
    }
    // Log all config names
    for c in emitter_configs {
        log(&format!(
            "[L3D] Config leo_name: '{}', flux: {:?}, temp: {:?}",
            c.leo_name, c.luminous_flux, c.color_temperature
        ));
    }

    for emitter in emitters {
        // Look up per-emitter configuration by LEO name
        let config = emitter_configs.iter().find(|c| c.leo_name == emitter.name);
        log(&format!(
            "[L3D] Matching '{}' -> config found: {}",
            emitter.name,
            config.is_some()
        ));

        // Use per-emitter flux if available, otherwise distribute default flux
        let flux = config
            .and_then(|c| c.luminous_flux)
            .map(|f| f as f32)
            .unwrap_or(default_flux / emitters.len() as f32);

        // Use per-emitter color temperature if available
        let color = config
            .and_then(|c| c.color_temperature)
            .map(|t| kelvin_to_rgb(t as f32))
            .unwrap_or(default_color);

        // Check if this is an emergency-only light (dim it significantly)
        let is_emergency_only = config
            .and_then(|c| c.emergency_behavior.as_ref())
            .map(|eb| eb == "EmergencyOnly")
            .unwrap_or(false);

        let final_flux = if is_emergency_only {
            flux * 0.1 // Emergency lights are much dimmer in normal operation
        } else {
            flux
        };

        // Bevy uses lumens * some factor for intensity
        let intensity = final_flux * 50.0;

        // Transform position from L3D (Z-up) to Bevy (Y-up)
        let local_pos = *z_to_y_rotation * emitter.world_position;
        let world_pos = local_pos + *mounting_offset;

        // In L3D, the default light direction is -Z (downward in Z-up system)
        let rot_x = emitter.world_rotation.x.to_radians();
        let rot_y = emitter.world_rotation.y.to_radians();
        let rot_z = emitter.world_rotation.z.to_radians();

        // Build rotation: first the emitter's own rotation, then convert coordinate system
        let emitter_rot = Quat::from_euler(EulerRot::XYZ, rot_x, rot_y, rot_z);

        // Default direction in L3D is -Z (pointing down in Z-up)
        let default_dir = Vec3::NEG_Z;
        // Apply emitter rotation to get light direction in L3D space
        let l3d_dir = emitter_rot * default_dir;
        // Convert to Bevy coordinate system (Z-up to Y-up): -Z becomes -Y (down)
        let bevy_dir = *z_to_y_rotation * l3d_dir;

        // Calculate a target point for the spotlight to look at
        let target = world_pos + bevy_dir * 10.0;

        // Get size info from shape for light radius
        let (light_radius, shape_desc) = match &emitter.shape {
            EmitterShape::Rectangle { width, height } => {
                (width.max(*height) / 2.0, format!("rect {:.0}x{:.0}mm", width * 1000.0, height * 1000.0))
            }
            EmitterShape::Circle { diameter } => {
                (diameter / 2.0, format!("circle ø{:.0}mm", diameter * 1000.0))
            }
            EmitterShape::Unknown => (0.05, "unknown".to_string()),
        };

        log(&format!("[L3D] *** SPAWN '{}': {:.0} lm, {}K, emerg={}, {}, pos({:.3},{:.3},{:.3}), dir({:.2},{:.2},{:.2})",
            emitter.name,
            final_flux,
            config.and_then(|c| c.color_temperature).unwrap_or(0),
            is_emergency_only,
            shape_desc,
            world_pos.x, world_pos.y, world_pos.z,
            bevy_dir.x, bevy_dir.y, bevy_dir.z));

        // Use SpotLight for main directional lighting (pointing down)
        commands.spawn((
            SpotLight {
                color,
                intensity,
                range: 20.0,
                radius: light_radius,
                inner_angle: 0.5, // ~30 degrees
                outer_angle: 1.2, // ~70 degrees
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(world_pos).looking_at(target, Vec3::X),
            L3dLight,
        ));

        // Add PointLight for ambient fill (illuminates the luminaire body too)
        commands.spawn((
            PointLight {
                color,
                intensity: intensity * 0.3, // 30% as ambient fill
                range: 5.0,
                radius: 0.02,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(world_pos),
            L3dLight,
        ));

        // Add emissive mesh to make the light source visible - shape matches LEO definition
        let linear_color = color.to_linear();
        let emissive_material = materials.add(StandardMaterial {
            base_color: color,
            emissive: LinearRgba::new(
                linear_color.red * 10.0,
                linear_color.green * 10.0,
                linear_color.blue * 10.0,
                1.0,
            ),
            unlit: true, // Don't receive lighting, just emit
            ..default()
        });

        // Position slightly in light direction to be visible from below
        let glow_pos = world_pos + bevy_dir * 0.02;

        // Create mesh based on shape - Rectangle or Circle
        let emissive_mesh: Mesh = match &emitter.shape {
            EmitterShape::Rectangle { width, height } => {
                // Use half-extents for Rectangle mesh, with minimum size
                let half_w = (width / 2.0).max(0.015);
                let half_h = (height / 2.0).max(0.015);
                Rectangle::new(half_w * 2.0, half_h * 2.0).into()
            }
            EmitterShape::Circle { diameter } => {
                let radius = (diameter / 2.0).max(0.03);
                Circle::new(radius).into()
            }
            EmitterShape::Unknown => {
                // Default to small circle
                Circle::new(0.03).into()
            }
        };

        // Orient the emissive mesh to face downward while staying aligned with luminaire
        // For ceiling-mounted luminaires, we use the same Z-to-Y rotation as the body
        // This keeps the emissive rectangle aligned with the luminaire body
        commands.spawn((
            Mesh3d(meshes.add(emissive_mesh)),
            MeshMaterial3d(emissive_material),
            Transform {
                translation: glow_pos,
                rotation: *z_to_y_rotation, // Same rotation as luminaire body
                scale: Vec3::ONE,
            },
            L3dLight,
        ));
    }
}

/// Parse color temperature from LDT color appearance string
fn parse_color_temperature(color_appearance: &str) -> f32 {
    // Try to parse as a number (e.g., "4000" or "4000K")
    let cleaned = color_appearance
        .trim()
        .trim_end_matches('K')
        .trim_end_matches('k');
    cleaned.parse().unwrap_or(4000.0)
}

/// Convert color temperature (Kelvin) to RGB color
#[allow(clippy::excessive_precision)]
fn kelvin_to_rgb(kelvin: f32) -> Color {
    let temp = kelvin / 100.0;

    let r = if temp <= 66.0 {
        1.0
    } else {
        let r = 329.698727446 * (temp - 60.0).powf(-0.1332047592);
        (r / 255.0).clamp(0.0, 1.0)
    };

    let g = if temp <= 66.0 {
        let g = 99.4708025861 * temp.ln() - 161.1195681661;
        (g / 255.0).clamp(0.0, 1.0)
    } else {
        let g = 288.1221695283 * (temp - 60.0).powf(-0.0755148492);
        (g / 255.0).clamp(0.0, 1.0)
    };

    let b = if temp >= 66.0 {
        1.0
    } else if temp <= 19.0 {
        0.0
    } else {
        let b = 138.5177312231 * (temp - 10.0).ln() - 305.0447927307;
        (b / 255.0).clamp(0.0, 1.0)
    };

    Color::srgb(r, g, b)
}

/// Calculate beam angle from LDT photometric data
/// Beam angle is defined as the angle where intensity drops to 50% of maximum
#[allow(dead_code)]
fn calculate_beam_angle(ldt: &eulumdat::Eulumdat) -> f32 {
    let max_intensity = ldt.max_intensity();
    if max_intensity <= 0.0 {
        return std::f32::consts::FRAC_PI_4; // Default 45 degrees
    }

    let half_max = max_intensity * 0.5;

    // Find the gamma angle where intensity drops to 50%
    for g in 0..90 {
        let intensity = ldt.sample(0.0, g as f64);
        if intensity < half_max {
            return (g as f32).to_radians();
        }
    }

    std::f32::consts::FRAC_PI_2 // 90 degrees if not found
}
