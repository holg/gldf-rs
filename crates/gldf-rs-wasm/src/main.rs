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
    ApplicationsEditorPage, ApplicationsReadPage, BuilderWidget, CustomPropertiesPage,
    ElectricalEditorPage, ElectricalReadPage, EmergencyEditorPage, EmergencyReadPage, EmittersPage,
    FileViewerPage, FilesListPage, HeaderDashboardPage, HeaderEditorPage, HyperlinksEditorPage,
    HyperlinksReadPage, IdentityEditorPage, LabelsEditorPage, LabelsReadPage, LightSourcesPage,
    LinkagePage, LinkageTarget, MechanicalEditorPage, MechanicalReadPage, OverviewPage,
    PhotometryEditorPage, PhotometryReadPage, PicturesEditorPage, PicturesReadPage, PluginsPage,
    RawDataPage, SpdViewer, StatisticsPage,
    VariantEditorPage, VariantsPage,
};
use gldf_rs_wasm::exports;
use gldf_rs_wasm::i18n::ui::t as ui_t;
use gldf_rs_wasm::state::{
    provide_gldf_state, ui_language_native_name, use_gldf, GldfAction, GldfContext,
    SUPPORTED_UI_LANGUAGES,
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
    Spectra,
}

/// View vs Edit mode. The Yew app's `ToggleEditor` flip lives here; each page
/// reads it to switch between its read view and its editor view.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppMode {
    Viewer,
    Editor,
}

#[derive(Clone, Copy)]
struct ShellState {
    nav: RwSignal<NavItem>,
    mode: RwSignal<AppMode>,
    is_dragging: RwSignal<bool>,
    ui_language: RwSignal<String>,
    show_help: RwSignal<bool>,
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

#[component]
fn App() -> impl IntoView {
    let state = provide_gldf_state();
    let shell = ShellState::new();
    provide_context(shell);

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
                    <NavItemViewStaticLabel item=NavItem::Labels icon="🏷️" label="Labels" badge=None has_file=Signal::derive(has_file) />
                    <NavItemViewStaticLabel item=NavItem::Hyperlinks icon="🔗" label="Hyperlinks" badge=None has_file=Signal::derive(has_file) />
                    <NavItemViewStaticLabel item=NavItem::Pictures icon="🖼️" label="Pictures" badge=None has_file=Signal::derive(has_file) />
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
                    <NavItemViewStaticLabel item=NavItem::Spectra icon="🌈" label="Spectra" badge=None has_file=Signal::derive(has_file) />
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
        <li class=class on:click=on_click style=style>
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
        <li class=class on:click=on_click style=style>
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
        <div class="main-content">
            {move || if has_file() {
                view! { <Toolbar /><ContentBody /> }.into_any()
            } else {
                view! { <Welcome /> }.into_any()
            }}
            <footer class="app-footer">
                <p>{ "Copyright Holger Trahe - " }<a href="mailto:trahe@mac.com">{ "trahe@mac.com" }</a></p>
            </footer>
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
                <button class="btn btn-secondary" on:click=on_toggle_mode>
                    { mode_label }
                </button>
                <button class="btn btn-success" on:click=on_export_gldf>
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
                <button class="btn btn-outline" on:click=on_clear title=move || t("gldf.toolbar.clear_tooltip")>
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
                type="file"
                accept=".gldf,.ldt,.ies,.ifc,.uld,.rolf"
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
        </div>
    }
}

/// Manual build stamp shown on the welcome screen. Bump on each change you
/// want users to verify they're loading. Cheaper than a full build.rs
/// + git-rev pipeline for now; replace when the build stamp matters enough
/// to be automatic.
const BUILD_STAMP: &str = "2026-06-01 emitter-edit-modal";

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
    } else {
        anyhow::bail!("Unsupported file: {}", filename);
    }
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
        | NavItem::Labels
        | NavItem::Hyperlinks
        | NavItem::Pictures
        | NavItem::Emergency
        | NavItem::CustomProperties => return None,
    })
}

fn static_nav_label(item: NavItem) -> &'static str {
    match item {
        NavItem::Builder => "Builder",
        NavItem::Linkage => "Linkage map",
        NavItem::Spectra => "Spectra",
        NavItem::Labels => "Labels",
        NavItem::Hyperlinks => "Hyperlinks",
        NavItem::Pictures => "Pictures",
        NavItem::Emergency => "Emergency",
        NavItem::CustomProperties => "Custom Properties",
        _ => "",
    }
}
