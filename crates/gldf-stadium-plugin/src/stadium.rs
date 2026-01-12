//! Stadium Scene Module
//!
//! Adds stadium geometry (field, stands, towers) and places floodlights.

use bevy::prelude::*;
use eulumdat::Eulumdat;
use eulumdat_bevy::photometric::{PhotometricLight, PhotometricLightBundle};

use crate::{log, StadiumGldfData};

/// Resource to track current intensity scale
#[derive(Resource)]
pub struct IntensityScale(pub f32);

impl Default for IntensityScale {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Resource to track current tilt angle (in degrees)
#[derive(Resource)]
pub struct TiltAngle(pub f32);

impl Default for TiltAngle {
    fn default() -> Self {
        Self(45.0) // Default 45 degrees - good for stadium floodlights
    }
}

/// Tower configuration: 4 corners or 6 (corners + middle)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum TowerLayout {
    #[default]
    FourCorners,
    SixWithMiddle,
}

/// Stadium dimensions and configuration
#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct StadiumSettings {
    pub field_length: f32,
    pub field_width: f32,
    pub tower_height: f32,
    pub tower_layout: TowerLayout,
}

impl Default for StadiumSettings {
    fn default() -> Self {
        Self {
            field_length: 105.0,
            field_width: 68.0,
            tower_height: 45.0,
            tower_layout: TowerLayout::FourCorners,
        }
    }
}

/// Marker for stadium geometry
#[derive(Component)]
pub struct StadiumGeometry;

/// Marker for floodlight entities
#[derive(Component)]
pub struct Floodlight;

/// Plugin for the stadium scene
pub struct StadiumPlugin;

impl Plugin for StadiumPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StadiumSettings>()
            .init_resource::<IntensityScale>()
            .init_resource::<TiltAngle>()
            .add_systems(Startup, setup_stadium)
            .add_systems(Update, handle_layout_toggle)
            .add_systems(Update, handle_intensity_controls)
            .add_systems(Update, handle_tilt_controls);
    }
}

/// Handle +/- keys to adjust light intensity
fn handle_intensity_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut intensity_scale: ResMut<IntensityScale>,
    mut photometric_lights: Query<&mut PhotometricLight<Eulumdat>, With<Floodlight>>,
) {
    let mut changed = false;
    let mut multiplier = 1.0f32;

    // = key (which is + without shift) and numpad +
    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        multiplier = 1.2;
        changed = true;
    }
    // - key and numpad -
    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        multiplier = 0.8;
        changed = true;
    }

    if changed {
        intensity_scale.0 *= multiplier;
        for mut light in photometric_lights.iter_mut() {
            light.intensity_scale *= multiplier;
        }
        log(&format!("[Stadium] Light intensity: {:.1}x", intensity_scale.0));
    }
}

/// Handle C/V keys to adjust tilt angle
fn handle_tilt_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut tilt_angle: ResMut<TiltAngle>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<StadiumSettings>,
    gldf_data: Res<StadiumGldfData>,
    geometry_query: Query<Entity, With<StadiumGeometry>>,
    light_query: Query<Entity, With<Floodlight>>,
) {
    let mut changed = false;

    // C key: decrease tilt (more horizontal)
    if keyboard.just_pressed(KeyCode::KeyC) {
        tilt_angle.0 = (tilt_angle.0 - 5.0).max(0.0);
        changed = true;
    }
    // V key: increase tilt (more angled down)
    if keyboard.just_pressed(KeyCode::KeyV) {
        tilt_angle.0 = (tilt_angle.0 + 5.0).min(90.0);
        changed = true;
    }

    if changed {
        log(&format!("[Stadium] Tilt angle: {:.0}°", tilt_angle.0));

        // Despawn and rebuild with new tilt
        for entity in geometry_query.iter() {
            commands.entity(entity).despawn_recursive();
        }
        for entity in light_query.iter() {
            commands.entity(entity).despawn_recursive();
        }

        build_stadium_with_tilt(
            &mut commands,
            &mut meshes,
            &mut materials,
            &settings,
            &gldf_data,
            tilt_angle.0,
        );
    }
}

/// Handle T key to toggle layout
fn handle_layout_toggle(
    mut settings: ResMut<StadiumSettings>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    gldf_data: Res<StadiumGldfData>,
    geometry_query: Query<Entity, With<StadiumGeometry>>,
    light_query: Query<Entity, With<Floodlight>>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        settings.tower_layout = match settings.tower_layout {
            TowerLayout::FourCorners => TowerLayout::SixWithMiddle,
            TowerLayout::SixWithMiddle => TowerLayout::FourCorners,
        };
        log(&format!("[Stadium] Switching to {:?} layout", settings.tower_layout));

        // Despawn existing
        for entity in geometry_query.iter() {
            commands.entity(entity).despawn_recursive();
        }
        for entity in light_query.iter() {
            commands.entity(entity).despawn_recursive();
        }

        // Rebuild
        build_stadium(&mut commands, &mut meshes, &mut materials, &settings, &gldf_data);
    }
}

/// Setup the stadium scene
fn setup_stadium(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<StadiumSettings>,
    gldf_data: Res<StadiumGldfData>,
) {
    log("[Stadium] Setting up stadium scene...");

    // Ambient light - enough to see the stadium
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.2, 0.2, 0.25),
        brightness: 800.0,  // Higher for visibility
    });

    log("[Stadium] Setup: Ambient light configured");

    build_stadium(&mut commands, &mut meshes, &mut materials, &settings, &gldf_data);
    log("[Stadium] Stadium scene setup complete");
}

/// Build all stadium elements with default tilt
fn build_stadium(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &StadiumSettings,
    gldf_data: &StadiumGldfData,
) {
    build_stadium_with_tilt(commands, meshes, materials, settings, gldf_data, 45.0);
}

/// Build all stadium elements with specified tilt angle
fn build_stadium_with_tilt(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &StadiumSettings,
    gldf_data: &StadiumGldfData,
    tilt_degrees: f32,
) {
    build_field(commands, meshes, materials, settings);
    build_stands(commands, meshes, materials, settings);
    build_towers_with_tilt(commands, meshes, materials, settings, tilt_degrees);
    spawn_floodlights_with_tilt(commands, meshes, materials, settings, gldf_data, tilt_degrees);
}

/// Build the football field
fn build_field(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &StadiumSettings,
) {
    let grass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.5, 0.15),
        perceptual_roughness: 0.9,
        ..default()
    });

    // Field plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(settings.field_length, settings.field_width))),
        MeshMaterial3d(grass),
        Transform::from_xyz(settings.field_length / 2.0, 0.0, settings.field_width / 2.0),
        StadiumGeometry,
    ));

    // Field markings
    let white = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::new(0.5, 0.5, 0.5, 1.0),
        ..default()
    });

    let line_h = 0.02;
    let line_w = 0.12;

    // Center line
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(line_w, line_h, settings.field_width))),
        MeshMaterial3d(white.clone()),
        Transform::from_xyz(settings.field_length / 2.0, line_h / 2.0, settings.field_width / 2.0),
        StadiumGeometry,
    ));

    // Center circle
    let radius = 9.15;
    let segments = 32;
    for i in 0..segments {
        let a1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let a2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
        let x1 = settings.field_length / 2.0 + radius * a1.cos();
        let z1 = settings.field_width / 2.0 + radius * a1.sin();
        let x2 = settings.field_length / 2.0 + radius * a2.cos();
        let z2 = settings.field_width / 2.0 + radius * a2.sin();
        let len = ((x2 - x1).powi(2) + (z2 - z1).powi(2)).sqrt();
        let ang = (z2 - z1).atan2(x2 - x1);

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(len, line_h, line_w))),
            MeshMaterial3d(white.clone()),
            Transform::from_xyz((x1 + x2) / 2.0, line_h / 2.0, (z1 + z2) / 2.0)
                .with_rotation(Quat::from_rotation_y(-ang)),
            StadiumGeometry,
        ));
    }

    // Touchlines
    for z in [0.0, settings.field_width] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(settings.field_length, line_h, line_w))),
            MeshMaterial3d(white.clone()),
            Transform::from_xyz(settings.field_length / 2.0, line_h / 2.0, z),
            StadiumGeometry,
        ));
    }

    // Goal lines
    for x in [0.0, settings.field_length] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(line_w, line_h, settings.field_width))),
            MeshMaterial3d(white.clone()),
            Transform::from_xyz(x, line_h / 2.0, settings.field_width / 2.0),
            StadiumGeometry,
        ));
    }
}

/// Build stadium stands/tribunes around the field
fn build_stands(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &StadiumSettings,
) {
    let stand_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.4, 0.45),
        perceptual_roughness: 0.8,
        ..default()
    });

    let seat_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.3, 0.6), // Blue seats
        perceptual_roughness: 0.7,
        ..default()
    });

    let fl = settings.field_length;
    let fw = settings.field_width;
    let stand_width = 20.0;  // How far stands extend from field
    let stand_offset = 5.0;  // Gap between field and stands

    // Long sides (along field length) - main tribunes
    for z_side in [-1.0, 1.0] {
        let z_base = if z_side < 0.0 { -stand_offset } else { fw + stand_offset };

        // Tiered seating - 5 tiers
        for tier in 0..5 {
            let tier_f = tier as f32;
            let tier_height = tier_f * 3.0;
            let tier_depth = stand_width - tier_f * 3.0;
            let tier_z = z_base + z_side * (tier_f * 1.5 + tier_depth / 2.0);

            // Concrete tier
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(fl + 10.0, 0.5, tier_depth.max(2.0)))),
                MeshMaterial3d(stand_mat.clone()),
                Transform::from_xyz(fl / 2.0, tier_height + 0.25, tier_z),
                StadiumGeometry,
            ));

            // Seat row
            if tier_depth > 3.0 {
                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(fl + 8.0, 0.8, 1.0))),
                    MeshMaterial3d(seat_mat.clone()),
                    Transform::from_xyz(fl / 2.0, tier_height + 0.9, tier_z - z_side * 1.0),
                    StadiumGeometry,
                ));
            }
        }
    }

    // Short sides (behind goals) - smaller tribunes
    for x_side in [-1.0, 1.0] {
        let x_base = if x_side < 0.0 { -stand_offset } else { fl + stand_offset };

        // 3 tiers for goal ends
        for tier in 0..3 {
            let tier_f = tier as f32;
            let tier_height = tier_f * 3.0;
            let tier_depth = 12.0 - tier_f * 3.0;
            let tier_x = x_base + x_side * (tier_f * 1.5 + tier_depth / 2.0);

            // Concrete tier
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(tier_depth.max(2.0), 0.5, fw + 10.0))),
                MeshMaterial3d(stand_mat.clone()),
                Transform::from_xyz(tier_x, tier_height + 0.25, fw / 2.0),
                StadiumGeometry,
            ));

            // Seat row
            if tier_depth > 3.0 {
                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(1.0, 0.8, fw + 8.0))),
                    MeshMaterial3d(seat_mat.clone()),
                    Transform::from_xyz(tier_x - x_side * 1.0, tier_height + 0.9, fw / 2.0),
                    StadiumGeometry,
                ));
            }
        }
    }
}

/// Build floodlight towers with 90-degree arms
fn build_towers_with_tilt(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &StadiumSettings,
    tilt_degrees: f32,
) {
    let tower_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.35),
        metallic: 0.7,
        ..default()
    });

    let positions = get_tower_positions(settings);
    let field_center = Vec3::new(
        settings.field_length / 2.0,
        0.0,
        settings.field_width / 2.0,
    );

    for pos in &positions {
        let pole_top = Vec3::new(pos.x, settings.tower_height, pos.z);

        // Calculate horizontal direction toward field (for the arm)
        let to_field = Vec3::new(
            field_center.x - pos.x,
            0.0,
            field_center.z - pos.z,
        ).normalize();

        // Tower pole
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.8, settings.tower_height))),
            MeshMaterial3d(tower_mat.clone()),
            Transform::from_xyz(pos.x, settings.tower_height / 2.0, pos.z),
            StadiumGeometry,
        ));

        // Horizontal arm extending toward field (90-degree bracket)
        let arm_length = 4.0;
        let arm_pos = pole_top + to_field * (arm_length / 2.0) - Vec3::Y * 0.3;

        // Arm should point in the to_field direction
        let arm_rotation = Quat::from_rotation_arc(Vec3::X, to_field);

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(arm_length, 0.5, 0.5))),
            MeshMaterial3d(tower_mat.clone()),
            Transform::from_translation(arm_pos)
                .with_rotation(arm_rotation),
            StadiumGeometry,
        ));

        // Lamp holder platform at end of arm
        let platform_pos = pole_top + to_field * arm_length - Vec3::Y * 0.5;

        // First rotate to face field (yaw), then tilt down (pitch)
        let tilt_angle = tilt_degrees.to_radians();

        // Yaw: rotate around Y axis to face field
        let yaw = (-to_field.x).atan2(-to_field.z);

        // Combined rotation: yaw first, then pitch (tilt down toward field)
        let platform_rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(tilt_angle);

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(4.0, 0.4, 2.5))),
            MeshMaterial3d(tower_mat.clone()),
            Transform::from_translation(platform_pos)
                .with_rotation(platform_rotation),
            StadiumGeometry,
        ));
    }
}

/// Get tower positions based on layout
fn get_tower_positions(settings: &StadiumSettings) -> Vec<Vec3> {
    let offset = 15.0; // Distance from field edge
    let fl = settings.field_length;
    let fw = settings.field_width;

    match settings.tower_layout {
        TowerLayout::FourCorners => vec![
            Vec3::new(-offset, 0.0, -offset),
            Vec3::new(fl + offset, 0.0, -offset),
            Vec3::new(-offset, 0.0, fw + offset),
            Vec3::new(fl + offset, 0.0, fw + offset),
        ],
        TowerLayout::SixWithMiddle => vec![
            Vec3::new(-offset, 0.0, -offset),
            Vec3::new(fl / 2.0, 0.0, -offset - 10.0),
            Vec3::new(fl + offset, 0.0, -offset),
            Vec3::new(-offset, 0.0, fw + offset),
            Vec3::new(fl / 2.0, 0.0, fw + offset + 10.0),
            Vec3::new(fl + offset, 0.0, fw + offset),
        ],
    }
}

/// Spawn floodlights as SpotLights that respond to tilt
fn spawn_floodlights_with_tilt(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &StadiumSettings,
    gldf_data: &StadiumGldfData,
    tilt_degrees: f32,
) {
    let positions = get_tower_positions(settings);
    let field_center = Vec3::new(
        settings.field_length / 2.0,
        0.0,
        settings.field_width / 2.0,
    );

    let arm_length = 4.0;  // Must match build_towers arm_length

    // Get light properties from LDT or use defaults
    let (flux, color_temp) = if let Some(ref ldt) = gldf_data.ldt_data {
        log(&format!("[Stadium] Using LDT data: {} lm", ldt.total_luminous_flux()));
        (ldt.total_luminous_flux() as f32, 5700.0)
    } else {
        log("[Stadium] No LDT data - using defaults");
        (50000.0, 5700.0)
    };

    let light_color = kelvin_to_rgb(color_temp);

    // Emissive material for lamp face
    let emissive_mat = materials.add(StandardMaterial {
        base_color: light_color,
        emissive: light_color.to_linear() * 10.0,
        unlit: true,
        ..default()
    });

    for (i, pos) in positions.iter().enumerate() {
        let pole_top = Vec3::new(pos.x, settings.tower_height, pos.z);

        // Calculate horizontal direction toward field
        let to_field_horizontal = Vec3::new(
            field_center.x - pos.x,
            0.0,
            field_center.z - pos.z,
        ).normalize();

        // Light position at end of arm
        let light_pos = pole_top + to_field_horizontal * arm_length - Vec3::Y * 0.5;

        // Calculate light direction based on tilt
        // Yaw to face field, then pitch DOWN by tilt angle (negative X rotation)
        let yaw = (-to_field_horizontal.x).atan2(-to_field_horizontal.z);
        let tilt_rad = tilt_degrees.to_radians();

        // Build rotation: first yaw around Y, then pitch DOWN (negative X rotation)
        let rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(-tilt_rad);

        // The light direction is -Z after rotation (SpotLight default)
        let light_dir = rotation * (-Vec3::Z);

        let light_transform = Transform::from_translation(light_pos)
            .with_rotation(rotation);

        // Intensity - very high value for stadium floodlights
        // Bevy 0.15 uses physical light units - need higher values
        let intensity = flux * 500.0 / positions.len() as f32;

        log(&format!(
            "[Stadium] Creating lights with intensity {} (flux={}, towers={})",
            intensity, flux, positions.len()
        ));

        // Spawn multiple SpotLights for better coverage
        // Main beam
        commands.spawn((
            SpotLight {
                color: light_color,
                intensity,
                range: 200.0,
                radius: 0.5,
                inner_angle: 0.2,
                outer_angle: 0.7,
                shadows_enabled: false,
                ..default()
            },
            light_transform,
            Floodlight,
        ));

        // Wider fill light
        commands.spawn((
            SpotLight {
                color: light_color,
                intensity: intensity * 0.6,
                range: 250.0,
                radius: 0.8,
                inner_angle: 0.4,
                outer_angle: 1.0,
                shadows_enabled: false,
                ..default()
            },
            light_transform,
            Floodlight,
        ));

        // Also add a PointLight at the same location for ambient fill
        commands.spawn((
            PointLight {
                color: light_color,
                intensity: intensity * 0.3,
                range: 150.0,
                radius: 1.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(light_pos),
            Floodlight,
        ));

        // Glowing lamp face
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(3.0, 0.2, 2.0))),
            MeshMaterial3d(emissive_mat.clone()),
            Transform::from_translation(light_pos - Vec3::Y * 0.4)
                .with_rotation(rotation),
            Floodlight,
        ));

        log(&format!(
            "[Stadium] Floodlight {} at ({:.1}, {:.1}, {:.1}) dir ({:.2}, {:.2}, {:.2})",
            i, light_pos.x, light_pos.y, light_pos.z,
            light_dir.x, light_dir.y, light_dir.z
        ));
    }

    log(&format!("[Stadium] Spawned {} floodlights", positions.len()));
}

/// Parse color temperature from string
fn parse_color_temperature(s: &str) -> f32 {
    s.trim().trim_end_matches('K').trim_end_matches('k')
        .parse().unwrap_or(5700.0)
}

/// Convert Kelvin to RGB color
fn kelvin_to_rgb(kelvin: f32) -> Color {
    let temp = kelvin / 100.0;
    let r = if temp <= 66.0 { 1.0 } else {
        (329.698727446 * (temp - 60.0).powf(-0.1332047592) / 255.0).clamp(0.0, 1.0)
    };
    let g = if temp <= 66.0 {
        (99.4708025861 * temp.ln() - 161.1195681661).clamp(0.0, 255.0) / 255.0
    } else {
        (288.1221695283 * (temp - 60.0).powf(-0.0755148492) / 255.0).clamp(0.0, 1.0)
    };
    let b = if temp >= 66.0 { 1.0 } else if temp <= 19.0 { 0.0 } else {
        (138.5177312231 * (temp - 10.0).ln() - 305.0447927307).clamp(0.0, 255.0) / 255.0
    };
    Color::srgb(r, g, b)
}
