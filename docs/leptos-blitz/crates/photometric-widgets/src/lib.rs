//! Photometric widgets for Blitz-based UIs (leptos-blitz / dioxus-native).
//!
//! Layered so the painting core is testable without a window:
//! - `polar` (always): anyrender painter, pure function of `PolarDiagram`
//! - `widget` (feature `blitz`): `blitz_dom::Widget` wrappers
//! - feature `gldf`: construct widgets straight from GLDF containers

pub mod polar;

#[cfg(feature = "blitz")]
pub mod widget;

pub use eulumdat;
#[cfg(feature = "gldf")]
pub use gldf_rs;
