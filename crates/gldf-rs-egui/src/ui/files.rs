//! Files list view

use crate::app::{FileCategory, GldfViewerApp};
use eframe::egui;

pub fn render_files(ui: &mut egui::Ui, app: &mut GldfViewerApp) {
    ui.heading("Files");
    ui.add_space(16.0);

    if app.files_list.is_empty() {
        empty_state(
            ui,
            "📁",
            "No Files",
            "Load a GLDF file to view file definitions",
        );
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Table header
        egui::Grid::new("files_table")
            .num_columns(4)
            .striped(true)
            .min_col_width(80.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("ID").strong());
                ui.label(egui::RichText::new("File Name").strong());
                ui.label(egui::RichText::new("Content Type").strong());
                ui.label(egui::RichText::new("Category").strong());
                ui.end_row();

                for file in &app.files_list {
                    let icon = match file.category {
                        FileCategory::Photometry => "☀️",
                        FileCategory::Image => "🖼",
                        FileCategory::Geometry => "🧊",
                        FileCategory::Document => "📄",
                        FileCategory::Other => "📎",
                    };

                    // Make row clickable
                    let is_selected = app.selected_file_id.as_ref() == Some(&file.id);

                    if ui
                        .selectable_label(is_selected, egui::RichText::new(&file.id).monospace())
                        .clicked()
                    {
                        app.selected_file_id = Some(file.id.clone());
                    }

                    ui.label(&file.name);
                    ui.label(&file.content_type);
                    ui.label(format!("{} {:?}", icon, file.category));
                    ui.end_row();
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
