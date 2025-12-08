//! GLDF Viewer - Cross-platform GUI application using egui
//!
//! This crate provides a GLDF file viewer that works on:
//! - Native: Windows, macOS, Linux

mod app;
pub mod l3d_render;
mod ui;

pub use app::GldfViewerApp;
