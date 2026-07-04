# HOWTO: leptos-blitz against your local leptos checkout

> **STATUS 2026-07-04: EXECUTED — M1 through M4 all pass.**
> Fork: `../leptos` branch `blitz-renderer` (commits 88f9aa2f, ee0754be).
> Workspace: `../leptos-blitz` (git init'd, eb9f690).
> Proof: `LEPTOS_BLITZ_SHOT_DIR=x cargo run -p counter` renders styled
> PNGs showing Count: 0 → Count: 3 via synthetic clicks through the
> real listener registry. `cargo run -p counter` opens the winit window.
> Deviations from the plan below: the crate calls `Rndr::*` as INHERENT
> fns (the Renderer trait is vestigial) so blitz.rs mirrors dom::Dom's
> inherent impl; Deref chains on the node newtypes replace web_sys
> interface inheritance; extra feature gotchas — blitz-dom needs
> `system_fonts`, reactive_graph needs `effects`, tachys needs
> `reactive_graph`. Phase 5 (leptos umbrella + view!) still open.
>
> **STATUS UPDATE (same day): Phase 5 core DONE.** `leptos` has a
> `blitz` feature (fork e65e028a); the real `view!` macro with
> `on:click` and `{signal}` children renders natively — see
> `examples/counter_view` (Count 0→7 headless proof). The macro's
> static-subtree fast path needs `blitz_html::HtmlProvider` installed
> as the document's `html_parser_provider` (LeptosDocument does this).
> Six green permutations: tachys default/blitz; leptos
> default/csr/blitz/ssr. `examples/components` additionally proves
> #[component] (nested, default props, children, Signal props) and
> <Show> branch-swapping (marker-based mount/unmount). Still open:
> templates (get_template), custom widgets (photometric-widgets hook),
> capture phase, stop_propagation, leptos_dom helpers audit.

Target: **M4 — a native window where `on:click` increments a counter.**
Strategy in one line: *fork scope = the `tachys` crate only.* The leptos
umbrella, `leptos_dom`, `leptos_macro`, router, server_fn — all untouched
until M5. tachys' builder API (`div().child(...)`) replaces the `view!`
macro for the spike, and `reactive_graph` (signals) compiles on native
unmodified already.

Second scope-cutter, verified: `wasm-bindgen`/`web-sys`/`js-sys` **compile
fine on native targets** (JsValue exists, methods just panic at runtime).
So Phase 1 does NOT rip the wasm deps out of tachys — it only (a) swaps
`Rndr`, (b) cfg-swaps the *event types*, and (c) cfg-gates the handful of
code paths that would *call* into JS at runtime. Deleting wasm deps is
upstream-politeness work for later, not spike work.

Assumed layout (adjust paths if yours differ):

```
~/develeop/rust/
├── leptos/          # your clone, branch blitz-renderer
└── leptos-blitz/    # this workspace
```

---

## Phase 0 — Wiring (15 min)

```bash
cd ../leptos
git checkout -b blitz-renderer
cargo check -p tachys        # baseline must be green before you touch anything
```

In `leptos-blitz/Cargo.toml` `[workspace.dependencies]`, activate:

```toml
tachys         = { path = "../leptos/tachys", default-features = false, features = ["blitz"] }
reactive_graph = { path = "../leptos/reactive_graph" }
any_spawner    = { path = "../leptos/any_spawner", features = ["futures-executor"] }
```

(path-deps beat `[patch]` here: no version dance, instant edit-recheck loop.)

In `crates/leptos-blitz/Cargo.toml`, flip the `tachys-patched` feature from
empty to real:

```toml
[features]
tachys-patched = ["dep:tachys", "dep:reactive_graph", "dep:any_spawner"]

[dependencies]
tachys         = { workspace = true, optional = true }
reactive_graph = { workspace = true, optional = true }
any_spawner    = { workspace = true, optional = true }
```

and in `examples/counter/Cargo.toml` re-enable what I removed after your
build failure: `leptos-blitz = { workspace = true, features = ["tachys-patched"] }`.

---

## Phase 1 — tachys: feature + Rndr swap (the grind, 0.5–1.5 days)

### 1.1 Cargo feature

`tachys/Cargo.toml`:

```toml
[features]
blitz = ["dep:blitz-dom", "dep:blitz-traits"]

[dependencies]
blitz-dom    = { version = "=0.3.0-alpha.6", default-features = false, optional = true }
blitz-traits = { version = "=0.3.0-alpha.6", default-features = false, optional = true }
```

### 1.2 The swap point

`tachys/src/renderer/mod.rs` — anchor: `grep -n "pub type Rndr" src/renderer/mod.rs`
(line ~20 as of 2026-07-03; the doc comment above it explains why generic
rendering was removed — you are doing the cfg variant it hints at):

```rust
#[cfg(not(feature = "blitz"))]
pub type Rndr = dom::Dom;
#[cfg(feature = "blitz")]
pub type Rndr = blitz::BlitzDom;

#[cfg(feature = "blitz")]
pub mod blitz;

pub mod types {
    #[cfg(not(feature = "blitz"))]
    pub use super::dom::{...};        // existing list, unchanged
    #[cfg(feature = "blitz")]
    pub use super::blitz::{
        BElement as Element, BNode as Node, BPlaceholder as Placeholder,
        BText as Text,
    };
}
```

### 1.3 Move the renderer in-crate

Copy from this repo into the fork:

- `crates/leptos-blitz/src/renderer.rs` → `tachys/src/renderer/blitz.rs`
  (drop the `#[cfg(feature = "tachys-patched")]` wrapper and the outer
  `mod renderer_impl`; fix paths `tachys::` → `crate::`)
- `crates/leptos-blitz/src/events.rs` → `tachys/src/renderer/blitz/events.rs`
- `crates/leptos-blitz/src/qual_name.rs` → inline into `blitz.rs`
  (keep using `blitz_dom::{ns, LocalName, Namespace, QualName}` re-exports —
  a direct markup5ever dep at the wrong version gives string_cache E0308s,
  you already met that one)

Why in-crate: `renderer::types` must re-export the node types, and tachys
can't depend on leptos-blitz (cycle). Orphan rules are fine for `Mountable`
either way, but `types` forces this.

### 1.4 The check loop

```bash
cd ../leptos
cargo check -p tachys --no-default-features --features blitz 2>&1 | head -50
```

Run, fix top error, repeat. What you WILL meet (from the earlier survey:
68 `web_sys` references in 14 files, but most are in signatures that stay
compilable — only *these* categories need edits):

| Symptom | Fix |
|---|---|
| `mock_dom`-style missing trait items on your impl | mirror `src/renderer/mock_dom.rs` — it's a complete non-web reference impl, commented out of the module tree; open it side by side |
| `html/event.rs`: `EventType: FromWasmAbi` unsatisfied / `unchecked_into` on your `DomEvent` | **the real Phase-1 work.** In the `EventDescriptor` trait: `#[cfg(not(feature="blitz"))] type EventType: FromWasmAbi;` / `#[cfg(feature="blitz")] type EventType: From<blitz_traits::events::DomEvent>;` and in the listener glue replace the wasm cast with `E::EventType::from(ev)`. In the event-type macro invocations, map: click/pointer*/mouse* → `BlitzPointerEvent`, key* → `BlitzKeyEvent`, input → `BlitzInputEvent`, focus* → `BlitzFocusEvent`, wheel → `BlitzWheelEvent`. Everything blitz doesn't emit (drag, clipboard, animation, media…): bind to an uninhabited `NeverEvent` — a compile error at the use-site beats a silent no-op. For **M4 you only need click** — it's fine to stub the macro so only pointer/key/input/focus resolve and the rest are NeverEvent. |
| `html/property.rs` (`prop:` syntax) calls `set_property(JsValue)` | `#[cfg(not(feature = "blitz"))]` the module + its re-exports; native has no JS properties |
| `hydration.rs` walks real DOM | cfg-gate the module; blitz backend is CSR-only |
| `reactive_graph/bind.rs` (two-way `bind:`) | cfg-gate; revisit in phase 2 with blitz's form-control APIs |
| `dom.rs` free fns (`document()`, `window()`, `body()`) used elsewhere | callers are mostly web-only paths — cfg the *caller*; don't try to fake a Window |
| `wasm_bindgen::intern` in shared code | `#[cfg(feature="blitz")] fn intern(s: &str) -> &str { s }` |

Acceptance: `cargo check -p tachys --no-default-features --features blitz`
green, AND the default check still green (`cargo check -p tachys`) so
you haven't broken web builds — check both after every session.

---

## Phase 2 — M1: static tree, no window (0.5 day)

In `crates/leptos-blitz/src/document.rs`, replace the `todo!` in `mount`:

```rust
pub fn mount<F, V>(&mut self, f: F)
where
    F: FnOnce() -> V + 'static,
    V: tachys::view::Render + 'static,
{
    use tachys::view::{Mountable, Render};
    let owner = reactive_graph::owner::Owner::new();
    let mut state = owner.with(|| f().build());
    let parent = crate::renderer::BElement(crate::renderer::BNode(self.main_element_id));
    state.mount(&parent, None);
    self.mount_handle = Some(crate::MountHandle { owner, state: Box::new(state) });
}
```

(Note: with the impls now living in tachys, `crate::renderer` in leptos-blitz
shrinks to re-exports: `pub use tachys::renderer::blitz::*;` — delete the
duplicated gated module.)

Smoke test, no winit yet — add `examples/tree_dump`:

```rust
use tachys::html::element::{div, h1, p};
use tachys::html::attribute::global::StyleAttribute; // if needed for .style()

fn main() {
    any_spawner::Executor::init_futures_executor().unwrap();
    let mut doc = leptos_blitz::LeptosDocument::new(Default::default());
    doc.mount(|| div().child((h1().child("leptos-blitz"), p().child("static tree"))));
    doc.inner.borrow().print_tree();   // expect: html > body > main > div > h1,p
}
```

**M1 done** when the dump shows your tree under `<main id="main">`.

## Phase 3 — M2: window (0.5 day)

`leptos_blitz::launch(app)` is already written (lib.rs) — it mirrors
dioxus-native's launch (`create_default_event_loop` → `WindowConfig` →
`BlitzApplication::add_window` → `run_app`). Expect small API drift against
alpha.6; fix against `blitz/packages/dioxus-native/src/lib.rs` as reference.
Add a `<style>` child or inline styles — stylo gives you real CSS.
**M2 done:** the M1 tree renders in a window, styled.

## Phase 4 — M3+M4: signals, then the counter (1–2 days)

M3: `RwSignal<i32>` + a text child closure + a `std::thread::spawn` timer
writing the signal (marshal via winit proxy or just poll) — proves the
reactive write path hits blitz nodes. Then M4:

```rust
use tachys::html::element::button;
use tachys::html::event as ev;

fn app() -> impl tachys::view::Render {
    let count = reactive_graph::signal::RwSignal::new(0);
    button()
        .on(ev::click, move |_| count.update(|n| *n += 1))
        .child(("Count: ", move || count.get().to_string()))
}
```

The dispatch path you're testing: winit → blitz-shell →
`LeptosDocument::handle_ui_event` → `EventDriver` (hit test + bubble chain) →
`LeptosEventHandler` → `events::dispatch` registry → your closure →
signal write → `Renderer::set_text` → blitz repaint (`request_redraw` is
already set in the handler). **M4 done = screenshot = the writeup moment.**

## Phase 5 (Wuhan stretch) — leptos umbrella & `view!`

Only now touch `leptos/Cargo.toml` (forward the `blitz` feature), cfg-gate
`leptos_dom`'s web helpers and `mount.rs`, and see whether `view!` expands
cleanly (it targets tachys builders underneath — decent odds). Also wire
`photometric-widgets`: intercept a `<ldt-polar>` custom element in the
renderer's `create_element` path → `set_custom_widget(node_id,
PolarWidget…)` → your LVK widgets inside leptos views.

---

## Ground rules while you're in there

- Commit to the fork per compiling milestone; note every upstream file you
  touched in the commit message — that list becomes the upstream discussion.
- Check `cargo check -p tachys` (default features) stays green — the diff
  that doesn't disturb web builds is the diff gbj can say yes to.
- leptos has an `AI_POLICY.md` now; read it before any upstream interaction
  (short version: issues before PRs, you own every line — your HN rule
  already complies).
- When alpha.6 fights you, `grep` the blitz repo checkout, not docs.rs —
  `~/develeop/rust/blitz` next to the others is worth having.
