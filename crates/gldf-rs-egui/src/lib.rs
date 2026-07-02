//! GLDF Viewer - Cross-platform GUI application using egui
//!
//! This crate provides a GLDF file viewer that works on:
//! - Native: Windows, macOS, Linux

mod app;
pub mod batch_app;
#[cfg(feature = "viewer3d")]
pub mod l3d_render;
pub mod template_editor;
mod ui;

pub use app::GldfViewerApp;
pub use batch_app::BatchApp;
