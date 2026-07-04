# leptos-blitz — Spike Notes

Goal: Leptos views rendered natively (winit + Vello/wgpu via Blitz), no
webview — the leptos analogue of dioxus-native. Research verified against
source on 2026-07-03: leptos master, blitz 0.3.0-alpha.6.

## Why this is architecturally *easier* than dioxus-native

Dioxus must diff a VirtualDom and translate `Mutations` through a
`MutationWriter` (dioxus-native-dom/src/mutation_writer.rs, ~400 lines of
path-walking against template nodes). Leptos' fine-grained reactivity calls
renderer methods directly per signal update — no diffing layer at all. Our
entire mutation surface is the ~15 methods of tachys `Renderer`, and blitz's
`DocumentMutator` matches them almost 1:1 (full table in
`crates/leptos-blitz/src/renderer.rs`). The mutator flushes style/layout
invalidation on Drop, so per-call mutators are correct out of the box.

What Dioxus solved that we inherit for free by using blitz-dom/blitz-shell:
hit testing, bubble-chain computation (`EventHandler::handle_event` receives
`chain: &[usize]` pre-computed), form-control default actions, focus,
scrolling, AccessKit, winit shell incl. Android entry points, wgpu surface
management.

## The three real problems

1. **`Rndr` is a hardcoded alias.** `tachys::renderer::Rndr = dom::Dom`
   since generic rendering was removed pre-0.7 for compile-time reasons.
   → Solution: cfg feature swap (like csr/ssr/hydrate), zero generics.
   The dormant `mock_dom.rs` in-tree is a complete reference impl of a
   non-web backend — our renderer copies its shape. See docs/PATCH_PLAN.md.

2. **Typed events are wasm-bound.** `EventDescriptor::EventType: FromWasmAbi`
   and `ev::click = web_sys::MouseEvent`. → cfg-swap the associated types to
   blitz's `BlitzPointerEvent`/`BlitzKeyEvent`/`BlitzInputEvent`/…, convert by
   matching `DomEventData`. This is the majority of the fork diff (~60% of
   effort). Events blitz doesn't emit yet map to an uninhabited type =
   compile error instead of silent no-op.

3. **Async runtime.** Resources/Effects need `any_spawner` wired to a native
   executor (tokio LocalSet) and driven from `Document::poll`. Blitz's shell
   already calls `poll` with a task context — same hook dioxus-native uses to
   pump its VirtualDom.

## Milestones (acceptance-test driven)

- **M1 — static tree** (½–1 day once the fork compiles): patched leptos
  builds with `--features blitz`; `view! { <div><h1>"hi"</h1></div> }`
  mounts into `LeptosDocument`, `print_tree()` shows the structure. No window.
- **M2 — window** (½ day): blitz-shell + VelloWindowRenderer shows the tree.
  CSS from a `<style>` tag applies (blitz: stylo — real flexbox/grid).
- **M3 — reactivity** (1 day): a signal updated from a `std::thread::sleep`
  timer task updates a text node on screen. Proves renderer write path +
  poll-driven effects.
- **M4 — the counter** (1–2 days): `on:click` increments. Proves the whole
  event bridge. **This is the "it works" demo → screenshot → discussion post.**
- **M5 — stretch**: input element two-way binding; a `<For>` list; a GLDF
  polar-curve viewer using blitz's custom `Widget` hook
  (`BaseDocument::set_custom_widget` — yes, blitz has a custom-widget node
  type that hands you anyrender draw access: **this is your wgpu/photometry
  embedding path**, worth a milestone on its own).

Realistic total: M1–M4 in 4–7 focused days, of which the tachys event patch
is the bulk. Compiler does most of the navigation once the cfg skeleton is in.

## Environment prep BEFORE the flight (do at home, on fast internet)

1. Fork `leptos-rs/leptos` → branch `blitz-renderer`; clone locally.
2. Clone `DioxusLabs/blitz` (reference reading + you may need git deps).
3. `cargo vendor` this workspace once deps resolve, or at minimum run a full
   `cargo fetch && cargo build` so ~/.cargo has **stylo prebuilt in the
   target cache** — stylo is a huge build and crates.io access from a hotel
   network in Wuhan may involve proxies/slowness. Take the populated
   `target/` + `~/.cargo` with you (or a warm sccache dir).
4. `rustup component add rust-src rust-analyzer` offline docs:
   `cargo doc --no-deps -p blitz-dom -p blitz-traits -p tachys` and keep
   the two cloned repos on disk — grep beats docs.rs on a bad connection.
5. Pin everything: blitz git rev, leptos fork rev, in Cargo.toml.

## Open questions to resolve during M1

- Does `DocumentMutator` per-call (create → drop → flush) cost too much in
  hot loops (`<For>` with 1k rows)? If so: thread-local open mutator with
  explicit flush at `poll()` boundaries.
- `get_template`/`clone_template`: browser tachys clones `<template>` for
  speed. Native options: (a) skip templates, build per instance (correct,
  simple — spike default), (b) cache a detached subtree per `TypeId` and
  `deep_clone_node`. Measure before optimizing.
- Text nodes inside inline contexts: blitz `set_node_text` vs tachys'
  expectations for empty text nodes as markers — watch M1 tree dumps.
- `stop_propagation`: thread `EventState` into the registry dispatch.

## Strategic notes

- Blitz targets production-ready sometime in 2026; Rustweek 2026 talk showed
  incremental rendering + custom elements landing. Alpha churn is real —
  hence pinned revs.
- This spike is *visible* infrastructure work at the seam of two flagship
  Rust projects (leptos #1743 has years of 👍s; nicoburns invites other
  frontends). Even a failed spike with a good writeup is a strong post — in
  your own voice, per your HN rule. Note leptos now has an AI_POLICY.md;
  read it before engaging upstream (summary: issues before PRs, small diffs,
  you must own every line).
- Custom `Widget` nodes in blitz-dom (anyrender access) mean a future
  leptos-blitz app can embed photometric wgpu viewports natively — the
  GLDF/L3D desktop-viewer story that egui currently owns for you, but with
  real CSS layout and your existing Leptos component vocabulary.
- NLnet: do NOT bundle this into anything grant-shaped now (photometric
  grant #2026-04-1c6 keeps sole priority). File as community/OSS work.

---

## Track B: Photometric widgets (fork-independent — START HERE)

Added 2026-07-03, second research pass. Key realization: **custom Blitz
widgets need zero tachys patching** — they're pure `blitz-dom` + `anyrender`.
So the photometric layer is buildable and testable *today*, and becomes the
instantly-demoable part of the Wuhan work.

### What's DONE and VERIFIED (compiled + rendered in this session)

- `crates/photometric-widgets/src/polar.rs` — polar intensity painter,
  generic over `anyrender::PaintScene`. Geometry comes 100% from
  `eulumdat::diagram::PolarDiagram` (symmetry expansion, interpolation,
  nice-number scale all stay upstream in eulumdat-rs — single source of
  truth shared with the SVG/egui/Slint frontends).
- `crates/photometric-widgets/src/widget.rs` — `PolarWidget` implementing
  `blitz_dom::Widget` (needs blitz-dom feature `custom-widget`). Attribute
  protocol: `c-plane` (float degrees) and `theme` (`dark`/`light`) via
  `attribute_changed` — i.e. **reactivity via plain set_attribute**, which
  leptos signals drive with no extra plumbing.
- `--features gldf`: `PolarWidget::from_gldf(&product, file_id)` via
  `gldf_rs::GldfProduct::get_ldc_by_id` (gldf-rs 0.4, default-features off).
- Proof renders in `docs/proof/`: `polar_sym.png` (fluorescent batwing,
  BothPlanes symmetry, C0-C180 solid blue + C90-C270 dashed red) and
  `polar_asym.png` (LEDVANCE FL MAX LUM 900W 50x110 asymmetric floodlight)
  — rasterized with vello_cpu through the SAME `PaintScene` calls
  `Widget::paint` records. `cargo run -p photometric-widgets --example
  render_png -- assets/sample.ldt out.png` reproduces.

Version pins that matter (learned the hard way): anyrender 0.11 → peniko 0.6
/ kurbo 0.13; mixing peniko 0.5 gives cross-version type errors. vello_cpu's
`ImageRenderer::render` wants a pre-sized `W*H*4` RGBA buffer.

### The leptos component this becomes (post-fork, M5)

```rust
#[component]
pub fn LdtPolar(ldt: Signal<Eulumdat>, c_plane: Signal<Option<f64>>) -> impl IntoView {
    // A custom element the BlitzDom renderer intercepts: on creation it
    // calls mutator.set_custom_widget(node_id, Box::new(PolarWidget::new(..))).
    // Signal changes write attributes; Widget::attribute_changed repaints.
    view! {
        <ldt-polar
            c-plane=move || c_plane.get().map(|c| c.to_string())
            style="width: 400px; height: 400px;"
        />
    }
}
```

Until the fork lands, the same widget runs under **dioxus-native today**
(their `CustomWidgetAttr`, see blitz examples/custom_widget.rs) — useful as
a reference harness and as a hedge: the photometric widget investment pays
off regardless of which frontend wins.

### Widget roadmap (each is a PolarWidget sibling; eulumdat::diagram already
has the geometry for ALL of these — butterfly, cartesian, cone, heatmap,
isocandela, isolux)

1. `CartesianWidget` (intensity vs gamma) — trivial port, same pattern
2. `ConeWidget` (beam/field angles + spacing table) — datasheet staple
3. `HeatmapWidget` (C×gamma intensity matrix) — needs color::heatmap_color
4. `GldfInspector` composite: file tree (HTML) + variant selector (HTML
   select) + PolarWidget — the "GLDF viewer that doesn't look like egui"
   demo, and the natural mare.trahe.eu-style portfolio piece for the
   China lab-aggregation pitch (labs deliver 5-10 isolated files; this
   renders the aggregated GLDF package).

### Revised Wuhan sequencing

Week 1 (family/setup, laptop evenings): Track B only — port cartesian +
cone widgets, wire the dioxus-native harness, screenshots. Zero fork risk,
visible output immediately.
Weeks 2-3: tachys fork M1-M4 (the counter).
Week 4+: M5 = LdtPolar leptos component + GldfInspector demo; writeup.
