//! GLDF Viewer - Cross-platform desktop application
//!
//! Built with Slint UI and gldf-rs for parsing GLDF files.
//! Works on Windows, macOS, Linux, and embedded systems.

use anyhow::Result;
use slint::{ModelRc, SharedString, VecModel};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

slint::include_modules!();

/// Application state shared between UI and logic
struct AppState {
    engine: Option<gldf_rs::GldfProduct>,
    current_path: Option<PathBuf>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            engine: None,
            current_path: None,
        }
    }
}

fn main() -> Result<()> {
    let app = GldfViewer::new()?;
    let state = Arc::new(RwLock::new(AppState::default()));

    // Setup callbacks
    setup_open_file_callback(&app, state.clone());
    setup_export_callbacks(&app, state.clone());
    setup_file_selected_callback(&app, state.clone());
    setup_view_3d_callback(&app, state.clone());

    // Check for command line argument
    if let Some(path) = std::env::args().nth(1) {
        let path = PathBuf::from(path);
        if path.exists() {
            load_file(&app, state.clone(), path);
        }
    }

    app.run()?;
    Ok(())
}

fn setup_open_file_callback(app: &GldfViewer, state: Arc<RwLock<AppState>>) {
    let app_weak = app.as_weak();
    app.on_open_file(move || {
        let app = app_weak.unwrap();
        let state = state.clone();

        // Use rfd for native file dialog
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("GLDF Files", &["gldf"])
            .add_filter("All Files", &["*"])
            .pick_file()
        {
            load_file(&app, state, path);
        }
    });
}

fn load_file(app: &GldfViewer, state: Arc<RwLock<AppState>>, path: PathBuf) {
    app.set_is_loading(true);
    app.set_status_message(SharedString::from("Loading..."));

    // Read and parse file
    match std::fs::read(&path) {
        Ok(data) => match gldf_rs::GldfProduct::load_gldf_from_buf(data) {
            Ok(product) => {
                // Update state
                {
                    let mut state = state.write().unwrap();
                    state.engine = Some(product);
                    state.current_path = Some(path.clone());
                }

                // Update UI
                let state = state.read().unwrap();
                if let Some(ref product) = state.engine {
                    update_ui_from_product(app, product);
                }

                let filename = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                app.set_current_file(SharedString::from(filename));
                app.set_status_message(SharedString::from("File loaded successfully"));
            }
            Err(e) => {
                app.set_status_message(SharedString::from(format!("Error parsing file: {}", e)));
            }
        },
        Err(e) => {
            app.set_status_message(SharedString::from(format!("Error reading file: {}", e)));
        }
    }

    app.set_is_loading(false);
}

fn update_ui_from_product(app: &GldfViewer, product: &gldf_rs::GldfProduct) {
    // Update header - fields are String, not Option<String>
    let header = &product.header;
    app.set_header(GldfHeader {
        manufacturer: SharedString::from(&header.manufacturer),
        author: SharedString::from(&header.author),
        format_version: SharedString::from(format!(
            "{}.{}.{}",
            header.format_version.major,
            header.format_version.minor,
            header.format_version.pre_release
        )),
        created_with: SharedString::from(&header.created_with_application),
        creation_date: SharedString::from(&header.creation_time_code),
    });

    // Collect files from general_definitions.files
    let files_data = &product.general_definitions.files.file;
    let mut files_vec: Vec<GldfFileInfo> = Vec::new();
    let mut geometry_count = 0;
    let mut image_count = 0;
    let mut photometry_count = 0;

    for file in files_data {
        let content_type = &file.content_type;
        let file_type = if content_type.starts_with("geo/") {
            geometry_count += 1;
            "geometry"
        } else if content_type.starts_with("image/") {
            image_count += 1;
            "image"
        } else if content_type.starts_with("ldc/") {
            photometry_count += 1;
            "photometry"
        } else if content_type.starts_with("document/") {
            "document"
        } else {
            "other"
        };

        files_vec.push(GldfFileInfo {
            id: SharedString::from(&file.id),
            name: SharedString::from(&file.file_name),
            content_type: SharedString::from(content_type),
            file_type: SharedString::from(file_type),
        });
    }

    // Update files list
    let files_model = std::rc::Rc::new(VecModel::from(files_vec));
    app.set_files(ModelRc::from(files_model));

    // Collect light sources
    let mut light_sources_vec: Vec<LightSourceInfo> = Vec::new();
    if let Some(ref ls_container) = product.general_definitions.light_sources {
        for ls in &ls_container.fixed_light_source {
            let name = ls
                .name
                .locale
                .first()
                .map(|l| l.value.as_str())
                .unwrap_or(&ls.id);

            let description = ls
                .description
                .as_ref()
                .and_then(|d| d.locale.first())
                .map(|l| l.value.as_str())
                .unwrap_or("");

            // Try to get lumens from rated input power or other sources
            let lumens = ls
                .rated_input_power
                .map(|p| format!("{:.0}", p))
                .unwrap_or_else(|| "-".to_string());

            light_sources_vec.push(LightSourceInfo {
                id: SharedString::from(&ls.id),
                name: SharedString::from(name),
                description: SharedString::from(description),
                lumens: SharedString::from(lumens),
            });
        }
        for ls in &ls_container.changeable_light_source {
            let description = ls
                .description
                .as_ref()
                .map(|d| d.value.as_str())
                .unwrap_or("");

            light_sources_vec.push(LightSourceInfo {
                id: SharedString::from(&ls.id),
                name: SharedString::from(&ls.name.value),
                description: SharedString::from(description),
                lumens: SharedString::from("-"),
            });
        }
    }

    let light_sources_model = std::rc::Rc::new(VecModel::from(light_sources_vec.clone()));
    app.set_light_sources(ModelRc::from(light_sources_model));

    // Count variants
    let variant_count = product
        .product_definitions
        .variants
        .as_ref()
        .map(|v| v.variant.len())
        .unwrap_or(0);

    // Update stats
    app.set_stats(GldfStats {
        file_count: files_data.len() as i32,
        geometry_count: geometry_count as i32,
        image_count: image_count as i32,
        photometry_count: photometry_count as i32,
        light_source_count: light_sources_vec.len() as i32,
        variant_count: variant_count as i32,
    });
}

fn setup_export_callbacks(app: &GldfViewer, state: Arc<RwLock<AppState>>) {
    // Export JSON
    let app_weak = app.as_weak();
    let state_clone = state.clone();
    app.on_export_json(move || {
        let app = app_weak.unwrap();
        let state = state_clone.read().unwrap();

        if let Some(ref product) = state.engine {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON Files", &["json"])
                .set_file_name("export.json")
                .save_file()
            {
                match product.to_pretty_json() {
                    Ok(json) => {
                        if let Err(e) = std::fs::write(&path, json) {
                            app.set_status_message(SharedString::from(format!(
                                "Error saving: {}",
                                e
                            )));
                        } else {
                            app.set_status_message(SharedString::from("Exported to JSON"));
                        }
                    }
                    Err(e) => {
                        app.set_status_message(SharedString::from(format!("Error: {}", e)));
                    }
                }
            }
        }
    });

    // Export XML
    let app_weak = app.as_weak();
    app.on_export_xml(move || {
        let app = app_weak.unwrap();
        let state = state.read().unwrap();

        if let Some(ref product) = state.engine {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("XML Files", &["xml"])
                .set_file_name("product.xml")
                .save_file()
            {
                match product.to_xml() {
                    Ok(xml) => {
                        if let Err(e) = std::fs::write(&path, xml) {
                            app.set_status_message(SharedString::from(format!(
                                "Error saving: {}",
                                e
                            )));
                        } else {
                            app.set_status_message(SharedString::from("Exported to XML"));
                        }
                    }
                    Err(e) => {
                        app.set_status_message(SharedString::from(format!("Error: {}", e)));
                    }
                }
            }
        }
    });
}

fn setup_file_selected_callback(app: &GldfViewer, state: Arc<RwLock<AppState>>) {
    let app_weak = app.as_weak();
    app.on_file_selected(move |file_id| {
        let app = app_weak.unwrap();
        let state = state.read().unwrap();

        if let Some(ref product) = state.engine {
            let file_id_str = file_id.to_string();

            // Find the file info
            let files = &product.general_definitions.files.file;
            if let Some(file_info) = files.iter().find(|f| f.id == file_id_str) {
                let content_type = &file_info.content_type;

                // Show content for text-based files
                if content_type.contains("xml") ||
                   content_type.starts_with("ldc/") ||
                   content_type == "text/plain" {
                    // Try to load file content
                    match product.load_gldf_file_str(file_info.file_name.clone()) {
                        Ok(content) => {
                            // Truncate if too long
                            let display = if content.len() > 50000 {
                                format!("{}...\n\n[Truncated - {} bytes total]",
                                    &content[..50000], content.len())
                            } else {
                                content
                            };
                            app.set_selected_file_content(SharedString::from(display));
                        }
                        Err(e) => {
                            app.set_selected_file_content(SharedString::from(format!(
                                "Error loading file: {}", e
                            )));
                        }
                    }
                } else {
                    app.set_selected_file_content(SharedString::from(format!(
                        "Binary file: {}\nContent-Type: {}\n\nBinary content cannot be displayed as text.",
                        file_info.file_name, content_type
                    )));
                }
            }
        }
    });
}

fn setup_view_3d_callback(app: &GldfViewer, state: Arc<RwLock<AppState>>) {
    let app_weak = app.as_weak();
    app.on_view_3d(move |file_id| {
        let app = app_weak.unwrap();
        let _state = state.read().unwrap();

        // TODO: Integrate with three-d or wgpu for 3D rendering
        // For now, show a message
        app.set_status_message(SharedString::from(format!(
            "3D Viewer for '{}' - Coming soon! (Integrate with wgpu/three-d)",
            file_id
        )));

        // Future implementation:
        // 1. Extract L3D file from GLDF
        // 2. Parse using l3d_rs
        // 3. Open a new window with wgpu/three-d renderer
        // 4. Display the 3D model with orbit controls
    });
}
