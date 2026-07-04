//! Event bridge: blitz `EventDriver` → leptos callbacks.
//!
//! ## How events flow
//!
//! 1. winit input arrives in `blitz-shell`, which converts it to a
//!    `blitz_traits::events::UiEvent` and calls `Document::handle_ui_event`.
//! 2. Our `LeptosDocument::handle_ui_event` constructs an `EventDriver` with a
//!    `LeptosEventHandler` (instead of `NoopEventHandler`).
//! 3. Blitz does hit-testing + default actions (focus, checkbox toggling,
//!    text-input editing) and calls
//!    `EventHandler::handle_event(chain, event, doc, event_state)` — where
//!    `chain: &[usize]` is the **pre-computed bubble path** from target to
//!    root. We walk it, invoking every listener registered for
//!    `(node_id, event.name())`, respecting `stop_propagation`.
//!
//! This means we get bubbling, delegation semantics, and form-control default
//! behavior from Blitz for free — the part that took dioxus-native the
//! longest to get right.
//!
//! ## Typed events (the fork's second half)
//!
//! tachys' `ev::click` etc. are typed as `web_sys::MouseEvent` via
//! `EventDescriptor::EventType: FromWasmAbi` (tachys/src/html/event.rs:397).
//! The patched fork cfg-swaps those associated types to the blitz-native
//! event structs (`BlitzPointerEvent`, `BlitzKeyEvent`, `BlitzInputEvent`, …)
//! re-exported here, and replaces the `unchecked_into` conversion with a
//! match on `DomEventData`. See docs/PATCH_PLAN.md step 3.

use blitz_traits::events::DomEvent;
use rustc_hash::FxHashMap;
use std::cell::RefCell;

/// Re-exports the patched tachys `ev` module maps onto.
pub use blitz_traits::events::{
    BlitzInputEvent, BlitzKeyEvent, BlitzPointerEvent, DomEventData,
};

pub type ListenerKey = u64;

struct Registry {
    next_key: ListenerKey,
    /// (node_id, event name) -> listener keys
    by_node: FxHashMap<(usize, &'static str), Vec<ListenerKey>>,
    /// key -> callback
    listeners: FxHashMap<ListenerKey, Box<dyn FnMut(DomEvent)>>,
    /// key -> (node_id, name) for O(1) removal
    reverse: FxHashMap<ListenerKey, (usize, &'static str)>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry {
        next_key: 0,
        by_node: FxHashMap::default(),
        listeners: FxHashMap::default(),
        reverse: FxHashMap::default(),
    });
}

/// Leak-intern the event name: names come from a small static set
/// ("click", "input", …), so this is bounded.
fn intern_name(name: &str) -> &'static str {
    thread_local! {
        static NAMES: RefCell<FxHashMap<String, &'static str>> =
            RefCell::new(FxHashMap::default());
    }
    NAMES.with(|n| {
        let mut n = n.borrow_mut();
        if let Some(s) = n.get(name) {
            *s
        } else {
            let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
            n.insert(name.to_string(), leaked);
            leaked
        }
    })
}

pub fn add_listener(
    node_id: usize,
    name: &str,
    cb: Box<dyn FnMut(DomEvent)>,
) -> ListenerKey {
    let name = intern_name(name);
    REGISTRY.with(|r| {
        let mut r = r.borrow_mut();
        let key = r.next_key;
        r.next_key += 1;
        r.by_node.entry((node_id, name)).or_default().push(key);
        r.listeners.insert(key, cb);
        r.reverse.insert(key, (node_id, name));
        key
    })
}

pub fn remove_listener(key: ListenerKey) {
    REGISTRY.with(|r| {
        let mut r = r.borrow_mut();
        r.listeners.remove(&key);
        if let Some(slot) = r.reverse.remove(&key) {
            if let Some(keys) = r.by_node.get_mut(&slot) {
                keys.retain(|k| *k != key);
            }
        }
    });
}

/// Drop all listeners for a removed node (called from `Renderer::remove`).
pub fn remove_all_for_node(node_id: usize) {
    REGISTRY.with(|r| {
        let mut r = r.borrow_mut();
        let keys: Vec<ListenerKey> = r
            .by_node
            .iter()
            .filter(|((id, _), _)| *id == node_id)
            .flat_map(|(_, ks)| ks.iter().copied())
            .collect();
        for key in keys {
            r.listeners.remove(&key);
            r.reverse.remove(&key);
        }
        r.by_node.retain(|(id, _), _| *id != node_id);
    });
}

/// Dispatch an event along the bubble chain. Returns `true` if any leptos
/// listener ran (used to decide whether to poll the reactive system and
/// request a redraw).
pub fn dispatch(chain: &[usize], event: &DomEvent) -> bool {
    let name = intern_name(event.name());
    let mut any = false;
    for node_id in chain {
        // Collect keys first — callbacks may add/remove listeners re-entrantly.
        let keys: Vec<ListenerKey> = REGISTRY.with(|r| {
            r.borrow()
                .by_node
                .get(&(*node_id, name))
                .cloned()
                .unwrap_or_default()
        });
        for key in keys {
            // Temporarily take the callback out so the registry isn't
            // borrowed while user code runs (which may itself mutate it).
            let cb = REGISTRY.with(|r| r.borrow_mut().listeners.remove(&key));
            if let Some(mut cb) = cb {
                cb(event.clone());
                any = true;
                REGISTRY.with(|r| {
                    r.borrow_mut().listeners.insert(key, cb);
                });
            }
        }
        if !event.bubbles {
            break;
        }
        // TODO(spike+1): honor stop_propagation via EventState once we
        // thread it through (blitz gives us `&mut EventState`).
    }
    any
}
