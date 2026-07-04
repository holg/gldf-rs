//! GLDF WebAssembly library — Leptos branch.
//!
//! The Yew implementation lived on `light-other` / `main`; this branch is the
//! in-place rewrite onto Leptos 0.8 (CSR). The slices and translation table
//! live in `docs/leptos_migration.md` (untracked). The bin entry is `main.rs`.

pub mod components;
#[cfg(target_arch = "wasm32")]
pub mod draw_l3d;
#[cfg(target_arch = "wasm32")]
pub mod exports;
pub mod i18n;
pub mod state;
#[cfg(target_arch = "wasm32")]
pub mod utils;
