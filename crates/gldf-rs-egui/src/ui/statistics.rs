//! Statistics view

use crate::app::GldfViewerApp;
use eframe::egui;

pub fn render_statistics(ui: &mut egui::Ui, app: &GldfViewerApp) {
    ui.heading("Statistics");
    ui.add_space(16.0);

    if !app.has_file() {
        empty_state(
            ui,
            "📊",
            "No Statistics",
            "Load a GLDF file to view statistics",
        );
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Files Section
        ui.group(|ui| {
            ui.heading("Files");
            ui.add_space(8.0);
            stat_row(ui, "Total Files", &app.stats.files_count.to_string(), false);
        });

        ui.add_space(16.0);

        // Light Sources Section
        ui.group(|ui| {
            ui.heading("Light Sources");
            ui.add_space(8.0);
            stat_row(
                ui,
                "Fixed Light Sources",
                &app.stats.fixed_light_sources.to_string(),
                false,
            );
            stat_row(
                ui,
                "Changeable Light Sources",
                &app.stats.changeable_light_sources.to_string(),
                false,
            );
            ui.separator();
            let total_ls = app.stats.fixed_light_sources + app.stats.changeable_light_sources;
            stat_row(ui, "Total Light Sources", &total_ls.to_string(), true);
        });

        ui.add_space(16.0);

        // Product Definitions Section
        ui.group(|ui| {
            ui.heading("Product Definitions");
            ui.add_space(8.0);
            stat_row(ui, "Variants", &app.stats.variants_count.to_string(), false);
        });

        ui.add_space(16.0);

        // Photometry & Geometry Section
        ui.group(|ui| {
            ui.heading("Photometry & Geometry");
            ui.add_space(8.0);
            stat_row(
                ui,
                "Photometries",
                &app.stats.photometries_count.to_string(),
                false,
            );
            stat_row(
                ui,
                "Geometries",
                &app.stats.geometries_count.to_string(),
                false,
            );
        });
    });
}

fn stat_row(ui: &mut egui::Ui, label: &str, value: &str, is_total: bool) {
    ui.horizontal(|ui| {
        if is_total {
            ui.label(egui::RichText::new(label).strong());
        } else {
            ui.label(egui::RichText::new(label).weak());
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if is_total {
                ui.label(egui::RichText::new(value).monospace().strong());
            } else {
                ui.label(egui::RichText::new(value).monospace());
            }
        });
    });
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
