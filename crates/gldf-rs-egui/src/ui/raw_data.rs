//! Raw data view (JSON)

use crate::app::GldfViewerApp;
use eframe::egui;

pub fn render_raw_data(ui: &mut egui::Ui, app: &GldfViewerApp) {
    ui.horizontal(|ui| {
        ui.heading("Raw Data");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if app.has_file() {
                if ui.button("Copy").clicked() {
                    if let Some(ref json) = app.raw_json {
                        ui.ctx().copy_text(json.clone());
                    }
                }

                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("Export JSON...").clicked() {
                    app.export_json();
                }
            }
        });
    });

    ui.add_space(16.0);

    if let Some(ref json) = app.raw_json {
        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut json.as_str())
                        .font(egui::TextStyle::Monospace)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(40),
                );
            });
    } else {
        empty_state(
            ui,
            "{ }",
            "No Raw Data",
            "Load a GLDF file to view raw data",
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
