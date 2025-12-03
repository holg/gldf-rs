#![recursion_limit = "256"]
//! GLDF WASM Editor Application

extern crate base64;
extern crate gldf_rs;

use std::collections::HashMap;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use gloo::file::callbacks::FileReader;
use gloo::file::File;
use gloo::console;
use web_sys::{FileList, HtmlInputElement, Blob};
use yew::prelude::*;
use gldf_rs::gldf::GldfProduct;
use gldf_rs::{FileBufGldf, BufFile};

mod components;
mod state;
mod draw_l3d;

use components::{EditorTabs, LdtViewer, L3dViewer};
use state::{GldfProvider, GldfAction, use_gldf};

/// Wrapper for GLDF product operations
#[allow(dead_code)]
struct WasmGldfProduct(GldfProduct);

impl WasmGldfProduct {
    pub fn load_gldf_from_buf_all(buf: Vec<u8>) -> anyhow::Result<FileBufGldf> {
        let file_buf = GldfProduct::load_gldf_from_buf_all(buf)?;
        Ok(file_buf)
    }
}

/// File details structure
struct FileDetails {
    name: String,
    file_type: String,
    data: Vec<u8>,
}

/// Navigation items
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NavItem {
    Overview,
    RawData,
    FileViewer,
    Header,
    Statistics,
    Files,
    LightSources,
    Variants,
}

/// Application messages
pub enum Msg {
    Loaded(String, String, Vec<u8>),
    Files(Vec<File>),
    Navigate(NavItem),
    ToggleEditor,
    ExportJson,
    ExportXml,
    SetDragging(bool),
}

/// Mode of the application
#[derive(Clone, PartialEq)]
enum AppMode {
    Viewer,
    Editor,
}

/// Main application state
pub struct App {
    readers: HashMap<String, FileReader>,
    files: Vec<FileDetails>,
    mode: AppMode,
    loaded_gldf: Option<FileBufGldf>,
    nav_item: NavItem,
    is_dragging: bool,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            readers: HashMap::default(),
            files: Vec::default(),
            mode: AppMode::Viewer,
            loaded_gldf: None,
            nav_item: NavItem::Overview,
            is_dragging: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Loaded(file_name, file_type, data) => {
                console::log!("Got Files:", file_type.as_str());

                // Try to parse GLDF
                if file_name.ends_with(".gldf") {
                    if let Ok(gldf) = WasmGldfProduct::load_gldf_from_buf_all(data.clone()) {
                        self.loaded_gldf = Some(gldf);
                    }
                }

                self.files.push(FileDetails {
                    data,
                    file_type,
                    name: file_name.clone(),
                });
                self.readers.remove(&file_name);
                true
            }
            Msg::Files(files) => {
                for file in files.into_iter() {
                    let file_name = file.name();
                    let file_type = file.raw_mime_type();
                    console::log!("A File:", file_name.as_str(), file_type.as_str());

                    let task = {
                        let link = ctx.link().clone();
                        let file_name = file_name.clone();
                        console::log!("A File:", file_name.as_str(), file_type.as_str());

                        gloo::file::callbacks::read_as_bytes(&file, move |res| {
                            link.send_message(Msg::Loaded(
                                file_name,
                                file_type,
                                res.expect("failed to read file"),
                            ))
                        })
                    };
                    self.readers.insert(file_name, task);
                }
                true
            }
            Msg::Navigate(item) => {
                self.nav_item = item;
                true
            }
            Msg::ToggleEditor => {
                self.mode = match self.mode {
                    AppMode::Viewer => AppMode::Editor,
                    AppMode::Editor => AppMode::Viewer,
                };
                true
            }
            Msg::ExportJson => {
                if let Some(ref gldf) = self.loaded_gldf {
                    if let Ok(json) = gldf.gldf.to_pretty_json() {
                        console::log!("Exported JSON:", json.as_str());
                        // TODO: Trigger download
                    }
                }
                false
            }
            Msg::ExportXml => {
                if let Some(ref gldf) = self.loaded_gldf {
                    if let Ok(xml) = gldf.gldf.to_xml() {
                        console::log!("Exported XML:", xml.as_str());
                        // TODO: Trigger download
                    }
                }
                false
            }
            Msg::SetDragging(dragging) => {
                self.is_dragging = dragging;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div id="wrapper">
                // Sidebar
                { self.view_sidebar(ctx) }

                // Main Content Area
                <div class="main-content">
                    {
                        if self.loaded_gldf.is_some() || !self.files.is_empty() {
                            self.view_content(ctx)
                        } else {
                            self.view_welcome(ctx)
                        }
                    }
                </div>
            </div>
        }
    }
}

fn parse_file_for_gldf(file: &FileDetails) -> FileBufGldf {
    WasmGldfProduct::load_gldf_from_buf_all(file.data.clone()).unwrap()
}

pub fn get_blob(buf_file: &BufFile) -> String {
    let uint8arr = js_sys::Uint8Array::new(
        &unsafe { js_sys::Uint8Array::view(&buf_file.content.clone().unwrap()) }.into(),
    );
    let array = js_sys::Array::new();
    array.push(&uint8arr.buffer());
    let blob = Blob::new_with_str_sequence_and_options(
        &array,
        web_sys::BlobPropertyBag::new()
            .type_("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
    )
    .unwrap();
    web_sys::Url::create_object_url_with_blob(&blob).unwrap()
}

impl App {
    fn view_sidebar(&self, ctx: &Context<Self>) -> Html {
        let has_file = self.loaded_gldf.is_some() || !self.files.is_empty();
        let files_count = self.loaded_gldf.as_ref().map(|g| g.files.len()).unwrap_or(0);
        let light_sources_count = self.loaded_gldf.as_ref()
            .map(|g| {
                g.gldf.general_definitions.light_sources.as_ref()
                    .map(|ls| ls.fixed_light_source.len() + ls.changeable_light_source.len())
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        let variants_count = self.loaded_gldf.as_ref()
            .map(|g| g.gldf.product_definitions.variants.as_ref().map(|v| v.variant.len()).unwrap_or(0))
            .unwrap_or(0);

        html! {
            <div class="sidebar">
                <div class="sidebar-header">
                    <h1>
                        <span class="icon">{ "💡" }</span>
                        { "GLDF Viewer" }
                    </h1>
                </div>

                <div class="sidebar-section">
                    <div class="sidebar-section-title">{ "Viewer" }</div>
                    <ul class="sidebar-nav">
                        { self.nav_item(ctx, NavItem::Overview, "📊", "Overview", None, has_file) }
                        { self.nav_item(ctx, NavItem::RawData, "{ }", "Raw Data", None, has_file) }
                        { self.nav_item(ctx, NavItem::FileViewer, "👁", "File Viewer", Some(self.files.len()), has_file) }
                    </ul>
                </div>

                <div class="sidebar-section">
                    <div class="sidebar-section-title">{ "Document" }</div>
                    <ul class="sidebar-nav">
                        { self.nav_item(ctx, NavItem::Header, "📄", "Header", None, has_file) }
                        { self.nav_item(ctx, NavItem::Statistics, "📈", "Statistics", None, has_file) }
                    </ul>
                </div>

                <div class="sidebar-section">
                    <div class="sidebar-section-title">{ "Definitions" }</div>
                    <ul class="sidebar-nav">
                        { self.nav_item(ctx, NavItem::Files, "📁", "Files", Some(files_count), has_file) }
                        { self.nav_item(ctx, NavItem::LightSources, "💡", "Light Sources", Some(light_sources_count), has_file) }
                        { self.nav_item(ctx, NavItem::Variants, "📦", "Variants", Some(variants_count), has_file) }
                    </ul>
                </div>

                // Links section at bottom
                <div style="margin-top: auto; padding: 16px;">
                    <div style="font-size: 11px; color: var(--text-tertiary); margin-bottom: 8px;">{ "Resources" }</div>
                    <a href="https://gldf.io" target="_blank" style="display: block; font-size: 12px; margin-bottom: 4px;">{ "GLDF.io" }</a>
                    <a href="https://eulumdat.icu/" target="_blank" style="display: block; font-size: 12px; margin-bottom: 8px;">{ "QLumEdit" }</a>
                    <p class="privacy-note">{ "All processing is local" }</p>
                </div>
            </div>
        }
    }

    fn nav_item(&self, ctx: &Context<Self>, item: NavItem, icon: &str, label: &str, badge: Option<usize>, enabled: bool) -> Html {
        let is_active = self.nav_item == item;
        let onclick = ctx.link().callback(move |_| Msg::Navigate(item));
        let class = classes!(
            "sidebar-nav-item",
            is_active.then(|| "active"),
            (!enabled).then(|| "disabled")
        );

        html! {
            <li class={class} onclick={onclick} style={if !enabled { "opacity: 0.5; pointer-events: none;" } else { "" }}>
                <span class="icon">{ icon }</span>
                <span>{ label }</span>
                if let Some(count) = badge {
                    <span class="badge">{ count }</span>
                }
            </li>
        }
    }

    fn view_welcome(&self, ctx: &Context<Self>) -> Html {
        let ondrop = ctx.link().callback(|event: DragEvent| {
            event.prevent_default();
            let files = event.data_transfer().unwrap().files();
            Self::upload_files(files)
        });
        let ondragover = Callback::from(|event: DragEvent| {
            event.prevent_default();
        });
        let ondragenter = ctx.link().callback(|event: DragEvent| {
            event.prevent_default();
            Msg::SetDragging(true)
        });
        let ondragleave = ctx.link().callback(|_: DragEvent| {
            Msg::SetDragging(false)
        });

        let welcome_class = classes!(
            "welcome-view",
            self.is_dragging.then(|| "dragging")
        );

        html! {
            <div class={welcome_class}
                ondrop={ondrop}
                ondragover={ondragover}
                ondragenter={ondragenter}
                ondragleave={ondragleave}
            >
                <div class="welcome-icon">{ "💡" }</div>
                <h1 class="welcome-title">{ "GLDF Viewer" }</h1>
                <p class="welcome-subtitle">{ "Global Lighting Data Format" }</p>

                <div class="welcome-divider"></div>

                <p class="welcome-instructions">{ "Drop a GLDF, LDT, or IES file here" }</p>
                <p class="welcome-or">{ "or" }</p>

                <div class="welcome-buttons">
                    <label for="file-upload" class="btn btn-primary">{ "Open File..." }</label>
                </div>

                <input
                    id="file-upload"
                    type="file"
                    accept=".gldf,.ldt,.ies"
                    multiple={false}
                    onchange={ctx.link().callback(move |e: Event| {
                        let input: HtmlInputElement = e.target_unchecked_into();
                        Self::upload_files(input.files())
                    })}
                />

                <div style="margin-top: 32px; max-width: 400px;">
                    <p class="privacy-note" style="text-align: center;">
                        { "All processing happens locally in your browser - no data is uploaded." }
                    </p>
                </div>
            </div>
        }
    }

    fn view_content(&self, ctx: &Context<Self>) -> Html {
        let title = match self.nav_item {
            NavItem::Overview => "Overview",
            NavItem::RawData => "Raw Data",
            NavItem::FileViewer => "File Viewer",
            NavItem::Header => "Header",
            NavItem::Statistics => "Statistics",
            NavItem::Files => "Files",
            NavItem::LightSources => "Light Sources",
            NavItem::Variants => "Variants",
        };

        html! {
            <>
                // Content Header with toolbar
                <div class="content-header">
                    <h1 class="content-title">{ title }</h1>

                    if self.loaded_gldf.is_some() {
                        <div class="toolbar-actions">
                            <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::ToggleEditor)}>
                                { if self.mode == AppMode::Viewer { "Edit Mode" } else { "View Mode" } }
                            </button>
                            <button class="btn btn-success" onclick={ctx.link().callback(|_| Msg::ExportJson)}>
                                { "Export JSON" }
                            </button>
                            <button class="btn btn-success" onclick={ctx.link().callback(|_| Msg::ExportXml)}>
                                { "Export XML" }
                            </button>
                        </div>
                    }
                </div>

                // Content Body
                <div class="content-body">
                    {
                        match self.nav_item {
                            NavItem::Overview => self.view_overview(),
                            NavItem::RawData => self.view_raw_data(),
                            NavItem::FileViewer => self.view_file_viewer(ctx),
                            NavItem::Header => self.view_header_editor(),
                            NavItem::Statistics => self.view_statistics(),
                            NavItem::Files => self.view_files_list(),
                            NavItem::LightSources => self.view_light_sources(),
                            NavItem::Variants => self.view_variants(),
                        }
                    }
                </div>

                // Footer
                <footer class="app-footer">
                    <p>{ "Copyright Holger Trahe - " }<a href="mailto:trahe@mac.com">{ "trahe@mac.com" }</a></p>
                </footer>
            </>
        }
    }

    fn view_overview(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            let header = &gldf.gldf.header;
            let files_count = gldf.files.len();
            let fixed_ls = gldf.gldf.general_definitions.light_sources.as_ref()
                .map(|ls| ls.fixed_light_source.len()).unwrap_or(0);
            let changeable_ls = gldf.gldf.general_definitions.light_sources.as_ref()
                .map(|ls| ls.changeable_light_source.len()).unwrap_or(0);
            let variants_count = gldf.gldf.product_definitions.variants.as_ref()
                .map(|v| v.variant.len()).unwrap_or(0);
            let photometries_count = gldf.gldf.general_definitions.photometries.as_ref()
                .map(|p| p.photometry.len()).unwrap_or(0);

            html! {
                <>
                    // Product Info Card
                    <div class="card product-info-card">
                        <div class="card-body">
                            <div class="product-info-header">
                                <span class="icon">{ "💡" }</span>
                                <div>
                                    <h2>{ if header.manufacturer.is_empty() { "Unknown Manufacturer" } else { &header.manufacturer } }</h2>
                                    <p class="format-version">{ format!("GLDF Format {:?}", header.format_version) }</p>
                                </div>
                            </div>
                            <div class="info-grid">
                                <div class="info-row">
                                    <span class="label">{ "Author" }</span>
                                    <span class="value">{ if header.author.is_empty() { "—" } else { &header.author } }</span>
                                </div>
                                <div class="info-row">
                                    <span class="label">{ "Created With" }</span>
                                    <span class="value">{ if header.created_with_application.is_empty() { "—" } else { &header.created_with_application } }</span>
                                </div>
                                <div class="info-row">
                                    <span class="label">{ "Creation Date" }</span>
                                    <span class="value">{ if header.creation_time_code.is_empty() { "—" } else { &header.creation_time_code } }</span>
                                </div>
                                <div class="info-row">
                                    <span class="label">{ "Format Version" }</span>
                                    <span class="value">{ format!("{:?}", header.format_version) }</span>
                                </div>
                            </div>
                        </div>
                    </div>

                    // Statistics Grid
                    <div class="stats-grid">
                        <div class="stat-card">
                            <div class="icon blue">{ "📁" }</div>
                            <div class="value">{ files_count }</div>
                            <div class="label">{ "Files" }</div>
                        </div>
                        <div class="stat-card">
                            <div class="icon yellow">{ "💡" }</div>
                            <div class="value">{ fixed_ls + changeable_ls }</div>
                            <div class="label">{ "Light Sources" }</div>
                        </div>
                        <div class="stat-card">
                            <div class="icon purple">{ "📦" }</div>
                            <div class="value">{ variants_count }</div>
                            <div class="label">{ "Variants" }</div>
                        </div>
                        <div class="stat-card">
                            <div class="icon orange">{ "☀️" }</div>
                            <div class="value">{ photometries_count }</div>
                            <div class="label">{ "Photometries" }</div>
                        </div>
                    </div>

                    // Content Columns
                    <div class="content-columns">
                        <div class="content-column">
                            // Files Overview
                            { self.view_files_overview() }

                            // Light Sources Overview
                            { self.view_light_sources_overview() }
                        </div>
                        <div class="content-column">
                            // Variants Overview
                            { self.view_variants_overview() }
                        </div>
                    </div>
                </>
            }
        } else {
            self.view_files_only_overview()
        }
    }

    fn view_files_only_overview(&self) -> Html {
        html! {
            <div class="stats-grid" style="grid-template-columns: 1fr;">
                <div class="stat-card">
                    <div class="icon blue">{ "📄" }</div>
                    <div class="value">{ self.files.len() }</div>
                    <div class="label">{ "Files Loaded" }</div>
                </div>
            </div>
        }
    }

    fn view_files_overview(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            let files = &gldf.files;
            let photometric: Vec<_> = files.iter().filter(|f| {
                f.name.as_ref().map(|n| n.ends_with(".ldt") || n.ends_with(".ies")).unwrap_or(false)
            }).collect();
            let images: Vec<_> = files.iter().filter(|f| {
                f.name.as_ref().map(|n| n.ends_with(".jpg") || n.ends_with(".png")).unwrap_or(false)
            }).collect();
            let geometry: Vec<_> = files.iter().filter(|f| {
                f.name.as_ref().map(|n| n.ends_with(".l3d")).unwrap_or(false)
            }).collect();

            html! {
                <div class="collapsible open">
                    <div class="collapsible-header">
                        <span class="icon blue">{ "📁" }</span>
                        <h4>{ "Files" }</h4>
                        <span class="count">{ files.len() }</span>
                        <span class="chevron">{ "▶" }</span>
                    </div>
                    <div class="collapsible-content">
                        if !photometric.is_empty() {
                            <div class="file-category">
                                <div class="file-category-title">{ "Photometric" }</div>
                                { for photometric.iter().take(3).map(|f| {
                                    html! {
                                        <div class="file-item">
                                            <span class="icon" style="color: var(--accent-orange);">{ "☀️" }</span>
                                            <span class="name">{ f.name.clone().unwrap_or_default() }</span>
                                        </div>
                                    }
                                })}
                                if photometric.len() > 3 {
                                    <div class="more-items">{ format!("+ {} more...", photometric.len() - 3) }</div>
                                }
                            </div>
                        }
                        if !images.is_empty() {
                            <div class="file-category">
                                <div class="file-category-title">{ "Images" }</div>
                                { for images.iter().take(3).map(|f| {
                                    html! {
                                        <div class="file-item">
                                            <span class="icon" style="color: var(--accent-blue);">{ "🖼" }</span>
                                            <span class="name">{ f.name.clone().unwrap_or_default() }</span>
                                        </div>
                                    }
                                })}
                                if images.len() > 3 {
                                    <div class="more-items">{ format!("+ {} more...", images.len() - 3) }</div>
                                }
                            </div>
                        }
                        if !geometry.is_empty() {
                            <div class="file-category">
                                <div class="file-category-title">{ "Geometry" }</div>
                                { for geometry.iter().take(3).map(|f| {
                                    html! {
                                        <div class="file-item">
                                            <span class="icon" style="color: var(--accent-green);">{ "🧊" }</span>
                                            <span class="name">{ f.name.clone().unwrap_or_default() }</span>
                                        </div>
                                    }
                                })}
                                if geometry.len() > 3 {
                                    <div class="more-items">{ format!("+ {} more...", geometry.len() - 3) }</div>
                                }
                            </div>
                        }
                    </div>
                </div>
            }
        } else {
            html! {}
        }
    }

    fn view_light_sources_overview(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            let ls = gldf.gldf.general_definitions.light_sources.as_ref();
            let fixed: Vec<_> = ls.map(|l| l.fixed_light_source.iter().collect()).unwrap_or_default();
            let changeable: Vec<_> = ls.map(|l| l.changeable_light_source.iter().collect()).unwrap_or_default();
            let total = fixed.len() + changeable.len();

            html! {
                <div class="collapsible open">
                    <div class="collapsible-header">
                        <span class="icon yellow">{ "💡" }</span>
                        <h4>{ "Light Sources" }</h4>
                        <span class="count">{ total }</span>
                        <span class="chevron">{ "▶" }</span>
                    </div>
                    <div class="collapsible-content">
                        if !fixed.is_empty() {
                            <div class="file-category-title">{ "Fixed" }</div>
                            { for fixed.iter().take(5).map(|ls| {
                                html! {
                                    <div class="light-source-item">
                                        <span class="icon" style="color: var(--accent-yellow);">{ "💡" }</span>
                                        <div class="info">
                                            <div class="name">{ ls.name.locale.first().map(|l| l.value.as_str()).unwrap_or("") }</div>
                                            <div class="id">{ format!("ID: {}", ls.id) }</div>
                                        </div>
                                    </div>
                                }
                            })}
                            if fixed.len() > 5 {
                                <div class="more-items">{ format!("+ {} more...", fixed.len() - 5) }</div>
                            }
                        }
                        if !changeable.is_empty() {
                            <div class="file-category-title" style="margin-top: 12px;">{ "Changeable" }</div>
                            { for changeable.iter().take(5).map(|ls| {
                                html! {
                                    <div class="light-source-item">
                                        <span class="icon" style="color: var(--accent-orange);">{ "💡" }</span>
                                        <div class="info">
                                            <div class="name">{ &ls.name.value }</div>
                                            <div class="id">{ format!("ID: {}", ls.id) }</div>
                                        </div>
                                    </div>
                                }
                            })}
                            if changeable.len() > 5 {
                                <div class="more-items">{ format!("+ {} more...", changeable.len() - 5) }</div>
                            }
                        }
                        if total == 0 {
                            <p style="color: var(--text-tertiary); font-size: 13px;">{ "No light sources defined" }</p>
                        }
                    </div>
                </div>
            }
        } else {
            html! {}
        }
    }

    fn view_variants_overview(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            let variants: Vec<_> = gldf.gldf.product_definitions.variants.as_ref()
                .map(|v| v.variant.iter().collect()).unwrap_or_default();

            html! {
                <div class="collapsible open">
                    <div class="collapsible-header">
                        <span class="icon purple">{ "📦" }</span>
                        <h4>{ "Variants" }</h4>
                        <span class="count">{ variants.len() }</span>
                        <span class="chevron">{ "▶" }</span>
                    </div>
                    <div class="collapsible-content">
                        if variants.is_empty() {
                            <p style="color: var(--text-tertiary); font-size: 13px;">{ "No variants defined" }</p>
                        } else {
                            { for variants.iter().take(10).map(|v| {
                                let name = v.name.as_ref()
                                    .and_then(|n| n.locale.first())
                                    .map(|l| l.value.as_str())
                                    .filter(|s| !s.is_empty())
                                    .unwrap_or(&v.id);
                                let desc = v.description.as_ref()
                                    .and_then(|d| d.locale.first())
                                    .map(|l| l.value.as_str())
                                    .filter(|s| !s.is_empty());
                                html! {
                                    <div class="variant-item">
                                        <div class="variant-item-header">
                                            <span class="icon">{ "📦" }</span>
                                            <span class="name">{ name }</span>
                                        </div>
                                        if let Some(description) = desc {
                                            <div class="description">{ description }</div>
                                        }
                                        <div class="id">{ format!("ID: {}", v.id) }</div>
                                    </div>
                                }
                            })}
                            if variants.len() > 10 {
                                <div class="more-items">{ format!("+ {} more...", variants.len() - 10) }</div>
                            }
                        }
                    </div>
                </div>
            }
        } else {
            html! {}
        }
    }

    fn view_raw_data(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            if let Ok(json) = gldf.gldf.to_pretty_json() {
                html! {
                    <div class="code-block">
                        <pre>{ json }</pre>
                    </div>
                }
            } else {
                html! { <p>{ "Failed to serialize GLDF data" }</p> }
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="icon">{ "{ }" }</div>
                    <h3>{ "No Raw Data" }</h3>
                    <p>{ "Load a GLDF file to view raw data" }</p>
                </div>
            }
        }
    }

    fn view_file_viewer(&self, _ctx: &Context<Self>) -> Html {
        console::log!(format!("[FileViewer] files count: {}", self.files.len()));

        if self.files.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="icon">{ "📂" }</div>
                    <h3>{ "No Files Loaded" }</h3>
                    <p>{ "Drop a file to view it here" }</p>
                </div>
            };
        }

        // Log file info
        for f in &self.files {
            let is_l3d = f.name.to_lowercase().ends_with(".l3d");
            console::log!(format!("[FileViewer] File: {}, type: {}, data: {} bytes, is_l3d: {}", f.name, f.file_type, f.data.len(), is_l3d));
        }

        html! {
            <div id="preview-area">
                { for self.files.iter().map(|f| Self::view_file(f)) }
            </div>
        }
    }

    fn view_header_editor(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            if self.mode == AppMode::Editor {
                html! {
                    <GldfProviderWithData gldf={gldf.gldf.clone()}>
                        <EditorTabs />
                    </GldfProviderWithData>
                }
            } else {
                let header = &gldf.gldf.header;
                html! {
                    <div class="card">
                        <div class="card-body">
                            <div class="form-row">
                                <div class="form-group">
                                    <label>{ "Manufacturer" }</label>
                                    <input type="text" readonly=true value={header.manufacturer.clone()} />
                                </div>
                                <div class="form-group">
                                    <label>{ "Author" }</label>
                                    <input type="text" readonly=true value={header.author.clone()} />
                                </div>
                            </div>
                            <div class="form-row">
                                <div class="form-group">
                                    <label>{ "Format Version" }</label>
                                    <input type="text" readonly=true value={format!("{:?}", header.format_version)} />
                                </div>
                                <div class="form-group">
                                    <label>{ "Created With" }</label>
                                    <input type="text" readonly=true value={header.created_with_application.clone()} />
                                </div>
                            </div>
                            <div class="form-group">
                                <label>{ "Creation Time" }</label>
                                <input type="text" readonly=true value={header.creation_time_code.clone()} />
                            </div>
                        </div>
                    </div>
                }
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="icon">{ "📄" }</div>
                    <h3>{ "No Header Data" }</h3>
                    <p>{ "Load a GLDF file to view header information" }</p>
                </div>
            }
        }
    }

    fn view_statistics(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            let files_count = gldf.files.len();
            let fixed_ls = gldf.gldf.general_definitions.light_sources.as_ref()
                .map(|ls| ls.fixed_light_source.len()).unwrap_or(0);
            let changeable_ls = gldf.gldf.general_definitions.light_sources.as_ref()
                .map(|ls| ls.changeable_light_source.len()).unwrap_or(0);
            let variants_count = gldf.gldf.product_definitions.variants.as_ref()
                .map(|v| v.variant.len()).unwrap_or(0);
            let photometries_count = gldf.gldf.general_definitions.photometries.as_ref()
                .map(|p| p.photometry.len()).unwrap_or(0);
            let geometries_count = gldf.gldf.general_definitions.geometries.as_ref()
                .map(|g| g.simple_geometry.len() + g.model_geometry.len()).unwrap_or(0);

            html! {
                <div class="stats-grid" style="grid-template-columns: repeat(3, 1fr);">
                    <div class="stat-card">
                        <div class="icon blue">{ "📁" }</div>
                        <div class="value">{ files_count }</div>
                        <div class="label">{ "Embedded Files" }</div>
                    </div>
                    <div class="stat-card">
                        <div class="icon yellow">{ "💡" }</div>
                        <div class="value">{ fixed_ls }</div>
                        <div class="label">{ "Fixed Light Sources" }</div>
                    </div>
                    <div class="stat-card">
                        <div class="icon orange">{ "💡" }</div>
                        <div class="value">{ changeable_ls }</div>
                        <div class="label">{ "Changeable Light Sources" }</div>
                    </div>
                    <div class="stat-card">
                        <div class="icon purple">{ "📦" }</div>
                        <div class="value">{ variants_count }</div>
                        <div class="label">{ "Variants" }</div>
                    </div>
                    <div class="stat-card">
                        <div class="icon orange">{ "☀️" }</div>
                        <div class="value">{ photometries_count }</div>
                        <div class="label">{ "Photometries" }</div>
                    </div>
                    <div class="stat-card">
                        <div class="icon green">{ "🧊" }</div>
                        <div class="value">{ geometries_count }</div>
                        <div class="label">{ "Geometries" }</div>
                    </div>
                </div>
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="icon">{ "📊" }</div>
                    <h3>{ "No Statistics" }</h3>
                    <p>{ "Load a GLDF file to view statistics" }</p>
                </div>
            }
        }
    }

    fn view_files_list(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            let files: &Vec<_> = &gldf.gldf.general_definitions.files.file;

            html! {
                <div class="card">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>{ "ID" }</th>
                                <th>{ "File Name" }</th>
                                <th>{ "Content Type" }</th>
                                <th>{ "Type" }</th>
                            </tr>
                        </thead>
                        <tbody>
                            { for files.iter().map(|f| {
                                html! {
                                    <tr>
                                        <td style="font-family: var(--font-mono);">{ &f.id }</td>
                                        <td>{ &f.file_name }</td>
                                        <td>{ &f.content_type }</td>
                                        <td>{ &f.type_attr }</td>
                                    </tr>
                                }
                            })}
                        </tbody>
                    </table>
                </div>
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="icon">{ "📁" }</div>
                    <h3>{ "No Files" }</h3>
                    <p>{ "Load a GLDF file to view file definitions" }</p>
                </div>
            }
        }
    }

    fn view_light_sources(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            let ls = gldf.gldf.general_definitions.light_sources.as_ref();
            let fixed: Vec<_> = ls.map(|l| l.fixed_light_source.iter().collect()).unwrap_or_default();
            let changeable: Vec<_> = ls.map(|l| l.changeable_light_source.iter().collect()).unwrap_or_default();

            html! {
                <>
                    if !fixed.is_empty() {
                        <h3 style="margin-bottom: 16px; color: var(--text-secondary);">{ "Fixed Light Sources" }</h3>
                        <div class="light-source-cards">
                            { for fixed.iter().map(|ls| {
                                let name = ls.name.locale.first().map(|l| l.value.as_str()).unwrap_or("");
                                let desc = ls.description.as_ref()
                                    .and_then(|d| d.locale.first())
                                    .map(|l| l.value.as_str())
                                    .filter(|s| !s.is_empty());
                                html! {
                                    <div class="light-source-card">
                                        <div class="card-header-row">
                                            <span class="card-id">{ &ls.id }</span>
                                            <span class="card-type">{ "Fixed" }</span>
                                        </div>
                                        <div class="card-content">
                                            <h4>{ name }</h4>
                                            if let Some(description) = desc {
                                                <div class="description">{ description }</div>
                                            }
                                        </div>
                                    </div>
                                }
                            })}
                        </div>
                    }
                    if !changeable.is_empty() {
                        <h3 style="margin: 24px 0 16px; color: var(--text-secondary);">{ "Changeable Light Sources" }</h3>
                        <div class="light-source-cards">
                            { for changeable.iter().map(|ls| {
                                let desc = ls.description.as_ref()
                                    .map(|d| d.value.as_str())
                                    .filter(|s| !s.is_empty());
                                html! {
                                    <div class="light-source-card">
                                        <div class="card-header-row">
                                            <span class="card-id">{ &ls.id }</span>
                                            <span class="card-type changeable">{ "Changeable" }</span>
                                        </div>
                                        <div class="card-content">
                                            <h4>{ &ls.name.value }</h4>
                                            if let Some(description) = desc {
                                                <div class="description">{ description }</div>
                                            }
                                        </div>
                                    </div>
                                }
                            })}
                        </div>
                    }
                    if fixed.is_empty() && changeable.is_empty() {
                        <div class="empty-state">
                            <div class="icon">{ "💡" }</div>
                            <h3>{ "No Light Sources" }</h3>
                            <p>{ "This GLDF file has no light source definitions" }</p>
                        </div>
                    }
                </>
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="icon">{ "💡" }</div>
                    <h3>{ "No Light Sources" }</h3>
                    <p>{ "Load a GLDF file to view light sources" }</p>
                </div>
            }
        }
    }

    fn view_variants(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            let variants: Vec<_> = gldf.gldf.product_definitions.variants.as_ref()
                .map(|v| v.variant.iter().collect()).unwrap_or_default();

            if variants.is_empty() {
                return html! {
                    <div class="empty-state">
                        <div class="icon">{ "📦" }</div>
                        <h3>{ "No Variants" }</h3>
                        <p>{ "This GLDF file has no variant definitions" }</p>
                    </div>
                };
            }

            html! {
                <div class="variant-cards">
                    { for variants.iter().map(|v| {
                        let name = v.name.as_ref()
                            .and_then(|n| n.locale.first())
                            .map(|l| l.value.as_str())
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&v.id);
                        let desc = v.description.as_ref()
                            .and_then(|d| d.locale.first())
                            .map(|l| l.value.as_str())
                            .filter(|s| !s.is_empty());
                        let product_num = v.product_number.as_ref()
                            .and_then(|p| p.locale.first())
                            .map(|l| l.value.as_str())
                            .filter(|s| !s.is_empty());
                        html! {
                            <div class="variant-card">
                                <div class="card-header-row">
                                    <span class="card-id">{ &v.id }</span>
                                    if let Some(order) = v.sort_order.filter(|&o| o > 0) {
                                        <span style="font-size: 11px; color: var(--text-tertiary);">{ format!("#{}", order) }</span>
                                    }
                                </div>
                                <div class="card-content">
                                    <h4>{ name }</h4>
                                    if let Some(description) = desc {
                                        <div class="description">{ description }</div>
                                    }
                                    if let Some(pn) = product_num {
                                        <div class="detail">
                                            <span class="label">{ "Product #:" }</span>
                                            <span>{ pn }</span>
                                        </div>
                                    }
                                </div>
                            </div>
                        }
                    })}
                </div>
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="icon">{ "📦" }</div>
                    <h3>{ "No Variants" }</h3>
                    <p>{ "Load a GLDF file to view variants" }</p>
                </div>
            }
        }
    }

    fn view_file(file: &FileDetails) -> Html {
        fn view_l3d_file(buf_file: &BufFile) -> Html {
            let l3d_name = buf_file.name.clone().unwrap_or_default();
            let l3d_content = buf_file.content.clone().unwrap_or_default();
            console::log!(format!("[GLDF->L3D] Rendering L3D: {}, content_len: {}", l3d_name, l3d_content.len()));
            if l3d_content.len() > 20 {
                console::log!(format!("[GLDF->L3D] First 20 bytes: {:?}", &l3d_content[..20]));
            }
            if !l3d_content.is_empty() {
                html! {
                    <div class="l3d-container">
                        <div class="l3d-header">
                            <span class="icon">{ "🧊" }</span>
                            <span class="name">{ l3d_name.clone() }</span>
                        </div>
                        <div class="l3d-canvas-container">
                            <L3dViewer l3d_data={l3d_content} width={500} height={400} />
                        </div>
                    </div>
                }
            } else {
                html! {
                    <p style="color: var(--text-tertiary);">{ "L3D file (external URL reference)" }</p>
                }
            }
        }

        fn view_gldf_file(buf_file: &BufFile) -> Html {
            html! {
                <div id="buf_file">
                    <p>{ format!("{}", buf_file.name.clone().unwrap_or_default()) }</p>
                    if buf_file.name.clone().unwrap_or_default().to_lowercase().ends_with(".jpg") {
                        if buf_file.content.clone().unwrap_or_default().len() > 0 {
                            <img src={format!("data:image/jpeg;base64,{}", BASE64_STANDARD.encode(buf_file.clone().content.unwrap_or_default()))} />
                        } else {
                            <p style="color: var(--text-tertiary);">{ "JPG image (external URL)" }</p>
                        }
                    }
                    else if buf_file.name.clone().unwrap_or_default().to_lowercase().ends_with(".png") {
                        if buf_file.content.clone().unwrap_or_default().len() > 0 {
                            <img src={format!("data:image/png;base64,{}", BASE64_STANDARD.encode(buf_file.clone().content.unwrap_or_default()))} />
                        } else {
                            <p style="color: var(--text-tertiary);">{ "PNG image (external URL)" }</p>
                        }
                    }
                    else if buf_file.name.clone().unwrap_or_default().to_lowercase().ends_with(".ldt") {
                        if buf_file.content.clone().unwrap_or_default().len() > 0 {
                            <a href={format!(r"/QLumEdit/QLumEdit.html?ldc_name=trahe.ldt&ldc_blob_url={}", get_blob(buf_file))} style="display: inline-block; margin-bottom: 8px;">
                                { "Open in QLumEdit" }
                            </a>
                            <LdtViewer ldt_data={buf_file.content.clone().unwrap_or_default()} width={400.0} height={400.0} />
                        } else {
                            <p style="color: var(--text-tertiary);">{ "LDT file (external URL reference)" }</p>
                        }
                    }
                    else if buf_file.name.clone().unwrap_or_default().to_lowercase().ends_with(".xml") {
                        if let Some(content) = buf_file.content.clone() {
                            if !content.is_empty() {
                                <textarea readonly=true value={format!(r"{}", String::from_utf8_lossy(content.as_slice()))}></textarea>
                            }
                        }
                    }
                    else if buf_file.name.clone().unwrap_or_default().to_lowercase().ends_with(".l3d") {
                        { view_l3d_file(buf_file) }
                    }
                </div>
            }
        }

        fn view_url_files(files: &gldf_rs::gldf::general_definitions::files::Files) -> Html {
            let files = &files.file;
            let url_files: Vec<_> = files.iter()
                .filter(|f| f.type_attr == "url")
                .collect();
            if url_files.is_empty() {
                return html! {};
            }
            html! {
                <div class="url-files">
                    <p style="font-size: 13px; font-weight: 500; color: var(--text-secondary); margin-bottom: 8px;">{ "External URL References:" }</p>
                    { for url_files.iter().map(|f| {
                        html! {
                            <div class="url-file-entry">
                                <p>{ format!("{} ({})", f.file_name, f.content_type) }</p>
                                <a href={f.file_name.clone()} target="_blank">
                                    { "Open URL" }
                                </a>
                            </div>
                        }
                    })}
                </div>
            }
        }

        console::log!(
            "Action for file_type:",
            file.file_type.as_str(),
            file.name.as_str()
        );

        let file_name_lower = file.name.to_lowercase();

        // Handle standalone LDT/IES files directly
        if file_name_lower.ends_with(".ldt") || file_name_lower.ends_with(".ies") {
            let file_type_label = if file_name_lower.ends_with(".ies") { "IES" } else { "LDT/Eulumdat" };
            return html! {
                <div class="preview-tile">
                    <div class="preview-header">
                        <span class="icon">{ "☀️" }</span>
                        <span class="preview-name">{ &file.name }</span>
                        <span class="preview-type">{ file_type_label }</span>
                    </div>
                    <div class="preview-media">
                        <LdtViewer ldt_data={file.data.clone()} width={500.0} height={500.0} />
                    </div>
                </div>
            };
        }

        // Handle GLDF files
        if file_name_lower.ends_with(".gldf") {
            let loaded = parse_file_for_gldf(file);
            let buf_files = loaded.files.to_vec();
            console::log!("Files:", buf_files.len());
            console::log!("Author:", loaded.gldf.header.author.as_str());

            return html! {
                <div class="preview-tile">
                    <div class="preview-header">
                        <span class="icon">{ "💡" }</span>
                        <span class="preview-name">{ &file.name }</span>
                        <span class="preview-type">{ "GLDF" }</span>
                    </div>
                    <div class="preview-media">
                        <textarea readonly=true value={format!(r#"{{"product": {}}}"#, loaded.gldf.to_pretty_json().expect("REASON").as_str())}></textarea>
                        { for buf_files.iter().map(view_gldf_file) }
                        { view_url_files(&loaded.gldf.general_definitions.files) }
                    </div>
                </div>
            };
        }

        // Handle other file types
        html! {
            <div class="preview-tile">
                <div class="preview-header">
                    <span class="icon">{ "📄" }</span>
                    <span class="preview-name">{ &file.name }</span>
                </div>
                <div class="preview-media">
                    if file.file_type.contains("url") {
                        <img src={file.name.clone()} />
                    } else if file.file_type.contains("image") {
                        <img src={format!("data:{};base64,{}", file.file_type, BASE64_STANDARD.encode(&file.data))} />
                    } else if file.file_type.contains("video") {
                        <video controls={true}>
                            <source src={format!("data:{};base64,{}", file.file_type, BASE64_STANDARD.encode(&file.data))} type={file.file_type.clone()}/>
                        </video>
                    } else {
                        <p>{ "Unknown file type" }</p>
                    }
                </div>
            </div>
        }
    }

    fn upload_files(files: Option<FileList>) -> Msg {
        let mut result = Vec::new();

        if let Some(files) = files {
            let files = js_sys::try_iter(&files)
                .unwrap()
                .unwrap()
                .map(|v| web_sys::File::from(v.unwrap()))
                .map(File::from);
            result.extend(files);
        }
        Msg::Files(result)
    }
}

/// GLDF Provider wrapper that initializes with data
#[derive(Properties, Clone, PartialEq)]
pub struct GldfProviderWithDataProps {
    pub gldf: GldfProduct,
    #[prop_or_default]
    pub children: Children,
}

#[function_component(GldfProviderWithData)]
pub fn gldf_provider_with_data(props: &GldfProviderWithDataProps) -> Html {
    html! {
        <GldfProvider>
            <GldfInitializer gldf={props.gldf.clone()}>
                { for props.children.iter() }
            </GldfInitializer>
        </GldfProvider>
    }
}

/// Component that initializes the GLDF state
#[derive(Properties, Clone, PartialEq)]
pub struct GldfInitializerProps {
    pub gldf: GldfProduct,
    #[prop_or_default]
    pub children: Children,
}

#[function_component(GldfInitializer)]
pub fn gldf_initializer(props: &GldfInitializerProps) -> Html {
    let state = use_gldf();
    let initialized = use_state(|| false);

    {
        let gldf = props.gldf.clone();
        let state = state.clone();
        let initialized = initialized.clone();
        use_effect_with((), move |_| {
            if !*initialized {
                state.dispatch(GldfAction::Load(gldf));
                initialized.set(true);
            }
            || ()
        });
    }

    html! {
        { for props.children.iter() }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    yew::Renderer::<App>::new().render();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Non-WASM build - do nothing
}
