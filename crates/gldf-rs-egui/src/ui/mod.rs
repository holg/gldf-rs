//! UI components for the GLDF Viewer

pub mod file_viewer;
mod files;
mod header;
mod light_sources;
mod overview;
mod raw_data;
mod sidebar;
mod statistics;
mod variants;
mod welcome;

use crate::app::{GldfViewerApp, NavItem};
use eframe::egui;

/// Main UI rendering function
pub fn render_ui(ctx: &egui::Context, app: &mut GldfViewerApp) {
    // Top panel with menu bar
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        render_menu_bar(ui, app);
    });

    // Bottom panel with status bar
    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
        render_status_bar(ui, app);
    });

    // Side panel with navigation
    if app.sidebar_expanded {
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(200.0)
            .min_width(150.0)
            .max_width(400.0)
            .show(ctx, |ui| {
                sidebar::render_sidebar(ui, app);
            });
    }

    // Central panel with main content
    egui::CentralPanel::default().show(ctx, |ui| {
        if app.has_file() {
            render_content(ui, app);
        } else {
            welcome::render_welcome(ui, app);
        }
    });
}

/// Render the menu bar
fn render_menu_bar(ui: &mut egui::Ui, app: &mut GldfViewerApp) {
    ui.horizontal(|ui| {
        ui.menu_button("File", |ui| {
            #[cfg(not(target_arch = "wasm32"))]
            if ui.button("Open...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("GLDF Files", &["gldf"])
                    .add_filter("All Files", &["*"])
                    .pick_file()
                {
                    app.load_from_path(path);
                }
                ui.close();
            }

            #[cfg(not(target_arch = "wasm32"))]
            if app.has_file() {
                ui.separator();
                if ui.button("Export JSON...").clicked() {
                    app.export_json();
                    ui.close();
                }
                if ui.button("Export XML...").clicked() {
                    app.export_xml();
                    ui.close();
                }
            }

            ui.separator();
            #[cfg(not(target_arch = "wasm32"))]
            if ui.button("Quit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });

        ui.menu_button("View", |ui| {
            if ui
                .checkbox(&mut app.sidebar_expanded, "Show Sidebar")
                .changed()
            {
                ui.close();
            }
            ui.separator();
            if ui.checkbox(&mut app.dark_mode, "Dark Mode").changed() {
                ui.close();
            }
        });

        ui.menu_button("Help", |ui| {
            if ui.button("About").clicked() {
                app.status_message = format!("GLDF Viewer v{}", env!("CARGO_PKG_VERSION"));
                ui.close();
            }
            ui.separator();
            if ui.hyperlink_to("GLDF.io", "https://gldf.io").clicked() {
                ui.close();
            }
            if ui
                .hyperlink_to("GitHub", "https://github.com/holg/gldf-rs")
                .clicked()
            {
                ui.close();
            }
        });

        // Right side: file name and toggle button
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !app.sidebar_expanded && ui.button("☰").clicked() {
                app.sidebar_expanded = true;
            }

            if let Some(ref name) = app.current_file_name {
                ui.label(egui::RichText::new(name).weak());
            }
        });
    });
}

/// Render the status bar
fn render_status_bar(ui: &mut egui::Ui, app: &GldfViewerApp) {
    ui.horizontal(|ui| {
        if !app.status_message.is_empty() {
            ui.label(&app.status_message);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new("All processing is local")
                    .weak()
                    .small(),
            );
            ui.separator();
            if app.has_file() {
                ui.label(format!("{} files", app.stats.files_count));
            }
        });
    });
}

/// Render main content based on navigation
fn render_content(ui: &mut egui::Ui, app: &mut GldfViewerApp) {
    match app.nav_item {
        NavItem::Overview => overview::render_overview(ui, app),
        NavItem::RawData => raw_data::render_raw_data(ui, app),
        NavItem::FileViewer => file_viewer::render_file_viewer(ui, app),
        NavItem::Header => header::render_header(ui, app),
        NavItem::Statistics => statistics::render_statistics(ui, app),
        NavItem::Files => files::render_files(ui, app),
        NavItem::LightSources => light_sources::render_light_sources(ui, app),
        NavItem::Variants => variants::render_variants(ui, app),
    }
}

/// Handle file drops
pub fn handle_file_drops(ctx: &egui::Context, app: &mut GldfViewerApp) {
    // Check for dropped files
    ctx.input(|i| {
        if !i.raw.dropped_files.is_empty() {
            for file in &i.raw.dropped_files {
                if let Some(ref path) = file.path {
                    if path.extension().map(|e| e == "gldf").unwrap_or(false) {
                        #[cfg(not(target_arch = "wasm32"))]
                        app.load_from_path(path.clone());
                    }
                } else if let Some(ref bytes) = file.bytes {
                    // WASM path: file dropped with bytes
                    let name = file.name.clone();
                    if name.ends_with(".gldf") {
                        app.load_from_bytes(bytes.to_vec(), Some(name));
                    }
                }
            }
        }

        // Update dragging state
        app.is_dragging = !i.raw.hovered_files.is_empty();
    });
}
