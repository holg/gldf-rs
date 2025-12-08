//! Standalone L3D 3D Viewer
//!
//! This binary is spawned as a subprocess to display L3D models in an interactive 3D viewer.
//! Usage: `gldf-l3d-viewer <path-to-l3d-file> [title]`

use std::collections::HashMap;
use std::env;
use std::fs;
use three_d::*;

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <l3d-file> [title]", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];
    let title = args.get(2).map(|s| s.as_str()).unwrap_or("L3D Model");

    // Read the L3D file
    let content = match fs::read(file_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to read file '{}': {}", file_path, e);
            std::process::exit(1);
        }
    };

    if let Err(e) = run_viewer(&content, title) {
        eprintln!("Viewer error: {}", e);
        std::process::exit(1);
    }
}

fn run_viewer(content: &[u8], title: &str) -> Result<(), String> {
    // Parse L3D
    let l3d = l3d_rs::from_buffer(content);

    if l3d.model.parts.is_empty() {
        return Err("No model parts found in L3D file".to_string());
    }

    // Create window
    let window = Window::new(WindowSettings {
        title: format!("L3D Viewer - {}", title),
        max_size: Some((1200, 900)),
        ..Default::default()
    })
    .map_err(|e| format!("Failed to create window: {:?}", e))?;

    let context = window.gl();

    // Load assets from L3D
    let mut raw_assets = three_d_asset::io::RawAssets::new();
    for asset in &l3d.file.assets {
        raw_assets.insert(&asset.name, asset.content.clone());
    }

    // Add stub MTL files for missing materials
    add_stub_mtls(&l3d, &mut raw_assets);

    // Create camera
    let mut camera = Camera::new_perspective(
        window.viewport(),
        vec3(4.0, 4.0, 5.0),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        degrees(45.0),
        0.1,
        1000.0,
    );

    // Create orbit control
    #[allow(clippy::clone_on_copy)]
    let mut control = OrbitControl::new(camera.target().clone(), 1.0, 100.0);

    // Create lights
    let light0 = DirectionalLight::new(&context, 2.0, Srgba::WHITE, vec3(-1.0, -1.0, -1.0));
    let light1 = DirectionalLight::new(&context, 0.3, Srgba::WHITE, vec3(0.5, 0.5, 0.5));
    let ambient = AmbientLight::new(&context, 0.4, Srgba::WHITE);

    // Load models
    let rotation_matrix = Mat4::from_angle_x(Deg(-90.0));
    let mut cpu_models: HashMap<String, CpuModel> = HashMap::new();
    let mut models: Vec<Model<PhysicalMaterial>> = Vec::new();
    let mut min_bound = vec3(f32::MAX, f32::MAX, f32::MAX);
    let mut max_bound = vec3(f32::MIN, f32::MIN, f32::MIN);

    for part in &l3d.model.parts {
        let cpu_mdl = match cpu_models.get(&part.path) {
            Some(m) => m,
            None => match raw_assets.deserialize::<CpuModel>(&part.path) {
                Ok(m) => {
                    cpu_models.insert(part.path.clone(), m);
                    cpu_models.get(&part.path).unwrap()
                }
                Err(e) => {
                    log::warn!("Failed to load model part '{}': {:?}", part.path, e);
                    continue;
                }
            },
        };

        match Model::<PhysicalMaterial>::new(&context, cpu_mdl) {
            Ok(mut mdl) => {
                let part_mat = Mat4::from_cols(
                    vec4(part.mat[0], part.mat[1], part.mat[2], part.mat[3]),
                    vec4(part.mat[4], part.mat[5], part.mat[6], part.mat[7]),
                    vec4(part.mat[8], part.mat[9], part.mat[10], part.mat[11]),
                    vec4(part.mat[12], part.mat[13], part.mat[14], part.mat[15]),
                );
                let mat = rotation_matrix * part_mat;
                mdl.iter_mut().for_each(|m| {
                    m.set_transformation(mat);
                    let aabb = m.aabb();
                    min_bound.x = min_bound.x.min(aabb.min().x);
                    min_bound.y = min_bound.y.min(aabb.min().y);
                    min_bound.z = min_bound.z.min(aabb.min().z);
                    max_bound.x = max_bound.x.max(aabb.max().x);
                    max_bound.y = max_bound.y.max(aabb.max().y);
                    max_bound.z = max_bound.z.max(aabb.max().z);
                });
                models.push(mdl);
            }
            Err(e) => {
                log::warn!("Failed to create GPU model for '{}': {:?}", part.path, e);
                continue;
            }
        }
    }

    if models.is_empty() {
        return Err("Failed to load any model parts".to_string());
    }

    // Auto-fit camera to model
    if min_bound.x < max_bound.x {
        let center = vec3(
            (min_bound.x + max_bound.x) / 2.0,
            (min_bound.y + max_bound.y) / 2.0,
            (min_bound.z + max_bound.z) / 2.0,
        );
        let size = vec3(
            max_bound.x - min_bound.x,
            max_bound.y - min_bound.y,
            max_bound.z - min_bound.z,
        );
        let max_dim = size.x.max(size.y).max(size.z);
        let distance = max_dim / (2.0 * (22.5_f32.to_radians()).tan());
        let camera_pos = center + vec3(distance * 0.7, distance * 0.7, distance);

        camera = Camera::new_perspective(
            window.viewport(),
            camera_pos,
            center,
            vec3(0.0, 1.0, 0.0),
            degrees(45.0),
            0.01,
            distance * 10.0,
        );

        control = OrbitControl::new(center, distance * 0.1, distance * 5.0);
    }

    // Render loop
    window.render_loop(move |mut frame_input| {
        camera.set_viewport(frame_input.viewport);
        control.handle_events(&mut camera, &mut frame_input.events);

        frame_input
            .screen()
            .clear(ClearState::color_and_depth(0.8, 0.8, 0.8, 1.0, 1.0))
            .render(
                &camera,
                models.iter().flatten(),
                &[&light0, &light1, &ambient],
            );

        FrameOutput::default()
    });

    Ok(())
}

fn add_stub_mtls(l3d: &l3d_rs::L3d, raw_assets: &mut three_d_asset::io::RawAssets) {
    for asset in &l3d.file.assets {
        if asset.name.to_lowercase().ends_with(".obj") {
            if let Ok(content) = std::str::from_utf8(&asset.content) {
                let base_path = if let Some(pos) = asset.name.rfind('/') {
                    &asset.name[..=pos]
                } else {
                    ""
                };

                for line in content.lines() {
                    let line = line.trim();
                    if let Some(mtl_name) = line.strip_prefix("mtllib ") {
                        let mtl_name = mtl_name.trim();
                        let mtl_path = format!("{}{}", base_path, mtl_name);

                        if !l3d.file.assets.iter().any(|a| a.name == mtl_path) {
                            let stub_mtl = b"# Stub material\nnewmtl default\nNs 100.0\nKa 0.2 0.2 0.2\nKd 0.8 0.8 0.8\nKs 0.1 0.1 0.1\nd 1.0\nillum 2\n";
                            raw_assets.insert(&mtl_path, stub_mtl.to_vec());
                        }
                    }
                }
            }
        }
    }
}
