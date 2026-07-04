//! `LeptosDocument`: owns a `BaseDocument` + the leptos reactive runtime, and
//! implements blitz's `Document` trait so `blitz-shell` can drive it.
//!
//! Direct structural analogue of `DioxusDocument`
//! (blitz/packages/dioxus-native-dom/src/dioxus_document.rs) — but simpler:
//! leptos has no VirtualDom to diff. `initial_build` mounts the view once;
//! afterwards signal updates write straight into the document through the
//! `BlitzDom` renderer. `poll()` only needs to drive queued reactive effects
//! and async tasks (any_spawner / tokio LocalSet), not a diffing pass.

use crate::events;
use crate::qual_name::qual_name;
use blitz_dom::{
    BaseDocument, DocGuard, DocGuardMut, Document, DocumentConfig, EventDriver,
    EventHandler, DEFAULT_CSS,
};
use blitz_traits::events::{DomEvent, EventState, UiEvent};
use std::cell::RefCell;
use std::rc::Rc;

pub struct LeptosDocument {
    pub inner: Rc<RefCell<BaseDocument>>,
    /// node id of `<main id="main">` — leptos mounts here.
    pub main_element_id: usize,
    /// Keeps the reactive Owner + mounted view state alive for the app's
    /// lifetime. Dropping this unmounts everything.
    /// Concretely: `(reactive_graph::owner::Owner, Box<dyn Any>)` where the
    /// Any is the retained `Mountable` state returned by `Render::build`.
    #[cfg(feature = "tachys-patched")]
    mount_handle: Option<crate::MountHandle>,
}

impl LeptosDocument {
    pub fn new(mut config: DocumentConfig) -> Self {
        config.base_url = Some(
            config
                .base_url
                .unwrap_or_else(|| String::from("leptos://index.html")),
        );
        let mut doc = BaseDocument::new(config);
        doc.add_user_agent_stylesheet(DEFAULT_CSS);

        // Skeleton: <html><head/><body><main id="main"/></body></html>
        // (same shape DioxusDocument builds)
        let mut mutr = doc.mutate();
        let root = mutr.doc.root_node().id;
        let html = mutr.create_element(qual_name("html", None), vec![]);
        mutr.append_children(root, &[html]);
        let head = mutr.create_element(qual_name("head", None), vec![]);
        mutr.append_children(html, &[head]);
        let body = mutr.create_element(qual_name("body", None), vec![]);
        mutr.append_children(html, &[body]);
        let main_attr = blitz_dom::Attribute {
            name: qual_name("id", None),
            value: "main".to_string(),
        };
        let main_element_id = mutr.create_element(qual_name("main", None), vec![main_attr]);
        mutr.append_children(body, &[main_element_id]);
        drop(mutr); // flush

        let inner = Rc::new(RefCell::new(doc));

        // Install ambient document BEFORE any leptos code runs.
        crate::renderer::set_ambient_document(inner.clone());

        Self {
            inner,
            main_element_id,
            #[cfg(feature = "tachys-patched")]
            mount_handle: None,
        }
    }

    /// Build and mount the leptos view into `<main>`.
    #[cfg(feature = "tachys-patched")]
    pub fn mount<F, V>(&mut self, f: F)
    where
        F: FnOnce() -> V + 'static,
        V: tachys::view::Render + 'static,
    {
        use crate::renderer::{BElement, BNode};
        // 1. Create a reactive Owner (reactive_graph::owner::Owner::new)
        // 2. owner.with(|| f()) to build the view tree
        // 3. Render::build(view) -> Mountable state
        // 4. state.mount(&BElement(BNode(self.main_element_id)), None)
        // 5. store (owner, Box::new(state)) in self.mount_handle
        //
        // This is ~20 lines mirroring leptos::mount::mount_to — write it
        // against the fork; kept as an explicit TODO so the borrow order
        // (ambient doc already installed, RefCell not held during build)
        // is a conscious decision, not an accident.
        let _ = f;
        todo!("see SPIKE.md §Mounting")
    }
}

impl Document for LeptosDocument {
    fn inner(&self) -> DocGuard<'_> {
        DocGuard::RefCell(self.inner.borrow())
    }

    fn inner_mut(&mut self) -> DocGuardMut<'_> {
        DocGuardMut::RefCell(self.inner.borrow_mut())
    }

    fn handle_ui_event(&mut self, event: UiEvent) {
        let mut doc = self.inner.borrow_mut();
        let mut driver = EventDriver::new(&mut *doc, LeptosEventHandler);
        driver.handle_ui_event(event);
        // NOTE: leptos listeners fired synchronously inside handle_event may
        // have queued reactive updates; effects run on the next poll(). The
        // shell polls after events, so updates land the same frame.
    }

    fn poll(&mut self, _task_context: Option<std::task::Context>) -> bool {
        // Drive the async executor (any_spawner tokio LocalSet / futures
        // LocalPool) so Resources, timers, and spawn_local tasks progress.
        // Return true if the document changed and a redraw is needed —
        // BaseDocument::has_changes() gives us that directly.
        self.inner.borrow().has_changes()
    }
}

/// Bridges blitz's event driver to the leptos listener registry.
/// blitz hands us the pre-computed bubble `chain` — dispatch handles the rest.
struct LeptosEventHandler;

impl EventHandler for LeptosEventHandler {
    fn handle_event(
        &mut self,
        chain: &[usize],
        event: &mut DomEvent,
        _doc: &mut dyn Document,
        _event_state: &mut EventState,
    ) {
        let handled = events::dispatch(chain, event);
        if handled {
            event.request_redraw = true;
        }
    }
}
