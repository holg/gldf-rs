//! Sidebar navigation

use crate::app::{GldfViewerApp, NavItem};
use eframe::egui;

pub fn render_sidebar(ui: &mut egui::Ui, app: &mut GldfViewerApp) {
    ui.vertical(|ui| {
        // Header
        ui.horizontal(|ui| {
            ui.heading("GLDF Viewer");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("×").clicked() {
                    app.sidebar_expanded = false;
                }
            });
        });
        ui.separator();

        let has_file = app.has_file();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Viewer section
            ui.label(egui::RichText::new("Viewer").strong().small());
            ui.add_space(4.0);
            nav_item(ui, app, NavItem::Overview, "📊", "Overview", None, has_file);
            nav_item(ui, app, NavItem::RawData, "{ }", "Raw Data", None, has_file);
            nav_item(
                ui,
                app,
                NavItem::FileViewer,
                "👁",
                "File Viewer",
                Some(app.embedded_files().len()),
                has_file,
            );

            ui.add_space(12.0);

            // Document section
            ui.label(egui::RichText::new("Document").strong().small());
            ui.add_space(4.0);
            nav_item(ui, app, NavItem::Header, "📄", "Header", None, has_file);
            nav_item(
                ui,
                app,
                NavItem::Statistics,
                "📈",
                "Statistics",
                None,
                has_file,
            );

            ui.add_space(12.0);

            // Definitions section
            ui.label(egui::RichText::new("Definitions").strong().small());
            ui.add_space(4.0);
            nav_item(
                ui,
                app,
                NavItem::Files,
                "📁",
                "Files",
                Some(app.files_list.len()),
                has_file,
            );
            nav_item(
                ui,
                app,
                NavItem::LightSources,
                "💡",
                "Light Sources",
                Some(app.stats.fixed_light_sources + app.stats.changeable_light_sources),
                has_file,
            );
            nav_item(
                ui,
                app,
                NavItem::Variants,
                "📦",
                "Variants",
                Some(app.stats.variants_count),
                has_file,
            );

            // Bottom section with links
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(8.0);

            ui.label(egui::RichText::new("Resources").small().weak());
            ui.hyperlink_to("GLDF.io", "https://gldf.io");
            ui.hyperlink_to("QLumEdit", "https://eulumdat.icu");
        });
    });
}

fn nav_item(
    ui: &mut egui::Ui,
    app: &mut GldfViewerApp,
    item: NavItem,
    icon: &str,
    label: &str,
    badge: Option<usize>,
    enabled: bool,
) {
    let is_active = app.nav_item == item;

    ui.add_enabled_ui(enabled, |ui| {
        let response = ui.selectable_label(is_active, format!("{} {}", icon, label));

        if let Some(count) = badge {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!("{}", count)).weak().small());
            });
        }

        if response.clicked() && enabled {
            app.nav_item = item;
        }
    });
}
