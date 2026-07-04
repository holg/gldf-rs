//! # leptos-blitz
//!
//! Spike: render Leptos views natively (no webview) via the Blitz engine —
//! the same wgpu/Vello pipeline that powers Dioxus Native.
//!
//! Status: **research skeleton**. The renderer/document/event architecture is
//! implemented against verified APIs of both sides; the typed-event layer and
//! the `Rndr` swap require the patched leptos fork (docs/PATCH_PLAN.md).
//!
//! ```ignore
//! fn app() -> impl IntoView {
//!     let count = RwSignal::new(0);
//!     view! {
//!         <button on:click=move |_| *count.write() += 1>
//!             "Count: " {count}
//!         </button>
//!     }
//! }
//!
//! fn main() {
//!     leptos_blitz::launch(app);
//! }
//! ```

pub mod document;
pub mod events;
pub mod renderer;
pub mod qual_name;

pub use document::LeptosDocument;
pub use renderer::BlitzDom;

/// Owner + retained view state, kept alive for the app lifetime.
#[cfg(feature = "tachys-patched")]
pub struct MountHandle {
    pub owner: reactive_graph::owner::Owner,
    pub state: Box<dyn std::any::Any>,
}

/// Launch a leptos app in a native Blitz window.
///
/// Wiring mirrors dioxus-native's `launch` (blitz/packages/dioxus-native/src/lib.rs):
/// `create_default_event_loop` → `WindowConfig::new(doc, renderer)` →
/// `BlitzApplication::add_window` → `event_loop.run_app`.
#[cfg(feature = "tachys-patched")]
pub fn launch<F, V>(f: F)
where
    F: FnOnce() -> V + 'static,
    V: tachys::view::Render + 'static,
{
    use blitz_dom::DocumentConfig;
    use blitz_shell::{create_default_event_loop, BlitzApplication, WindowConfig};

    // Tokio LocalSet or futures LocalPool for any_spawner; register with
    // any_spawner::Executor::init_* before building the view so that
    // Effect/Resource work.
    let event_loop = create_default_event_loop();

    let mut doc = LeptosDocument::new(DocumentConfig::default());
    doc.mount(f);

    let renderer = anyrender_vello::VelloWindowRenderer::new(Default::default());
    let window = WindowConfig::new(Box::new(doc) as _, renderer);

    let mut app = BlitzApplication::new(event_loop.create_proxy());
    app.add_window(window);
    event_loop.run_app(&mut app).unwrap();
}
