//! `gldf-batch-egui` — desktop batch converter: many .ldt/.ies + one
//! JSON template → GLDF / IFC per input. UI shell over
//! `gldf_rs_editor::batch` (same engine as `gldf-cli batch`).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 640.0])
            .with_min_inner_size([680.0, 420.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "GLDF Batch Converter",
        native_options,
        Box::new(|cc| Ok(Box::new(gldf_egui::BatchApp::new(cc)))),
    )
}
