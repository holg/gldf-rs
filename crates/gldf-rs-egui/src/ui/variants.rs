//! Variants view

use crate::app::GldfViewerApp;
use eframe::egui;

pub fn render_variants(ui: &mut egui::Ui, app: &GldfViewerApp) {
    ui.heading("Variants");
    ui.add_space(16.0);

    let Some(ref gldf) = app.loaded_gldf else {
        empty_state(ui, "📦", "No Variants", "Load a GLDF file to view variants");
        return;
    };

    let Some(ref variants_container) = gldf.gldf.product_definitions.variants else {
        empty_state(
            ui,
            "📦",
            "No Variants",
            "This GLDF file has no variant definitions",
        );
        return;
    };

    if variants_container.variant.is_empty() {
        empty_state(
            ui,
            "📦",
            "No Variants",
            "This GLDF file has no variant definitions",
        );
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for variant in &variants_container.variant {
            ui.group(|ui| {
                // Header row
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&variant.id).monospace().weak());
                    if let Some(order) = variant.sort_order.filter(|&o| o > 0) {
                        ui.label(egui::RichText::new(format!("#{}", order)).small().weak());
                    }
                });

                // Name
                let name = variant
                    .name
                    .as_ref()
                    .and_then(|n| n.locale.first())
                    .map(|l| l.value.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&variant.id);
                ui.label(egui::RichText::new(name).strong().size(16.0));

                // Description
                if let Some(ref desc) = variant.description {
                    if let Some(locale) = desc.locale.first() {
                        if !locale.value.is_empty() {
                            ui.label(egui::RichText::new(&locale.value).weak());
                        }
                    }
                }

                // Product Number
                if let Some(ref pn) = variant.product_number {
                    if let Some(locale) = pn.locale.first() {
                        if !locale.value.is_empty() {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label("Product #:");
                                ui.label(egui::RichText::new(&locale.value).monospace());
                            });
                        }
                    }
                }

                // GTIN if available
                if let Some(ref gtin) = variant.product_number {
                    if let Some(locale) = gtin.locale.first() {
                        if !locale.value.is_empty() {
                            ui.horizontal(|ui| {
                                ui.label("Product:");
                                ui.label(&locale.value);
                            });
                        }
                    }
                }
            });
            ui.add_space(8.0);
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
