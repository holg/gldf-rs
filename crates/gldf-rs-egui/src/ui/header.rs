//! Header information view

use crate::app::GldfViewerApp;
use eframe::egui;

pub fn render_header(ui: &mut egui::Ui, app: &GldfViewerApp) {
    ui.heading("Header Information");
    ui.add_space(16.0);

    if let Some(header) = app.header() {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Header Info Group
            ui.group(|ui| {
                ui.heading("General");
                ui.add_space(8.0);

                egui::Grid::new("header_grid")
                    .num_columns(2)
                    .spacing([20.0, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Manufacturer").strong());
                        ui.label(if header.manufacturer.is_empty() {
                            "—"
                        } else {
                            &header.manufacturer
                        });
                        ui.end_row();

                        ui.label(egui::RichText::new("Author").strong());
                        ui.label(if header.author.is_empty() {
                            "—"
                        } else {
                            &header.author
                        });
                        ui.end_row();

                        ui.label(egui::RichText::new("Created With").strong());
                        ui.label(if header.created_with_application.is_empty() {
                            "—"
                        } else {
                            &header.created_with_application
                        });
                        ui.end_row();

                        ui.label(egui::RichText::new("Creation Time").strong());
                        ui.label(if header.creation_time_code.is_empty() {
                            "—"
                        } else {
                            &header.creation_time_code
                        });
                        ui.end_row();
                    });
            });

            ui.add_space(16.0);

            // Format Version Group
            ui.group(|ui| {
                ui.heading("Format Version");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Version:");
                    ui.label(
                        egui::RichText::new(header.format_version.to_version_string()).monospace(),
                    );
                });
            });

            // License/Copyright if available
            if let Some(ref license_keys) = header.license_keys {
                if !license_keys.license_key.is_empty() {
                    ui.add_space(16.0);
                    ui.group(|ui| {
                        ui.heading("License Keys");
                        ui.add_space(8.0);
                        for (i, key) in license_keys.license_key.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}.", i + 1));
                                ui.label(egui::RichText::new(&key.application).monospace());
                                ui.label(":");
                                ui.label(&key.license_key);
                            });
                        }
                    });
                }
            }
        });
    } else {
        empty_state(
            ui,
            "📄",
            "No Header Data",
            "Load a GLDF file to view header information",
        );
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
