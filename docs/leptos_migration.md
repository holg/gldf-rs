# Leptos port — honest plan (rev 2)

**Status:** rev 1 of this plan was wrong. It sliced "shell + 1 page" and
prioritized SPD over the user-facing app. After the user pointed out we shipped
nothing usable, I re-planned around **app-shell parity first**, then content.

The `leptos` branch sits on `light-other` tip. The first checkpoint commit
(`WIP: Leptos port: scaffold + state + Spectra page`) preserves the scaffold,
the state port, and the SPD viewer. Everything else is unbuilt.

## What the Yew app actually has

This is the table the original plan should have led with. Anything the user
touches in the live app must port before "feature parity" is true.

### Shell

- **Sidebar** with 3 sections and 16 nav items total:
  - File: Overview, RawData, FileViewer, IfcViewer, Plugins
  - Product Metadata: Header, Electrical, Mechanical, Applications, Photometry, Statistics
  - Definitions: Builder, Files, LightSources, Emitters, Variants, Spectra
- **Edit Mode toggle** at the top of the sidebar (drives every page's editor/dashboard split).
- **Toolbar** (top bar): Export XML / JSON / GLDF / IFC, language picker, help button.
- **Welcome / empty state** when no file is loaded: drop-zone hint, demo file buttons (4 bundled GLDFs + 1 IES sample).
- **Drop affordance** on the whole page with visual feedback during drag.
- **LanguageBanner** (shows the document's locales when more than one).
- **XSD violations banner** (closed-enum violations + dismiss).
- **Help overlay** (context-sensitive per nav item).
- **URL `?file=` deep linking**.

### Read-only pages (no Edit Mode required)

- Overview (file size, language coverage, embedded file list, download buttons)
- RawData (XML / JSON / pretty)
- FileViewer (in-zip browser, per-file preview)
- IfcViewer (when file has IFC data)
- Plugins
- Statistics
- Files list, LightSources, Emitters, Variants, Photometries browser, Geometries browser

### Editors (Edit Mode)

- HeaderEditor, IdentityEditor, ElectricalEditor, MechanicalEditor, ApplicationsEditor, PhotometryEditor, VariantEditor
- FilesEditor, LightSourceEditor, PicturesEditor
- HyperlinksEditor, LabelsEditor, EmergencyEditor, CustomPropertiesEditor
- ScopePicker (Product vs Variant scope chooser used by several editors)
- LocaleInput / LocaleFooEditor / LocaleFooView
- EditorTabs (the General Definitions / Product Metadata sub-tab nav inside the Header page)

### Viewers & integrations

- LdtViewer, L3dViewer (with three-d WebGL renderer)
- BevySceneViewer (Bevy 3D viewer trigger — loads the separate Bevy WASM bundle)
- IfcViewer, IfcBuildingContext, IfcPropertyTree
- UrlFileViewer
- PluginViewer

### Specialized panels

- BuilderWidget (the drop-and-link panel I added late in the Yew session)
- SpdViewer (✓ already done on the Leptos branch)

### Cross-cutting infrastructure

- i18n shell (eulumdat-i18n integration; XSD enum value translation tables)
- EditorStateSync sentinel (round-trips bytes back to App.loaded_gldf after each edit, so non-editor pages see Builder edits)
- IFC export pipeline (`GldfToIfc::export_with_mesh_and_files`)
- Per-variant photometry export (`Msg::DownloadVariantPhotometry`)

## New slice plan (no more "vertical slice 1.x with one page")

### Slice S — App-shell parity (must be done before anything else)

Acceptance: the Leptos app on `http://localhost:8052/` is **visually
indistinguishable from the Yew app at gldf.icu before any file is loaded.**
Click any nav item, see a "Not ported yet" placeholder. The user can:

- See the welcome screen with demo file buttons that actually load demo files.
- Drop a .gldf and see it loaded (the state sync to the existing summary).
- Click Edit Mode toggle (just a state flip; pages don't react yet).
- Click any export button (stubbed alert OK; will be wired in slice 1.3).
- See the language picker (functional UI; the language switch may noop until pages port).
- See the help button (overlay shows; copy may be sparse).

Subsections:

- **S.1 Layout chrome.** Sidebar grid, content area, top toolbar, footer. Use the same CSS as the Yew side — `crates/gldf-rs-wasm/src/static/styles.css` is shared, no rework.
- **S.2 Sidebar nav.** All 16 nav items in 3 sections. `NavItem` enum + `RwSignal<NavItem>` for the selected item. Each item renders a `"Not ported yet — was: <yew description>"` placeholder.
- **S.3 Welcome screen.** Drop affordance, demo file buttons, format hints. The four bundled GLDFs (already in `src/static/`) wired to the file-load path.
- **S.4 Toolbar.** Edit Mode toggle (flips `RwSignal<AppMode>`), export buttons (alerts for now), help button, language picker.
- **S.5 Drop zone visible.** Drag enter/leave/over handlers update an `is_dragging` signal; show a translucent overlay during drag.

### Slice 1 — Read-only pages + working exports + Builder

Acceptance: you can drop a real GLDF, click through every left-nav item, and
see Yew-equivalent content. Export buttons produce correct files. Builder
ports because it's tied to the GLDF builder API on the lib (which is already
in place from the previous gldf-rs work).

Subsections:

- **1.1 Overview** — file size, language coverage, embedded files list with per-file download buttons.
- **1.2 Header dashboard** (read-only side of `view_header_editor`) — six identity fields + derived facts.
- **1.3 Exports.** Wire `ExportXml`, `ExportJson`, `ExportGldf` (via `EditableGldf::save_to_buf`), and `ExportIfc` (via `GldfToIfc::export_with_mesh_and_files`). Each downloads a blob.
- **1.4 RawData** — XML / pretty XML / JSON pane.
- **1.5 FileViewer** — list of embedded files with type detection + per-file preview component.
- **1.6 Files list page** — same content as Files in the read mode.
- **1.7 LightSources** — read view (the listing + flux/power/CCT/CRI per source).
- **1.8 Emitters** — read view with photometry preview, rated flux, light source ref.
- **1.9 Variants** — read view with mounting info, geometry ref, photometry per variant, per-variant photometry download.
- **1.10 Photometries browser** + **Geometries browser** + **Equipments browser** + **Sensors browser**.
- **1.11 Statistics** — counts dashboard.
- **1.12 BuilderWidget.** Port verbatim — the Yew widget on `light-other` is the reference. Sub-controls: photometry drop, replace-photometry, spectrum drop with light-source picker, variant add, identity inputs (rename product / manufacturer / light source id), Download modified GLDF button.
- **1.13 EditorStateSync** sentinel (Effect that watches the state signal and round-trips bytes back). Needed so Files / Photometry / 3D viewer reflect Builder edits.

### Slice 2 — Editor pages (Edit Mode)

Each item below is one component port + its `<NavItem>` route inside `view_content`.

- HeaderEditor + IdentityEditor + LocaleInput + LocaleFooEditor + LocaleFooView
- ElectricalEditor + ScopePicker
- MechanicalEditor
- ApplicationsEditor + LabelsEditor + HyperlinksEditor + PicturesEditor + EmergencyEditor + CustomPropertiesEditor
- PhotometryEditor
- VariantEditor (per-variant editing + per-variant photometry download)
- FilesEditor
- LightSourceEditor

### Slice 3 — Viewers & integrations

- LdtViewer (Yew side has a renderer; LDT data is parsed via eulumdat-rs)
- L3dViewer (uses three-d for WebGL; the renderer is in `src/draw_l3d.rs`, already framework-agnostic)
- BevySceneViewer (launches the Bevy WASM bundle; just a launcher button + iframe / canvas mount)
- IfcViewer + IfcBuildingContext + IfcPropertyTree
- UrlFileViewer
- PluginViewer
- URL `?file=` deep linking (parse query, dispatch `LoadFromUrl`)
- XSD violations banner
- Help overlay

### Slice 4 — Specialized panels & polish

- SpdViewer is already ported. Move it into the Spectra nav-item route (it's currently inlined under the summary).
- The New GLDF wizard (slice 1.5 from rev 1 of this plan) — guided 5-step builder for users who want to start clean instead of dropping an IES. Lands after Slice 1 because Finish needs a real editor to land in.
- Mobile / responsive polish.

### Definition of done — Leptos branch ready to merge

- Every nav item works.
- Every editor reaches feature parity with the Yew side.
- Round-trip an existing GLDF: open → edit → save → reopen with the edits.
- Both the SOLART spectral example (`tests/data/spctral_gldf-example.gldf`) and a normal photometry-only GLDF render correctly.
- Bundle size still smaller than Yew (the existing 4.2 MB raw / 621 KB brotli baseline beats Yew's 10 MB / 1.4 MB, so we have headroom).
- Test deploy to gldf.icu from `leptos` branch + smoke check, then merge to `main`.

## Translation table (Yew 0.22 → Leptos 0.8)

(Carried from rev 1; verified against actual code during the SPD viewer port.)

| Yew | Leptos 0.8 |
|---|---|
| `#[function_component(Foo)]` + `pub fn foo() -> Html` | `#[component] pub fn Foo() -> impl IntoView` |
| `html! { <div>{ ... }</div> }` | `view! { <div>{ ... }</div> }` |
| `use_state(\|\| 0_f64)` | `RwSignal::new(0_f64)` (`signal.set(x)` / `signal.get()`) |
| `use_state(...)` then derive | `Memo::new(\|_\| ...)` — **requires `T: PartialEq`**, so doesn't fit `SpectralDistribution` etc. (saw this in the SPD viewer port; use `state.with` inline instead) |
| `use_effect_with(deps, \|deps\| ...)` | `Effect::new(move \|_\| { /* tracks signals read in body */ })` |
| `use_reducer(GldfState::default)` | `RwSignal<GldfState>` + `impl GldfState { fn apply(&mut self, action) }`. Dispatch is `signal.update(\|s\| s.apply(...))`. The Yew "clone self, mutate new_state, rewrap in Rc" pattern becomes in-place mutation. The "revert is_modified on certain arms" trick needs capturing `prev_modified` at the top of `apply`. |
| `Callback::from(move \|e\| ...)` | A regular `move \|e\| ...` closure passed into `on:click=...` |
| `Properties + #[derive(Properties, PartialEq)]` | `#[component]` function args + `#[prop(into)]` / `#[prop(optional)]`; Leptos generates the Props struct |
| `Children` | `Children` from `leptos::prelude::Children` |
| `use_context::<T>()` | `expect_context::<T>()` (or `use_context::<T>()` for the Option form) |
| `<ContextProvider<T> context={state}>` | `provide_context(state)` inside a parent component (no wrapper element) |
| `Html::from_html_unchecked(svg.into())` | **Don't.** SVG geometry gets stripped. Use a `NodeRef::<leptos::html::Div>::new()` + `Effect` that calls `node_ref.get().set_inner_html(&svg_string)`. The `SvgEmbed` component on this branch already implements it; reuse, don't reinvent. |
| `web_sys::HtmlInputElement` via `e.target_unchecked_into()` | `let Some(target) = e.target() else { return }; let input: HtmlInputElement = target.dyn_into().ok()?;` (Leptos doesn't have `target_unchecked_into`) |
| `gloo::file::callbacks::read_as_bytes(&file, cb)` | `gloo::file::futures::read_as_bytes(&file).await` (need `gloo = { features = ["futures"] }`) |

## What's intentionally **not** ported (and why)

- The Yew `App` struct's `selected_3d_variant`, `selected_file`, etc. — these are small UI selection bits that belong in the components that own them, not in a god struct. Port each into its own `RwSignal` inside the relevant component.
- The `GldfProvider` / `GldfInitializer` wrapper components — Leptos doesn't need a wrapper, `provide_context` works at any level.
- `Html::from_html_unchecked` — actively harmful for SVG, see the row above.

## Build/serve cadence

Unchanged from rev 1:

- `scripts/build-wasm-split.sh` builds, the cache skips Bevy (unchanged) so iteration is fast.
- `scripts/build-wasm-split.sh serve` runs `python3 -m http.server 8052 -d dist/` for local testing.
- `scripts/build-wasm-split.sh deploy` rsyncs to `trahe.eu:/var/www/gldf.icu/html/`. Don't deploy until at least Slice S is done and visually correct.

## Where the lib work helps

Everything in `crates/gldf-rs-lib/` that the previous gldf-rs sessions
landed (the EditableGldf builder API, the rename_fixed_light_source +
replace_photometry_file methods, the file_id loader fix, the rc.3
conformance fixes, the GldfProduct CRUD) is framework-agnostic and feeds the
Leptos port for free. The Builder widget in slice 1.12 just dispatches into
this API.
