//! Star Sky rendering for Bevy
//!
//! Renders a realistic night sky from star catalogue data.
//! Stars are positioned using Alt/Az coordinates from the JSON data.

use bevy::prelude::*;

/// Star data from JSON
#[derive(Debug, Clone, serde::Deserialize)]
pub struct StarData {
    pub name: String,
    pub alt: f64,  // Altitude in degrees (0-90)
    pub az: f64,   // Azimuth in degrees (0-360)
    pub mag: f32,  // Apparent magnitude
    #[serde(default)]
    pub spectral: String,
    pub temp: f32, // Color temperature in Kelvin
    #[serde(default)]
    pub ra: f64,
    #[serde(default)]
    pub dec: f64,
}

/// Location metadata
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LocationData {
    pub name: String,
    pub lat: f64,
    pub lng: f64,
}

/// Full star sky JSON structure
#[derive(Debug, Clone, serde::Deserialize)]
pub struct StarSkyData {
    pub location: LocationData,
    pub time: String,
    pub stars: Vec<StarData>,
}

/// Resource to store star sky data
#[derive(Resource, Default)]
pub struct StarSkyResource {
    pub data: Option<StarSkyData>,
    pub loaded: bool,
}

/// Marker component for star entities
#[derive(Component)]
pub struct StarMarker;

/// Marker for the sky dome
#[derive(Component)]
pub struct SkyDome;

/// Storage key for star data (WASM)
#[cfg(target_arch = "wasm32")]
pub const STAR_SKY_STORAGE_KEY: &str = "gldf_star_sky_json";

/// Convert color temperature (Kelvin) to RGB using Tanner Helland's algorithm
pub fn temperature_to_color(kelvin: f32) -> Color {
    let temp = (kelvin / 100.0).clamp(10.0, 400.0);

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

/// Convert Alt/Az to Bevy Vec3 (Y-up coordinate system)
/// Altitude: 0 = horizon, 90 = zenith
/// Azimuth: 0 = North, 90 = East, 180 = South, 270 = West
pub fn altaz_to_vec3(alt_deg: f64, az_deg: f64, radius: f32) -> Vec3 {
    let alt = alt_deg.to_radians();
    let az = az_deg.to_radians();

    // Y is up, X is East, Z is South
    Vec3::new(
        (alt.cos() * az.sin()) as f32 * radius,  // X: East component
        alt.sin() as f32 * radius,                // Y: Up (altitude)
        (alt.cos() * az.cos()) as f32 * radius,  // Z: North component
    )
}

/// Calculate star brightness for rendering (inverse log scale)
/// Magnitude 0 = 1.0, magnitude 6 = ~0.004
pub fn mag_to_brightness(mag: f32) -> f32 {
    // Clamp very bright objects like the Sun
    let clamped_mag = mag.max(-1.5);
    10.0_f32.powf(-0.4 * clamped_mag)
}

/// Calculate star visual size based on magnitude
pub fn mag_to_size(mag: f32) -> f32 {
    // Bright stars (mag < 1) are larger, dim stars smaller
    let base_size = 0.02;
    let brightness = mag_to_brightness(mag);
    (base_size * brightness.sqrt()).clamp(0.005, 0.15)
}

/// Plugin for star sky rendering
pub struct StarSkyPlugin;

impl Plugin for StarSkyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StarSkyResource>()
            .add_systems(Startup, setup_sky_dome)
            .add_systems(Update, load_star_data_system)
            .add_systems(Update, spawn_stars_system);
    }
}

/// Setup sky dome (dark background sphere)
fn setup_sky_dome(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create inverted sphere for sky dome
    let sky_radius = 500.0;
    let mut sky_mesh = Sphere::new(sky_radius).mesh().build();

    // Flip normals inward so we see the inside
    if let Some(normals) = sky_mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL) {
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(ref mut n) = normals {
            for normal in n.iter_mut() {
                normal[0] = -normal[0];
                normal[1] = -normal[1];
                normal[2] = -normal[2];
            }
        }
    }

    // Reverse winding order
    if let Some(indices) = sky_mesh.indices_mut() {
        let new_indices: Vec<u32> = match indices {
            bevy::render::mesh::Indices::U32(ref idx) => {
                idx.chunks(3).flat_map(|tri| [tri[2], tri[1], tri[0]]).collect()
            }
            bevy::render::mesh::Indices::U16(ref idx) => {
                idx.chunks(3).flat_map(|tri| [tri[2] as u32, tri[1] as u32, tri[0] as u32]).collect()
            }
        };
        sky_mesh.insert_indices(bevy::render::mesh::Indices::U32(new_indices));
    }

    // Deep blue-black night sky color
    let sky_color = Color::srgb(0.01, 0.01, 0.03);

    commands.spawn((
        Mesh3d(meshes.add(sky_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: sky_color,
            unlit: true,
            cull_mode: None, // Render both sides
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        SkyDome,
    ));
}

/// Load star data from localStorage (WASM)
#[cfg(target_arch = "wasm32")]
fn load_star_data_system(mut star_sky: ResMut<StarSkyResource>) {
    if star_sky.loaded {
        return;
    }

    if let Some(json) = get_star_sky_json() {
        match serde_json::from_str::<StarSkyData>(&json) {
            Ok(data) => {
                web_sys::console::log_1(&format!(
                    "[StarSky] Loaded {} stars for {}",
                    data.stars.len(),
                    data.location.name
                ).into());
                star_sky.data = Some(data);
                star_sky.loaded = true;
            }
            Err(e) => {
                web_sys::console::log_1(&format!("[StarSky] JSON parse error: {}", e).into());
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_star_data_system(_star_sky: ResMut<StarSkyResource>) {
    // Desktop: would load from file
}

/// Get star sky JSON from localStorage
#[cfg(target_arch = "wasm32")]
fn get_star_sky_json() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item(STAR_SKY_STORAGE_KEY).ok()?
}

/// Spawn star entities when data is loaded
fn spawn_stars_system(
    mut commands: Commands,
    star_sky: Res<StarSkyResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing_stars: Query<Entity, With<StarMarker>>,
    mut spawned: Local<bool>,
) {
    // Only spawn once
    if *spawned {
        return;
    }

    let Some(ref data) = star_sky.data else {
        return;
    };

    // Remove any existing stars
    for entity in existing_stars.iter() {
        commands.entity(entity).despawn();
    }

    let sky_radius = 100.0; // Distance to place stars

    // Create point mesh for stars (billboard sprites would be better but this works)
    let star_mesh = meshes.add(
        Sphere::new(1.0).mesh().ico(1).unwrap()
    );

    // Filter to visible stars (above horizon, reasonable magnitude)
    let visible_stars: Vec<_> = data.stars.iter()
        .filter(|s| s.alt > 0.0 && s.mag < 6.5 && s.mag > -2.0)
        .collect();

    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&format!(
        "[StarSky] Spawning {} stars (filtered from {})",
        visible_stars.len(),
        data.stars.len()
    ).into());

    for star in visible_stars {
        let position = altaz_to_vec3(star.alt, star.az, sky_radius);
        let color = temperature_to_color(star.temp);
        let size = mag_to_size(star.mag);
        let brightness = mag_to_brightness(star.mag).min(2.0);

        // Create emissive material for star glow
        let material = materials.add(StandardMaterial {
            base_color: color,
            emissive: color.to_linear() * brightness * 5.0,
            unlit: true,
            ..default()
        });

        commands.spawn((
            Mesh3d(star_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(position).with_scale(Vec3::splat(size)),
            StarMarker,
        ));
    }

    *spawned = true;

    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&format!(
        "[StarSky] Star sky ready: {} at ({:.2}°N, {:.2}°E)",
        data.location.name, data.location.lat, data.location.lng
    ).into());
}
