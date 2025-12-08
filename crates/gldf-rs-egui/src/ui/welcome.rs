//! Welcome view for when no file is loaded

use crate::app::GldfViewerApp;
use eframe::egui;

pub fn render_welcome(ui: &mut egui::Ui, app: &mut GldfViewerApp) {
    // Center the content
    ui.vertical_centered(|ui| {
        ui.add_space(80.0);

        // Icon
        ui.label(egui::RichText::new("💡").size(64.0));
        ui.add_space(16.0);

        // Title
        ui.heading("GLDF Viewer");
        ui.label(egui::RichText::new("Global Lighting Data Format").weak());

        ui.add_space(24.0);

        // Library version
        ui.label(
            egui::RichText::new(format!("Library Version: {}", env!("CARGO_PKG_VERSION")))
                .small()
                .weak(),
        );

        ui.add_space(32.0);
        ui.separator();
        ui.add_space(32.0);

        // Drop zone indicator
        if app.is_dragging {
            ui.label(
                egui::RichText::new("Drop GLDF file here")
                    .size(20.0)
                    .color(egui::Color32::from_rgb(100, 200, 255)),
            );
        } else {
            ui.label("Drop a GLDF file here");
            ui.add_space(8.0);
            ui.label(egui::RichText::new("or").weak());
            ui.add_space(16.0);

            // Open file button
            if ui.button("Open File...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("GLDF Files", &["gldf"])
                    .add_filter("All Files", &["*"])
                    .pick_file()
                {
                    app.load_from_path(path);
                }
            }

            ui.add_space(16.0);

            // Demo button
            if ui.button("Load Demo").clicked() {
                // Load embedded demo file or fetch from URL
                load_demo(app);
            }
        }

        ui.add_space(40.0);

        // Privacy note
        ui.label(
            egui::RichText::new("All processing happens locally - no data is uploaded")
                .small()
                .weak(),
        );
    });
}

fn load_demo(app: &mut GldfViewerApp) {
    // Embed the demo GLDF file
    let demo_data = include_bytes!("../../assets/demo.gldf");
    app.load_from_bytes(demo_data.to_vec(), Some("demo.gldf".to_string()));
}
