//! `BlitzDom`: an implementation of tachys' `Renderer` + `DomRenderer` traits
//! over `blitz_dom::BaseDocument`.
//!
//! ## Architecture
//!
//! tachys' `Renderer` methods are *static* (no `&self`, no document handle) —
//! in the browser the DOM is ambient global state. We replicate that with a
//! thread-local `Rc<RefCell<BaseDocument>>`, exactly the pattern tachys' own
//! `mock_dom.rs` uses (thread_local Document + slotmap keys as nodes).
//! The winit event loop is single-threaded, so this is sound in practice.
//!
//! Node identity: blitz nodes are `usize` ids into a `Slab<Node>` that is
//! pinned in memory. Our `Node`/`Element`/`Text`/`Placeholder` newtypes wrap
//! that id — `Copy`, `'static`, trivially `Clone`, satisfying tachys' bounds.
//!
//! Mutations: every call opens a `DocumentMutator` via `doc.mutate()`;
//! the mutator flushes style/layout invalidation on `Drop` (verified:
//! blitz-dom/src/mutator.rs `impl Drop for DocumentMutator`), so short-lived
//! per-call mutators are correct, if not yet optimal. If profiling shows
//! flush overhead, batch via a thread-local "open mutator" cell later.
//!
//! ## Verified API mapping (blitz 0.3.0-alpha.6, leptos master 2026-07-03)
//!
//! | tachys `Renderer`                | blitz `DocumentMutator`                |
//! |----------------------------------|----------------------------------------|
//! | `create_text_node(text)`         | `create_text_node(text)`               |
//! | `create_placeholder()`           | `create_comment_node()`                |
//! | `set_text(node, text)`           | `set_node_text(id, value)`             |
//! | `set_attribute(el, name, value)` | `set_attribute(id, QualName, value)`   |
//! | `remove_attribute(el, name)`     | `clear_attribute(id, QualName)`        |
//! | `insert_node(parent, child, m)`  | `insert_nodes_before` / `append_children` |
//! | `remove_node(parent, child)`     | `remove_node(id)`                      |
//! | `clear_children(parent)`         | `remove_and_drop_all_children(id)`     |
//! | `remove(node)`                   | `remove_and_drop_node(id)`             |
//! | `get_parent(node)`               | `parent_id(id)`                        |
//! | `first_child(node)`              | `child_ids(id).first()`                |
//! | `next_sibling(node)`             | `next_sibling_id(id)`                  |
//! | `CreateElement::create_element`  | `create_element(QualName, attrs)`      |
//!
//! | tachys `DomRenderer`             | blitz                                  |
//! |----------------------------------|----------------------------------------|
//! | `set_css_property(style, k, v)`  | `set_style_property(id, name, value)`  |
//! | `add_class`/`remove_class`       | read+rewrite `class` attribute         |
//! | `set_inner_html(el, html)`       | `set_inner_html(id, html)` (needs blitz-html feature) |
//! | `add_event_listener(el, name, cb)` | events registry, see `events.rs`     |
//! | `set_property(el, k, JsValue)`   | ⚠ wasm-only concept — see PATCH_PLAN   |
//! | `get_template`/`clone_template`  | `deep_clone_node(id)` for cloning      |
//! | `create_element_from_html(html)` | via blitz-html parsing                 |

use blitz_dom::BaseDocument;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    /// The ambient document, installed by `LeptosDocument::new` before the
    /// leptos view is built (mirrors `MockDom`'s thread_local pattern).
    static DOCUMENT: RefCell<Option<Rc<RefCell<BaseDocument>>>> = const { RefCell::new(None) };
}

/// Install the ambient document for this thread. Must be called before any
/// tachys rendering happens.
pub fn set_ambient_document(doc: Rc<RefCell<BaseDocument>>) {
    DOCUMENT.with(|d| *d.borrow_mut() = Some(doc));
}

/// Run `f` with mutable access to the ambient document.
pub fn with_document_mut<R>(f: impl FnOnce(&mut BaseDocument) -> R) -> R {
    DOCUMENT.with(|d| {
        let doc = d.borrow();
        let doc = doc
            .as_ref()
            .expect("BlitzDom used before set_ambient_document()");
        let mut doc = doc.borrow_mut();
        f(&mut doc)
    })
}

/// Run `f` with shared access to the ambient document.
pub fn with_document<R>(f: impl FnOnce(&BaseDocument) -> R) -> R {
    DOCUMENT.with(|d| {
        let doc = d.borrow();
        let doc = doc
            .as_ref()
            .expect("BlitzDom used before set_ambient_document()");
        let doc = doc.borrow();
        f(&doc)
    })
}

/// The renderer unit struct — becomes `tachys::renderer::Rndr` behind the
/// `blitz` feature flag of the patched leptos fork.
#[derive(Debug, Clone, Copy)]
pub struct BlitzDom;

/// Node id newtypes. blitz node ids are `usize` slab keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BNode(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BElement(pub BNode);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BText(pub BNode);
/// tachys placeholders are DOM comment nodes; blitz has
/// `DocumentMutator::create_comment_node` for exactly this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BPlaceholder(pub BNode);

impl AsRef<BNode> for BNode {
    fn as_ref(&self) -> &BNode {
        self
    }
}
impl AsRef<BNode> for BElement {
    fn as_ref(&self) -> &BNode {
        &self.0
    }
}
impl AsRef<BNode> for BText {
    fn as_ref(&self) -> &BNode {
        &self.0
    }
}
impl AsRef<BNode> for BPlaceholder {
    fn as_ref(&self) -> &BNode {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// The actual trait impls live behind the `tachys-patched` feature because they
// compile against your leptos fork (see docs/PATCH_PLAN.md). The bodies below
// are written against the verified signatures of both sides and are expected
// to need only compiler-driven touch-ups, not redesign.
// ---------------------------------------------------------------------------
#[cfg(feature = "tachys-patched")]
mod renderer_impl {
    use super::*;
    use crate::events;
    use crate::qual_name::qual_name;
    use std::borrow::Cow;
    use tachys::renderer::{CastFrom, DomRenderer, RemoveEventHandler, Renderer};
    use tachys::view::Mountable;

    impl Renderer for BlitzDom {
        type Node = BNode;
        type Element = BElement;
        type Text = BText;
        type Placeholder = BPlaceholder;

        fn intern(text: &str) -> &str {
            // wasm_bindgen::intern is a wasm-only optimization; no-op natively.
            text
        }

        fn create_text_node(text: &str) -> Self::Text {
            let id = with_document_mut(|doc| doc.mutate().create_text_node(text));
            BText(BNode(id))
        }

        fn create_placeholder() -> Self::Placeholder {
            let id = with_document_mut(|doc| doc.mutate().create_comment_node());
            BPlaceholder(BNode(id))
        }

        fn set_text(node: &Self::Text, text: &str) {
            with_document_mut(|doc| doc.mutate().set_node_text(node.0 .0, text));
        }

        fn set_attribute(node: &Self::Element, name: &str, value: &str) {
            with_document_mut(|doc| {
                doc.mutate()
                    .set_attribute(node.0 .0, qual_name(name, None), value)
            });
        }

        fn remove_attribute(node: &Self::Element, name: &str) {
            with_document_mut(|doc| doc.mutate().clear_attribute(node.0 .0, qual_name(name, None)));
        }

        fn insert_node(
            parent: &Self::Element,
            new_child: &Self::Node,
            marker: Option<&Self::Node>,
        ) {
            with_document_mut(|doc| {
                let mut mutr = doc.mutate();
                match marker {
                    Some(anchor) => mutr.insert_nodes_before(anchor.0, &[new_child.0]),
                    None => mutr.append_children(parent.0 .0, &[new_child.0]),
                }
            });
        }

        fn remove_node(_parent: &Self::Element, child: &Self::Node) -> Option<Self::Node> {
            with_document_mut(|doc| doc.mutate().remove_node(child.0));
            Some(*child)
        }

        fn clear_children(parent: &Self::Element) {
            with_document_mut(|doc| doc.mutate().remove_and_drop_all_children(parent.0 .0));
        }

        fn remove(node: &Self::Node) {
            with_document_mut(|doc| {
                doc.mutate().remove_and_drop_node(node.0);
            });
            events::remove_all_for_node(node.0);
        }

        fn get_parent(node: &Self::Node) -> Option<Self::Node> {
            with_document_mut(|doc| doc.mutate().parent_id(node.0).map(BNode))
        }

        fn first_child(node: &Self::Node) -> Option<Self::Node> {
            with_document_mut(|doc| doc.mutate().child_ids(node.0).first().copied().map(BNode))
        }

        fn next_sibling(node: &Self::Node) -> Option<Self::Node> {
            with_document_mut(|doc| doc.mutate().next_sibling_id(node.0).map(BNode))
        }

        fn log_node(node: &Self::Node) {
            with_document(|doc| doc.print_subtree(node.0));
        }
    }

    impl DomRenderer for BlitzDom {
        /// Blitz's native event: carries target node id + typed `DomEventData`.
        type Event = blitz_traits::events::DomEvent;
        /// No live CSSOM objects in blitz — carry the element id and go
        /// through the attribute/style APIs.
        type ClassList = BElement;
        type CssStyleDeclaration = BElement;
        type TemplateElement = BElement;

        fn set_property(_el: &Self::Element, _key: &str, _value: &wasm_bindgen::JsValue) {
            // JS property assignment does not exist natively. The patched
            // tachys cfg-gates `property()` support out for the blitz
            // backend (PATCH_PLAN step 4). Reaching this is a bug.
            unreachable!("set_property is web-only; patched tachys must not call it")
        }

        fn add_event_listener(
            el: &Self::Element,
            name: &str,
            cb: Box<dyn FnMut(Self::Event)>,
        ) -> RemoveEventHandler<Self::Element> {
            let key = events::add_listener(el.0 .0, name, cb);
            RemoveEventHandler::new(move || events::remove_listener(key))
        }

        fn add_event_listener_delegated(
            el: &Self::Element,
            name: Cow<'static, str>,
            _delegation_key: Cow<'static, str>,
            cb: Box<dyn FnMut(Self::Event)>,
        ) -> RemoveEventHandler<Self::Element> {
            // Blitz's EventDriver already computes the bubble chain and hands
            // it to our EventHandler, so "delegation" is free: register
            // directly on the element; dispatch walks the chain.
            let key = events::add_listener(el.0 .0, &name, cb);
            RemoveEventHandler::new(move || events::remove_listener(key))
        }

        fn event_target<T>(ev: &Self::Event) -> T
        where
            T: CastFrom<Self::Element>,
        {
            T::cast_from(BElement(BNode(ev.target)))
                .expect("event target could not be cast")
        }

        fn class_list(el: &Self::Element) -> Self::ClassList {
            *el
        }

        fn add_class(class_list: &Self::ClassList, name: &str) {
            with_document_mut(|doc| {
                let id = class_list.0 .0;
                let current = read_attr(doc, id, "class").unwrap_or_default();
                if !current.split_ascii_whitespace().any(|c| c == name) {
                    let next = if current.is_empty() {
                        name.to_string()
                    } else {
                        format!("{current} {name}")
                    };
                    doc.mutate().set_attribute(id, qual_name("class", None), &next);
                }
            });
        }

        fn remove_class(class_list: &Self::ClassList, name: &str) {
            with_document_mut(|doc| {
                let id = class_list.0 .0;
                if let Some(current) = read_attr(doc, id, "class") {
                    let next = current
                        .split_ascii_whitespace()
                        .filter(|c| *c != name)
                        .collect::<Vec<_>>()
                        .join(" ");
                    doc.mutate().set_attribute(id, qual_name("class", None), &next);
                }
            });
        }

        fn style(el: &Self::Element) -> Self::CssStyleDeclaration {
            *el
        }

        fn set_css_property(style: &Self::CssStyleDeclaration, name: &str, value: &str) {
            with_document_mut(|doc| doc.mutate().set_style_property(style.0 .0, name, value));
        }

        fn set_inner_html(el: &Self::Element, html: &str) {
            with_document_mut(|doc| doc.mutate().set_inner_html(el.0 .0, html));
        }

        fn get_template<V>() -> Self::TemplateElement
        where
            V: tachys::view::ToTemplate + 'static,
        {
            // Browser fast-path: build a <template>, deep-clone per instance.
            // Natively: build the template subtree once per type via
            // ToTemplate::to_template into a detached node, cache in a
            // thread_local map keyed by TypeId. SPIKE SCOPE: build fresh each
            // time (correct, slower); optimize later.
            todo!("template caching: see SPIKE.md §Templates")
        }

        fn clone_template(tpl: &Self::TemplateElement) -> Self::Element {
            let id = with_document_mut(|doc| doc.mutate().deep_clone_node(tpl.0 .0));
            BElement(BNode(id))
        }

        fn create_element_from_html(html: &str) -> Self::Element {
            // Requires blitz-html (html5ever) — parse into a detached
            // fragment, return its single root element.
            let _ = html;
            todo!("wire blitz-html fragment parsing: see SPIKE.md §Templates")
        }
    }

    /// Helper: read an attribute value off a node.
    fn read_attr(doc: &mut BaseDocument, id: usize, name: &str) -> Option<String> {
        doc.get_node(id)?
            .element_data()?
            .attrs
            .iter()
            .find(|a| &*a.name.local == name)
            .map(|a| a.value.clone())
    }

    // ------------------------------------------------------------------
    // CastFrom lattice (Node <-> Element/Text/Placeholder), mirrors mock_dom
    // ------------------------------------------------------------------
    impl CastFrom<BNode> for BElement {
        fn cast_from(source: BNode) -> Option<Self> {
            with_document(|doc| {
                doc.get_node(source.0)
                    .and_then(|n| n.element_data().map(|_| BElement(source)))
            })
        }
    }
    impl CastFrom<BNode> for BText {
        fn cast_from(source: BNode) -> Option<Self> {
            with_document(|doc| {
                doc.get_node(source.0)
                    .and_then(|n| n.text_data().map(|_| BText(source)))
            })
        }
    }
    impl CastFrom<BNode> for BPlaceholder {
        fn cast_from(source: BNode) -> Option<Self> {
            // comment nodes: neither element nor text data
            Some(BPlaceholder(source))
        }
    }
    impl CastFrom<BElement> for BElement {
        fn cast_from(source: BElement) -> Option<Self> {
            Some(source)
        }
    }

    // ------------------------------------------------------------------
    // Mountable impls: tachys mounts against renderer::types::Element,
    // which the patched fork aliases to these types under cfg(blitz).
    // ------------------------------------------------------------------
    impl Mountable for BNode {
        fn unmount(&mut self) {
            with_document_mut(|doc| doc.mutate().remove_node(self.0));
        }
        fn mount(&mut self, parent: &BElement, marker: Option<&BNode>) {
            BlitzDom::insert_node(parent, self, marker);
        }
        fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
            let parent =
                with_document_mut(|doc| doc.mutate().parent_id(self.0).map(BNode));
            if let Some(parent) = parent {
                if let Some(parent) = BElement::cast_from(parent) {
                    child.mount(&parent, Some(self));
                    return true;
                }
            }
            false
        }
    }
    // Element/Text/Placeholder delegate to the inner BNode — mechanical,
    // identical to mock_dom.rs; fill in with the compiler.
}
