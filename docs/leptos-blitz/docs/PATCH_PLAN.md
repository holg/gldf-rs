# tachys Fork: Patch Plan (`blitz` feature flag)

Verified against `leptos-rs/leptos` master, 2026-07-03. Fork → branch
`blitz-renderer`. Strategy: **cfg-based backend swap, zero new generics** —
the same mechanism as the existing `csr`/`ssr`/`hydrate` features, which is
exactly what avoids the compile-time explosion that killed generic rendering
before 0.7.0 (see the note in `tachys/src/renderer/mod.rs:8-19`).

## Ground truth in the current tree

- `tachys/src/renderer/mod.rs:20` — `pub type Rndr = dom::Dom;` (the swap point)
- `tachys/src/renderer/mod.rs:24-31` — `pub mod types { pub use super::dom::{...} }`
  re-exports the concrete node types the whole crate mounts against
  (`Mountable::mount` takes `&renderer::types::Element`, `view/mod.rs:329`)
- `tachys/src/renderer/mock_dom.rs` — complete non-web `Renderer` +
  `DomRenderer` reference impl (thread_local doc + slotmap ids). **Copy its
  shape.** Currently commented out of the module tree (`mod.rs:33-38`).
- `tachys/src/html/event.rs:397-415` — `trait EventDescriptor { type EventType: FromWasmAbi; ... }`
  plus the macro at `:541` binding `ev::click → web_sys::MouseEvent` etc.
- web_sys leakage is contained: 68 references across 14 files
  (`grep -rln web_sys tachys/src`), the heavy ones being
  `html/event.rs`, `html/element/elements.rs`, `renderer/dom.rs`,
  `hydration.rs`, `reactive_graph/bind.rs`.

## Steps

### 1. Feature plumbing
`tachys/Cargo.toml`: add feature `blitz`; make `wasm-bindgen`, `web-sys`,
`js-sys` optional and implied by the (new) `web` feature, default = `web`.
Same for `leptos/Cargo.toml`, forwarding `blitz` down. Deps for `blitz`:
`blitz-dom`, `blitz-traits` (default-features = false).

### 2. Renderer swap
In `renderer/mod.rs`:
```rust
#[cfg(not(feature = "blitz"))]
pub type Rndr = dom::Dom;
#[cfg(feature = "blitz")]
pub type Rndr = blitz::BlitzDom;

pub mod types {
    #[cfg(not(feature = "blitz"))]
    pub use super::dom::{ ... };
    #[cfg(feature = "blitz")]
    pub use super::blitz::{
        BElement as Element, BNode as Node, BPlaceholder as Placeholder,
        BText as Text, /* Event, ClassList, CssStyleDeclaration, TemplateElement */
    };
}
```
Move `leptos-blitz/src/renderer.rs` (this repo) into
`tachys/src/renderer/blitz.rs` — the impl must live in-crate because
`Mountable` for the node types + `renderer::types` aliasing are coherent only
there. Keep the events registry in the external `leptos-blitz` crate and have
`blitz.rs` depend on it (or move it in too — maintainer taste).

### 3. Events (the real work, ~60% of the effort)
`html/event.rs`:
- cfg-gate `FromWasmAbi` bound: `#[cfg(not(feature = "blitz"))] type EventType: FromWasmAbi;`
  vs `#[cfg(feature = "blitz")] type EventType: From<blitz_traits::events::DomEvent>;`
  (or a small `FromDomEvent` trait to avoid orphan issues).
- In the `generate_event_types!` macro output, map per event name to the
  blitz payload struct:
  - click/dblclick/mouse*/pointer*/touch* → `BlitzPointerEvent`
  - keydown/keyup/keypress → `BlitzKeyEvent`
  - input → `BlitzInputEvent`, focus/blur/focusin/focusout → `BlitzFocusEvent`
  - wheel → `BlitzWheelEvent`, scroll → `BlitzScrollEvent`
  - everything blitz doesn't emit yet (animation, clipboard, drag, media…):
    map to a `NeverEvent` uninhabited type so `on:` handlers for them are a
    compile error rather than a silent no-op. dioxus-native has the same
    holes (`unimplemented!` in their NativeConverter) — ours fail earlier.
- The listener plumbing converts `DomEvent` → typed payload by matching
  `event.data` (`DomEventData` enum, blitz-traits/src/events.rs:210).

### 4. Neutralize wasm-only surface under cfg(blitz)
- `DomRenderer::set_property(JsValue)`: gate the `prop:` attribute path out
  (`html/property.rs`) — no JS properties natively.
- `reactive_graph/bind.rs` (two-way `bind:` sugar): gate out for the spike,
  revisit with blitz form-control APIs (`BaseDocument::toggle_checkbox` etc.).
- `hydration.rs`: blitz backend is always client-rendered; gate hydration off.
- `html/element/element_ext.rs`, `custom.rs`, `svg`, `mathml` `.dyn_ref` /
  JsCast helpers: cfg out or shim.

### 5. leptos umbrella crate
- `mount.rs`: `mount_to` variants call `document()`/web_sys — add a
  blitz-flavored `mount_to_blitz(parent_id, f)` or just let `leptos-blitz`
  own mounting (current plan: the latter — keeps the fork diff minimal).
- `leptos_dom` logging: route `log!` to `tracing`/stderr under cfg(blitz).

## Deliberately out of scope for the spike
- Router (needs a History shim — design doc after M4)
- SSR/hydrate interop, islands
- `<For>` keyed-list perf, `AnimatedShow`, transitions
- IME/composition, accessibility passthrough (blitz-shell has AccessKit —
  wire in phase 2; note leptos ecosystem cares, see accesskit usage in dom.rs)

## Upstreaming posture
Read `leptos/AI_POLICY.md` (new, by gbj) before opening anything: issues
before PRs, small self-contained diffs, and you personally must understand
and stand behind every line — which matches your own-voice HN rule anyway.
Realistic path: keep the fork as `leptos-blitz` for months, open a
*discussion* (not PR) linking issue #1743 once M4 demos, and let gbj decide
whether a `blitz` feature belongs upstream or stays a maintained fork.
nicoburns (Blitz) explicitly welcomes non-Dioxus frontends.
