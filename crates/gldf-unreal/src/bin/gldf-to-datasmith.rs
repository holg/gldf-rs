//! CLI entry point for the exporter.
//!
//! Phase 1 prints a placeholder; Phase 4 wires up `clap` flags and calls
//! [`gldf_unreal::export_gldf_to_datasmith`].

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("gldf-to-datasmith {} (phase-1 stub)", env!("CARGO_PKG_VERSION"));
    eprintln!("Phase 4 wires this up. For now, use the Rust API directly:");
    eprintln!("  gldf_unreal::Exporter::from_path(...)?.export(&opts)?;");
    ExitCode::SUCCESS
}
