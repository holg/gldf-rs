//! Thin entry: the browser shell lives in `app.rs` (wasm-only). The
//! native (leptos-blitz) shell is a separate crate in ../leptos-blitz
//! (`gldf-editor-native`) consuming this crate's LIB — same components,
//! two render targets.

#[cfg(target_arch = "wasm32")]
#[path = "app.rs"]
mod app;

#[cfg(target_arch = "wasm32")]
fn main() {
    app::run()
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!(
        "gldf_rs_wasm is the browser bin; the native editor is \
         `cargo run -p gldf-editor-native` in ../leptos-blitz"
    );
}
