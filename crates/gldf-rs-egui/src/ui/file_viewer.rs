//! File viewer for embedded files with proper LDT/IES and L3D support

use crate::app::GldfViewerApp;
use eframe::egui;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Cache for rendered SVG images
lazy_static::lazy_static! {
    static ref SVG_CACHE: Arc<Mutex<HashMap<String, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
    pub static ref URL_CACHE: Arc<Mutex<HashMap<String, UrlFetchState>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Clone)]
pub enum UrlFetchState {
    Loading,
    Loaded(Vec<u8>),
    Error(String),
}

/// Fetch all URL-type files in the background
/// Called when a GLDF with URL references is loaded
pub fn fetch_all_urls(urls: Vec<String>) {
    for url in urls {
        // Skip if not a valid URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            continue;
        }

        // Check if already fetching or fetched
        {
            let cache = URL_CACHE.lock().unwrap();
            if cache.contains_key(&url) {
                continue;
            }
        }

        // Mark as loading
        {
            let mut cache = URL_CACHE.lock().unwrap();
            cache.insert(url.clone(), UrlFetchState::Loading);
        }

        // Spawn fetch thread
        let url_clone = url.clone();
        std::thread::spawn(move || {
            let result = fetch_url_blocking(&url_clone);
            let mut cache = URL_CACHE.lock().unwrap();
            match result {
                Ok(data) => {
                    log::info!("Downloaded URL: {} ({} bytes)", url_clone, data.len());
                    cache.insert(url_clone, UrlFetchState::Loaded(data));
                }
                Err(e) => {
                    log::error!("Failed to download URL {}: {}", url_clone, e);
                    cache.insert(url_clone, UrlFetchState::Error(e));
                }
            }
        });
    }
}

pub fn render_file_viewer(ui: &mut egui::Ui, app: &mut GldfViewerApp) {
    ui.heading("File Viewer");
    ui.add_space(8.0);

    // Clone the files to avoid borrow issues
    let embedded_files: Vec<_> = app
        .embedded_files()
        .iter()
        .filter(|f| {
            let name = f.name.as_deref().unwrap_or("");
            !name.ends_with("product.xml") && !name.ends_with("meta-signature.xml")
        })
        .cloned()
        .collect();

    if embedded_files.is_empty() {
        empty_state(
            ui,
            "👁",
            "No Embedded Files",
            "Load a GLDF file to view embedded files",
        );
        return;
    }

    ui.label(format!("{} embedded files", embedded_files.len()));
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    let mut new_selection: Option<String> = None;
    let current_selection = app.selected_file_id.clone();

    ui.columns(2, |columns| {
        // Left column: file list
        columns[0].heading("Files");
        columns[0].add_space(8.0);

        egui::ScrollArea::vertical()
            .id_salt("file_list_scroll")
            .show(&mut columns[0], |ui| {
                for file in &embedded_files {
                    let full_path = file.name.as_deref().unwrap_or("Unknown");
                    let display_name = full_path.rsplit('/').next().unwrap_or(full_path);
                    let is_selected = current_selection.as_deref() == Some(full_path);
                    let icon = get_file_icon(display_name);
                    let has_content = file
                        .content
                        .as_ref()
                        .map(|c| !c.is_empty())
                        .unwrap_or(false);

                    let label_text = if has_content {
                        format!("{} {}", icon, display_name)
                    } else {
                        format!("{} {} (URL)", icon, display_name)
                    };

                    if ui.selectable_label(is_selected, label_text).clicked() {
                        new_selection = Some(full_path.to_string());
                    }
                }
            });

        // Right column: preview
        columns[1].heading("Preview");
        columns[1].add_space(8.0);

        if let Some(ref selected_id) = current_selection {
            if let Some(file) = embedded_files
                .iter()
                .find(|f| f.name.as_deref() == Some(selected_id.as_str()))
            {
                render_file_preview(&mut columns[1], file);
            } else {
                columns[1].label("File not found");
            }
        } else {
            columns[1].vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(egui::RichText::new("👈").size(32.0));
                ui.label("Select a file from the list");
            });
        }
    });

    if let Some(sel) = new_selection {
        app.selected_file_id = Some(sel);
    }
}

fn render_file_preview(ui: &mut egui::Ui, file: &gldf_rs::BufFile) {
    let full_path = file.name.as_deref().unwrap_or("Unknown");
    let display_name = full_path.rsplit('/').next().unwrap_or(full_path);
    let name_lower = display_name.to_lowercase();

    ui.label(egui::RichText::new(display_name).strong());
    ui.label(
        egui::RichText::new(format!("Path: {}", full_path))
            .small()
            .weak(),
    );
    ui.add_space(4.0);

    if let Some(ref content) = file.content {
        if content.is_empty() {
            ui.label(egui::RichText::new("File is empty").weak());
            return;
        }

        ui.label(
            egui::RichText::new(format!("{} bytes", content.len()))
                .weak()
                .small(),
        );
        ui.add_space(8.0);

        if name_lower.ends_with(".jpg")
            || name_lower.ends_with(".jpeg")
            || name_lower.ends_with(".png")
        {
            render_image_preview(ui, content, full_path);
        } else if name_lower.ends_with(".ldt") || name_lower.ends_with(".ies") {
            render_photometry_preview(ui, content, full_path);
        } else if name_lower.ends_with(".l3d") {
            render_l3d_preview(ui, content, full_path);
        } else if name_lower.ends_with(".xml") {
            ui.label(egui::RichText::new("XML File").color(egui::Color32::from_rgb(66, 133, 244)));
            ui.add_space(8.0);
            render_text_preview(ui, content);
        } else if name_lower.ends_with(".txt") || name_lower.ends_with(".json") {
            render_text_preview(ui, content);
        } else if name_lower.ends_with(".pdf") {
            render_pdf_preview(ui, content, full_path);
        } else {
            ui.label("Binary file");
            ui.add_space(8.0);
            render_hex_preview(ui, content);
        }
    } else {
        // URL reference - try to fetch
        render_url_file(ui, full_path, &name_lower);
    }
}

/// Render photometry (LDT/IES) files using eulumdat
fn render_photometry_preview(ui: &mut egui::Ui, content: &[u8], path: &str) {
    use eulumdat::{
        diagram::{PolarDiagram, SvgTheme},
        Eulumdat, IesParser,
    };

    ui.label(
        egui::RichText::new("☀️ Photometric Data").color(egui::Color32::from_rgb(251, 188, 5)),
    );
    ui.add_space(8.0);

    // Parse the photometric data
    let text = match std::str::from_utf8(content) {
        Ok(s) => s,
        Err(_) => {
            ui.label(
                egui::RichText::new("Error: Invalid UTF-8 encoding").color(egui::Color32::RED),
            );
            return;
        }
    };

    let ldt = if text.trim_start().starts_with("IESNA") {
        match IesParser::parse(text) {
            Ok(ldt) => ldt,
            Err(e) => {
                ui.label(
                    egui::RichText::new(format!("Error parsing IES: {:?}", e))
                        .color(egui::Color32::RED),
                );
                render_text_preview(ui, content);
                return;
            }
        }
    } else {
        match Eulumdat::parse(text) {
            Ok(ldt) => ldt,
            Err(e) => {
                ui.label(
                    egui::RichText::new(format!("Error parsing LDT: {:?}", e))
                        .color(egui::Color32::RED),
                );
                render_text_preview(ui, content);
                return;
            }
        }
    };

    // Show luminaire info
    ui.group(|ui| {
        ui.label(egui::RichText::new("Luminaire Information").strong());
        if !ldt.luminaire_name.is_empty() {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.label(&ldt.luminaire_name);
            });
        }
        if !ldt.luminaire_number.is_empty() {
            ui.horizontal(|ui| {
                ui.label("Number:");
                ui.label(&ldt.luminaire_number);
            });
        }
        ui.horizontal(|ui| {
            ui.label("Lumens:");
            ui.label(format!("{:.0} lm", ldt.total_luminous_flux()));
        });
    });

    ui.add_space(8.0);

    // Generate and render SVG diagram
    let cache_key = format!("ldt_{}", path);
    let svg_png = {
        let mut cache = SVG_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(&cache_key) {
            cached.clone()
        } else {
            // Generate SVG
            let theme = SvgTheme::dark();
            let diagram = PolarDiagram::from_eulumdat(&ldt);
            let svg_str = diagram.to_svg(400.0, 400.0, &theme);

            // Convert SVG to PNG using resvg
            match svg_to_png(&svg_str, 400, 400) {
                Ok(png_data) => {
                    cache.insert(cache_key.clone(), png_data.clone());
                    png_data
                }
                Err(e) => {
                    ui.label(
                        egui::RichText::new(format!("SVG render error: {}", e))
                            .color(egui::Color32::RED),
                    );
                    return;
                }
            }
        }
    };

    // Display the PNG
    let uri = format!("bytes://ldt/{}", path.replace('/', "_"));
    ui.ctx().include_bytes(uri.clone(), svg_png);
    ui.add(
        egui::Image::from_uri(&uri)
            .max_width(400.0)
            .max_height(400.0),
    );

    ui.add_space(8.0);
    ui.collapsing("Raw Data", |ui| {
        render_text_preview(ui, content);
    });
}

/// Convert SVG string to PNG bytes
fn svg_to_png(svg_str: &str, width: u32, height: u32) -> Result<Vec<u8>, String> {
    use resvg::tiny_skia::Pixmap;
    use resvg::usvg::{Options, Tree as UsvgTree};

    let opts = Options::default();
    let tree = UsvgTree::from_str(svg_str, &opts).map_err(|e| format!("SVG parse error: {}", e))?;

    let mut pixmap = Pixmap::new(width, height).ok_or("Failed to create pixmap")?;

    let scale_x = width as f32 / tree.size().width();
    let scale_y = height as f32 / tree.size().height();
    let scale = scale_x.min(scale_y);

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap
        .encode_png()
        .map_err(|e| format!("PNG encode error: {}", e))
}

/// Render L3D 3D model preview
fn render_l3d_preview(ui: &mut egui::Ui, content: &[u8], path: &str) {
    ui.label(egui::RichText::new("🧊 L3D 3D Model").color(egui::Color32::from_rgb(52, 168, 83)));
    ui.add_space(8.0);

    // Parse L3D and show info
    let l3d = l3d_rs::from_buffer(content);

    ui.group(|ui| {
        ui.label(egui::RichText::new("Model Information").strong());
        ui.horizontal(|ui| {
            ui.label("Parts:");
            ui.label(format!("{}", l3d.model.parts.len()));
        });
        ui.horizontal(|ui| {
            ui.label("Assets:");
            ui.label(format!("{}", l3d.file.assets.len()));
        });

        if !l3d.model.parts.is_empty() {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Parts:").small());
            for part in l3d.model.parts.iter().take(5) {
                ui.label(
                    egui::RichText::new(format!("  • {}", part.path))
                        .small()
                        .weak(),
                );
            }
            if l3d.model.parts.len() > 5 {
                ui.label(
                    egui::RichText::new(format!("  ... and {} more", l3d.model.parts.len() - 5))
                        .small()
                        .weak(),
                );
            }
        }
    });

    ui.add_space(16.0);

    // Open interactive 3D viewer button
    let content_for_viewer = content.to_vec();
    let display_name = path.rsplit('/').next().unwrap_or(path).to_string();
    if ui.button("Open Interactive 3D Viewer").clicked() {
        crate::l3d_render::open_l3d_viewer(content_for_viewer, &display_name);
    }
    ui.label(
        egui::RichText::new("Drag to rotate, scroll to zoom")
            .small()
            .weak(),
    );

    ui.add_space(8.0);
    ui.collapsing("Raw Hex Data", |ui| {
        render_hex_preview(ui, content);
    });
}

/// Render PDF preview with option to open in system viewer
fn render_pdf_preview(ui: &mut egui::Ui, content: &[u8], path: &str) {
    ui.label(egui::RichText::new("📕 PDF Document").color(egui::Color32::from_rgb(220, 53, 69)));
    ui.add_space(8.0);

    ui.label(
        egui::RichText::new(format!("{} bytes", content.len()))
            .weak()
            .small(),
    );
    ui.add_space(16.0);

    // Button to open in system PDF viewer
    let display_name = path.rsplit('/').next().unwrap_or(path);
    if ui.button("Open in System PDF Viewer").clicked() {
        open_in_system_viewer(content, display_name, "pdf");
    }

    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Click to open PDF in your default PDF viewer")
            .small()
            .weak(),
    );
}

/// Open file content in system's default viewer
fn open_in_system_viewer(content: &[u8], filename: &str, extension: &str) {
    use std::io::Write;

    // Create temp file
    let temp_dir = std::env::temp_dir();
    let safe_filename = filename.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
    let temp_file = temp_dir.join(format!(
        "gldf_{}_{}.{}",
        std::process::id(),
        safe_filename,
        extension
    ));

    // Write content to temp file
    match std::fs::File::create(&temp_file) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(content) {
                log::error!("Failed to write temp file: {}", e);
                return;
            }
        }
        Err(e) => {
            log::error!("Failed to create temp file: {}", e);
            return;
        }
    }

    // Open with system default application
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&temp_file).spawn();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", &temp_file.to_string_lossy()])
            .spawn();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&temp_file)
            .spawn();
    }
}

/// Render URL-referenced file (fetch from URL)
fn render_url_file(ui: &mut egui::Ui, path: &str, name_lower: &str) {
    ui.label(
        egui::RichText::new("🌐 External URL Reference")
            .color(egui::Color32::from_rgb(230, 126, 34)),
    );
    ui.add_space(4.0);

    // Check if path looks like a URL
    let is_url = path.starts_with("http://") || path.starts_with("https://");

    if !is_url {
        ui.label(egui::RichText::new("File referenced by path, not embedded").weak());
        ui.label(
            egui::RichText::new(format!("Path: {}", path))
                .small()
                .monospace(),
        );
        return;
    }

    // Check cache
    let cache_state = {
        let cache = URL_CACHE.lock().unwrap();
        cache.get(path).cloned()
    };

    match cache_state {
        Some(UrlFetchState::Loading) => {
            ui.spinner();
            ui.label("Loading...");
        }
        Some(UrlFetchState::Loaded(data)) => {
            ui.label(egui::RichText::new(format!("Downloaded: {} bytes", data.len())).small());
            ui.add_space(8.0);

            // Render based on file type
            if name_lower.ends_with(".ldt") || name_lower.ends_with(".ies") {
                render_photometry_preview(ui, &data, path);
            } else if name_lower.ends_with(".jpg")
                || name_lower.ends_with(".jpeg")
                || name_lower.ends_with(".png")
            {
                render_image_preview(ui, &data, path);
            } else if name_lower.ends_with(".l3d") {
                render_l3d_preview(ui, &data, path);
            } else if name_lower.ends_with(".pdf") {
                render_pdf_preview(ui, &data, path);
            } else {
                render_hex_preview(ui, &data);
            }
        }
        Some(UrlFetchState::Error(e)) => {
            ui.label(egui::RichText::new(format!("Error: {}", e)).color(egui::Color32::RED));
            if ui.button("Retry").clicked() {
                let mut cache = URL_CACHE.lock().unwrap();
                cache.remove(path);
            }
        }
        None => {
            // Auto-start fetch if it's a valid URL
            if path.starts_with("http://") || path.starts_with("https://") {
                let path_owned = path.to_string();
                {
                    let mut cache = URL_CACHE.lock().unwrap();
                    cache.insert(path_owned.clone(), UrlFetchState::Loading);
                }

                // Spawn blocking fetch
                std::thread::spawn(move || {
                    let result = fetch_url_blocking(&path_owned);
                    let mut cache = URL_CACHE.lock().unwrap();
                    match result {
                        Ok(data) => {
                            cache.insert(path_owned, UrlFetchState::Loaded(data));
                        }
                        Err(e) => {
                            cache.insert(path_owned, UrlFetchState::Error(e));
                        }
                    }
                });

                ui.spinner();
                ui.label("Starting download...");
            } else {
                ui.label(egui::RichText::new("File not embedded (path reference)").weak());
            }
            ui.label(egui::RichText::new(path).small().monospace());
        }
    }

    // Request repaint to update loading state
    ui.ctx().request_repaint();
}

fn fetch_url_blocking(url: &str) -> Result<Vec<u8>, String> {
    reqwest::blocking::get(url)
        .map_err(|e| format!("Network error: {}", e))?
        .bytes()
        .map(|b| b.to_vec())
        .map_err(|e| format!("Read error: {}", e))
}

fn render_image_preview(ui: &mut egui::Ui, content: &[u8], path: &str) {
    let uri = format!("bytes://gldf/{}", path.replace('/', "_"));
    ui.ctx().include_bytes(uri.clone(), content.to_vec());
    ui.add(
        egui::Image::from_uri(&uri)
            .max_width(ui.available_width().min(500.0))
            .max_height(400.0),
    );
}

fn render_text_preview(ui: &mut egui::Ui, content: &[u8]) {
    let text = String::from_utf8_lossy(content);
    let display_text = if text.len() > 30000 {
        format!(
            "{}...\n\n[Truncated - {} bytes total]",
            &text[..30000],
            text.len()
        )
    } else {
        text.to_string()
    };

    egui::ScrollArea::both()
        .id_salt("text_preview_scroll")
        .max_height(400.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut display_text.as_str())
                    .font(egui::TextStyle::Monospace)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(20),
            );
        });
}

fn render_hex_preview(ui: &mut egui::Ui, content: &[u8]) {
    ui.label(egui::RichText::new("Hex preview (first 512 bytes):").small());
    let hex: String = content
        .iter()
        .take(512)
        .enumerate()
        .map(|(i, b)| {
            if i > 0 && i % 16 == 0 {
                format!("\n{:02x} ", b)
            } else {
                format!("{:02x} ", b)
            }
        })
        .collect();

    egui::ScrollArea::vertical()
        .id_salt("hex_preview_scroll")
        .max_height(200.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut hex.as_str())
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(10),
            );
        });
}

fn get_file_icon(name: &str) -> &'static str {
    let name_lower = name.to_lowercase();
    if name_lower.ends_with(".jpg") || name_lower.ends_with(".jpeg") || name_lower.ends_with(".png")
    {
        "🖼"
    } else if name_lower.ends_with(".ldt") || name_lower.ends_with(".ies") {
        "☀️"
    } else if name_lower.ends_with(".l3d") {
        "🧊"
    } else if name_lower.ends_with(".xml") {
        "📝"
    } else if name_lower.ends_with(".pdf") {
        "📕"
    } else {
        "📄"
    }
}

fn empty_state(ui: &mut egui::Ui, icon: &str, title: &str, message: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(80.0);
        ui.label(egui::RichText::new(icon).size(48.0));
        ui.add_space(16.0);
        ui.heading(title);
        ui.label(egui::RichText::new(message).weak());
    });
}
