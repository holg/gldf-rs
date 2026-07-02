//! L3D Model Loader Plugin for Bevy
//!
//! Loads L3D 3D models (OBJ format) into Bevy meshes with proper transformations.
//! Extracts Light Emitting Surfaces (LES) and places photometric lights at those positions.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::{log, EmitterConfig, GldfSceneData, SceneSettings};

/// Plugin for loading L3D models into the Bevy scene
pub struct L3dLoaderPlugin;

impl Plugin for L3dLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShowLightCones>()
            .add_systems(Startup, setup_l3d_model)
            .add_systems(Update, update_l3d_model)
            .add_systems(Update, apply_light_cone_visibility);
    }
}

/// Component to mark L3D model entities
#[derive(Component)]
pub struct L3dModel;

/// Component to mark lights spawned from L3D light emitting objects
#[derive(Component)]
pub struct L3dLight;

/// Component to mark the translucent "light cone" debug visualisation
/// spawned alongside each spotlight. Toggleable at runtime via the
/// `K` keybind (see `ui_controls_system`) and the `ShowLightCones`
/// resource — the cones are debug aids for the `(LUMINAIRE-TILT …)`
/// Lisp primitive, NOT a realistic render of the light beam, so they
/// default to OFF. With them on a fully-emissive 60 cm cone hangs
/// below each LEO and dominates an otherwise-dark scene.
#[derive(Component)]
pub struct L3dLightCone;

/// Runtime flag that drives `apply_light_cone_visibility`. Default
/// `false` so a clean load shows physically realistic spotlight
/// illumination (lit floor, dark cup interior) instead of the debug
/// cones. Toggled by pressing `K`.
#[derive(Resource, Default, Debug)]
pub struct ShowLightCones(pub bool);

/// Bevy system: drive each `L3dLightCone` entity's `Visibility` from
/// the `ShowLightCones` resource. Wired in `L3dLoaderPlugin::build`.
/// Runs every frame — cheap (just iterates the cone tag) and means
/// the K-key toggle is reflected on the next render without explicit
/// change tracking.
pub fn apply_light_cone_visibility(
    show: Res<ShowLightCones>,
    mut q: Query<&mut Visibility, With<L3dLightCone>>,
) {
    let target = if show.0 {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut v in q.iter_mut() {
        if *v != target {
            *v = target;
        }
    }
}

/// Joint-name tag attached to every entity spawned for one
/// `LightEmitter` (the SpotLight, PointLight, and emissive disc all
/// share the same joint name). Lets the `(LUMINAIRE-TILT …)` Lisp
/// primitive locate "all the bits of LEO0" by partName.
///
/// The strings are L3D partNames, e.g. `"LEO0"`, `"LEO1"`, `"Sensor"`.
/// The Lisp engine prefixes those with `"l3d://"` when addressing them
/// from a script — we store just the bare names here so this crate
/// doesn't grow a URI-parsing dep.
///
/// Stored as `Vec<String>` (not `String`) because one head OBJ can hold
/// multiple LEOs — e.g. an articulated luminaire whose single `<geometry
/// partName="head-0">` contains both `LEO0` and `LEO1`. A tilt request
/// for either name needs to swing the same head mesh. The apply system
/// at `js_joint_bridge_apply` (`lib.rs:761`) iterates this list and
/// applies the first matching tilt — "first-LEO-wins" on conflicting
/// concurrent tilts (acceptable: two coincident LEOs sharing a head
/// almost always move together in practice).
#[derive(Component, Clone, Debug)]
pub struct LightEmitterJoint(pub Vec<String>);

/// Per-emitter tilt baked into the entity's `Transform` by the
/// `js_joint_bridge_apply` system. We keep the **base** (post-LEO-walk,
/// pre-script) rotation here so re-applying a different tilt doesn't
/// compound: the apply system re-derives `transform.rotation = base *
/// requested-tilt` each frame.
#[derive(Component, Clone, Debug)]
pub struct LightEmitterBaseRotation(pub Quat);

/// Pivot point (= LEO world position) AND the entity's *base* translation
/// — both stored so the apply system can orbit the entity around the
/// LEO instead of rotating in place. Without this, the cone rotates
/// around its own centre and barely moves; with it, the cone (offset
/// half-length away from the LEO at spawn) swings about the LEO like a
/// real gimbal.
#[derive(Component, Clone, Debug)]
pub struct LightEmitterPivot {
    pub pivot: Vec3,
    pub base_translation: Vec3,
}

/// Extracted light emitting object info with accumulated world transform
struct LightEmitter {
    /// World position (accumulated from parent joints)
    world_position: Vec3,
    /// World rotation (accumulated from parent joints)
    world_rotation: Vec3,
    /// Size of the emitting area (width, height) for rectangles, or (diameter, diameter) for circles
    size: (f32, f32),
    /// Name for debugging
    name: String,
    /// L3D `partName` of the `<Geometry>` node that directly contains this
    /// LEO/Sensor in the structure tree. Used to match the LEO against
    /// the head OBJ that physically houses it: any spawned mesh whose
    /// owning `<Geometry>` shares this `part_name` gets the LEO's
    /// `LightEmitterJoint` tag so it tilts together with the LEO.
    parent_geometry_partname: String,
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

/// Update L3D model when data or scene settings change
fn update_l3d_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    scene_data: Res<GldfSceneData>,
    settings: Res<SceneSettings>,
    existing_models: Query<Entity, With<L3dModel>>,
    existing_lights: Query<Entity, With<L3dLight>>,
) {
    // Respawn when L3D data changes OR when scene type/settings change
    // (scene type affects mounting position)
    if !scene_data.is_changed() && !settings.is_changed() {
        return;
    }
    // Skip if no L3D data loaded
    if scene_data.l3d_data.is_none() {
        return;
    }
    log(&format!(
        "[L3D] update_l3d_model triggered, has L3D: {}, emitters: {}, scene: {:?}",
        scene_data.l3d_data.is_some(),
        scene_data.emitter_config.len(),
        settings.scene_type
    ));

    // Remove existing models
    for entity in existing_models.iter() {
        commands.entity(entity).despawn();
    }

    // Remove existing L3D lights
    for entity in existing_lights.iter() {
        commands.entity(entity).despawn();
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

    // Default material for luminaire body. When the editor pushed a
    // housing-color override into localStorage (driven by the active
    // variant's `HousingColor.@ral` value), use it instead of light
    // grey — this is the renderer-side variant preview, independent
    // of any MTL inside the L3D archive itself.
    let (housing_base, housing_metallic) = match scene_data.housing_color_rgb {
        Some([r, g, b]) => {
            log(&format!(
                "[L3D] Applying housing-color override: ({:.3}, {:.3}, {:.3})",
                r, g, b
            ));
            // Coloured housings are painted, so back off the metallic
            // factor — high-metallic samples mute strong colours into
            // grey. Painted/anodised RAL surfaces sit around 0.05–0.1.
            (Color::srgb(r, g, b), 0.05_f32)
        }
        None => (Color::srgb(0.9, 0.9, 0.9), 0.3_f32),
    };
    let luminaire_material = materials.add(StandardMaterial {
        base_color: housing_base,
        metallic: housing_metallic,
        perceptual_roughness: 0.5,
        ..default()
    });

    // Rotation matrix: L3D uses Z-up, Bevy uses Y-up
    let z_to_y_rotation = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);

    // (No `ceiling_flip` — the single `z_to_y_rotation = Rot_X(-90°)`
    // already lands L3D's mount-plate at Bevy's +Y. l3d-egui (canonical
    // Rust L3D viewer) does the same with no extra rotation.)

    // Position luminaire based on scene type and mounting configuration
    let is_room = matches!(settings.scene_type, eulumdat_bevy::SceneType::Room);
    let (luminaire_y, is_ceiling) = if scene_data.mounting.mounting_type.is_some() {
        // Use GLDF mounting info
        let (y, is_ceil) = scene_data.mounting.get_y_position(settings.room_height);
        log(&format!(
            "[L3D] Using GLDF mounting: type={:?}, recessed={:?}mm, pendant={:?}mm, pole={:?}mm, wall_h={:?}mm",
            scene_data.mounting.mounting_type,
            scene_data.mounting.recessed_depth_mm,
            scene_data.mounting.pendant_length_mm,
            scene_data.mounting.pole_height_mm,
            scene_data.mounting.mounting_height_mm,
        ));
        if is_room {
            // Room scene: clamp to room height so luminaire stays visible
            let clamped_y = y.min(settings.room_height - 0.05);
            if (clamped_y - y).abs() > 0.01 {
                log(&format!(
                    "[L3D] Room: clamped luminaire Y from {:.2}m to {:.2}m (room height {:.2}m)",
                    y, clamped_y, settings.room_height
                ));
            }
            (clamped_y, is_ceil)
        } else {
            // Outdoor scenes (Road, Parking, Outdoor): mount on pole at scene mounting height
            // The pole is built at settings.mounting_height, so place luminaire there
            let pole_y = settings.mounting_height;
            log(&format!(
                "[L3D] Outdoor: placing luminaire at pole height {:.2}m (GLDF wanted {:.2}m)",
                pole_y, y
            ));
            (pole_y, false)
        }
    } else {
        // Fallback to scene type
        let y = match settings.scene_type {
            eulumdat_bevy::SceneType::Room => settings.room_height - settings.pendulum_length,
            _ => settings.mounting_height,
        };
        (
            y,
            matches!(settings.scene_type, eulumdat_bevy::SceneType::Room),
        )
    };

    // Position luminaire in the scene based on mounting type and scene type
    let (mount_x, mount_z) = if is_room {
        if scene_data.mounting.mounting_type.as_deref() == Some("wall") {
            // Room + wall mount: position against the wall
            (settings.room_width / 2.0, 0.1)
        } else {
            // Room + ceiling/other: center of room
            (settings.room_width / 2.0, settings.room_length / 2.0)
        }
    } else {
        // Outdoor scenes: center position (matches pole placement in Parking/Outdoor)
        (settings.room_width / 2.0, settings.room_length / 2.0)
    };

    let mounting_offset = Vec3::new(mount_x, luminaire_y, mount_z);

    log(&format!(
        "[L3D] Mounting offset: ({:.2}, {:.2}, {:.2}) scene={:?} ceiling={}",
        mounting_offset.x, mounting_offset.y, mounting_offset.z, settings.scene_type, is_ceiling
    ));

    // Parse the structure XML to get light emitting objects
    let mut light_emitters: Vec<LightEmitter> = Vec::new();
    // Same length and ordering as `l3d.model.parts`. Each entry is the
    // L3D `<Geometry partName>` whose `<GeometryReference>` produced the
    // matching `l3d.model.parts[i].path`. Lets the part-load loop look
    // up which structure-tree node owns each spawned mesh, so it can
    // tag the head OBJ with the LEOs that live inside it. Empty if the
    // structure XML failed to parse (we fall back to leaving meshes
    // untagged, just like before).
    let mut part_geom_partnames: Vec<String> = Vec::new();
    if let Ok(luminaire) = l3d_rs::Luminaire::from_xml(&l3d.file.structure) {
        // Start with identity matrix
        let identity = Mat4::IDENTITY;
        extract_light_emitters(&luminaire.structure.geometry, &mut light_emitters, identity);
        collect_part_geom_partnames(&luminaire.structure.geometry, &mut part_geom_partnames);
        log(&format!(
            "[L3D] Found {} light emitting objects",
            light_emitters.len()
        ));
        log(&format!(
            "[L3D] Collected {} geometry partNames in part order (vs {} parts in model)",
            part_geom_partnames.len(),
            l3d.model.parts.len()
        ));
    } else {
        log("[L3D] Failed to parse structure XML for light emitting objects");
    }

    // Load each part
    let mut loaded_count = 0;
    for (part_idx, part) in l3d.model.parts.iter().enumerate() {
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

        // Apply ONLY the Z-up → Y-up coordinate conversion. This
        // mirrors `l3d-egui` (the canonical Rust L3D viewer), whose
        // load path is literally `mat = Rot_X(-90°) * part_mat`
        // (`crates/l3d-egui/src/viewer.rs:882-910`). No `ceiling_flip`,
        // no `is_ceiling` branch, no axis negation.
        //
        // Why: L3D archives store geometry **Z-up** with the
        // mount-plate at +Z. A single `Rot_X(-90°)` maps +Z onto Bevy's
        // +Y, so the mount-plate naturally lands at the top and the
        // housing renders the right way up for a ceiling fixture.
        // The historical extra `ceiling_flip = Rot_X(180°)` we used to
        // multiply on top double-rotated the housing — gimbal cups
        // pointing UP — which is why the user saw upside-down luminaires.
        // See `feedback_use_typed_apis` / `reference_light_other_rs_helpers`
        // memory for the cross-reference work that confirmed this.
        let final_rotation = z_to_y_rotation * rotation;
        let final_translation = z_to_y_rotation * translation;

        // Tag this part with a `LightEmitterJoint` carrying every LEO
        // that lives in the same L3D `<Geometry>` node. This is what
        // makes the gimbal head OBJ rotate when the Lisp
        // `LUMINAIRE-TILT` primitive fires — without this tag, the
        // SpotLight + cone swing but the physical head stays welded to
        // the housing, breaking the illusion.
        //
        // Matching is **structural**, not by file path. We look up the
        // L3D `<Geometry partName>` that owns this OBJ instance via
        // `part_geom_partnames` (populated in walker order to match
        // `l3d.model.parts`), then collect every LEO whose
        // `parent_geometry_partname` equals it.
        //
        // Why not substring-match `part.path`? Because real L3D files
        // name their OBJs generically (e.g. `geom_3/head.obj`) — the
        // LEO partName lives in the structure XML, not the asset path.
        // The old substring rule silently failed on every Tria-2-style
        // file: `head.obj` contains no `"LEO0"` substring, so the head
        // mesh stayed untagged and only the lit cone tilted.
        //
        // Skip "Sensor" matches: a SensorObject mounts the emergency
        // LED, not a gimbal; tilting the housing geometry for "Sensor"
        // would yank the wrong piece. SensorObjects also don't have
        // their own visible meshes typically — the sensor is part of
        // the housing.
        let owning_geom = part_geom_partnames.get(part_idx);
        let matched_leos: Vec<&LightEmitter> = owning_geom
            .map(|g| {
                light_emitters
                    .iter()
                    .filter(|e| {
                        !e.name.eq_ignore_ascii_case("Sensor")
                            && e.parent_geometry_partname.eq_ignore_ascii_case(g)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let world_translation = final_translation + mounting_offset;
        if let Some(first_leo) = matched_leos.first() {
            // Pivot = the first LEO's world position. Multiple LEOs
            // sharing one head all swing about the same pivot — the
            // head's centre — so picking any LEO's position works in
            // practice (they cluster). Matches what
            // `spawn_lights_from_emitters` uses for the SpotLight, so
            // mesh + light swing about the same point.
            //
            // L3D positions are in meters (per spec + l3d_rs convention);
            // no `/ 1000.0` scale — see comment in spawn_lights_from_emitters.
            let leo_local_pos = z_to_y_rotation * first_leo.world_position;
            let leo_world_pos = leo_local_pos + mounting_offset;
            let leo_names: Vec<String> =
                matched_leos.iter().map(|e| e.name.clone()).collect();
            commands.spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(luminaire_material.clone()),
                Transform {
                    translation: world_translation,
                    rotation: final_rotation,
                    scale,
                },
                L3dModel,
                LightEmitterJoint(leo_names),
                LightEmitterBaseRotation(final_rotation),
                LightEmitterPivot {
                    pivot: leo_world_pos,
                    base_translation: world_translation,
                },
            ));
        } else {
            commands.spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(luminaire_material.clone()),
                Transform {
                    translation: world_translation,
                    rotation: final_rotation,
                    scale,
                },
                L3dModel,
            ));
        }
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
        is_ceiling,
        settings,
        &scene_data.emitter_config,
    );
}

/// Convert l3d_rs Mat4 ([f32; 16] column-major) to Bevy Mat4
fn l3d_mat_to_bevy(m: &l3d_rs::Mat4) -> Mat4 {
    Mat4::from_cols_array(m)
}

/// Walk the L3D structure tree and push each `<Geometry partName>` into
/// `out` in **the same order** that `l3d_rs::parse_geometry` populates
/// `L3dModel.parts`. Mirrors the upstream walker at
/// `../l3d-rs/crates/l3d_rs/src/lib.rs:484-514`: recurse into joints'
/// child geometries first (post-order), then push this node's partName.
///
/// Result: `out[i]` is the L3D `<Geometry partName>` whose
/// `<GeometryReference>` produced `l3d.model.parts[i].path`. The
/// part-load loop reads `out[i]` to learn which structure node owns
/// the i-th spawned mesh.
fn collect_part_geom_partnames(geometry: &l3d_rs::Geometry, out: &mut Vec<String>) {
    if let Some(j) = &geometry.joints {
        for joint in &j.joint {
            for child_geom in &joint.geometries.geometry {
                collect_part_geom_partnames(child_geom, out);
            }
        }
    }
    out.push(geometry.part_name.clone());
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

            // Get size from rectangle or circle
            let size = if let Some(rect) = obj.rectangle() {
                let (w, h) = rect.size();
                (w as f32, h as f32)
            } else if let Some(circle) = obj.circle() {
                let d = circle.diameter() as f32;
                (d, d)
            } else {
                (0.1, 0.1) // Default small size
            };

            log(&format!(
                "[L3D] *** EMITTER '{}': world({:.3},{:.3},{:.3}), size({:.3},{:.3})",
                obj.part_name(),
                world_pos.x,
                world_pos.y,
                world_pos.z,
                size.0,
                size.1
            ));

            emitters.push(LightEmitter {
                world_position: world_pos,
                world_rotation: world_rot,
                size,
                name: obj.part_name().to_string(),
                parent_geometry_partname: geometry.part_name.clone(),
            });
        }
    }

    // SensorObjects → also treat as candidate light mounts. The L3D
    // schema models SensorObjects (presence sensors, dimmers, …) as
    // partName-pinned anchors with Position+Rotation only — no shape,
    // no LuminousHeights. We push them into the same emitter list
    // because the downstream config-match (`target_name() ==
    // emitter.name`) is what decides whether a light spawns; if no
    // variant config targets this partName the entry is harmless.
    // Practical use: an integrator pins a small "Sensor" partName at
    // the centre of a fixture and routes the emergency LED through
    // that mount point.
    if let Some(sensors) = &geometry.sensor_objects {
        log(&format!(
            "[L3D] Found SensorObjects with {} objects",
            sensors.objects().len()
        ));
        for obj in sensors.objects() {
            let (px, py, pz): (f32, f32, f32) = obj.position();
            let (rx, ry, rz): (f32, f32, f32) = obj.rotation();

            let sens_pos = l3d_rs::Vec3f {
                x: px,
                y: py,
                z: pz,
            };
            let sens_rot = l3d_rs::Vec3f {
                x: rx,
                y: ry,
                z: rz,
            };
            let sens_matrix_raw = l3d_rs::build_transform(&sens_pos, &sens_rot);
            let sens_matrix = l3d_mat_to_bevy(&sens_matrix_raw);

            let final_matrix = current_matrix * sens_matrix;
            let world_pos = Vec3::new(
                final_matrix.w_axis.x,
                final_matrix.w_axis.y,
                final_matrix.w_axis.z,
            );
            let world_rot = Vec3::new(rx, ry, rz);

            // Default a tiny size — SensorObjects have no native shape.
            // Used only when a light actually spawns here and the
            // photometry doesn't override the disc size.
            let size = (0.05f32, 0.05f32);

            log(&format!(
                "[L3D] *** SENSOR EMITTER '{}': world({:.3},{:.3},{:.3})",
                obj.part_name(),
                world_pos.x,
                world_pos.y,
                world_pos.z,
            ));

            emitters.push(LightEmitter {
                world_position: world_pos,
                world_rotation: world_rot,
                size,
                name: obj.part_name().to_string(),
                parent_geometry_partname: geometry.part_name.clone(),
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
    // `is_ceiling` used to gate a `bevy_dir = -|y|` force-down; now the
    // orientation module returns the spec-correct direction directly,
    // so the flag is unused inside this fn. Kept in the signature so
    // future per-scene-type tweaks (e.g. floor uplights) have a hook
    // without an API churn.
    _is_ceiling: bool,
    settings: &SceneSettings,
    emitter_configs: &[EmitterConfig],
) {
    if emitters.is_empty() {
        log("[L3D] No light emitters found in L3D structure");
        return;
    }

    // Get default light properties from LDT data. eulumdat-bevy renamed
    // `ldt_data` → `ldc_data` (format-agnostic LDC = light distribution
    // curve); the binding still points at an `Eulumdat`.
    let (default_color, default_flux) = if let Some(ref ldt) = settings.ldc_data {
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
            "[L3D] Config target_name: '{}' (leo_name: '{}', external_name: {:?}), flux: {:?}, temp: {:?}",
            c.target_name(),
            c.leo_name,
            c.external_name,
            c.luminous_flux,
            c.color_temperature
        ));
    }

    for emitter in emitters {
        // Look up per-emitter configuration by partName. `target_name()`
        // prefers the new `external_name` field when present (so a config
        // can target a SensorObject's partName like "Sensor"), falling
        // back to `leo_name` for older configs. The match is the same
        // either way — `emitter.name` is the L3D partName, whether it
        // came from `<LightEmittingObject>` or `<SensorObject>`.
        let config = emitter_configs
            .iter()
            .find(|c| c.target_name() == emitter.name);
        log(&format!(
            "[L3D] Matching '{}' -> config found: {}",
            emitter.name,
            config.is_some()
        ));

        // Skip Sensor emitters when the active variant doesn't bind them.
        // A SensorObject is a partName-anchor for an OPTIONAL emergency
        // LED — only the dedicated emergency variant binds it. Without
        // this skip, every non-emergency variant gets a free 1/N share
        // of the default flux at the Sensor position, showing up as a
        // dim dot in the middle of the luminaire. That's physically
        // wrong: the sensor mount is dark unless the variant routes a
        // light through it.
        if config.is_none() && emitter.name.eq_ignore_ascii_case("Sensor") {
            log(&format!(
                "[L3D] Skipping unbound Sensor emitter '{}' — no config in this variant",
                emitter.name
            ));
            continue;
        }

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

        // Scale intensity to Bevy candela — cap to avoid WebGL issues
        let intensity = (final_flux * 50.0).min(2_000_000.0);

        // Transform position from L3D (Z-up, meters) to Bevy (Y-up,
        // meters). Per the L3D spec — and confirmed by l3d_rs's
        // `fbx.rs` ("L3D coordinates are in meters after unit
        // scaling") — structure positions are already in meters; the
        // earlier `/ 1000.0` shrank them 1000× and made multi-emitter
        // luminaires (SLV Tria 2 etc.) converge to a single sub-mm
        // point at the origin, looking like one tiny dot.
        let local_pos = *z_to_y_rotation * emitter.world_position;
        let world_pos = local_pos + *mounting_offset;

        // Light range: must reach the ground from mounting height, plus some margin
        let light_range = (mounting_offset.y * 3.0).max(30.0);

        // Compute Bevy-space emission direction via the orientation
        // module — single source of truth, tested in
        // `crates/gldf-bevy/src/orientation.rs`. For a zero-rotation
        // L3D emitter this produces `(0, -1, 0)`: straight down.
        //
        // No `is_ceiling` clamp on the Y component: if a LEO is
        // mis-authored to point UP, we render it pointing UP. That's
        // a data bug in the L3D, not something a viewer should silently
        // mask — the user should see the error and fix the file.
        let bevy_dir = crate::orientation::l3d_emitter_bevy_direction(emitter.world_rotation);

        // Calculate a target point for the spotlight to look at
        let target = world_pos + bevy_dir * 10.0;

        log(&format!("[L3D] *** SPAWN '{}': {:.0} lm, {}K, emerg={}, pos({:.3},{:.3},{:.3}), dir({:.2},{:.2},{:.2}), range={:.1}m",
            emitter.name,
            final_flux,
            config.and_then(|c| c.color_temperature).unwrap_or(0),
            is_emergency_only,
            world_pos.x, world_pos.y, world_pos.z,
            bevy_dir.x, bevy_dir.y, bevy_dir.z,
            light_range));

        // Build the SpotLight base transform once, capture its rotation
        // so the `(LUMINAIRE-TILT …)` Lisp primitive can re-apply the
        // base+tilt cleanly on every script invocation (otherwise tilts
        // would compound across calls).
        let spot_transform = Transform::from_translation(world_pos).looking_at(target, Vec3::X);
        let spot_base_rot = spot_transform.rotation;

        // Use SpotLight for main directional lighting (pointing down)
        commands.spawn((
            SpotLight {
                color,
                intensity,
                range: light_range,
                radius: emitter.size.0.max(emitter.size.1) / 2.0,
                inner_angle: 0.5, // ~30 degrees
                outer_angle: 1.2, // ~70 degrees
                shadow_maps_enabled: false,
                ..default()
            },
            spot_transform,
            L3dLight,
            LightEmitterJoint(vec![emitter.name.clone()]),
            LightEmitterBaseRotation(spot_base_rot),
            // SpotLight pivots in place — translation === pivot ===
            // base_translation. Orbit math is a no-op here, but we
            // attach the component so a single apply-system query
            // can rotate everything together.
            LightEmitterPivot {
                pivot: world_pos,
                base_translation: world_pos,
            },
        ));

        // Translucent visible "light cone" so the user can actually SEE
        // the spotlight's direction (and watch it swing when the Lisp
        // tilt primitive fires). Without this the SpotLight is invisible
        // — only the ground/objects it hits show its presence, and on a
        // mostly-empty stage that's not enough.
        //
        // PIVOT-AROUND-APEX geometry:
        //   parent entity (the "joint")
        //     translation = world_pos (= LEO position, = cone apex)
        //     rotation    = base_rot (maps +Y down the light dir)
        //     LightEmitterJoint + LightEmitterBaseRotation tags here
        //   `-- child entity (the actual cone mesh)
        //         local translation = (0, -cone_len/2, 0)
        //                            ⇒ in parent's local +Y is the
        //                              apex direction, so -Y/2 puts
        //                              the cone's CENTRE half a height
        //                              behind the apex — apex sits at
        //                              parent's origin = world_pos
        //
        // When `js_joint_bridge_apply` rotates the parent by
        // `base * tilt`, the cone child orbits around `world_pos`
        // (the LEO position), not around the cone's middle. The apex
        // stays anchored at the LEO position — exactly what a real
        // gimballed spotlight does.
        // Build the visible cone. Made BIG and BRIGHT for the demo so
        // the tilt animation reads unambiguously: 1 m long, narrow
        // base (~25cm radius for a ~30° half-angle), high emissive so
        // it's visible even with the canvas dark. Slight alpha blend
        // so you can still see through it to the housing behind.
        // Skip the cone for SensorObject / emergency-only emitters —
        // those are sensors / emergency LEDs, not directional spots,
        // so a visible cone is misleading (and ugly). Detected via:
        //   - partName "Sensor" (case-insensitive) — covers the
        //     SensorObject mount in stock SLV / multi-emitter L3Ds.
        //   - emergency_behavior == "EmergencyOnly" on the config.
        // Both checks because not every emergency-only emitter is a
        // SensorObject, and not every SensorObject is wired emergency.
        let is_sensor_or_emergency = emitter.name.eq_ignore_ascii_case("Sensor")
            || is_emergency_only;
        if is_sensor_or_emergency {
            // Skip cone spawn for this emitter; the SpotLight + glow
            // disc spawned above already represent the emergency LED.
            // The remaining PointLight + glow-disc spawn calls below
            // continue, since they're correct for sensors too.
            //
            // (The early-`continue` would skip those, hence the if/else
            //  rather than a continue.)
        } else {
        // Translucent visible spotlight cone — readable without
        // dominating the scene. 60 cm long, modest 18 cm base radius
        // (~17° half-angle), low-alpha emissive in the LEO's CCT
        // color. Visible against both light and dark backgrounds.
        let cone_len = 0.60_f32;
        let cone_base_radius = 0.18_f32;
        let cone_mesh = meshes.add(Cone::new(cone_base_radius, cone_len));
        let cone_color = color;
        let cone_linear = cone_color.to_linear();
        let cone_material = materials.add(StandardMaterial {
            // Demo-tuned (post task #197 rebalance): the previous
            // alpha=0.25 + 3× emissive boost over-bloomed in
            // recordings — readable but dominating. alpha=0.15 +
            // 1.2× emissive keeps the cone visible against both
            // light and dark backgrounds without halo'ing the
            // luminaire.
            base_color: cone_color.with_alpha(0.15),
            emissive: LinearRgba::new(
                cone_linear.red * 1.2,
                cone_linear.green * 1.2,
                cone_linear.blue * 1.2,
                1.0,
            ),
            alpha_mode: AlphaMode::Blend,
            // Default backface culling so the inside (apex-facing) of
            // the cone doesn't emit into the ceiling cavity. Removing
            // `cull_mode: None` + `double_sided: true` was the
            // remaining upward leak — both sides previously glowed.
            unlit: true,
            ..default()
        });
        // The parent is a pure pivot (identity rotation, position at
        // LEO). The cone child carries BOTH the cone's mesh-aligning
        // rotation AND the half-height offset that puts the apex on
        // the pivot. Then when the tilt animation rotates the parent,
        // the child orbits around the LEO position — the apex stays
        // pinned, exactly like a real gimbal.
        //
        // Cone-mesh local axes (Bevy convention):
        //   +Y/2 = apex
        //   -Y/2 = base
        // World goal:
        //   apex at world_pos (LEO)
        //   base at world_pos + bevy_dir * cone_len
        // ⇒ the cone-local +Y direction must map to -bevy_dir
        //   (because the apex is "back" along the light path from the
        //    base). So q_cone = from_rotation_arc(+Y, -bevy_dir).
        //   The cone's CENTRE in world is then world_pos + bevy_dir *
        //   (cone_len/2). Since the parent is at world_pos with
        //   identity rotation, the child's local translation is just
        //   bevy_dir * (cone_len/2) in world coordinates.
        let bevy_dir_n = bevy_dir.normalize_or_zero();
        let cone_local_rot = Quat::from_rotation_arc(Vec3::Y, -bevy_dir_n);
        let cone_centre_offset = bevy_dir_n * (cone_len / 2.0);
        // Compose the cone's final world transform inline rather than
        // using a parent/child hierarchy. Reason: in Bevy 0.19 a child
        // entity needs the full visibility bundle
        // (Visibility/InheritedVisibility/ViewVisibility) to inherit
        // updates from its parent, AND the parent's transform updates
        // only reach the child after one frame of propagation. The
        // child cone's world rotation didn't update reliably — the
        // tilt wrote to the parent's Transform but the cone mesh stayed
        // put for the user. Spawning a single mesh entity with the
        // pre-composed transform sidesteps the hierarchy entirely:
        // `js_joint_bridge_apply` writes directly to this entity's
        // Transform, no propagation needed.
        //
        // For apex-pinning we redo the math: rotate THIS entity by
        // base * tilt around its translation = world_pos. Without
        // child-offset trickery the cone's CENTRE sits at world_pos,
        // not its apex — so it rotates around its centre and the apex
        // swings off. Compromise: shift the mesh inside the entity by
        // pushing the centre of the cone away from world_pos along
        // bevy_dir at spawn time. Since the apply rotation is applied
        // around translation (world_pos), the cone's apex stays
        // approximately at world_pos for small tilts because the apex
        // is on the rotation axis through `(world_pos, world_pos +
        // bevy_dir)`.
        //
        // For larger tilts the apex drifts, but for the ±35° we use
        // in the demo the drift is < 5cm and reads correctly as
        // "spotlight swinging."
        let cone_centre = world_pos + cone_centre_offset;
        commands.spawn((
            Mesh3d(cone_mesh),
            MeshMaterial3d(cone_material),
            Transform {
                translation: cone_centre,
                rotation: cone_local_rot,
                scale: Vec3::ONE,
            },
            L3dLight,
            // L3dLightCone tag + Hidden default → cones are invisible
            // unless the user presses `K` to toggle the debug overlay.
            // Without this gating the 60 cm emissive cone hangs below
            // every LEO and dominates the scene (e.g. on the SLV Tria
            // 2 fixture it looks like a glowing pole stuck to the
            // bottom of the luminaire). The Lisp tilt animation still
            // tags the cone via LightEmitterJoint so the tilt math
            // applies whether or not it's visible.
            L3dLightCone,
            Visibility::Hidden,
            LightEmitterJoint(vec![emitter.name.clone()]),
            // Capture the rest rotation so the apply system's
            // `base * tilt` math doesn't drift away from the original
            // direction when tilt is zero.
            LightEmitterBaseRotation(cone_local_rot),
            // Orbit the cone around the LEO position — without this
            // the cone rotates around its OWN centre (50cm below the
            // LEO) and barely moves visually. With the pivot at the
            // LEO and base_translation at the cone's centre, the apply
            // system swings the cone about the LEO like a gimballed
            // spotlight pointing in a new direction.
            LightEmitterPivot {
                pivot: world_pos,
                base_translation: cone_centre,
            },
        ));
        } // end else (non-sensor / non-emergency: spawn cone)

        // "Rim-glow" PointLight at the LED face.
        //
        // Bevy has no radiosity / GI bounce — so the cup walls and
        // bezel rim around each LED get NO light from the SpotLight
        // (which only illuminates whatever its cone hits below). In
        // reality the LED emits into a reflective cup whose walls
        // re-radiate light over the surrounding bezel; without
        // simulating that, the housing right around each LED is
        // pitch black and looks like "magic light" from nowhere.
        //
        // A short-range PointLight at the LED position fakes that
        // bounce: bright over the immediate cup walls, falling off
        // before it reaches the housing's outer top plate.
        //
        // Range: 0.30 m — covers the cup and the bezel ring around
        // it, falls off well before reaching the ceiling cavity above
        // the housing. The L3D mount plate is much further from the
        // LEO than 30 cm so no spurious top-plate illumination.
        //
        // Intensity: 20 % of the SpotLight — enough to clearly read
        // "this is the light source" without competing with the
        // SpotLight's floor pool.
        commands.spawn((
            PointLight {
                color,
                intensity: intensity * 0.20,
                range: 0.30,
                radius: emitter.size.0.max(emitter.size.1) / 2.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(world_pos),
            L3dLight,
            LightEmitterJoint(vec![emitter.name.clone()]),
            LightEmitterBaseRotation(Quat::IDENTITY),
            LightEmitterPivot {
                pivot: world_pos,
                base_translation: world_pos,
            },
        ));

        // Render the LED face EXACTLY as the L3D structure.xml
        // declares it — nothing added, nothing rounded, nothing
        // offset. The spec gives us:
        //   - position: world_pos (from joint accumulation)
        //   - rotation: emitter.world_rotation (Euler XYZ degrees, L3D)
        //   - shape:    Circle(diameter) or Rectangle(w, h)
        //
        // Build the disc transform from those three pieces and stop.
        // No `looking_at`, no 2 cm forward offset, no 3 cm minimum
        // size, no emissive multiplier — those were all patches for
        // earlier bugs (positions /1000'd to 0, wrong orientation
        // pipeline) and they distort the result.
        //
        // For a Bevy `Circle` primitive, the disc lies in the local
        // XY plane with normal along **+Z local**. The L3D LED has
        // its emission axis along **-Z L3D** by spec. So in L3D-local
        // frame, the disc is rotated π around X to make its +Z (the
        // primitive's normal) align with -Z L3D (the emission axis).
        // Then apply the LEO's own Euler rotation, then the Z-up→Y-up
        // coordinate conversion.
        //
        // Composing right-to-left in world-space matrix terms:
        //   disc_world = R_Zup→Yup · R_LEO · R_X(π)
        let disc_align = Quat::from_rotation_x(std::f32::consts::PI);
        let leo_rot = Quat::from_euler(
            EulerRot::XYZ,
            emitter.world_rotation.x.to_radians(),
            emitter.world_rotation.y.to_radians(),
            emitter.world_rotation.z.to_radians(),
        );
        let disc_rot = *z_to_y_rotation * leo_rot * disc_align;

        // Sizes from spec. Rectangle uses width × height; Circle uses
        // diameter (we receive `(d, d)` from the walker). Pick the
        // half-extent as radius — accurate for both.
        let emitter_radius = emitter.size.0.max(emitter.size.1) / 2.0;

        // Material: emissive at face value, unlit so the disc
        // self-renders at the LEO's CCT colour regardless of scene
        // lighting. Double-sided so a camera looking AT the back of
        // the disc still sees a glow (no hidden-face surprises).
        let linear_color = color.to_linear();
        // Demo-tuned (post task #197): full-strength emissive blew
        // out on the recordings — the LEO disc visually overpowered
        // the housing it sits in. 0.4× keeps the lamp clearly lit
        // but readable against the housing material.
        let leo_emissive_scale = 0.4_f32;
        let emissive_material = materials.add(StandardMaterial {
            base_color: color,
            emissive: LinearRgba::from_vec3(linear_color.to_vec3() * leo_emissive_scale),
            unlit: true,
            cull_mode: None,
            double_sided: true,
            ..default()
        });

        let glow_transform = Transform {
            translation: world_pos,
            rotation: disc_rot,
            scale: Vec3::ONE,
        };
        commands.spawn((
            Mesh3d(meshes.add(Circle::new(emitter_radius))),
            MeshMaterial3d(emissive_material),
            glow_transform,
            L3dLight,
            LightEmitterJoint(vec![emitter.name.clone()]),
            LightEmitterBaseRotation(disc_rot),
            LightEmitterPivot {
                pivot: world_pos,
                base_translation: world_pos,
            },
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

#[cfg(test)]
mod walker_tests {
    //! Pure tests for the structure-tree walkers. No Bevy `App`, no
    //! WASM bridge — just the parsing helpers above.
    //!
    //! These regression-guard the bug where the head OBJ failed to
    //! rotate with its LEO: the old substring-match expected the LEO
    //! partName in `part.path` ("geom_3/head.obj"), but real L3D files
    //! keep partNames in the structure XML only. The fix walks the
    //! structure tree to learn the parent `<Geometry partName>` for
    //! each LEO and for each part-load slot.

    use super::*;

    /// Tria-2-style L3D: two heads, one LEO each, both heads reuse the
    /// same `head.obj` asset via separate `<Geometry>` nodes.
    const TRIA_LIKE_XML: &str = r#"<Luminaire>
        <Header>
            <Name/>
            <CreatedWithApplication>walker_tests</CreatedWithApplication>
            <CreationTimeCode>2026-06-10T00:00:00Z</CreationTimeCode>
            <FormatVersion major="0" minor="11"/>
        </Header>
        <GeometryDefinitions>
            <GeometryFileDefinition id="geom_1" filename="base.obj" units="mm"/>
            <GeometryFileDefinition id="geom_3" filename="head.obj" units="mm"/>
        </GeometryDefinitions>
        <Structure>
            <Geometry partName="base">
                <Position x="0" y="0" z="0"/>
                <Rotation x="0" y="0" z="0"/>
                <GeometryReference geometryId="geom_1"/>
                <Joints>
                    <Joint partName="head-joint-0">
                        <Position x="-0.4" y="0" z="0.0375"/>
                        <Rotation x="0" y="0" z="0"/>
                        <XAxis min="-45" max="45" step="1"/>
                        <DefaultRotation x="0" y="0" z="0"/>
                        <Geometries>
                            <Geometry partName="head-0">
                                <Position x="0.4" y="0" z="-0.0375"/>
                                <Rotation x="0" y="0" z="0"/>
                                <GeometryReference geometryId="geom_3"/>
                                <LightEmittingObjects>
                                    <LightEmittingObject partName="LEO0">
                                        <Position x="-0.4" y="0" z="0.01"/>
                                        <Rotation x="0" y="0" z="90"/>
                                        <Circle diameter="0.0575"/>
                                        <LuminousHeights c0="10" c90="10" c180="10" c270="10"/>
                                    </LightEmittingObject>
                                </LightEmittingObjects>
                            </Geometry>
                        </Geometries>
                    </Joint>
                    <Joint partName="head-joint-1">
                        <Position x="0.4" y="0" z="0.0375"/>
                        <Rotation x="0" y="0" z="0"/>
                        <XAxis min="-45" max="45" step="1"/>
                        <DefaultRotation x="0" y="0" z="0"/>
                        <Geometries>
                            <Geometry partName="head-1">
                                <Position x="0.4" y="0" z="-0.0375"/>
                                <Rotation x="0" y="0" z="0"/>
                                <GeometryReference geometryId="geom_3"/>
                                <LightEmittingObjects>
                                    <LightEmittingObject partName="LEO1">
                                        <Position x="-0.4" y="0" z="0.01"/>
                                        <Rotation x="0" y="0" z="90"/>
                                        <Circle diameter="0.0575"/>
                                        <LuminousHeights c0="10" c90="10" c180="10" c270="10"/>
                                    </LightEmittingObject>
                                </LightEmittingObjects>
                            </Geometry>
                        </Geometries>
                    </Joint>
                </Joints>
            </Geometry>
        </Structure>
    </Luminaire>"#;

    #[test]
    fn each_leo_records_its_owning_geometry_partname() {
        let luminaire = l3d_rs::Luminaire::from_xml(TRIA_LIKE_XML)
            .expect("Tria-2-like XML must parse");
        let mut emitters = Vec::new();
        extract_light_emitters(&luminaire.structure.geometry, &mut emitters, Mat4::IDENTITY);

        assert_eq!(emitters.len(), 2, "must find LEO0 and LEO1");

        let leo0 = emitters.iter().find(|e| e.name == "LEO0").expect("LEO0 present");
        let leo1 = emitters.iter().find(|e| e.name == "LEO1").expect("LEO1 present");

        // The key regression guard: each LEO records the partName of
        // the <Geometry> node that directly contains it — not the
        // OBJ-file path it references.
        assert_eq!(
            leo0.parent_geometry_partname, "head-0",
            "LEO0 lives in <Geometry partName=head-0>"
        );
        assert_eq!(
            leo1.parent_geometry_partname, "head-1",
            "LEO1 lives in <Geometry partName=head-1>"
        );
    }

    #[test]
    fn part_geom_partnames_matches_part_walker_order() {
        // The collector must produce partNames in the SAME order as
        // l3d_rs::parse_geometry pushes into L3dModel.parts — i.e.
        // post-order. Tria-like tree:
        //   base
        //     joint head-joint-0 → head-0
        //     joint head-joint-1 → head-1
        // → post-order children push: head-0, head-1, then base.
        let luminaire = l3d_rs::Luminaire::from_xml(TRIA_LIKE_XML)
            .expect("Tria-2-like XML must parse");
        let mut names = Vec::new();
        collect_part_geom_partnames(&luminaire.structure.geometry, &mut names);
        assert_eq!(
            names,
            vec![
                "head-0".to_string(),
                "head-1".to_string(),
                "base".to_string(),
            ],
            "post-order traversal: children first, then self"
        );
    }

    #[test]
    fn shared_obj_path_disambiguated_by_geometry_partname() {
        // Both head-0 and head-1 reference geom_3 (head.obj). The OLD
        // substring-match would have failed completely (no "LEO0" in
        // "head.obj"). The NEW logic disambiguates by the structure-tree
        // partName: head-0 owns LEO0, head-1 owns LEO1, even though they
        // share the same OBJ asset.
        let luminaire = l3d_rs::Luminaire::from_xml(TRIA_LIKE_XML)
            .expect("Tria-2-like XML must parse");
        let mut emitters = Vec::new();
        extract_light_emitters(&luminaire.structure.geometry, &mut emitters, Mat4::IDENTITY);
        let mut names = Vec::new();
        collect_part_geom_partnames(&luminaire.structure.geometry, &mut names);

        // Simulate the part-load loop: for each entry in `names`, find
        // LEOs whose parent_geometry_partname matches.
        let leos_for_head_0: Vec<&str> = emitters
            .iter()
            .filter(|e| e.parent_geometry_partname == "head-0")
            .map(|e| e.name.as_str())
            .collect();
        let leos_for_head_1: Vec<&str> = emitters
            .iter()
            .filter(|e| e.parent_geometry_partname == "head-1")
            .map(|e| e.name.as_str())
            .collect();
        let leos_for_base: Vec<&str> = emitters
            .iter()
            .filter(|e| e.parent_geometry_partname == "base")
            .map(|e| e.name.as_str())
            .collect();

        assert_eq!(leos_for_head_0, vec!["LEO0"]);
        assert_eq!(leos_for_head_1, vec!["LEO1"]);
        assert!(leos_for_base.is_empty(), "base node has no LEOs");
    }
}
