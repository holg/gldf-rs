# Native app for iOS / Android / Windows — options (2026-07-02)

Goal: an app "near the WASM version" (the full Leptos editor experience,
Bevy 3D included) but running **native** on iOS, Android, Windows
(+ macOS). The egui batch app covers the simple batch workflow; the
sophisticated editor UI is the open question — egui *can* do complex UIs
(rerun.io is the proof), but only after building a whole widget system,
and the current sophistication lives in ~40 Leptos components that lean
on HTML/CSS/SVG.

## Options

### 1. Tauri v2 shell + native Bevy view  ← recommended
- Reuses the existing Leptos editor UI **unchanged** (compiled to wasm,
  rendered in the system webview) on Win/mac/iOS/Android.
- Real filesystem via Tauri commands — the `gldf-rs-editor::batch` lib
  plugs in directly (native `run_batch`, no browser sandbox).
- Bevy runs native, embedded as a child surface / layered platform view
  (raw-window-handle on desktop; Metal/Vulkan view composited with the
  webview on mobile).
- **Risk to spike FIRST**: Bevy-view embedding on iOS/Android. Desktop
  child-window layering is a known pattern; mobile compositing is the
  unproven part and is per-platform glue.

### 2. Dioxus port
- Rust, signal-based, React-like — mechanical-ish translation from
  Leptos; first-class mobile packaging.
- Still a full UI rewrite; Bevy embedding question is the same as Tauri.

### 3. egui all the way (rerun-style)
- Best Bevy integration of all (`bevy_egui`, one codebase, one renderer).
- Furthest from the current UI; requires investing in a widget/design
  system like rerun did. i18n (7 locales incl. zh/ru) workable but DIY.

### 4. Revive `gldf-rs-slint` (parked crate)
- Slint's mobile + external-GPU-underlay support has matured.
- Full rewrite in .slint markup; least reuse of anything existing.

## Existing assets that survive any choice
- `gldf-rs-editor` — UI-agnostic reducer (all frontends drive it)
- `gldf-rs-editor::batch` — batch engine (LDT/IES/ULD + template →
  GLDF/IFC)
- `gldf-bevy` — the 3D scene as a lib (same code the WASM build uses)
- `gldf-rs-ffi` — UniFFI bindings (iOS/macOS/Android) if a native-shell
  route is ever wanted

## Suggested spike order (option 1)
1. Desktop: Tauri v2 window + Bevy child surface showing a GLDF scene
   next to the Leptos UI (2–3 days, proves the layering).
2. iOS: same composition with a Metal-backed Bevy view under/over the
   WKWebView (the real risk; go/no-go gate).
3. Only then wire the editor reducer + batch commands through Tauri IPC.

Note (memory): the gldf-bevy L3D orientation pipeline should be
re-derived from the L3D spec before a native app makes the 3D view a
headline feature — it has been patched 6+ times without a root fix.
