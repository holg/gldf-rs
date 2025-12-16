//! Overview view showing product summary

use crate::app::{FileCategory, GldfViewerApp, NavItem};
use eframe::egui;

/// Returns Some(file_name) if a file was clicked
pub fn render_overview(ui: &mut egui::Ui, app: &mut GldfViewerApp) {
    let mut clicked_file: Option<String> = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        if let Some(header) = app.header() {
            // Product Info Card
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("💡").size(32.0));
                    ui.vertical(|ui| {
                        let manufacturer = if header.manufacturer.is_empty() {
                            "Unknown Manufacturer"
                        } else {
                            &header.manufacturer
                        };
                        ui.heading(manufacturer);
                        ui.label(
                            egui::RichText::new(format!(
                                "GLDF Format {}",
                                header.format_version.to_version_string()
                            ))
                            .weak(),
                        );
                    });
                });

                ui.add_space(12.0);

                egui::Grid::new("header_info").striped(true).show(ui, |ui| {
                    ui.label("Author");
                    ui.label(if header.author.is_empty() {
                        "—"
                    } else {
                        &header.author
                    });
                    ui.end_row();

                    ui.label("Created With");
                    ui.label(if header.created_with_application.is_empty() {
                        "—"
                    } else {
                        &header.created_with_application
                    });
                    ui.end_row();

                    ui.label("Creation Date");
                    ui.label(if header.creation_time_code.is_empty() {
                        "—"
                    } else {
                        &header.creation_time_code
                    });
                    ui.end_row();
                });
            });

            ui.add_space(16.0);

            // Statistics Grid
            ui.heading("Statistics");
            ui.add_space(8.0);

            egui::Grid::new("stats_grid")
                .num_columns(4)
                .spacing([20.0, 20.0])
                .show(ui, |ui| {
                    stat_card(
                        ui,
                        "📁",
                        &app.stats.files_count.to_string(),
                        "Files",
                        egui::Color32::from_rgb(66, 133, 244),
                    );
                    stat_card(
                        ui,
                        "💡",
                        &(app.stats.fixed_light_sources + app.stats.changeable_light_sources)
                            .to_string(),
                        "Light Sources",
                        egui::Color32::from_rgb(251, 188, 5),
                    );
                    stat_card(
                        ui,
                        "📦",
                        &app.stats.variants_count.to_string(),
                        "Variants",
                        egui::Color32::from_rgb(155, 89, 182),
                    );
                    stat_card(
                        ui,
                        "☀️",
                        &app.stats.photometries_count.to_string(),
                        "Photometries",
                        egui::Color32::from_rgb(230, 126, 34),
                    );
                    ui.end_row();
                });

            ui.add_space(24.0);

            // Files Overview
            ui.collapsing(egui::RichText::new("📁 Files").heading(), |ui| {
                if let Some(file) = render_files_overview(ui, app) {
                    clicked_file = Some(file);
                }
            });

            ui.add_space(12.0);

            // Light Sources Overview
            ui.collapsing(egui::RichText::new("💡 Light Sources").heading(), |ui| {
                render_light_sources_overview(ui, app);
            });

            ui.add_space(12.0);

            // Variants Overview
            ui.collapsing(egui::RichText::new("📦 Variants").heading(), |ui| {
                render_variants_overview(ui, app);
            });
        }
    });

    // Handle file click - navigate to file viewer
    if let Some(file_name) = clicked_file {
        app.selected_file_id = Some(file_name);
        app.nav_item = NavItem::FileViewer;
    }
}

fn stat_card(ui: &mut egui::Ui, icon: &str, value: &str, label: &str, color: egui::Color32) {
    ui.group(|ui| {
        ui.set_min_width(100.0);
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new(icon).size(24.0).color(color));
            ui.label(egui::RichText::new(value).size(28.0).strong());
            ui.label(egui::RichText::new(label).weak());
        });
    });
}

fn render_files_overview(ui: &mut egui::Ui, app: &GldfViewerApp) -> Option<String> {
    let mut clicked: Option<String> = None;

    let photometric: Vec<_> = app
        .files_list
        .iter()
        .filter(|f| f.category == FileCategory::Photometry)
        .collect();
    let images: Vec<_> = app
        .files_list
        .iter()
        .filter(|f| f.category == FileCategory::Image)
        .collect();
    let geometry: Vec<_> = app
        .files_list
        .iter()
        .filter(|f| f.category == FileCategory::Geometry)
        .collect();

    if !photometric.is_empty() {
        ui.label(egui::RichText::new("Photometric").strong());
        for f in photometric.iter().take(3) {
            ui.horizontal(|ui| {
                ui.label("☀️");
                if ui.link(&f.name).clicked() {
                    clicked = Some(f.name.clone());
                }
            });
        }
        if photometric.len() > 3 {
            ui.label(egui::RichText::new(format!("+ {} more...", photometric.len() - 3)).weak());
        }
        ui.add_space(8.0);
    }

    if !images.is_empty() {
        ui.label(egui::RichText::new("Images").strong());
        for f in images.iter().take(3) {
            ui.horizontal(|ui| {
                ui.label("🖼");
                if ui.link(&f.name).clicked() {
                    clicked = Some(f.name.clone());
                }
            });
        }
        if images.len() > 3 {
            ui.label(egui::RichText::new(format!("+ {} more...", images.len() - 3)).weak());
        }
        ui.add_space(8.0);
    }

    if !geometry.is_empty() {
        ui.label(egui::RichText::new("Geometry").strong());
        for f in geometry.iter().take(3) {
            ui.horizontal(|ui| {
                ui.label("🧊");
                if ui.link(&f.name).clicked() {
                    clicked = Some(f.name.clone());
                }
            });
        }
        if geometry.len() > 3 {
            ui.label(egui::RichText::new(format!("+ {} more...", geometry.len() - 3)).weak());
        }
    }

    clicked
}

fn render_light_sources_overview(ui: &mut egui::Ui, app: &GldfViewerApp) {
    if let Some(ref gldf) = app.loaded_gldf {
        if let Some(ref ls_container) = gldf.gldf.general_definitions.light_sources {
            if !ls_container.fixed_light_source.is_empty() {
                ui.label(egui::RichText::new("Fixed").strong());
                for ls in ls_container.fixed_light_source.iter().take(5) {
                    let name = ls
                        .name
                        .locale
                        .first()
                        .map(|l| l.value.as_str())
                        .unwrap_or(&ls.id);
                    ui.horizontal(|ui| {
                        ui.label("💡");
                        ui.vertical(|ui| {
                            ui.label(name);
                            ui.label(egui::RichText::new(format!("ID: {}", ls.id)).weak().small());
                        });
                    });
                }
                if ls_container.fixed_light_source.len() > 5 {
                    ui.label(
                        egui::RichText::new(format!(
                            "+ {} more...",
                            ls_container.fixed_light_source.len() - 5
                        ))
                        .weak(),
                    );
                }
            }

            if !ls_container.changeable_light_source.is_empty() {
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Changeable").strong());
                for ls in ls_container.changeable_light_source.iter().take(5) {
                    ui.horizontal(|ui| {
                        ui.label("💡");
                        ui.vertical(|ui| {
                            ui.label(&ls.name.value);
                            ui.label(egui::RichText::new(format!("ID: {}", ls.id)).weak().small());
                        });
                    });
                }
                if ls_container.changeable_light_source.len() > 5 {
                    ui.label(
                        egui::RichText::new(format!(
                            "+ {} more...",
                            ls_container.changeable_light_source.len() - 5
                        ))
                        .weak(),
                    );
                }
            }
        }
    }
}

fn render_variants_overview(ui: &mut egui::Ui, app: &GldfViewerApp) {
    if let Some(ref gldf) = app.loaded_gldf {
        if let Some(ref variants_container) = gldf.gldf.product_definitions.variants {
            for variant in variants_container.variant.iter().take(10) {
                let name = variant
                    .name
                    .as_ref()
                    .and_then(|n| n.locale.first())
                    .map(|l| l.value.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&variant.id);

                ui.horizontal(|ui| {
                    ui.label("📦");
                    ui.vertical(|ui| {
                        ui.label(name);
                        ui.label(
                            egui::RichText::new(format!("ID: {}", variant.id))
                                .weak()
                                .small(),
                        );
                    });
                });
            }
            if variants_container.variant.len() > 10 {
                ui.label(
                    egui::RichText::new(format!(
                        "+ {} more...",
                        variants_container.variant.len() - 10
                    ))
                    .weak(),
                );
            }
        }
    }
}
