# leptos-blitz

**Spike:** render [Leptos](https://github.com/leptos-rs/leptos) natively —
no webview — via the [Blitz](https://github.com/DioxusLabs/blitz) engine
(Stylo CSS + Taffy layout + Parley text + Vello/wgpu rendering), the same
stack behind Dioxus Native.

```
view! { <button on:click=...> }  ──tachys Renderer──▶  blitz-dom (Slab<Node>)
        fine-grained signals                             │ stylo / taffy / parley
        (no VDOM, no diffing)                            ▼
                                    winit window ◀── blitz-shell ── Vello (wgpu)
```

## Status

Research skeleton. Read `SPIKE.md` (findings, milestones, Wuhan prep) and
`docs/PATCH_PLAN.md` (the required tachys fork, with file:line anchors
verified 2026-07-03).

- `crates/leptos-blitz/src/renderer.rs` — tachys `Renderer`+`DomRenderer`
  over `blitz_dom::DocumentMutator` (API mapping table inside)
- `crates/leptos-blitz/src/events.rs` — listener registry + bridge from
  blitz's `EventDriver` bubble chain
- `crates/leptos-blitz/src/document.rs` — `LeptosDocument` implementing
  blitz's `Document` trait (analogue of `DioxusDocument`)
- `crates/photometric-widgets` — **verified** polar-diagram painter (eulumdat + anyrender, PNG proof in docs/proof/) with `blitz_dom::Widget` + GLDF integration behind features
- `examples/counter` — the M4 acceptance test

## License

MIT OR Apache-2.0
