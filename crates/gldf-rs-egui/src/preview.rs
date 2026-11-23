//! Embedded-file preview renderers (photometry polar diagram, images,
//! L3D, text/hex, URL fetch). Shared by the editor's Files /
//! Photometries pages; formerly part of the retired gldf-viewer-egui
//! app's file viewer.

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

/// Render photometry (LDT/IES) files using eulumdat
pub(crate) fn render_photometry_preview(ui: &mut egui::Ui, content: &[u8], path: &str) {
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
            ui.label(format!("{:.0} lm", ldt.luminous_flux()));
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
pub(crate) fn render_l3d_preview(ui: &mut egui::Ui, content: &[u8], path: &str) {
    // `path` only feeds the viewer3d button's window title below.
    #[cfg(not(feature = "viewer3d"))]
    let _ = path;
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

    // Open interactive 3D viewer button (three-d based; compiled out of
    // binaries built without the `viewer3d` feature, e.g. the batch app).
    #[cfg(feature = "viewer3d")]
    {
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
    }

    ui.add_space(8.0);
    ui.collapsing("Raw Hex Data", |ui| {
        render_hex_preview(ui, content);
    });
}

/// Render PDF preview with option to open in system viewer
pub(crate) fn render_pdf_preview(ui: &mut egui::Ui, content: &[u8], path: &str) {
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
pub(crate) fn render_url_file(ui: &mut egui::Ui, path: &str, name_lower: &str) {
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

pub(crate) fn render_image_preview(ui: &mut egui::Ui, content: &[u8], path: &str) {
    let uri = format!("bytes://gldf/{}", path.replace('/', "_"));
    ui.ctx().include_bytes(uri.clone(), content.to_vec());
    ui.add(
        egui::Image::from_uri(&uri)
            .max_width(ui.available_width().min(500.0))
            .max_height(400.0),
    );
}

pub(crate) fn render_text_preview(ui: &mut egui::Ui, content: &[u8]) {
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

pub(crate) fn render_hex_preview(ui: &mut egui::Ui, content: &[u8]) {
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
