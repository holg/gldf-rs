//! Light sources view

use crate::app::GldfViewerApp;
use eframe::egui;

pub fn render_light_sources(ui: &mut egui::Ui, app: &GldfViewerApp) {
    ui.heading("Light Sources");
    ui.add_space(16.0);

    let Some(ref gldf) = app.loaded_gldf else {
        empty_state(
            ui,
            "💡",
            "No Light Sources",
            "Load a GLDF file to view light sources",
        );
        return;
    };

    let Some(ref ls_container) = gldf.gldf.general_definitions.light_sources else {
        empty_state(
            ui,
            "💡",
            "No Light Sources",
            "This GLDF file has no light source definitions",
        );
        return;
    };

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Fixed Light Sources
        if !ls_container.fixed_light_source.is_empty() {
            ui.heading(egui::RichText::new("Fixed Light Sources").size(16.0));
            ui.add_space(8.0);

            for ls in &ls_container.fixed_light_source {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&ls.id).monospace().weak());
                        ui.label(
                            egui::RichText::new("Fixed")
                                .small()
                                .color(egui::Color32::from_rgb(251, 188, 5)),
                        );
                    });

                    let name = ls
                        .name
                        .locale
                        .first()
                        .map(|l| l.value.as_str())
                        .unwrap_or("");
                    ui.label(egui::RichText::new(name).strong());

                    if let Some(ref desc) = ls.description {
                        if let Some(locale) = desc.locale.first() {
                            if !locale.value.is_empty() {
                                ui.label(egui::RichText::new(&locale.value).weak());
                            }
                        }
                    }

                    // Show some technical details
                    ui.add_space(4.0);
                    egui::Grid::new(format!("fixed_ls_{}", ls.id))
                        .num_columns(2)
                        .spacing([20.0, 4.0])
                        .show(ui, |ui| {
                            if let Some(power) = ls.rated_input_power {
                                ui.label("Rated Input Power:");
                                ui.label(format!("{:.1} W", power));
                                ui.end_row();
                            }
                            if let Some(color_temp) = ls
                                .color_information
                                .as_ref()
                                .and_then(|c| c.color_temperature_adjusting_range.as_ref())
                            {
                                if let (Some(lower), Some(upper)) =
                                    (color_temp.lower, color_temp.upper)
                                {
                                    ui.label("Color Temp Range:");
                                    ui.label(format!("{} - {} K", lower, upper));
                                    ui.end_row();
                                }
                            }
                        });
                });
                ui.add_space(8.0);
            }
        }

        // Changeable Light Sources
        if !ls_container.changeable_light_source.is_empty() {
            ui.add_space(16.0);
            ui.heading(egui::RichText::new("Changeable Light Sources").size(16.0));
            ui.add_space(8.0);

            for ls in &ls_container.changeable_light_source {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&ls.id).monospace().weak());
                        ui.label(
                            egui::RichText::new("Changeable")
                                .small()
                                .color(egui::Color32::from_rgb(230, 126, 34)),
                        );
                    });

                    ui.label(egui::RichText::new(&ls.name.value).strong());

                    if let Some(ref desc) = ls.description {
                        if !desc.value.is_empty() {
                            ui.label(egui::RichText::new(&desc.value).weak());
                        }
                    }
                });
                ui.add_space(8.0);
            }
        }

        // Empty state if both are empty
        if ls_container.fixed_light_source.is_empty()
            && ls_container.changeable_light_source.is_empty()
        {
            empty_state(
                ui,
                "💡",
                "No Light Sources",
                "This GLDF file has no light source definitions",
            );
        }
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
