//! GLDF WASM Editor — Leptos shell (slice S).
//!
//! Goal of slice S: visually indistinguishable from the Yew app at gldf.icu
//! before any file is loaded. Sidebar with 16 nav items in 3 sections, top
//! toolbar with Edit Mode toggle + 4 export buttons + language picker + clear
//! + help, welcome screen with drop affordance and 4 demo file buttons, drop
//! zone with visual feedback. Every nav item routes to a placeholder until
//! its real content lands in slice 1+.
//!
//! Match the Yew side's class names exactly — `crates/gldf-rs-wasm/src/static/
//! styles.css` is shared. Match the i18n keys (`gldf.nav.*`, `gldf.sidebar.*`,
//! `gldf.welcome.*`, `gldf.toolbar.*`) exactly — `crates/gldf-rs-wasm/src/i18n/
//! ui.rs` is framework-agnostic.

use gldf_rs::GldfProduct;
use gldf_rs_wasm::components::{
    ApplicationsEditorPage, ApplicationsReadPage, BevyViewerPage, BuilderWidget, ControlGearPage,
    CustomPropertiesPage, ElectricalEditorPage, ElectricalReadPage, EmergencyEditorPage,
    EmergencyReadPage, EmittersPage, EquipmentPage, FileViewerPage, FilesListPage, GeometriesPage,
    HeaderDashboardPage, HeaderEditorPage, HyperlinksEditorPage, HyperlinksReadPage,
    IdentityEditorPage, IfcViewerPage, LabelsEditorPage, LabelsReadPage, LightSourcesPage, LinkagePage,
    LinkageTarget, MarketingEditorPage, MechanicalEditorPage, MechanicalReadPage, NewGldfWizard,
    OverviewPage, PhotometryEditorPage, PhotometryReadPage, PicturesEditorPage, PicturesReadPage,
    PluginsPage, RawDataPage, SpdViewer, StatisticsPage, VariantEditorPage, VariantsPage,
};
use std::sync::Arc;
use gldf_rs_wasm::exports;
use gldf_rs_wasm::i18n::ui::t as ui_t;
use gldf_rs_wasm::state::{
    provide_active_scope, provide_gldf_state, ui_language_native_name, use_gldf, GldfAction,
    GldfContext, SUPPORTED_UI_LANGUAGES,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

// --- Shell-level state ---

/// Which content page is currently visible in the main pane. Matches the
/// Yew `NavItem` enum 1:1 — the section/title/icon mapping below mirrors
/// `view_sidebar` + the section_title match in the Yew main.rs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NavItem {
    // File section
    Overview,
    Linkage,
    RawData,
    FileViewer,
    BevyViewer,
    IfcViewer,
    Plugins,
    // Document metadata section
    Header,
    Electrical,
    Mechanical,
    Applications,
    Labels,
    Hyperlinks,
    Pictures,
    Emergency,
    CustomProperties,
    Photometry,
    Statistics,
    // Definitions section
    Builder,
    Files,
    LightSources,
    Emitters,
    Variants,
    Geometries,
    Spectra,
    Marketing,
    ControlGear,
    Equipment,
}

impl NavItem {
    /// Stable kebab-case slug used as the sidebar `<li>`'s `data-testid`.
    /// E2E tests pin clicks to these — change them and the Playwright
    /// suite breaks deliberately.
    pub fn testid(&self) -> &'static str {
        match self {
            NavItem::Overview => "nav-overview",
            NavItem::Linkage => "nav-linkage",
            NavItem::RawData => "nav-raw-data",
            NavItem::FileViewer => "nav-file-viewer",
            NavItem::BevyViewer => "nav-bevy-viewer",
            NavItem::IfcViewer => "nav-ifc-viewer",
            NavItem::Plugins => "nav-plugins",
            NavItem::Header => "nav-header",
            NavItem::Electrical => "nav-electrical",
            NavItem::Mechanical => "nav-mechanical",
            NavItem::Applications => "nav-applications",
            NavItem::Labels => "nav-labels",
            NavItem::Hyperlinks => "nav-hyperlinks",
            NavItem::Pictures => "nav-pictures",
            NavItem::Emergency => "nav-emergency",
            NavItem::CustomProperties => "nav-custom-properties",
            NavItem::Photometry => "nav-photometry",
            NavItem::Statistics => "nav-statistics",
            NavItem::Builder => "nav-builder",
            NavItem::Files => "nav-files",
            NavItem::LightSources => "nav-light-sources",
            NavItem::Emitters => "nav-emitters",
            NavItem::Variants => "nav-variants",
            NavItem::Geometries => "nav-geometries",
            NavItem::Spectra => "nav-spectra",
            NavItem::Marketing => "nav-marketing",
            NavItem::ControlGear => "nav-control-gear",
            NavItem::Equipment => "nav-equipment",
        }
    }
}

/// View vs Edit mode. The Yew app's `ToggleEditor` flip lives here; each page
/// reads it to switch between its read view and its editor view.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppMode {
    Viewer,
    Editor,
}

#[derive(Clone, Copy)]
pub struct ShellState {
    pub nav: RwSignal<NavItem>,
    pub mode: RwSignal<AppMode>,
    pub is_dragging: RwSignal<bool>,
    pub ui_language: RwSignal<String>,
    pub show_help: RwSignal<bool>,
}

impl ShellState {
    fn new() -> Self {
        Self {
            nav: RwSignal::new(NavItem::Overview),
            mode: RwSignal::new(AppMode::Viewer),
            is_dragging: RwSignal::new(false),
            ui_language: RwSignal::new("en".to_string()),
            show_help: RwSignal::new(false),
        }
    }
}

// --- Root component ---

// JS-callable bridge to dispatch GldfActions from outside the Leptos
// component tree (e.g. Playwright's `page.evaluate(...)` or DevTools).
// The GldfContext lives in Leptos's context system; we mirror a copy
// into a thread-local on app boot so wasm_bindgen functions can reach
// it without a component handle. Used by the Trahe ACME e2e to bypass
// the VariantModalHost race (#136) when rebinding variant geometry.
thread_local! {
    static GLDF_CTX: std::cell::OnceCell<GldfContext> = const { std::cell::OnceCell::new() };
}

/// JS entry: apply a single GldfAction encoded as JSON. Returns the
/// empty string on success, or the error message on failure.
///
///     window.gldf_apply_action_json(JSON.stringify({
///         "SetVariantGeometryModel": {
///             "variant_idx": 1,
///             "geometry_id": "geometry_1",
///             "bindings": [{ "emitter_id": "emitter_3", "external_name": "Primary", "target_model_type": "l3d" }]
///         }
///     }))
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_apply_action_json)]
pub fn apply_action_json(json: &str) -> String {
    let action: GldfAction = match serde_json::from_str(json) {
        Ok(a) => a,
        Err(e) => return format!("parse: {}", e),
    };
    let mut result = String::new();
    GLDF_CTX.with(|cell| {
        match cell.get() {
            Some(ctx) => {
                ctx.update(|s| {
                    s.apply(action);
                    if let Some(err) = &s.last_error {
                        result = format!("apply: {}", err);
                    }
                });
            }
            None => result = "GldfContext not yet initialised".to_string(),
        }
    });
    result
}

/// JS read-only state probe: returns the current `GldfProduct` as a
/// JSON string so external callers (Playwright `page.evaluate`,
/// DevTools console) can inspect things like declared File ids
/// without scraping the DOM. The companion to
/// `gldf_apply_action_json`.
///
/// Returns `"null"` if the editor hasn't initialised yet, or the
/// serialized product on success. Errors during serialisation come
/// back as a JSON error object.
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_product_state)]
pub fn product_state() -> String {
    let mut out = "null".to_string();
    GLDF_CTX.with(|cell| {
        if let Some(ctx) = cell.get() {
            ctx.with(|s| match serde_json::to_string(&s.product) {
                Ok(j) => out = j,
                Err(e) => {
                    out = format!("{{\"error\":\"product_state: {}\"}}", e);
                }
            });
        }
    });
    out
}

#[component]
fn App() -> impl IntoView {
    let state = provide_gldf_state();
    // Mirror into the wasm_bindgen-reachable thread-local so the JS
    // bridge above can dispatch actions.
    GLDF_CTX.with(|cell| {
        let _ = cell.set(state);
    });
    // Attach the JSON-apply bridge to `window` so external JS (Playwright
    // page.evaluate, DevTools console) can call it directly without
    // importing the trunk-generated module. Done lazily on first App
    // mount so we capture a freshly-initialised GldfContext.
    if let Some(window) = web_sys::window() {
        let cb = wasm_bindgen::closure::Closure::wrap(
            Box::new(|json: wasm_bindgen::JsValue| -> wasm_bindgen::JsValue {
                let s = json.as_string().unwrap_or_default();
                apply_action_json(&s).into()
            }) as Box<dyn Fn(wasm_bindgen::JsValue) -> wasm_bindgen::JsValue>,
        );
        let _ = js_sys::Reflect::set(
            &window,
            &wasm_bindgen::JsValue::from_str("gldf_apply_action_json"),
            cb.as_ref(),
        );
        cb.forget();
        // Read-only state probe — same pattern.
        let probe = wasm_bindgen::closure::Closure::wrap(
            Box::new(|| -> wasm_bindgen::JsValue { product_state().into() })
                as Box<dyn Fn() -> wasm_bindgen::JsValue>,
        );
        let _ = js_sys::Reflect::set(
            &window,
            &wasm_bindgen::JsValue::from_str("gldf_product_state"),
            probe.as_ref(),
        );
        probe.forget();
    }
    let shell = ShellState::new();
    provide_context(shell);
    // Shared scope signal — see `provide_active_scope`'s docs for why
    // this is shell-level instead of per-page.
    provide_active_scope();

    // Deep-link from the URL on first mount. Supports `?url=<absolute or
    // same-origin path>` — we fetch the bytes and feed them through the
    // same `try_load` pipeline that powers drag-and-drop. CORS applies:
    // cross-origin servers must send `Access-Control-Allow-Origin`. The
    // earlier `?file=<id>` shortcut for bundled samples is honoured too.
    if let Some(window) = web_sys::window() {
        if let Ok(search) = window.location().search() {
            if let Some(query) = search.strip_prefix('?') {
                if let Ok(params) = web_sys::UrlSearchParams::new_with_str(query) {
                    let url = params.get("url");
                    let file = params.get("file");
                    if let Some(target) = url.clone().or(file.clone()) {
                        spawn_local(async move {
                            match fetch_bytes(&target).await {
                                Ok(bytes) => {
                                    // Filename hint: last path segment (or the
                                    // whole param if it has none) — try_load
                                    // routes by extension.
                                    let name = target
                                        .rsplit('/')
                                        .next()
                                        .filter(|s| !s.is_empty())
                                        .unwrap_or(&target)
                                        .to_string();
                                    if let Err(e) = try_load(state, &name, bytes) {
                                        log::error!("deep-link load failed for {target}: {e}");
                                    }
                                }
                                Err(e) => log::error!("deep-link fetch failed for {target}: {e}"),
                            }
                        });
                    }
                }
            }
        }
    }

    // Page-level drop handler — fires when the user drops anywhere in the
    // wrapper. The welcome screen has its own dedicated handler with visual
    // feedback; this one catches drops onto a loaded view (e.g. drop a new
    // file to swap the current document).
    let on_drop = move |e: web_sys::DragEvent| {
        e.prevent_default();
        shell.is_dragging.set(false);
        let Some(dt) = e.data_transfer() else { return };
        let Some(files) = dt.files() else { return };
        let Some(file) = files.get(0) else { return };
        load_browser_file(file, state);
    };

    let on_drag_over = move |e: web_sys::DragEvent| {
        e.prevent_default();
    };
    let on_drag_enter = move |e: web_sys::DragEvent| {
        e.prevent_default();
        shell.is_dragging.set(true);
    };
    let on_drag_leave = move |_e: web_sys::DragEvent| {
        shell.is_dragging.set(false);
    };

    view! {
        <div
            id="wrapper"
            on:dragover=on_drag_over
            on:dragenter=on_drag_enter
            on:dragleave=on_drag_leave
            on:drop=on_drop
        >
            <Sidebar />
            <MainArea />
            <Show when=move || shell.show_help.get() fallback=|| ().into_view()>
                <HelpOverlay />
            </Show>
        </div>
    }
}

// --- Sidebar ---

#[component]
fn Sidebar() -> impl IntoView {
    let shell = expect_context::<ShellState>();
    let state = use_gldf();

    // Local translator. Recomputes when the UI language changes — Leptos
    // signal subscriptions handle the rerender for us.
    let t = move |key: &str| ui_t(key, &shell.ui_language.get());

    // Derived signals for the right-hand badge counts. They re-read whenever
    // the GldfState signal updates.
    let files_count = move || state.with(|s| s.embedded_files.len());
    let light_sources_count = move || {
        state.with(|s| {
            s.product
                .general_definitions
                .light_sources
                .as_ref()
                .map(|ls| ls.fixed_light_source.len() + ls.changeable_light_source.len())
                .unwrap_or(0)
        })
    };
    let variants_count = move || {
        state.with(|s| {
            s.product
                .product_definitions
                .variants
                .as_ref()
                .map(|v| v.variant.len())
                .unwrap_or(0)
        })
    };
    let emitters_count = move || {
        state.with(|s| {
            s.product
                .general_definitions
                .emitters
                .as_ref()
                .map(|e| e.emitter.len())
                .unwrap_or(0)
        })
    };
    let has_file = move || {
        state.with(|s| {
            !s.embedded_files.is_empty()
                || s.product.product_definitions.product_meta_data.is_some()
        })
    };

    view! {
        <div class="sidebar">
            <div class="sidebar-header">
                <h1>
                    <span class="icon">{ "💡" }</span>
                    { move || t("gldf.app.title") }
                </h1>
            </div>

            <div class="sidebar-section">
                <div class="sidebar-section-title">{ move || t("gldf.sidebar.viewer") }</div>
                <ul class="sidebar-nav">
                    <NavItemView item=NavItem::Overview icon="📊" label_key="gldf.nav.overview" badge=None has_file=Signal::derive(has_file) />
                    <NavItemViewStaticLabel item=NavItem::Linkage icon="🧭" label="Linkage map" badge=None has_file=Signal::derive(has_file) />
                    <NavItemView item=NavItem::RawData icon="{ }" label_key="gldf.nav.raw_data" badge=None has_file=Signal::derive(has_file) />
                    <NavItemView item=NavItem::FileViewer icon="👁" label_key="gldf.nav.file_viewer" badge=Some(Signal::derive(files_count)) has_file=Signal::derive(has_file) />
                    <NavItemViewStaticLabel item=NavItem::BevyViewer icon="🎬" label="3D variants" badge=None has_file=Signal::derive(has_file) />
                    <NavItemView item=NavItem::IfcViewer icon="🏗️" label_key="gldf.nav.ifc_viewer" badge=None has_file=Signal::derive(has_file) />
                </ul>
            </div>

            <div class="sidebar-section">
                <div class="sidebar-section-title">{ move || t("gldf.sidebar.document") }</div>
                <ul class="sidebar-nav">
                    <NavItemView item=NavItem::Header icon="📄" label_key="gldf.nav.header" badge=None has_file=Signal::derive(has_file) />
                    <NavItemView item=NavItem::Electrical icon="⚡" label_key="gldf.nav.electrical" badge=None has_file=Signal::derive(has_file) />
                    <NavItemView item=NavItem::Mechanical icon="🔩" label_key="gldf.nav.mechanical" badge=None has_file=Signal::derive(has_file) />
                    <NavItemView item=NavItem::Applications icon="🏢" label_key="gldf.nav.applications" badge=None has_file=Signal::derive(has_file) />
                    <NavItemViewStaticLabel item=NavItem::Labels icon="✅" label="Approval Marks" badge=None has_file=Signal::derive(has_file) />
                    <NavItemViewStaticLabel item=NavItem::Hyperlinks icon="🔗" label="Hyperlinks" badge=None has_file=Signal::derive(has_file) />
                    <NavItemViewStaticLabel item=NavItem::Pictures icon="🖼️" label="Pictures" badge=None has_file=Signal::derive(has_file) />
                    <NavItemViewStaticLabel item=NavItem::Marketing icon="🎨" label="Marketing" badge=None has_file=Signal::derive(has_file) />
                    <NavItemViewStaticLabel item=NavItem::Emergency icon="🚨" label="Emergency" badge=None has_file=Signal::derive(has_file) />
                    <NavItemViewStaticLabel item=NavItem::CustomProperties icon="📋" label="Custom Properties" badge=None has_file=Signal::derive(has_file) />
                    <NavItemView item=NavItem::Photometry icon="🔬" label_key="gldf.nav.photometry" badge=None has_file=Signal::derive(has_file) />
                    <NavItemView item=NavItem::Statistics icon="📈" label_key="gldf.nav.statistics" badge=None has_file=Signal::derive(has_file) />
                </ul>
            </div>

            <div class="sidebar-section">
                <div class="sidebar-section-title">{ move || t("gldf.sidebar.definitions") }</div>
                <ul class="sidebar-nav">
                    <NavItemViewStaticLabel item=NavItem::Builder icon="🏗️" label="Builder" badge=None has_file=Signal::derive(has_file) />
                    <NavItemView item=NavItem::Files icon="📁" label_key="gldf.nav.files" badge=Some(Signal::derive(files_count)) has_file=Signal::derive(has_file) />
                    <NavItemView item=NavItem::LightSources icon="💡" label_key="gldf.nav.light_sources" badge=Some(Signal::derive(light_sources_count)) has_file=Signal::derive(has_file) />
                    <NavItemView item=NavItem::Emitters icon="🔆" label_key="gldf.nav.emitters" badge=Some(Signal::derive(emitters_count)) has_file=Signal::derive(has_file) />
                    <NavItemView item=NavItem::Variants icon="📦" label_key="gldf.nav.variants" badge=Some(Signal::derive(variants_count)) has_file=Signal::derive(has_file) />
                    <NavItemViewStaticLabel item=NavItem::Geometries icon="📐" label="Geometries" badge=None has_file=Signal::derive(has_file) />
                    <NavItemViewStaticLabel item=NavItem::Spectra icon="🌈" label="Spectra" badge=None has_file=Signal::derive(has_file) />
                    <NavItemViewStaticLabel item=NavItem::ControlGear icon="⚡" label="Control Gear" badge=None has_file=Signal::derive(has_file) />
                    <NavItemViewStaticLabel item=NavItem::Equipment icon="🔌" label="Equipment" badge=None has_file=Signal::derive(has_file) />
                </ul>
            </div>

            // Links section at bottom — matches Yew layout (margin-top: auto)
            <div style="margin-top: auto; padding: 16px;">
                <div style="font-size: 11px; color: var(--text-tertiary); margin-bottom: 8px;">
                    { move || t("gldf.sidebar.resources") }
                </div>
                <a href="https://github.com/holg/gldf-rs" target="_blank" style="display: block; font-size: 12px; margin-bottom: 4px;">{ "gldf-rs (GitHub)" }</a>
                <a href="https://gldf.io" target="_blank" style="display: block; font-size: 12px; margin-bottom: 4px;">{ "GLDF.io" }</a>
                <a href="https://eulumdat.icu/" target="_blank" style="display: block; font-size: 12px; margin-bottom: 8px;">{ "QLumEdit" }</a>
                <p class="privacy-note">{ move || t("gldf.sidebar.privacy_note") }</p>
            </div>
        </div>
    }
}

/// One sidebar nav item — i18n label.
#[component]
fn NavItemView(
    item: NavItem,
    icon: &'static str,
    label_key: &'static str,
    /// Optional badge count; pass `Some(signal)` to show a count chip.
    #[prop(default = None)]
    badge: Option<Signal<usize>>,
    has_file: Signal<bool>,
) -> impl IntoView {
    let shell = expect_context::<ShellState>();
    let is_active = move || shell.nav.get() == item;
    let enabled = move || has_file.get();
    let label = move || ui_t(label_key, &shell.ui_language.get());
    let on_click = move |_| {
        if enabled() {
            shell.nav.set(item);
        }
    };
    let class = move || {
        let mut c = String::from("sidebar-nav-item");
        if is_active() {
            c.push_str(" active");
        }
        if !enabled() {
            c.push_str(" disabled");
        }
        c
    };
    let style = move || {
        if !enabled() {
            "opacity: 0.5; pointer-events: none;".to_string()
        } else {
            String::new()
        }
    };

    view! {
        <li class=class on:click=on_click style=style data-testid=item.testid()>
            <span class="icon">{ icon }</span>
            <span>{ label }</span>
            {move || badge.map(|b| view! { <span class="badge">{ b.get() }</span> })}
        </li>
    }
}

/// Static-label variant for items the Yew side hardcodes (Builder, Spectra).
#[component]
fn NavItemViewStaticLabel(
    item: NavItem,
    icon: &'static str,
    label: &'static str,
    #[prop(default = None)] badge: Option<Signal<usize>>,
    has_file: Signal<bool>,
) -> impl IntoView {
    let shell = expect_context::<ShellState>();
    let is_active = move || shell.nav.get() == item;
    let enabled = move || has_file.get();
    let on_click = move |_| {
        if enabled() {
            shell.nav.set(item);
        }
    };
    let class = move || {
        let mut c = String::from("sidebar-nav-item");
        if is_active() {
            c.push_str(" active");
        }
        if !enabled() {
            c.push_str(" disabled");
        }
        c
    };
    let style = move || {
        if !enabled() {
            "opacity: 0.5; pointer-events: none;".to_string()
        } else {
            String::new()
        }
    };

    view! {
        <li class=class on:click=on_click style=style data-testid=item.testid()>
            <span class="icon">{ icon }</span>
            <span>{ label }</span>
            {move || badge.map(|b| view! { <span class="badge">{ b.get() }</span> })}
        </li>
    }
}

// --- Main content area (toolbar + body) ---

#[component]
fn MainArea() -> impl IntoView {
    let shell = expect_context::<ShellState>();
    let state = use_gldf();
    let has_file = move || {
        state.with(|s| {
            !s.embedded_files.is_empty()
                || s.product.product_definitions.product_meta_data.is_some()
        })
    };

    view! {
        <div class="main-content" style="position: relative;">
            {move || if has_file() {
                view! { <Toolbar /><ContentBody /> }.into_any()
            } else {
                view! { <Welcome /> }.into_any()
            }}
            <BevyCanvasShell />
            <footer class="app-footer">
                <p>{ "Copyright Holger Trahe - " }<a href="mailto:trahe@mac.com">{ "trahe@mac.com" }</a></p>
            </footer>
        </div>
    }
}

/// Persistent host for the Bevy `#bevy-canvas`. Mounted at the app
/// shell so the Bevy WebGL context survives navigation — the user can
/// jump to Marketing to change a HousingColor, come back to the 3D
/// page, and the canvas is still alive with their updates.
///
/// We position the canvas absolutely over the main content area; CSS
/// toggles `display` so it's visible only when the user is on
/// [`NavItem::BevyViewer`]. The `BevyViewerPage` itself just renders
/// the variant picker; the canvas is hoisted out of that page on
/// purpose.
///
/// Hidden ≠ stopped: Bevy keeps running its render loop while the
/// canvas is hidden. The cost (~1–2% CPU on a static scene) is the
/// trade we pay to never have to re-create the WebGL context.
#[component]
fn BevyCanvasShell() -> impl IntoView {
    let shell = expect_context::<ShellState>();
    // Visible whenever the user is on the Bevy viewer page. We also
    // need a `min-height` so the canvas doesn't collapse to 0 (which
    // would freeze the render loop until the next resize event).
    let style = move || {
        let on_page = shell.nav.get() == NavItem::BevyViewer;
        let display = if on_page { "block" } else { "none" };
        format!(
            // Leave room above for the page header (h2 + description)
            // and the variant chips row — roughly 240px including
            // padding. Footer needs ~60px. left/right match content
            // padding.
            "position: absolute; top: 280px; bottom: 60px; left: 24px; right: 24px; \
             display: {}; border: 1px solid var(--border-color); border-radius: 6px; \
             overflow: hidden; background: var(--bg-primary); z-index: 1;",
            display
        )
    };
    view! {
        <div style=style>
            <canvas
                id="bevy-canvas"
                style="width: 100%; height: 100%; display: block;"
            ></canvas>
            <BevyKeyboardHelp />
        </div>
    }
}

/// Floating "[?]" button + collapsible cheat-sheet overlay for the
/// Bevy viewer's keyboard shortcuts. Sits in the bottom-right of the
/// canvas region so it doesn't fight the variant picker above.
#[component]
fn BevyKeyboardHelp() -> impl IntoView {
    let open = RwSignal::new(false);
    let toggle = move |_| open.set(!open.get());

    let panel_style = move || {
        if open.get() {
            "position: absolute; bottom: 56px; right: 12px; \
             background: rgba(20, 20, 24, 0.92); color: white; \
             border: 1px solid rgba(255,255,255,0.15); border-radius: 6px; \
             padding: 10px 14px; font-size: 12px; line-height: 1.6; \
             box-shadow: 0 4px 16px rgba(0,0,0,0.4); z-index: 3; \
             min-width: 220px;"
                .to_string()
        } else {
            "display: none;".to_string()
        }
    };
    view! {
        <button
            type="button"
            on:click=toggle
            title="Show keyboard shortcuts"
            style="position: absolute; bottom: 12px; right: 12px; z-index: 4; \
                   width: 32px; height: 32px; border-radius: 50%; border: none; \
                   background: rgba(20, 20, 24, 0.8); color: white; cursor: pointer; \
                   font-size: 16px; box-shadow: 0 2px 8px rgba(0,0,0,0.4);"
        >
            { "?" }
        </button>
        <div style=panel_style>
            <strong style="display: block; margin-bottom: 6px;">{ "Keyboard shortcuts" }</strong>
            <div>{ "R · reset view (or Home)" }</div>
            <div>{ "P · toggle photometric solid" }</div>
            <div>{ "L · toggle luminaire mesh" }</div>
            <div>{ "H · toggle shadows" }</div>
            <div style="margin-top: 6px; padding-top: 6px; border-top: 1px solid rgba(255,255,255,0.15);">
                <strong>{ "Camera (first-person):" }</strong>
            </div>
            <div>{ "W A S D · move (or arrows)" }</div>
            <div>{ "Q · down      E · up" }</div>
            <div style="margin-top: 6px; padding-top: 6px; border-top: 1px solid rgba(255,255,255,0.15);">
                <strong>{ "Scene type:" }</strong>
            </div>
            <div>{ "1 · Room      2 · Road" }</div>
            <div>{ "3 · Parking   4 · Outdoor" }</div>
            <div style="margin-top: 6px; padding-top: 6px; border-top: 1px solid rgba(255,255,255,0.15);">
                <strong>{ "Mouse:" }</strong>
            </div>
            <div>{ "Right-drag · look" }</div>
            <div>{ "Wheel · zoom (FOV)" }</div>
            <div style="margin-top: 6px; font-size: 11px; opacity: 0.7;">
                { "Click on the canvas first to focus, then use keys." }
            </div>
        </div>
    }
}

#[component]
fn Toolbar() -> impl IntoView {
    let shell = expect_context::<ShellState>();
    let state = use_gldf();
    let t = move |key: &str| ui_t(key, &shell.ui_language.get());

    // The title comes from `gldf.nav.<item>` — same logic as the Yew side.
    let title = move || {
        let item = shell.nav.get();
        let key = nav_item_label_key(item);
        match key {
            Some(k) => ui_t(k, &shell.ui_language.get()),
            None => static_nav_label(item).to_string(),
        }
    };

    let on_toggle_mode = move |_| {
        shell.mode.update(|m| {
            *m = match *m {
                AppMode::Viewer => AppMode::Editor,
                AppMode::Editor => AppMode::Viewer,
            };
        });
    };

    let on_export_xml = move |_| {
        if let Err(e) = exports::export_xml(state) {
            log::error!("Export XML: {}", e);
        }
    };
    let on_export_json = move |_| {
        if let Err(e) = exports::export_json(state) {
            log::error!("Export JSON: {}", e);
        }
    };
    let on_export_gldf = move |_| {
        if let Err(e) = exports::export_gldf(state) {
            log::error!("Export GLDF: {}", e);
        }
    };
    let on_export_ifc = move |_| {
        if let Err(e) = exports::export_ifc(state) {
            log::error!("Export IFC: {}", e);
        }
    };

    let on_lang_change = move |e: web_sys::Event| {
        let Some(target) = e.target() else { return };
        let select: HtmlSelectElement = match target.dyn_into() {
            Ok(s) => s,
            Err(_) => return,
        };
        shell.ui_language.set(select.value());
    };

    let on_clear = move |_| {
        state.update(|s| s.apply(GldfAction::Reset));
        shell.nav.set(NavItem::Overview);
        shell.mode.set(AppMode::Viewer);
    };

    let on_help = move |_| {
        shell.show_help.update(|v| *v = !*v);
    };

    let mode_label = move || {
        let lang = shell.ui_language.get();
        if shell.mode.get() == AppMode::Viewer {
            ui_t("gldf.toolbar.edit_mode", &lang)
        } else {
            ui_t("gldf.toolbar.view_mode", &lang)
        }
    };

    view! {
        <div class="content-header">
            <h1 class="content-title">{ title }</h1>
            <div class="toolbar-actions">
                <button class="btn btn-secondary" data-testid="toolbar-toggle-mode" on:click=on_toggle_mode>
                    { mode_label }
                </button>
                <button class="btn btn-success" data-testid="toolbar-export-gldf" on:click=on_export_gldf>
                    { move || t("gldf.toolbar.export_gldf") }
                </button>
                <button class="btn btn-success" on:click=on_export_json>
                    { move || t("gldf.toolbar.export_json") }
                </button>
                <button class="btn btn-success" on:click=on_export_xml>
                    { move || t("gldf.toolbar.export_xml") }
                </button>
                <button
                    class="btn btn-info"
                    on:click=on_export_ifc
                    title=move || t("gldf.toolbar.export_ifc_tooltip")
                >
                    { move || t("gldf.toolbar.export_ifc") }
                </button>
                <select
                    class="ui-language-toolbar-select"
                    title=move || t("gldf.toolbar.ui_lang_tooltip")
                    prop:value=move || shell.ui_language.get()
                    on:change=on_lang_change
                >
                    {SUPPORTED_UI_LANGUAGES.iter().map(|l| {
                        let code = (*l).to_string();
                        let label = ui_language_native_name(l).unwrap_or(l).to_string();
                        view! { <option value=code.clone()>{ format!("🖥 {}", label) }</option> }
                    }).collect::<Vec<_>>()}
                </select>
                <button class="btn btn-outline" data-testid="toolbar-clear" on:click=on_clear title=move || t("gldf.toolbar.clear_tooltip")>
                    { move || t("gldf.toolbar.clear") }
                </button>
                <button class="btn btn-outline" on:click=on_help title=move || t("gldf.toolbar.help_tooltip")>
                    { move || t("gldf.toolbar.help") }
                </button>
            </div>
        </div>
    }
}

#[component]
fn ContentBody() -> impl IntoView {
    let shell = expect_context::<ShellState>();

    view! {
        <div class="content-body">
            {move || {
                let nav = shell.nav.get();
                let mode = shell.mode.get();
                let edit = mode == AppMode::Editor;
                match nav {
                    NavItem::Overview => view! { <OverviewPage /> }.into_any(),
                    NavItem::Linkage => {
                        // Plumb the bin's NavItem nav signal down as a
                        // LinkageTarget-typed callback so the page (in the
                        // lib crate) doesn't have to know about NavItem.
                        let nav_signal = shell.nav;
                        let jump: gldf_rs_wasm::components::NavJump = std::sync::Arc::new(move |target: LinkageTarget| {
                            let item = match target {
                                LinkageTarget::Files => NavItem::FileViewer,
                                LinkageTarget::Photometries => NavItem::Photometry,
                                LinkageTarget::Spectrums => NavItem::Spectra,
                                LinkageTarget::LightSources => NavItem::LightSources,
                                LinkageTarget::Emitters => NavItem::Emitters,
                                LinkageTarget::Variants => NavItem::Variants,
                            };
                            nav_signal.set(item);
                        });
                        view! { <LinkagePage jump=jump /> }.into_any()
                    }
                    NavItem::Header => {
                        if edit {
                            // Edit Mode shows both Header and Identity editors
                            // stacked — they cover the same XSD scope (document
                            // identity) on the Yew side's EditorTabs.
                            view! {
                                <HeaderEditorPage />
                                <IdentityEditorPage />
                            }.into_any()
                        } else {
                            view! { <HeaderDashboardPage /> }.into_any()
                        }
                    }
                    NavItem::Electrical => {
                        if edit { view! { <ElectricalEditorPage /> }.into_any() }
                        else { view! { <ElectricalReadPage /> }.into_any() }
                    }
                    NavItem::Mechanical => {
                        if edit { view! { <MechanicalEditorPage /> }.into_any() }
                        else { view! { <MechanicalReadPage /> }.into_any() }
                    }
                    NavItem::Applications => {
                        if edit { view! { <ApplicationsEditorPage /> }.into_any() }
                        else { view! { <ApplicationsReadPage /> }.into_any() }
                    }
                    NavItem::Labels => {
                        if edit { view! { <LabelsEditorPage /> }.into_any() }
                        else { view! { <LabelsReadPage /> }.into_any() }
                    }
                    NavItem::Hyperlinks => {
                        if edit { view! { <HyperlinksEditorPage /> }.into_any() }
                        else { view! { <HyperlinksReadPage /> }.into_any() }
                    }
                    NavItem::Pictures => {
                        if edit { view! { <PicturesEditorPage /> }.into_any() }
                        else { view! { <PicturesReadPage /> }.into_any() }
                    }
                    NavItem::Emergency => {
                        if edit { view! { <EmergencyEditorPage /> }.into_any() }
                        else { view! { <EmergencyReadPage /> }.into_any() }
                    }
                    NavItem::CustomProperties => view! { <CustomPropertiesPage /> }.into_any(),
                    NavItem::Photometry => {
                        if edit { view! { <PhotometryEditorPage /> }.into_any() }
                        else { view! { <PhotometryReadPage /> }.into_any() }
                    }
                    NavItem::Files => view! { <FilesListPage editable=edit /> }.into_any(),
                    NavItem::FileViewer => view! { <FileViewerPage /> }.into_any(),
                    NavItem::BevyViewer => view! { <BevyViewerPage /> }.into_any(),
                    NavItem::IfcViewer => view! { <IfcViewerPage /> }.into_any(),
                    NavItem::LightSources => view! { <LightSourcesPage editable=edit /> }.into_any(),
                    NavItem::Emitters => view! { <EmittersPage editable=edit /> }.into_any(),
                    NavItem::Variants => {
                        if edit { view! { <VariantEditorPage /> }.into_any() }
                        else { view! { <VariantsPage /> }.into_any() }
                    }
                    NavItem::Statistics => view! { <StatisticsPage /> }.into_any(),
                    NavItem::RawData => view! { <RawDataPage /> }.into_any(),
                    NavItem::Plugins => view! { <PluginsPage /> }.into_any(),
                    NavItem::Builder => view! { <BuilderWidget /> }.into_any(),
                    NavItem::Spectra => view! { <SpdViewer /> }.into_any(),
                    NavItem::Geometries => view! { <GeometriesPage editable=edit /> }.into_any(),
                    NavItem::Marketing => view! { <MarketingEditorPage /> }.into_any(),
                    NavItem::ControlGear => view! { <ControlGearPage editable=edit /> }.into_any(),
                    NavItem::Equipment => view! { <EquipmentPage editable=edit /> }.into_any(),
                    // Safety-net for future NavItem additions; all current variants are wired above.
                    #[allow(unreachable_patterns)]
                    other => view! { <PlaceholderPage item=other /> }.into_any(),
                }
            }}
        </div>
    }
}

/// Render a "not ported yet" placeholder. Each placeholder names the page so
/// the user can see exactly what's still missing during the port.
#[component]
fn PlaceholderPage(item: NavItem) -> impl IntoView {
    let shell = expect_context::<ShellState>();
    let label = move || {
        let lang = shell.ui_language.get();
        match nav_item_label_key(item) {
            Some(k) => ui_t(k, &lang),
            None => static_nav_label(item).to_string(),
        }
    };
    let blurb = move || {
        let lang = shell.ui_language.get();
        match nav_item_label_key(item) {
            Some(_) => format!(
                "{} — {}",
                ui_t("gldf.port.not_ported_yet", &lang),
                ui_t("gldf.port.slice_pending", &lang)
            ),
            None => "Not ported yet — slice 1 work".to_string(),
        }
    };
    view! {
        <div style="padding: 24px;">
            <h2 style="margin-top: 0;">{ label }</h2>
            <p style="color: #888;">{ blurb }</p>
        </div>
    }
}

// --- Welcome (empty state) ---

#[component]
fn Welcome() -> impl IntoView {
    let shell = expect_context::<ShellState>();
    let state = use_gldf();
    let t = move |key: &str| ui_t(key, &shell.ui_language.get());

    // Wizard modal visibility. Flipped by the "Create new" button and
    // reset by the wizard's on_close (Finish or Cancel).
    let wizard_open = RwSignal::new(false);
    let on_wizard_close: Arc<dyn Fn() + Send + Sync> = {
        Arc::new(move || {
            wizard_open.set(false);
        })
    };

    let class = move || {
        let mut c = String::from("welcome-view");
        if shell.is_dragging.get() {
            c.push_str(" dragging");
        }
        c
    };

    let on_drop = move |e: web_sys::DragEvent| {
        e.prevent_default();
        shell.is_dragging.set(false);
        let Some(dt) = e.data_transfer() else { return };
        let Some(files) = dt.files() else { return };
        let Some(file) = files.get(0) else { return };
        load_browser_file(file, state);
    };
    let on_drag_over = move |e: web_sys::DragEvent| {
        e.prevent_default();
    };
    let on_drag_enter = move |e: web_sys::DragEvent| {
        e.prevent_default();
        shell.is_dragging.set(true);
    };
    let on_drag_leave = move |_: web_sys::DragEvent| {
        shell.is_dragging.set(false);
    };

    let on_file_input = move |e: web_sys::Event| {
        let Some(target) = e.target() else { return };
        let input: HtmlInputElement = match target.dyn_into() {
            Ok(i) => i,
            Err(_) => return,
        };
        let Some(files) = input.files() else { return };
        let Some(file) = files.get(0) else { return };
        load_browser_file(file, state);
        input.set_value("");
    };

    // Demo files live in `src/static/` and are copied into `dist/` by trunk
    // (see `index.html`'s `data-trunk rel="copy-file"` entries). Loading is a
    // plain `fetch(name).bytes()`.
    let load_demo = move |name: &'static str| {
        let st = state;
        spawn_local(async move {
            match fetch_bytes(name).await {
                Ok(bytes) => {
                    if let Err(e) = try_load(st, name, bytes) {
                        log::error!("demo load failed: {}", e);
                    }
                }
                Err(e) => log::error!("fetch demo failed: {}", e),
            }
        });
    };

    view! {
        <div
            class=class
            on:drop=on_drop
            on:dragover=on_drag_over
            on:dragenter=on_drag_enter
            on:dragleave=on_drag_leave
        >
            <div class="welcome-icon">{ "💡" }</div>
            <h1 class="welcome-title">{ move || t("gldf.app.title") }</h1>
            <p class="welcome-subtitle">{ move || t("gldf.welcome.subtitle") }</p>

            <div class="welcome-divider"></div>

            <p class="welcome-instructions">{ move || t("gldf.welcome.drop_hint") }</p>
            <p class="welcome-or">{ move || t("gldf.welcome.or") }</p>

            <div class="welcome-buttons">
                <button
                    class="btn btn-primary"
                    data-testid="welcome-open-wizard"
                    on:click=move |_| wizard_open.set(true)
                >
                    { "+ Create new GLDF" }
                </button>
                <label for="file-upload" class="btn btn-primary">
                    { move || t("gldf.welcome.open_file") }
                </label>
                <button class="btn btn-secondary" on:click=move |_| load_demo("slv_tria_2.gldf")>
                    { move || t("gldf.welcome.load_slv_tria_2") }
                </button>
                <button class="btn btn-secondary" on:click=move |_| load_demo("FLOODLIGHT_MAX_1200W_757_SYM_10_WAL_enriched.gldf")>
                    { move || t("gldf.welcome.load_floodlight_max") }
                </button>
                <button class="btn btn-secondary" on:click=move |_| load_demo("LEDVANCE_FLOODLIGHT_MAX.gldf")>
                    { move || t("gldf.welcome.load_floodlight_combined") }
                </button>
                <button class="btn btn-secondary" on:click=move |_| load_demo("biolux_hcl.gldf")>
                    { move || t("gldf.welcome.load_biolux") }
                </button>
            </div>

            <input
                id="file-upload"
                data-testid="welcome-file-input"
                type="file"
                accept=".gldf,.ldt,.ies,.ifc,.uld,.rolf,.xml,.tm33.xml"
                on:change=on_file_input
                style="display: none;"
            />

            <div style="margin-top: 32px; max-width: 400px;">
                <p class="privacy-note" style="text-align: center;">
                    { move || t("gldf.welcome.privacy_note") }
                </p>
                // Build stamp — bump when shipping cache-sensitive changes so
                // a hard-refresh is visible. Helps users tell whether they're
                // running a stale bundle from the browser's disk cache.
                <p style="text-align: center; font-size: 10px; color: var(--text-tertiary); margin-top: 8px;">
                    { format!("build {}", BUILD_STAMP) }
                </p>
            </div>
            <NewGldfWizard open=wizard_open on_close=on_wizard_close />
        </div>
    }
}

/// Manual build stamp shown on the welcome screen. Bump on each change you
/// want users to verify they're loading. Cheaper than a full build.rs
/// + git-rev pipeline for now; replace when the build stamp matters enough
/// to be automatic.
const BUILD_STAMP: &str = "2026-06-02 bevy-ldt+emitter-config";

// --- Help overlay ---

#[component]
fn HelpOverlay() -> impl IntoView {
    let shell = expect_context::<ShellState>();
    let t = move |key: &str| ui_t(key, &shell.ui_language.get());

    let on_close = move |_| shell.show_help.set(false);

    view! {
        <div
            style="position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.6); z-index: 1000; display: flex; align-items: center; justify-content: center;"
            on:click=on_close
        >
            <div
                style="background: white; padding: 24px; border-radius: 8px; max-width: 600px; max-height: 80vh; overflow: auto;"
                on:click=move |e| e.stop_propagation()
            >
                <h2 style="margin-top: 0;">{ move || t("gldf.help.title") }</h2>
                <p>{ move || t("gldf.help.body") }</p>
                <p style="color: #666; font-style: italic;">
                    { "Slice S — full per-page help comes in slice 3 alongside the help-overlay port." }
                </p>
                <button class="btn btn-outline" on:click=on_close>
                    { move || t("gldf.toolbar.help") }
                </button>
            </div>
        </div>
    }
}

// --- File loading plumbing ---

fn load_browser_file(file: web_sys::File, state: GldfContext) {
    let name = file.name();
    let gloo_file = gloo::file::File::from(file);
    spawn_local(async move {
        match gloo::file::futures::read_as_bytes(&gloo_file).await {
            Ok(bytes) => {
                if let Err(e) = try_load(state, &name, bytes) {
                    log::error!("load failed: {}", e);
                }
            }
            Err(e) => log::error!("read failed: {}", e),
        }
    });
}

async fn fetch_bytes(path: &str) -> Result<Vec<u8>, String> {
    let resp = reqwest_wasm::get(path).await.map_err(|e| e.to_string())?;
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    Ok(bytes.to_vec())
}

fn try_load(state: GldfContext, filename: &str, bytes: Vec<u8>) -> anyhow::Result<()> {
    let lower = filename.to_lowercase();
    if lower.ends_with(".gldf") {
        let fb = GldfProduct::load_gldf_from_buf_all(bytes)?;
        let embedded_files: HashMap<String, Vec<u8>> = fb
            .files
            .into_iter()
            .filter_map(|f| {
                let content = f.content?;
                let key = f.file_id.or(f.path).or(f.name)?;
                Some((key, content))
            })
            .collect();
        state.update(|s| {
            s.apply(GldfAction::LoadWithFiles {
                product: fb.gldf,
                embedded_files,
            });
        });
        Ok(())
    } else if lower.ends_with(".ies") || lower.ends_with(".ldt") {
        let converted = gldf_rs::convert::ldt_to_gldf(&bytes, filename)
            .map_err(|e| anyhow::anyhow!("ldt_to_gldf: {}", e))?;
        let embedded_files: HashMap<String, Vec<u8>> = converted
            .files
            .into_iter()
            .filter_map(|f| {
                let content = f.content?;
                let key = f.file_id.or(f.path).or(f.name)?;
                Some((key, content))
            })
            .collect();
        state.update(|s| {
            s.apply(GldfAction::LoadWithFiles {
                product: converted.gldf,
                embedded_files,
            });
        });
        Ok(())
    } else if lower.ends_with(".tm33.xml")
        || (lower.ends_with(".xml") && is_tm33_xml(&bytes))
    {
        let converted = gldf_rs::convert::tm33_to_gldf(&bytes, filename)
            .map_err(|e| anyhow::anyhow!("tm33_to_gldf: {}", e))?;
        let embedded_files: HashMap<String, Vec<u8>> = converted
            .files
            .into_iter()
            .filter_map(|f| {
                let content = f.content?;
                let key = f.file_id.or(f.path).or(f.name)?;
                Some((key, content))
            })
            .collect();
        state.update(|s| {
            s.apply(GldfAction::LoadWithFiles {
                product: converted.gldf,
                embedded_files,
            });
        });
        Ok(())
    } else {
        anyhow::bail!("Unsupported file: {}", filename);
    }
}

/// Sniff the first 2 KiB for TM-33-23 / ATLA-S001 root tags so a
/// generic `.xml` drop can still route to `tm33_to_gldf`.
fn is_tm33_xml(bytes: &[u8]) -> bool {
    let head = std::str::from_utf8(&bytes[..bytes.len().min(2048)]).unwrap_or("");
    head.contains("IESTM33-22") || head.contains("LuminaireOpticalData")
}

// --- NavItem ↔ label/section lookup ---

/// Maps NavItem → its i18n key, matching the Yew side's title match. Returns
/// `None` for items that hardcode their label (Builder, Spectra).
fn nav_item_label_key(item: NavItem) -> Option<&'static str> {
    Some(match item {
        NavItem::Overview => "gldf.nav.overview",
        NavItem::RawData => "gldf.nav.raw_data",
        NavItem::FileViewer => "gldf.nav.file_viewer",
        NavItem::IfcViewer => "gldf.nav.ifc_viewer",
        NavItem::Plugins => "gldf.nav.plugins",
        NavItem::Header => "gldf.nav.header",
        NavItem::Electrical => "gldf.nav.electrical",
        NavItem::Mechanical => "gldf.nav.mechanical",
        NavItem::Applications => "gldf.nav.applications",
        NavItem::Photometry => "gldf.nav.photometry",
        NavItem::Statistics => "gldf.nav.statistics",
        NavItem::Files => "gldf.nav.files",
        NavItem::LightSources => "gldf.nav.light_sources",
        NavItem::Emitters => "gldf.nav.emitters",
        NavItem::Variants => "gldf.nav.variants",
        NavItem::Builder
        | NavItem::Linkage
        | NavItem::Spectra
        | NavItem::Geometries
        | NavItem::Labels
        | NavItem::Hyperlinks
        | NavItem::Pictures
        | NavItem::Emergency
        | NavItem::CustomProperties
        | NavItem::BevyViewer
        | NavItem::Marketing
        | NavItem::ControlGear
        | NavItem::Equipment => return None,
    })
}

fn static_nav_label(item: NavItem) -> &'static str {
    match item {
        NavItem::Builder => "Builder",
        NavItem::Linkage => "Linkage map",
        NavItem::Spectra => "Spectra",
        NavItem::Geometries => "Geometries",
        NavItem::Labels => "Approval Marks",
        NavItem::Hyperlinks => "Hyperlinks",
        NavItem::Pictures => "Pictures",
        NavItem::Emergency => "Emergency",
        NavItem::CustomProperties => "Custom Properties",
        NavItem::Marketing => "Marketing",
        NavItem::ControlGear => "Control Gear",
        NavItem::Equipment => "Equipment",
        NavItem::BevyViewer => "3D variants",
        _ => "",
    }
}
