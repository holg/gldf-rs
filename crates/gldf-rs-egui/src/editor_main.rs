//! `gldf-editor-egui` — native GLDF editor. Same reducer as the WASM /
//! TUI editors, egui widgets, no three-d dependency (cross-buildable).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1150.0, 760.0])
            .with_min_inner_size([820.0, 520.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "GLDF Editor",
        native_options,
        Box::new(|cc| {
            // Image loaders power the picture previews on the Files page.
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let mut app = gldf_egui::editor::GldfEditorApp::new(cc);
            // `gldf-editor-egui path/to.gldf` opens straight into it.
            if let Some(path) = std::env::args().nth(1) {
                app.open_path(path.into());
            }
            Ok(Box::new(app))
        }),
    )
}
