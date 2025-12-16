#![recursion_limit = "256"]
//! GLDF WASM Editor Application

extern crate base64;
extern crate gldf_rs;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use gldf_rs::convert::ldt_to_gldf;
use gldf_rs::gldf::GldfProduct;
use gldf_rs::{BufFile, FileBufGldf};
use gloo::console;
use gloo::file::callbacks::FileReader;
use gloo::file::File;
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use web_sys::{Blob, FileList, HtmlInputElement};
use yew::prelude::*;

mod components;
mod draw_l3d;
mod state;
mod utils;

use components::{BevySceneViewer, EditorTabs, EmitterConfig, L3dViewer, LdtViewer, UrlFileViewer};
use state::{use_gldf, GldfAction, GldfProvider};

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
    Electrical,
    Applications,
    Photometry,
    Statistics,
    Files,
    LightSources,
    Emitters,
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
    ExportGldf,
    SetDragging(bool),
    LoadDemo,
    DemoLoaded(Result<Vec<u8>, String>),
    Select3dVariant(Option<String>),
    SelectFile(Option<String>),
    // Mounting updates
    ToggleCeilingMount(String, bool),
    ToggleWallMount(String, bool),
    ToggleGroundMount(String, bool),
    ToggleWorkingPlaneMount(String, bool),
    SetCeilingRecessed(String, bool, i32),
    SetCeilingSurfaceMounted(String, bool),
    SetCeilingPendant(String, bool, f64),
    SetWallMountingHeight(String, i32),
    SetGroundPoleTop(String, bool, Option<i32>),
    SetGroundPoleIntegrated(String, bool, Option<i32>),
    // App actions
    ClearAll,
    ToggleHelp,
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
    selected_3d_variant: Option<String>,
    selected_file: Option<String>,
    show_help: bool,
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
            selected_3d_variant: None,
            selected_file: None,
            show_help: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Loaded(file_name, file_type, data) => {
                console::log!("Got Files:", file_type.as_str());

                let file_name_lower = file_name.to_lowercase();

                // Try to parse GLDF
                if file_name_lower.ends_with(".gldf") {
                    if let Ok(gldf) = WasmGldfProduct::load_gldf_from_buf_all(data.clone()) {
                        self.loaded_gldf = Some(gldf);
                    }
                }
                // Handle ULD (DIALux) and ROLF (Relux) files - convert to GLDF
                else if file_name_lower.ends_with(".uld") || file_name_lower.ends_with(".rolf") {
                    #[cfg(feature = "light-convert")]
                    {
                        let format_name = if file_name_lower.ends_with(".uld") {
                            "ULD"
                        } else {
                            "ROLF"
                        };
                        console::log!(format!("Converting {} to GLDF...", format_name).as_str());

                        match light_convert::convert_to_gldf_full(&data, Some(&file_name), None) {
                            Ok(result) => {
                                console::log!(format!(
                                    "{} converted to GLDF: {} bytes, product: {}, geometry: {}, photometry: {}",
                                    format_name,
                                    result.gldf_bytes.len(),
                                    result.product_name,
                                    result.has_geometry,
                                    result.has_photometry
                                ).as_str());

                                // Parse GLDF and load it
                                if let Ok(gldf) = WasmGldfProduct::load_gldf_from_buf_all(
                                    result.gldf_bytes.clone(),
                                ) {
                                    self.loaded_gldf = Some(gldf);
                                }

                                // Generate output filename
                                let output_name = file_name
                                    .replace(".uld", ".gldf")
                                    .replace(".ULD", ".gldf")
                                    .replace(".rolf", ".gldf")
                                    .replace(".ROLF", ".gldf");

                                // Store GLDF for viewing
                                self.files.push(FileDetails {
                                    data: result.gldf_bytes,
                                    file_type: "application/gldf".to_string(),
                                    name: output_name,
                                });
                                self.readers.remove(&file_name);
                                return true;
                            }
                            Err(e) => {
                                console::log!(
                                    format!("Failed to convert {}: {}", format_name, e).as_str()
                                );
                            }
                        }
                    }
                    #[cfg(not(feature = "light-convert"))]
                    {
                        console::log!(
                            "ULD/ROLF support not enabled. Build with 'light-convert' feature."
                        );
                    }
                }
                // Handle LDT/IES files - convert to minimal GLDF
                else if file_name_lower.ends_with(".ldt") || file_name_lower.ends_with(".ies") {
                    console::log!("Converting LDT/IES to GLDF...");
                    match ldt_to_gldf(&data, &file_name) {
                        Ok(gldf) => {
                            console::log!("LDT/IES converted to GLDF structure");
                            self.loaded_gldf = Some(gldf);
                            // Also store the original file for viewing
                            self.files.push(FileDetails {
                                data,
                                file_type: file_type.clone(),
                                name: file_name.clone(),
                            });
                            self.readers.remove(&file_name);
                            return true;
                        }
                        Err(e) => {
                            console::log!("LDT/IES conversion info:", e.as_str());
                            // Still show the file even if conversion fails
                        }
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
                console::log!("Msg::Files received:", files.len(), "file(s)");
                for file in files.into_iter() {
                    let file_name = file.name();
                    let file_type = file.raw_mime_type();
                    let file_size = file.size();
                    console::log!(
                        "Processing file:",
                        file_name.as_str(),
                        "type:",
                        file_type.as_str(),
                        "size:",
                        file_size
                    );

                    let task = {
                        let link = ctx.link().clone();
                        let file_name = file_name.clone();
                        let file_type = file_type.clone();

                        gloo::file::callbacks::read_as_bytes(&file, move |res| match res {
                            Ok(data) => {
                                console::log!(
                                    "File read success:",
                                    file_name.as_str(),
                                    "bytes:",
                                    data.len()
                                );
                                link.send_message(Msg::Loaded(file_name, file_type, data))
                            }
                            Err(e) => {
                                console::log!(
                                    "Failed to read file:",
                                    file_name.as_str(),
                                    format!("{:?}", e).as_str()
                                );
                            }
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
            Msg::ExportGldf => {
                if let Some(ref gldf) = self.loaded_gldf {
                    // Create GLDF zip file
                    use std::io::{Cursor, Write};
                    use zip::write::SimpleFileOptions;
                    use zip::ZipWriter;

                    let cursor = Cursor::new(Vec::new());
                    let mut zip = ZipWriter::new(cursor);
                    let options = SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated);

                    // Write product.xml
                    if let Ok(xml) = gldf.gldf.to_xml() {
                        if zip.start_file("product.xml", options).is_ok() {
                            let _ = zip.write_all(xml.as_bytes());
                        }
                    }

                    // Write embedded files
                    for buf_file in &gldf.files {
                        if let (Some(name), Some(content)) = (&buf_file.name, &buf_file.content) {
                            // Determine the zip path based on content type
                            let zip_path = name.clone();
                            if zip.start_file(&zip_path, options).is_ok() {
                                let _ = zip.write_all(content);
                            }
                        }
                    }

                    if let Ok(cursor) = zip.finish() {
                        let gldf_bytes = cursor.into_inner();
                        console::log!("Exporting GLDF:", gldf_bytes.len(), "bytes");

                        // Create blob and trigger download
                        let uint8arr = js_sys::Uint8Array::new(
                            &unsafe { js_sys::Uint8Array::view(&gldf_bytes) }.into(),
                        );
                        let array = js_sys::Array::new();
                        array.push(&uint8arr.buffer());
                        let opts = web_sys::BlobPropertyBag::new();
                        opts.set_type("application/zip");
                        if let Ok(blob) =
                            web_sys::Blob::new_with_u8_array_sequence_and_options(&array, &opts)
                        {
                            if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                                // Create download link and click it
                                let window = web_sys::window().unwrap();
                                let document = window.document().unwrap();
                                if let Ok(a) = document.create_element("a") {
                                    let _ = a.set_attribute("href", &url);
                                    let _ = a.set_attribute("download", "exported.gldf");
                                    let _ = a.set_attribute("style", "display: none");
                                    let _ = document.body().unwrap().append_child(&a);
                                    if let Some(html_a) = a.dyn_ref::<web_sys::HtmlElement>() {
                                        html_a.click();
                                    }
                                    let _ = document.body().unwrap().remove_child(&a);
                                    let _ = web_sys::Url::revoke_object_url(&url);
                                }
                            }
                        }
                    }
                }
                false
            }
            Msg::SetDragging(dragging) => {
                self.is_dragging = dragging;
                true
            }
            Msg::LoadDemo => {
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let result = gloo::net::http::Request::get("/slv_tria_2.gldf")
                        .send()
                        .await
                        .map_err(|e| format!("Network error: {}", e))
                        .and_then(|resp| {
                            if resp.ok() {
                                Ok(resp)
                            } else {
                                Err(format!("HTTP error: {}", resp.status()))
                            }
                        });

                    let data = match result {
                        Ok(resp) => resp
                            .binary()
                            .await
                            .map_err(|e| format!("Read error: {}", e)),
                        Err(e) => Err(e),
                    };

                    link.send_message(Msg::DemoLoaded(data));
                });
                false
            }
            Msg::DemoLoaded(result) => {
                match result {
                    Ok(data) => {
                        console::log!("Demo loaded:", data.len(), "bytes");
                        if let Ok(gldf) = WasmGldfProduct::load_gldf_from_buf_all(data.clone()) {
                            self.loaded_gldf = Some(gldf);
                        }
                        self.files.push(FileDetails {
                            data,
                            file_type: "application/gldf".to_string(),
                            name: "slv_tria_2.gldf".to_string(),
                        });
                    }
                    Err(e) => {
                        console::log!("Failed to load demo:", e.as_str());
                    }
                }
                true
            }
            Msg::Select3dVariant(variant_id) => {
                self.selected_3d_variant = variant_id;
                true
            }
            Msg::SelectFile(file_id) => {
                self.selected_file = file_id;
                true
            }
            // Mounting handlers
            Msg::ToggleCeilingMount(variant_id, enabled) => {
                if let Some(variant) = self.get_variant_mut(&variant_id) {
                    if enabled {
                        let mountings = variant.mountings.get_or_insert_with(Default::default);
                        mountings.ceiling =
                            Some(gldf_rs::gldf::product_definitions::Ceiling::default());
                    } else if let Some(ref mut mountings) = variant.mountings {
                        mountings.ceiling = None;
                    }
                }
                true
            }
            Msg::ToggleWallMount(variant_id, enabled) => {
                if let Some(variant) = self.get_variant_mut(&variant_id) {
                    if enabled {
                        let mountings = variant.mountings.get_or_insert_with(Default::default);
                        mountings.wall = Some(gldf_rs::gldf::product_definitions::Wall::default());
                    } else if let Some(ref mut mountings) = variant.mountings {
                        mountings.wall = None;
                    }
                }
                true
            }
            Msg::ToggleGroundMount(variant_id, enabled) => {
                if let Some(variant) = self.get_variant_mut(&variant_id) {
                    if enabled {
                        let mountings = variant.mountings.get_or_insert_with(Default::default);
                        mountings.ground =
                            Some(gldf_rs::gldf::product_definitions::Ground::default());
                    } else if let Some(ref mut mountings) = variant.mountings {
                        mountings.ground = None;
                    }
                }
                true
            }
            Msg::ToggleWorkingPlaneMount(variant_id, enabled) => {
                if let Some(variant) = self.get_variant_mut(&variant_id) {
                    if enabled {
                        let mountings = variant.mountings.get_or_insert_with(Default::default);
                        mountings.working_plane =
                            Some(gldf_rs::gldf::product_definitions::WorkingPlane::default());
                    } else if let Some(ref mut mountings) = variant.mountings {
                        mountings.working_plane = None;
                    }
                }
                true
            }
            Msg::SetCeilingRecessed(variant_id, enabled, depth) => {
                if let Some(variant) = self.get_variant_mut(&variant_id) {
                    let mountings = variant.mountings.get_or_insert_with(Default::default);
                    let ceiling = mountings.ceiling.get_or_insert_with(Default::default);
                    if enabled {
                        ceiling.recessed = Some(gldf_rs::gldf::product_definitions::Recessed {
                            recessed_depth: depth,
                            ..Default::default()
                        });
                    } else {
                        ceiling.recessed = None;
                    }
                }
                true
            }
            Msg::SetCeilingSurfaceMounted(variant_id, enabled) => {
                if let Some(variant) = self.get_variant_mut(&variant_id) {
                    let mountings = variant.mountings.get_or_insert_with(Default::default);
                    let ceiling = mountings.ceiling.get_or_insert_with(Default::default);
                    ceiling.surface_mounted = if enabled {
                        Some(gldf_rs::gldf::product_definitions::SurfaceMounted {})
                    } else {
                        None
                    };
                }
                true
            }
            Msg::SetCeilingPendant(variant_id, enabled, length) => {
                if let Some(variant) = self.get_variant_mut(&variant_id) {
                    let mountings = variant.mountings.get_or_insert_with(Default::default);
                    let ceiling = mountings.ceiling.get_or_insert_with(Default::default);
                    if enabled {
                        ceiling.pendant = Some(gldf_rs::gldf::product_definitions::Pendant {
                            pendant_length: length,
                        });
                    } else {
                        ceiling.pendant = None;
                    }
                }
                true
            }
            Msg::SetWallMountingHeight(variant_id, height) => {
                if let Some(variant) = self.get_variant_mut(&variant_id) {
                    let mountings = variant.mountings.get_or_insert_with(Default::default);
                    let wall = mountings.wall.get_or_insert_with(Default::default);
                    wall.mounting_height = height;
                }
                true
            }
            Msg::SetGroundPoleTop(variant_id, enabled, height) => {
                if let Some(variant) = self.get_variant_mut(&variant_id) {
                    let mountings = variant.mountings.get_or_insert_with(Default::default);
                    let ground = mountings.ground.get_or_insert_with(Default::default);
                    if enabled {
                        ground.pole_top = Some(gldf_rs::gldf::product_definitions::PoleTop {
                            pole_height: height,
                            pole_height_element: None,
                        });
                    } else {
                        ground.pole_top = None;
                    }
                }
                true
            }
            Msg::SetGroundPoleIntegrated(variant_id, enabled, height) => {
                if let Some(variant) = self.get_variant_mut(&variant_id) {
                    let mountings = variant.mountings.get_or_insert_with(Default::default);
                    let ground = mountings.ground.get_or_insert_with(Default::default);
                    if enabled {
                        ground.pole_integrated =
                            Some(gldf_rs::gldf::product_definitions::PoleIntegrated {
                                pole_height: height,
                                pole_height_element: None,
                            });
                    } else {
                        ground.pole_integrated = None;
                    }
                }
                true
            }
            Msg::ClearAll => {
                console::log!("Clearing all data...");
                self.loaded_gldf = None;
                self.files.clear();
                self.readers.clear();
                self.mode = AppMode::Viewer;
                self.nav_item = NavItem::Overview;
                self.selected_3d_variant = None;
                self.selected_file = None;
                self.show_help = false;
                true
            }
            Msg::ToggleHelp => {
                self.show_help = !self.show_help;
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

                // Help overlay
                if self.show_help {
                    { self.view_help_overlay(ctx) }
                }
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
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("application/vnd.openxmlformats-officedocument.wordprocessingml.document");
    let blob = Blob::new_with_str_sequence_and_options(&array, &opts).unwrap();
    web_sys::Url::create_object_url_with_blob(&blob).unwrap()
}

impl App {
    /// Helper to get mutable reference to a variant by ID
    fn get_variant_mut(
        &mut self,
        variant_id: &str,
    ) -> Option<&mut gldf_rs::gldf::product_definitions::Variant> {
        self.loaded_gldf.as_mut().and_then(|gldf| {
            gldf.gldf
                .product_definitions
                .variants
                .as_mut()
                .and_then(|variants| variants.variant.iter_mut().find(|v| v.id == variant_id))
        })
    }

    /// Render mountings editor for a variant
    fn render_mountings_editor(
        &self,
        ctx: &Context<Self>,
        variant: &gldf_rs::gldf::product_definitions::Variant,
    ) -> Html {
        let mountings = variant.mountings.as_ref();
        let variant_id = variant.id.clone();
        let has_ceiling = mountings.map(|m| m.ceiling.is_some()).unwrap_or(false);
        let has_wall = mountings.map(|m| m.wall.is_some()).unwrap_or(false);
        let has_ground = mountings.map(|m| m.ground.is_some()).unwrap_or(false);
        let has_working_plane = mountings
            .map(|m| m.working_plane.is_some())
            .unwrap_or(false);

        // Callbacks for toggling
        let vid = variant_id.clone();
        let on_toggle_ceiling = ctx.link().callback(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            Msg::ToggleCeilingMount(vid.clone(), input.checked())
        });
        let vid = variant_id.clone();
        let on_toggle_wall = ctx.link().callback(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            Msg::ToggleWallMount(vid.clone(), input.checked())
        });
        let vid = variant_id.clone();
        let on_toggle_ground = ctx.link().callback(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            Msg::ToggleGroundMount(vid.clone(), input.checked())
        });
        let vid = variant_id.clone();
        let on_toggle_wp = ctx.link().callback(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            Msg::ToggleWorkingPlaneMount(vid.clone(), input.checked())
        });

        // Build details strings
        let ceiling_details = mountings
            .and_then(|m| m.ceiling.as_ref())
            .map(|c| {
                let mut parts = Vec::new();
                if let Some(ref r) = c.recessed {
                    parts.push(format!("Recessed ({}mm)", r.recessed_depth));
                }
                if c.surface_mounted.is_some() {
                    parts.push("Surface mounted".to_string());
                }
                if let Some(ref p) = c.pendant {
                    parts.push(format!("Pendant ({:.0}mm)", p.pendant_length));
                }
                parts
            })
            .unwrap_or_default();

        let wall_details = mountings
            .and_then(|m| m.wall.as_ref())
            .map(|w| {
                let mut parts = Vec::new();
                if w.mounting_height > 0 {
                    parts.push(format!("Height: {}mm", w.mounting_height));
                }
                if w.recessed.is_some() {
                    parts.push("Recessed".to_string());
                }
                if w.surface_mounted.is_some() {
                    parts.push("Surface mounted".to_string());
                }
                parts
            })
            .unwrap_or_default();

        let ground_details = mountings
            .and_then(|m| m.ground.as_ref())
            .map(|g| {
                let mut parts = Vec::new();
                if let Some(ref pt) = g.pole_top {
                    parts.push(format!(
                        "Pole top{}",
                        pt.get_pole_height()
                            .map(|h| format!(" ({}mm)", h))
                            .unwrap_or_default()
                    ));
                }
                if let Some(ref pi) = g.pole_integrated {
                    parts.push(format!(
                        "Pole integrated{}",
                        pi.get_pole_height()
                            .map(|h| format!(" ({}mm)", h))
                            .unwrap_or_default()
                    ));
                }
                if g.free_standing.is_some() {
                    parts.push("Free standing".to_string());
                }
                parts
            })
            .unwrap_or_default();

        html! {
            <div class="mountings-section" style="margin-top: 12px; padding-top: 12px; border-top: 1px solid var(--border-color);">
                <div style="font-size: 12px; font-weight: 500; color: var(--text-secondary); margin-bottom: 8px;">
                    { "Mountings" }
                </div>
                <div style="display: flex; flex-wrap: wrap; gap: 12px;">
                    // Ceiling
                    <div class="mounting-edit-card" style="background: var(--bg-secondary); padding: 10px 14px; border-radius: 8px; border-left: 3px solid var(--accent-blue); min-width: 160px;">
                        <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; margin-bottom: 6px;">
                            <input type="checkbox" checked={has_ceiling} onchange={on_toggle_ceiling}
                                style="width: 16px; height: 16px; accent-color: var(--accent-blue);" />
                            <span style="font-size: 12px; font-weight: 600; color: var(--accent-blue);">{ "Ceiling" }</span>
                        </label>
                        if !ceiling_details.is_empty() {
                            <div style="font-size: 10px; color: var(--text-secondary); padding-left: 24px;">
                                { for ceiling_details.iter().map(|d| html! { <div>{ d }</div> }) }
                            </div>
                        }
                    </div>

                    // Wall
                    <div class="mounting-edit-card" style="background: var(--bg-secondary); padding: 10px 14px; border-radius: 8px; border-left: 3px solid var(--accent-green); min-width: 160px;">
                        <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; margin-bottom: 6px;">
                            <input type="checkbox" checked={has_wall} onchange={on_toggle_wall}
                                style="width: 16px; height: 16px; accent-color: var(--accent-green);" />
                            <span style="font-size: 12px; font-weight: 600; color: var(--accent-green);">{ "Wall" }</span>
                        </label>
                        if !wall_details.is_empty() {
                            <div style="font-size: 10px; color: var(--text-secondary); padding-left: 24px;">
                                { for wall_details.iter().map(|d| html! { <div>{ d }</div> }) }
                            </div>
                        }
                    </div>

                    // Ground
                    <div class="mounting-edit-card" style="background: var(--bg-secondary); padding: 10px 14px; border-radius: 8px; border-left: 3px solid var(--accent-orange); min-width: 160px;">
                        <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; margin-bottom: 6px;">
                            <input type="checkbox" checked={has_ground} onchange={on_toggle_ground}
                                style="width: 16px; height: 16px; accent-color: var(--accent-orange);" />
                            <span style="font-size: 12px; font-weight: 600; color: var(--accent-orange);">{ "Ground" }</span>
                        </label>
                        if !ground_details.is_empty() {
                            <div style="font-size: 10px; color: var(--text-secondary); padding-left: 24px;">
                                { for ground_details.iter().map(|d| html! { <div>{ d }</div> }) }
                            </div>
                        }
                    </div>

                    // Working Plane
                    <div class="mounting-edit-card" style="background: var(--bg-secondary); padding: 10px 14px; border-radius: 8px; border-left: 3px solid var(--accent-purple); min-width: 160px;">
                        <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; margin-bottom: 6px;">
                            <input type="checkbox" checked={has_working_plane} onchange={on_toggle_wp}
                                style="width: 16px; height: 16px; accent-color: var(--accent-purple);" />
                            <span style="font-size: 12px; font-weight: 600; color: var(--accent-purple);">{ "Working Plane" }</span>
                        </label>
                        if has_working_plane {
                            <div style="font-size: 10px; color: var(--text-secondary); padding-left: 24px;">
                                <div>{ "Free standing" }</div>
                            </div>
                        }
                    </div>
                </div>
            </div>
        }
    }

    /// Render mountings as read-only badges (for viewer mode)
    fn render_mountings_readonly(
        &self,
        variant: &gldf_rs::gldf::product_definitions::Variant,
    ) -> Html {
        let mountings = variant.mountings.as_ref();
        let has_ceiling = mountings.map(|m| m.ceiling.is_some()).unwrap_or(false);
        let has_wall = mountings.map(|m| m.wall.is_some()).unwrap_or(false);
        let has_ground = mountings.map(|m| m.ground.is_some()).unwrap_or(false);
        let has_working_plane = mountings
            .map(|m| m.working_plane.is_some())
            .unwrap_or(false);

        // Build details strings
        let ceiling_details = mountings
            .and_then(|m| m.ceiling.as_ref())
            .map(|c| {
                let mut parts = Vec::new();
                if let Some(ref r) = c.recessed {
                    parts.push(format!("Recessed ({}mm)", r.recessed_depth));
                }
                if c.surface_mounted.is_some() {
                    parts.push("Surface mounted".to_string());
                }
                if let Some(ref p) = c.pendant {
                    parts.push(format!("Pendant ({:.0}mm)", p.pendant_length));
                }
                parts
            })
            .unwrap_or_default();

        let wall_details = mountings
            .and_then(|m| m.wall.as_ref())
            .map(|w| {
                let mut parts = Vec::new();
                if w.mounting_height > 0 {
                    parts.push(format!("Height: {}mm", w.mounting_height));
                }
                if w.recessed.is_some() {
                    parts.push("Recessed".to_string());
                }
                if w.surface_mounted.is_some() {
                    parts.push("Surface mounted".to_string());
                }
                parts
            })
            .unwrap_or_default();

        let ground_details = mountings
            .and_then(|m| m.ground.as_ref())
            .map(|g| {
                let mut parts = Vec::new();
                if let Some(ref pt) = g.pole_top {
                    parts.push(format!(
                        "Pole top{}",
                        pt.get_pole_height()
                            .map(|h| format!(" ({}mm)", h))
                            .unwrap_or_default()
                    ));
                }
                if let Some(ref pi) = g.pole_integrated {
                    parts.push(format!(
                        "Pole integrated{}",
                        pi.get_pole_height()
                            .map(|h| format!(" ({}mm)", h))
                            .unwrap_or_default()
                    ));
                }
                if g.free_standing.is_some() {
                    parts.push("Free standing".to_string());
                }
                parts
            })
            .unwrap_or_default();

        // Only show if there are any mountings
        if !has_ceiling && !has_wall && !has_ground && !has_working_plane {
            return html! {};
        }

        html! {
            <div class="mountings-section" style="margin-top: 12px; padding-top: 12px; border-top: 1px solid var(--border-color);">
                <div style="font-size: 12px; font-weight: 500; color: var(--text-secondary); margin-bottom: 8px;">
                    { "Mountings" }
                </div>
                <div style="display: flex; flex-wrap: wrap; gap: 8px;">
                    if has_ceiling {
                        <div class="mounting-badge" style="display: inline-flex; align-items: center; gap: 6px; background: rgba(10, 132, 255, 0.15); color: var(--accent-blue); padding: 4px 10px; border-radius: 12px; font-size: 11px; font-weight: 500;">
                            <span>{ "Ceiling" }</span>
                            if !ceiling_details.is_empty() {
                                <span style="opacity: 0.7;">{ format!("({})", ceiling_details.join(", ")) }</span>
                            }
                        </div>
                    }
                    if has_wall {
                        <div class="mounting-badge" style="display: inline-flex; align-items: center; gap: 6px; background: rgba(48, 209, 88, 0.15); color: var(--accent-green); padding: 4px 10px; border-radius: 12px; font-size: 11px; font-weight: 500;">
                            <span>{ "Wall" }</span>
                            if !wall_details.is_empty() {
                                <span style="opacity: 0.7;">{ format!("({})", wall_details.join(", ")) }</span>
                            }
                        </div>
                    }
                    if has_ground {
                        <div class="mounting-badge" style="display: inline-flex; align-items: center; gap: 6px; background: rgba(255, 159, 10, 0.15); color: var(--accent-orange); padding: 4px 10px; border-radius: 12px; font-size: 11px; font-weight: 500;">
                            <span>{ "Ground" }</span>
                            if !ground_details.is_empty() {
                                <span style="opacity: 0.7;">{ format!("({})", ground_details.join(", ")) }</span>
                            }
                        </div>
                    }
                    if has_working_plane {
                        <div class="mounting-badge" style="display: inline-flex; align-items: center; gap: 6px; background: rgba(191, 90, 242, 0.15); color: var(--accent-purple); padding: 4px 10px; border-radius: 12px; font-size: 11px; font-weight: 500;">
                            <span>{ "Working Plane" }</span>
                        </div>
                    }
                </div>
            </div>
        }
    }

    /// Render help overlay with context-sensitive help
    fn view_help_overlay(&self, ctx: &Context<Self>) -> Html {
        let on_close = ctx.link().callback(|_| Msg::ToggleHelp);
        let on_overlay_click = on_close.clone();

        // Context-sensitive help based on current view
        let (section_title, section_help) = match self.nav_item {
            NavItem::Overview => (
                "Overview",
                vec![
                    (
                        "File Loading",
                        "Drop GLDF, LDT, IES, ULD, or ROLF files onto the window to load them.",
                    ),
                    (
                        "Product Info",
                        "Shows basic product information like manufacturer and product name.",
                    ),
                    (
                        "Quick Stats",
                        "Displays counts of light sources, variants, files, etc.",
                    ),
                ],
            ),
            NavItem::Header => (
                "Header Editor",
                vec![
                    ("Format Version", "GLDF format version (e.g., 1.0.0)."),
                    (
                        "Manufacturer",
                        "Company name of the luminaire manufacturer.",
                    ),
                    ("Author", "Person or system that created the file."),
                    ("License Key", "Optional license key for the product data."),
                ],
            ),
            NavItem::Electrical => (
                "Electrical Editor",
                vec![
                    (
                        "Safety Class",
                        "Electrical safety classification (I, II, or III).",
                    ),
                    ("IP Code", "Ingress Protection rating (e.g., IP65)."),
                    ("Power Factor", "Ratio of real to apparent power (0-1)."),
                    (
                        "CLO",
                        "Constant Light Output - maintains brightness over time.",
                    ),
                ],
            ),
            NavItem::Photometry => (
                "Photometry Editor",
                vec![
                    (
                        "GLDF Values",
                        "Values stored in the GLDF file (editable, shown in blue).",
                    ),
                    (
                        "Calculated Values",
                        "Values calculated from LDT/IES files (shown in orange).",
                    ),
                    (
                        "CIE Flux Code",
                        "5-digit code describing light distribution.",
                    ),
                    ("LOR/DLOR/ULOR", "Light Output Ratios - efficiency metrics."),
                ],
            ),
            NavItem::Variants => (
                "Variants",
                vec![
                    (
                        "Product Variants",
                        "Different configurations of the luminaire.",
                    ),
                    (
                        "Mountings",
                        "Installation options: Ceiling, Wall, Ground, Working Plane.",
                    ),
                    ("Geometry", "Reference to 3D model geometry."),
                    (
                        "3D View",
                        "Click 'View 3D Scene' to see the luminaire in 3D.",
                    ),
                ],
            ),
            NavItem::Files => (
                "Files",
                vec![
                    ("Embedded Files", "Files contained within the GLDF package."),
                    ("LDT/IES", "Photometry files with light distribution data."),
                    ("L3D", "3D geometry files for the luminaire model."),
                    ("Images", "Product photos and thumbnails."),
                ],
            ),
            NavItem::LightSources => (
                "Light Sources",
                vec![
                    (
                        "Fixed Sources",
                        "Light sources permanently installed in the luminaire.",
                    ),
                    ("Changeable Sources", "Replaceable light sources (lamps)."),
                    ("Luminous Flux", "Light output in lumens."),
                    (
                        "Color Temperature",
                        "CCT in Kelvin (e.g., 3000K warm, 4000K neutral).",
                    ),
                ],
            ),
            _ => (
                "General Help",
                vec![
                    (
                        "Navigation",
                        "Use the sidebar to navigate between sections.",
                    ),
                    ("Edit Mode", "Click 'Edit Mode' to enable editing features."),
                    (
                        "Export",
                        "Use export buttons to save as GLDF, JSON, or XML.",
                    ),
                    ("Clear", "Click Clear to reset and load a new file."),
                ],
            ),
        };

        html! {
            <div class="help-overlay" onclick={on_overlay_click}>
                <div class="help-modal" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="help-header">
                        <h2>{ "Help" }</h2>
                        <button class="help-close" onclick={on_close.clone()}>{ "✕" }</button>
                    </div>
                    <div class="help-content">
                        // Current section help
                        <div class="help-section">
                            <h3>{ section_title }</h3>
                            <dl class="help-list">
                                { for section_help.iter().map(|(term, desc)| html! {
                                    <>
                                        <dt>{ *term }</dt>
                                        <dd>{ *desc }</dd>
                                    </>
                                })}
                            </dl>
                        </div>

                        // General shortcuts
                        <div class="help-section">
                            <h3>{ "Supported File Formats" }</h3>
                            <ul class="help-formats">
                                <li><strong>{ ".gldf" }</strong>{ " - Global Lighting Data Format" }</li>
                                <li><strong>{ ".ldt" }</strong>{ " - EULUMDAT photometry" }</li>
                                <li><strong>{ ".ies" }</strong>{ " - IES photometry" }</li>
                            </ul>
                        </div>

                        <div class="help-section">
                            <h3>{ "Tips" }</h3>
                            <ul class="help-tips">
                                <li>{ "Drag & drop files anywhere on the page to load them" }</li>
                                <li>{ "Click on LDT diagrams to zoom in" }</li>
                                <li>{ "Use 'View 3D Scene' to see luminaire geometry" }</li>
                                <li>{ "Orange values in Photometry are calculated from LDT files" }</li>
                            </ul>
                        </div>
                    </div>
                    <div class="help-footer">
                        <span class="help-version">{ "GLDF Viewer v0.3" }</span>
                        <a href="https://github.com/holg/gldf-rs" target="_blank" class="help-link">{ "GitHub" }</a>
                    </div>
                </div>
            </div>
        }
    }

    fn view_sidebar(&self, ctx: &Context<Self>) -> Html {
        let has_file = self.loaded_gldf.is_some() || !self.files.is_empty();
        let files_count = self
            .loaded_gldf
            .as_ref()
            .map(|g| g.files.len())
            .unwrap_or(0);
        let light_sources_count = self
            .loaded_gldf
            .as_ref()
            .map(|g| {
                g.gldf
                    .general_definitions
                    .light_sources
                    .as_ref()
                    .map(|ls| ls.fixed_light_source.len() + ls.changeable_light_source.len())
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        let variants_count = self
            .loaded_gldf
            .as_ref()
            .map(|g| {
                g.gldf
                    .product_definitions
                    .variants
                    .as_ref()
                    .map(|v| v.variant.len())
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        let emitters_count = self
            .loaded_gldf
            .as_ref()
            .map(|g| {
                g.gldf
                    .general_definitions
                    .emitters
                    .as_ref()
                    .map(|e| e.emitter.len())
                    .unwrap_or(0)
            })
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
                        { self.nav_item(ctx, NavItem::Electrical, "⚡", "Electrical", None, has_file) }
                        { self.nav_item(ctx, NavItem::Applications, "🏢", "Applications", None, has_file) }
                        { self.nav_item(ctx, NavItem::Photometry, "🔬", "Photometry", None, has_file) }
                        { self.nav_item(ctx, NavItem::Statistics, "📈", "Statistics", None, has_file) }
                    </ul>
                </div>

                <div class="sidebar-section">
                    <div class="sidebar-section-title">{ "Definitions" }</div>
                    <ul class="sidebar-nav">
                        { self.nav_item(ctx, NavItem::Files, "📁", "Files", Some(files_count), has_file) }
                        { self.nav_item(ctx, NavItem::LightSources, "💡", "Light Sources", Some(light_sources_count), has_file) }
                        { self.nav_item(ctx, NavItem::Emitters, "🔆", "Emitters", Some(emitters_count), has_file) }
                        { self.nav_item(ctx, NavItem::Variants, "📦", "Variants", Some(variants_count), has_file) }
                    </ul>
                </div>

                // Links section at bottom
                <div style="margin-top: auto; padding: 16px;">
                    <div style="font-size: 11px; color: var(--text-tertiary); margin-bottom: 8px;">{ "Resources" }</div>
                    <a href="https://github.com/holg/gldf-rs" target="_blank" style="display: block; font-size: 12px; margin-bottom: 4px;">{ "gldf-rs (GitHub)" }</a>
                    <a href="https://gldf.io" target="_blank" style="display: block; font-size: 12px; margin-bottom: 4px;">{ "GLDF.io" }</a>
                    <a href="https://eulumdat.icu/" target="_blank" style="display: block; font-size: 12px; margin-bottom: 8px;">{ "QLumEdit" }</a>
                    <p class="privacy-note">{ "All processing is local" }</p>
                </div>
            </div>
        }
    }

    fn nav_item(
        &self,
        ctx: &Context<Self>,
        item: NavItem,
        icon: &str,
        label: &str,
        badge: Option<usize>,
        enabled: bool,
    ) -> Html {
        let is_active = self.nav_item == item;
        let onclick = ctx.link().callback(move |_| Msg::Navigate(item));
        let class = classes!(
            "sidebar-nav-item",
            is_active.then_some("active"),
            (!enabled).then_some("disabled")
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
        let ondragleave = ctx.link().callback(|_: DragEvent| Msg::SetDragging(false));

        let welcome_class = classes!("welcome-view", self.is_dragging.then_some("dragging"));

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
                    <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadDemo)}>
                        { "Load SLV Tria 2" }
                    </button>
                </div>

                <input
                    id="file-upload"
                    type="file"
                    accept=".gldf,.ldt,.ies,.uld,.rolf"
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
            NavItem::Electrical => "Electrical",
            NavItem::Applications => "Applications",
            NavItem::Photometry => "Photometry",
            NavItem::Statistics => "Statistics",
            NavItem::Files => "Files",
            NavItem::LightSources => "Light Sources",
            NavItem::Emitters => "Emitters",
            NavItem::Variants => "Variants",
        };

        html! {
            <>
                // Content Header with toolbar
                <div class="content-header">
                    <h1 class="content-title">{ title }</h1>

                    <div class="toolbar-actions">
                        if self.loaded_gldf.is_some() {
                            <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::ToggleEditor)}>
                                { if self.mode == AppMode::Viewer { "Edit Mode" } else { "View Mode" } }
                            </button>
                            <button class="btn btn-success" onclick={ctx.link().callback(|_| Msg::ExportGldf)}>
                                { "Export GLDF" }
                            </button>
                            <button class="btn btn-success" onclick={ctx.link().callback(|_| Msg::ExportJson)}>
                                { "Export JSON" }
                            </button>
                            <button class="btn btn-success" onclick={ctx.link().callback(|_| Msg::ExportXml)}>
                                { "Export XML" }
                            </button>
                        }
                        // Always show Clear and Help buttons
                        <button class="btn btn-outline" onclick={ctx.link().callback(|_| Msg::ClearAll)} title="Clear all and start over">
                            { "Clear" }
                        </button>
                        <button class="btn btn-outline" onclick={ctx.link().callback(|_| Msg::ToggleHelp)} title="Show help">
                            { "?" }
                        </button>
                    </div>
                </div>

                // Content Body
                <div class="content-body">
                    {
                        match self.nav_item {
                            NavItem::Overview => self.view_overview(),
                            NavItem::RawData => self.view_raw_data(),
                            NavItem::FileViewer => self.view_file_viewer(ctx),
                            NavItem::Header => self.view_header_editor(),
                            NavItem::Electrical => self.view_electrical_editor(),
                            NavItem::Applications => self.view_applications_editor(),
                            NavItem::Photometry => self.view_photometry_editor(),
                            NavItem::Statistics => self.view_statistics(),
                            NavItem::Files => self.view_files_list(ctx),
                            NavItem::LightSources => self.view_light_sources(),
                            NavItem::Emitters => self.view_emitters(),
                            NavItem::Variants => self.view_variants(ctx),
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
            let fixed_ls = gldf
                .gldf
                .general_definitions
                .light_sources
                .as_ref()
                .map(|ls| ls.fixed_light_source.len())
                .unwrap_or(0);
            let changeable_ls = gldf
                .gldf
                .general_definitions
                .light_sources
                .as_ref()
                .map(|ls| ls.changeable_light_source.len())
                .unwrap_or(0);
            let variants_count = gldf
                .gldf
                .product_definitions
                .variants
                .as_ref()
                .map(|v| v.variant.len())
                .unwrap_or(0);
            let photometries_count = gldf
                .gldf
                .general_definitions
                .photometries
                .as_ref()
                .map(|p| p.photometry.len())
                .unwrap_or(0);

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
            let photometric: Vec<_> = files
                .iter()
                .filter(|f| {
                    f.name
                        .as_ref()
                        .map(|n| n.ends_with(".ldt") || n.ends_with(".ies"))
                        .unwrap_or(false)
                })
                .collect();
            let images: Vec<_> = files
                .iter()
                .filter(|f| {
                    f.name
                        .as_ref()
                        .map(|n| n.ends_with(".jpg") || n.ends_with(".png"))
                        .unwrap_or(false)
                })
                .collect();
            let geometry: Vec<_> = files
                .iter()
                .filter(|f| {
                    f.name
                        .as_ref()
                        .map(|n| n.ends_with(".l3d"))
                        .unwrap_or(false)
                })
                .collect();

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
            let fixed: Vec<_> = ls
                .map(|l| l.fixed_light_source.iter().collect())
                .unwrap_or_default();
            let changeable: Vec<_> = ls
                .map(|l| l.changeable_light_source.iter().collect())
                .unwrap_or_default();
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
            let variants: Vec<_> = gldf
                .gldf
                .product_definitions
                .variants
                .as_ref()
                .map(|v| v.variant.iter().collect())
                .unwrap_or_default();

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
            console::log!(format!(
                "[FileViewer] File: {}, type: {}, data: {} bytes, is_l3d: {}",
                f.name,
                f.file_type,
                f.data.len(),
                is_l3d
            ));
        }

        html! {
            <div id="preview-area">
                { for self.files.iter().map(Self::view_file) }
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

    fn view_electrical_editor(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            if self.mode == AppMode::Editor {
                html! {
                    <GldfProviderWithData gldf={gldf.gldf.clone()}>
                        <components::ElectricalEditor />
                    </GldfProviderWithData>
                }
            } else {
                // Read-only view
                let electrical = gldf
                    .gldf
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|d| d.electrical.as_ref());

                html! {
                    <div class="card">
                        <div class="card-body">
                            if let Some(elec) = electrical {
                                <div class="properties-grid">
                                    if let Some(ref class) = elec.electrical_safety_class {
                                        <div class="property">
                                            <span class="property-label">{ "Safety Class" }</span>
                                            <span class="property-value">{ format!("Class {}", class) }</span>
                                        </div>
                                    }
                                    if let Some(ref ip) = elec.ingress_protection_ip_code {
                                        <div class="property">
                                            <span class="property-label">{ "IP Code" }</span>
                                            <span class="property-value">{ ip }</span>
                                        </div>
                                    }
                                    if let Some(pf) = elec.power_factor {
                                        <div class="property">
                                            <span class="property-label">{ "Power Factor" }</span>
                                            <span class="property-value">{ format!("{:.2}", pf) }</span>
                                        </div>
                                    }
                                    if let Some(clo) = elec.constant_light_output {
                                        <div class="property">
                                            <span class="property-label">{ "Constant Light Output" }</span>
                                            <span class="property-value">{ if clo { "Yes" } else { "No" } }</span>
                                        </div>
                                    }
                                    if let Some(ref dist) = elec.light_distribution {
                                        <div class="property">
                                            <span class="property-label">{ "Light Distribution" }</span>
                                            <span class="property-value">{ dist }</span>
                                        </div>
                                    }
                                </div>
                            } else {
                                <p class="empty-message">{ "No electrical data available" }</p>
                            }
                        </div>
                    </div>
                }
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="icon">{ "⚡" }</div>
                    <h3>{ "No Electrical Data" }</h3>
                    <p>{ "Load a GLDF file to view electrical attributes" }</p>
                </div>
            }
        }
    }

    fn view_applications_editor(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            if self.mode == AppMode::Editor {
                html! {
                    <GldfProviderWithData gldf={gldf.gldf.clone()}>
                        <components::ApplicationsEditor />
                    </GldfProviderWithData>
                }
            } else {
                // Read-only view
                let applications: Vec<String> = gldf
                    .gldf
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|d| d.marketing.as_ref())
                    .and_then(|m| m.applications.as_ref())
                    .map(|a| a.application.clone())
                    .unwrap_or_default();

                html! {
                    <div class="card">
                        <div class="card-body">
                            if !applications.is_empty() {
                                <div class="tags-container">
                                    { for applications.iter().map(|app| html! {
                                        <span class="tag application-tag">{ app }</span>
                                    })}
                                </div>
                            } else {
                                <p class="empty-message">{ "No applications defined" }</p>
                            }
                        </div>
                    </div>
                }
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="icon">{ "🏢" }</div>
                    <h3>{ "No Applications Data" }</h3>
                    <p>{ "Load a GLDF file to view application areas" }</p>
                </div>
            }
        }
    }

    fn view_photometry_editor(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            if self.mode == AppMode::Editor {
                // Build a map from GLDF file definition id -> filename
                // The photometry references file by id (e.g., "ldtnarrow")
                // The file definition maps id to filename (e.g., "ldc/narrow.ldt")
                let file_id_to_filename: std::collections::HashMap<String, String> = gldf
                    .gldf
                    .general_definitions
                    .files
                    .file
                    .iter()
                    .map(|f| (f.id.clone(), f.file_name.clone()))
                    .collect();

                console::log!(format!(
                    "File ID to filename map: {:?}",
                    file_id_to_filename
                ));

                // Extract photometry files as (file_definition_id, content) pairs
                let photometry_files: Vec<(String, Vec<u8>)> = gldf
                    .files
                    .iter()
                    .filter_map(|f| {
                        let content = f.content.clone()?;
                        let file_path = f.name.clone().or_else(|| f.path.clone())?;

                        // Check if this looks like LDT or IES content
                        let path_lower = file_path.to_lowercase();
                        let is_photometry =
                            path_lower.ends_with(".ldt") || path_lower.ends_with(".ies");

                        if is_photometry {
                            // Find the file definition ID that maps to this filename
                            let file_def_id = file_id_to_filename
                                .iter()
                                .find(|(_, filename)| {
                                    filename.to_lowercase() == path_lower
                                        || file_path
                                            .to_lowercase()
                                            .ends_with(&filename.to_lowercase())
                                })
                                .map(|(id, _)| id.clone());

                            if let Some(id) = file_def_id {
                                console::log!(format!(
                                    "Found photometry file: def_id={}, path={}, content_len={}",
                                    id,
                                    file_path,
                                    content.len()
                                ));
                                Some((id, content))
                            } else {
                                // Fallback: use the path as id
                                console::log!(format!(
                                    "No file def found for: path={}, using path as id",
                                    file_path
                                ));
                                Some((file_path, content))
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                console::log!(format!(
                    "Total photometry files found: {}",
                    photometry_files.len()
                ));

                html! {
                    <GldfProviderWithData gldf={gldf.gldf.clone()}>
                        <components::PhotometryEditor photometry_files={photometry_files} />
                    </GldfProviderWithData>
                }
            } else {
                // Read-only view
                let photometries = gldf
                    .gldf
                    .general_definitions
                    .photometries
                    .as_ref()
                    .map(|p| &p.photometry)
                    .cloned()
                    .unwrap_or_default();

                html! {
                    <div class="card">
                        <div class="card-body">
                            if photometries.is_empty() {
                                <p class="empty-message">{ "No photometry definitions" }</p>
                            } else {
                                { for photometries.iter().map(|phot| {
                                    let desc = phot.descriptive_photometry.as_ref();
                                    html! {
                                        <div class="photometry-card">
                                            <h3>{ &phot.id }</h3>
                                            if let Some(ref file_ref) = phot.photometry_file_reference {
                                                <p class="file-ref">{ format!("File: {}", file_ref.file_id) }</p>
                                            }
                                            if let Some(d) = desc {
                                                <div class="properties-grid">
                                                    if let Some(ref code) = d.cie_flux_code {
                                                        <div class="property">
                                                            <span class="property-label">{ "CIE Flux Code" }</span>
                                                            <span class="property-value">{ code }</span>
                                                        </div>
                                                    }
                                                    if let Some(lor) = d.light_output_ratio {
                                                        <div class="property">
                                                            <span class="property-label">{ "LOR" }</span>
                                                            <span class="property-value">{ format!("{:.1}%", lor * 100.0) }</span>
                                                        </div>
                                                    }
                                                    if let Some(eff) = d.luminous_efficacy {
                                                        <div class="property">
                                                            <span class="property-label">{ "Efficacy" }</span>
                                                            <span class="property-value">{ format!("{:.1} lm/W", eff) }</span>
                                                        </div>
                                                    }
                                                    if let Some(dlor) = d.downward_light_output_ratio {
                                                        <div class="property">
                                                            <span class="property-label">{ "DLOR" }</span>
                                                            <span class="property-value">{ format!("{:.1}%", dlor * 100.0) }</span>
                                                        </div>
                                                    }
                                                    if let Some(ulor) = d.upward_light_output_ratio {
                                                        <div class="property">
                                                            <span class="property-label">{ "ULOR" }</span>
                                                            <span class="property-value">{ format!("{:.1}%", ulor * 100.0) }</span>
                                                        </div>
                                                    }
                                                </div>
                                            }
                                        </div>
                                    }
                                })}
                            }
                        </div>
                    </div>
                }
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="icon">{ "🔬" }</div>
                    <h3>{ "No Photometry Data" }</h3>
                    <p>{ "Load a GLDF file to view photometric data" }</p>
                </div>
            }
        }
    }

    fn view_statistics(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            let files_count = gldf.files.len();
            let fixed_ls = gldf
                .gldf
                .general_definitions
                .light_sources
                .as_ref()
                .map(|ls| ls.fixed_light_source.len())
                .unwrap_or(0);
            let changeable_ls = gldf
                .gldf
                .general_definitions
                .light_sources
                .as_ref()
                .map(|ls| ls.changeable_light_source.len())
                .unwrap_or(0);
            let variants_count = gldf
                .gldf
                .product_definitions
                .variants
                .as_ref()
                .map(|v| v.variant.len())
                .unwrap_or(0);
            let photometries_count = gldf
                .gldf
                .general_definitions
                .photometries
                .as_ref()
                .map(|p| p.photometry.len())
                .unwrap_or(0);
            let geometries_count = gldf
                .gldf
                .general_definitions
                .geometries
                .as_ref()
                .map(|g| g.simple_geometry.len() + g.model_geometry.len())
                .unwrap_or(0);

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

    fn view_file_content(file_name: &str, content: Vec<u8>) -> Html {
        let fname_lower = file_name.to_lowercase();
        if fname_lower.ends_with(".l3d") {
            html! {
                <div class="l3d-viewer-container">
                    <L3dViewer l3d_data={content} width={700} height={500} />
                </div>
            }
        } else if fname_lower.ends_with(".ldt") || fname_lower.ends_with(".ies") {
            html! {
                <div class="ldt-viewer-container">
                    <LdtViewer ldt_data={content} width={500.0} height={500.0} />
                </div>
            }
        } else if fname_lower.ends_with(".jpg")
            || fname_lower.ends_with(".jpeg")
            || fname_lower.ends_with(".png")
        {
            let mime = if fname_lower.ends_with(".png") {
                "png"
            } else {
                "jpeg"
            };
            html! {
                <img
                    src={format!("data:image/{};base64,{}", mime, BASE64_STANDARD.encode(&content))}
                    style="max-width: 100%; max-height: 400px; border-radius: 4px;"
                />
            }
        } else {
            html! {
                <div class="text-preview" style="font-family: var(--font-mono); font-size: 12px; white-space: pre-wrap; max-height: 400px; overflow: auto;">
                    { String::from_utf8_lossy(&content[..content.len().min(10000)]).to_string() }
                </div>
            }
        }
    }

    fn view_files_list(&self, ctx: &Context<Self>) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            let files: &Vec<_> = &gldf.gldf.general_definitions.files.file;

            // Get selected file content for viewer
            let selected_content = self.selected_file.as_ref().and_then(|file_id| {
                // Find the file definition
                let file_def = files.iter().find(|f| &f.id == file_id)?;
                // Find the actual file content
                let content = gldf.files.iter().find(|bf| {
                    bf.name
                        .as_ref()
                        .map(|n| {
                            let stored = n.rsplit('/').next().unwrap_or(n);
                            stored.eq_ignore_ascii_case(&file_def.file_name)
                        })
                        .unwrap_or(false)
                })?;
                Some((file_def.clone(), content.content.clone()?))
            });

            let close_viewer = ctx.link().callback(|_| Msg::SelectFile(None));

            // Pre-compute viewer HTML if file selected
            let viewer_html = selected_content.as_ref().map(|(file_def, content)| {
                let content_viewer = Self::view_file_content(&file_def.file_name, content.clone());
                let file_name = file_def.file_name.clone();
                let content_len = content.len();
                (file_name, content_len, content_viewer)
            });

            // Helper to get content type class
            let content_type_class = |ct: &str| -> &'static str {
                if ct.starts_with("ldc/") {
                    "content-type-ldc"
                } else if ct.starts_with("geo/") {
                    "content-type-geo"
                } else if ct.starts_with("image/") {
                    "content-type-image"
                } else if ct.starts_with("sensor/") {
                    "content-type-sensor"
                } else if ct.starts_with("document/") {
                    "content-type-doc"
                } else {
                    "content-type-other"
                }
            };

            html! {
                <div class="files-container">
                    // Files table
                    <div class="card">
                        <table class="data-table files-table-clickable">
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
                                    let file_id = f.id.clone();
                                    let is_selected = self.selected_file.as_ref() == Some(&f.id);
                                    let on_click = ctx.link().callback(move |_| Msg::SelectFile(Some(file_id.clone())));

                                    // Check if file content exists
                                    let has_content = gldf.files.iter().any(|bf| {
                                        bf.name.as_ref().map(|n| {
                                            let stored = n.rsplit('/').next().unwrap_or(n);
                                            stored.eq_ignore_ascii_case(&f.file_name)
                                        }).unwrap_or(false)
                                    });

                                    let row_class = classes!(
                                        is_selected.then_some("selected"),
                                        has_content.then_some("clickable")
                                    );

                                    html! {
                                        <tr
                                            class={row_class}
                                            onclick={if has_content { Some(on_click) } else { None }}
                                            style={if has_content { "cursor: pointer;" } else { "" }}
                                        >
                                            <td class="file-id">{ &f.id }</td>
                                            <td class="file-name">{ &f.file_name }</td>
                                            <td>
                                                <span class={classes!("content-type-badge", content_type_class(&f.content_type))}>
                                                    { &f.content_type }
                                                </span>
                                            </td>
                                            <td class="file-type">{ &f.type_attr }</td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>
                        if self.selected_file.is_none() {
                            <p style="text-align: center; color: var(--text-tertiary); margin-top: 16px; font-size: 13px;">
                                { "Click on a file above to preview" }
                            </p>
                        }
                    </div>

                    // File viewer at bottom (when file selected)
                    if let Some((file_name, content_len, content_viewer)) = viewer_html {
                        <div class="file-viewer-panel" style="margin-top: 20px; background: var(--bg-card); border-radius: 8px; overflow: hidden;">
                            <div style="display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--border-color); background: var(--bg-sidebar);">
                                <div style="font-weight: 500;">
                                    { &file_name }
                                    <span style="color: var(--text-tertiary); margin-left: 8px; font-size: 12px;">
                                        { format!("({} bytes)", content_len) }
                                    </span>
                                </div>
                                <button
                                    onclick={close_viewer}
                                    style="background: none; border: none; cursor: pointer; font-size: 18px; color: var(--text-secondary);"
                                >
                                    { "✕" }
                                </button>
                            </div>
                            <div style="padding: 16px;">
                                { content_viewer }
                            </div>
                        </div>
                    }
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
            let fixed: Vec<_> = ls
                .map(|l| l.fixed_light_source.iter().collect())
                .unwrap_or_default();
            let changeable: Vec<_> = ls
                .map(|l| l.changeable_light_source.iter().collect())
                .unwrap_or_default();

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
                                            <div class="properties-grid">
                                                if let Some(ref mfr) = ls.manufacturer {
                                                    <div class="property">
                                                        <span class="property-label">{ "Manufacturer" }</span>
                                                        <span class="property-value">{ mfr }</span>
                                                    </div>
                                                }
                                                if let Some(ref gtin) = ls.gtin {
                                                    <div class="property">
                                                        <span class="property-label">{ "GTIN" }</span>
                                                        <span class="property-value">{ gtin }</span>
                                                    </div>
                                                }
                                                if let Some(power) = ls.rated_input_power {
                                                    <div class="property">
                                                        <span class="property-label">{ "Rated Power" }</span>
                                                        <span class="property-value">{ format!("{} W", power) }</span>
                                                    </div>
                                                }
                                                if let Some(ref voltage) = ls.rated_input_voltage {
                                                    <div class="property">
                                                        <span class="property-label">{ "Rated Voltage" }</span>
                                                        <span class="property-value">{
                                                            if let Some(fixed) = voltage.fixed_voltage {
                                                                format!("{} V ({:?})", fixed, voltage.type_attr)
                                                            } else if let Some(ref range) = voltage.voltage_range {
                                                                format!("{}-{} V ({:?})", range.min, range.max, voltage.type_attr)
                                                            } else {
                                                                format!("{:?}", voltage.type_attr)
                                                            }
                                                        }</span>
                                                    </div>
                                                }
                                                if let Some(ref color) = ls.color_information {
                                                    if let Some(cct) = color.correlated_color_temperature {
                                                        <div class="property">
                                                            <span class="property-label">{ "Color Temp" }</span>
                                                            <span class="property-value">{ format!("{} K", cct) }</span>
                                                        </div>
                                                    }
                                                    if let Some(cri) = color.color_rendering_index {
                                                        <div class="property">
                                                            <span class="property-label">{ "CRI" }</span>
                                                            <span class="property-value">{ format!("{}", cri) }</span>
                                                        </div>
                                                    }
                                                }
                                                if let Some(zhaga) = ls.zhaga_standard {
                                                    <div class="property">
                                                        <span class="property-label">{ "Zhaga" }</span>
                                                        <span class="property-value">{ if zhaga { "Yes" } else { "No" } }</span>
                                                    </div>
                                                }
                                            </div>
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
                                            <div class="properties-grid">
                                                if let Some(ref mfr) = ls.manufacturer {
                                                    <div class="property">
                                                        <span class="property-label">{ "Manufacturer" }</span>
                                                        <span class="property-value">{ mfr }</span>
                                                    </div>
                                                }
                                                if let Some(ref photo_ref) = ls.photometry_reference {
                                                    <div class="property">
                                                        <span class="property-label">{ "Photometry" }</span>
                                                        <span class="property-value">{ &photo_ref.photometry_id }</span>
                                                    </div>
                                                }
                                            </div>
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

    fn view_emitters(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            let emitters = gldf
                .gldf
                .general_definitions
                .emitters
                .as_ref()
                .map(|e| &e.emitter)
                .map(|e| e.iter().collect::<Vec<_>>())
                .unwrap_or_default();

            if emitters.is_empty() {
                return html! {
                    <div class="empty-state">
                        <div class="icon">{ "🔆" }</div>
                        <h3>{ "No Emitters" }</h3>
                        <p>{ "This GLDF file has no emitter definitions" }</p>
                    </div>
                };
            }

            // Helper to get photometry file content
            let get_photometry = |photo_id: &str| -> Option<Vec<u8>> {
                // Find photometry definition
                let photometries = gldf.gldf.general_definitions.photometries.as_ref()?;
                let photo = photometries.photometry.iter().find(|p| p.id == photo_id)?;

                // Get file reference (it's an Option, not Vec)
                let file_ref = photo.photometry_file_reference.as_ref()?;
                let file_id = &file_ref.file_id;

                // Find file definition
                let file_def = gldf
                    .gldf
                    .general_definitions
                    .files
                    .file
                    .iter()
                    .find(|f| &f.id == file_id)?;

                // Find content
                gldf.files
                    .iter()
                    .find(|bf| {
                        bf.name
                            .as_ref()
                            .map(|n| {
                                let stored = n.rsplit('/').next().unwrap_or(n);
                                stored.eq_ignore_ascii_case(&file_def.file_name)
                            })
                            .unwrap_or(false)
                    })
                    .and_then(|bf| bf.content.clone())
            };

            html! {
                <div class="emitters-container">
                    { for emitters.iter().map(|emitter| {
                        html! {
                            <div class="emitter-card" style="background: var(--bg-secondary); border-radius: 8px; padding: 16px; margin-bottom: 16px;">
                                <div class="emitter-header" style="display: flex; align-items: center; gap: 8px; margin-bottom: 12px;">
                                    <span style="font-size: 20px;">{ "🔆" }</span>
                                    <span style="font-family: var(--font-mono); font-weight: 600; font-size: 14px;">{ &emitter.id }</span>
                                </div>

                                // Fixed Light Emitters
                                if !emitter.fixed_light_emitter.is_empty() {
                                    <div class="fixed-emitters" style="margin-bottom: 16px;">
                                        <h4 style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">{ "Fixed Light Emitters" }</h4>
                                        { for emitter.fixed_light_emitter.iter().map(|fle| {
                                            let photo_id = &fle.photometry_reference.photometry_id;
                                            let ldt_data = get_photometry(photo_id);

                                            html! {
                                                <div style="background: var(--bg-primary); padding: 12px; border-radius: 6px; margin-bottom: 8px;">
                                                    <div style="display: flex; flex-wrap: wrap; gap: 16px; font-size: 12px;">
                                                        // Emergency behavior
                                                        if let Some(ref eb) = fle.emergency_behaviour {
                                                            <div>
                                                                <span style="color: var(--text-tertiary);">{ "Emergency: " }</span>
                                                                <span style={format!("background: {}; color: white; padding: 2px 6px; border-radius: 3px; font-size: 10px;",
                                                                    if eb == "EmergencyOnly" { "var(--accent-orange)" } else { "var(--accent-green)" }
                                                                )}>{ eb }</span>
                                                            </div>
                                                        }
                                                        // Light source reference
                                                        if let Some(ref ls_id) = fle.light_source_reference.fixed_light_source_id {
                                                            <div>
                                                                <span style="color: var(--text-tertiary);">{ "Light Source: " }</span>
                                                                <span style="color: var(--accent-blue);">{ ls_id }</span>
                                                            </div>
                                                        }
                                                        // Photometry reference
                                                        <div>
                                                            <span style="color: var(--text-tertiary);">{ "Photometry: " }</span>
                                                            <span style="color: var(--accent-purple);">{ &fle.photometry_reference.photometry_id }</span>
                                                        </div>
                                                    </div>

                                                    // Show LDT/IES viewer if photometry data available
                                                    if let Some(data) = ldt_data {
                                                        <div style="margin-top: 12px; border-top: 1px solid var(--border-color); padding-top: 12px;">
                                                            <div style="font-size: 11px; color: var(--text-secondary); margin-bottom: 8px;">{ "Photometric Distribution" }</div>
                                                            <LdtViewer ldt_data={data} width={400.0} height={300.0} />
                                                        </div>
                                                    }
                                                </div>
                                            }
                                        })}
                                    </div>
                                }

                                // Changeable Light Emitters
                                if !emitter.changeable_light_emitter.is_empty() {
                                    <div class="changeable-emitters">
                                        <h4 style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">{ "Changeable Light Emitters" }</h4>
                                        { for emitter.changeable_light_emitter.iter().map(|cle| {
                                            let photo_id = &cle.photometry_reference.photometry_id;
                                            html! {
                                                <div style="background: var(--bg-primary); padding: 12px; border-radius: 6px; margin-bottom: 8px;">
                                                    <div style="display: flex; flex-wrap: wrap; gap: 16px; font-size: 12px;">
                                                        // Emergency behavior
                                                        if let Some(ref eb) = cle.emergency_behaviour {
                                                            <div>
                                                                <span style="color: var(--text-tertiary);">{ "Emergency: " }</span>
                                                                <span style={format!("background: {}; color: white; padding: 2px 6px; border-radius: 3px; font-size: 10px;",
                                                                    if eb == "EmergencyOnly" { "var(--accent-orange)" } else { "var(--accent-green)" }
                                                                )}>{ eb }</span>
                                                            </div>
                                                        }
                                                        // Photometry reference
                                                        <div>
                                                            <span style="color: var(--text-tertiary);">{ "Photometry: " }</span>
                                                            <span style="color: var(--accent-purple);">{ photo_id }</span>
                                                        </div>
                                                    </div>
                                                </div>
                                            }
                                        })}
                                    </div>
                                }

                                // Sensors
                                if !emitter.sensor.is_empty() {
                                    <div class="sensors" style="margin-top: 12px;">
                                        <h4 style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">{ "Sensors" }</h4>
                                        <div style="display: flex; flex-wrap: wrap; gap: 8px;">
                                            { for emitter.sensor.iter().map(|s| {
                                                html! {
                                                    <span style="background: var(--bg-primary); padding: 4px 8px; border-radius: 4px; font-size: 11px;">
                                                        { &s.id }
                                                    </span>
                                                }
                                            })}
                                        </div>
                                    </div>
                                }
                            </div>
                        }
                    })}
                </div>
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="icon">{ "🔆" }</div>
                    <h3>{ "No Emitters" }</h3>
                    <p>{ "Load a GLDF file to view emitters" }</p>
                </div>
            }
        }
    }

    fn get_variant_l3d_ldt(&self, variant_id: &str) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        let gldf = match &self.loaded_gldf {
            Some(g) => g,
            None => return (None, None),
        };

        // Use the mapping helper
        let mappings = gldf_rs::get_l3d_ldt_mappings(&gldf.gldf);

        // Find mapping for this variant
        let mapping = match mappings.iter().find(|m| m.variant_id == variant_id) {
            Some(m) => m,
            None => return (None, None),
        };

        // Find L3D content
        let l3d_content = gldf
            .files
            .iter()
            .find(|f| {
                if let Some(ref name) = f.name {
                    let stored_name = name.rsplit('/').next().unwrap_or(name);
                    mapping
                        .l3d_file_name
                        .as_ref()
                        .map(|n| stored_name.eq_ignore_ascii_case(n))
                        .unwrap_or(false)
                } else {
                    false
                }
            })
            .and_then(|f| f.content.clone());

        // Find LDT content
        let ldt_content = mapping.ldt_file_name.as_ref().and_then(|ldt_name| {
            gldf.files
                .iter()
                .find(|f| {
                    if let Some(ref name) = f.name {
                        let stored_name = name.rsplit('/').next().unwrap_or(name);
                        stored_name.eq_ignore_ascii_case(ldt_name)
                    } else {
                        false
                    }
                })
                .and_then(|f| f.content.clone())
        });

        (l3d_content, ldt_content)
    }

    fn view_variants(&self, ctx: &Context<Self>) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            let variants: Vec<_> = gldf
                .gldf
                .product_definitions
                .variants
                .as_ref()
                .map(|v| v.variant.iter().collect())
                .unwrap_or_default();

            if variants.is_empty() {
                return html! {
                    <div class="empty-state">
                        <div class="icon">{ "📦" }</div>
                        <h3>{ "No Variants" }</h3>
                        <p>{ "This GLDF file has no variant definitions" }</p>
                    </div>
                };
            }

            // Get selected variant's data for 3D viewer
            let viewer_data = self.selected_3d_variant.as_ref().and_then(|variant_id| {
                let (l3d_data, ldt_data) = self.get_variant_l3d_ldt(variant_id);
                l3d_data.map(|l3d| {
                    let emitter_data = gldf_rs::get_variant_emitter_data(&gldf.gldf, variant_id);
                    let emitter_config: Vec<EmitterConfig> = emitter_data
                        .emitters
                        .iter()
                        .map(|em| EmitterConfig {
                            leo_name: em.leo_name.clone(),
                            luminous_flux: em.luminous_flux,
                            color_temperature: em.color_temperature,
                            emergency_behavior: em.emergency_behavior.clone(),
                        })
                        .collect();
                    (l3d, ldt_data, emitter_config, variant_id.clone())
                })
            });

            // Close button callback
            let close_viewer = ctx.link().callback(|_| Msg::Select3dVariant(None));

            html! {
                <div class="variants-container">
                    // 3D Scene Viewer (when variant selected)
                    if let Some((l3d, ldt_data, emitter_config, variant_id)) = viewer_data {
                        <div class="variant-3d-viewer" style="margin-bottom: 20px; background: var(--bg-secondary); border-radius: 8px; overflow: hidden;">
                            <div style="display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--border-color);">
                                <div style="font-weight: 500;">
                                    { "3D Scene: " }
                                    <span style="color: var(--accent-blue);">{ &variant_id }</span>
                                </div>
                                <button
                                    onclick={close_viewer}
                                    style="background: none; border: none; cursor: pointer; font-size: 18px; color: var(--text-secondary);"
                                >
                                    { "✕" }
                                </button>
                            </div>
                            <div style="padding: 0;">
                                <BevySceneViewer
                                    l3d_data={l3d}
                                    ldt_data={ldt_data}
                                    emitter_config={emitter_config}
                                    variant_id={variant_id}
                                    width={800}
                                    height={500}
                                />
                            </div>
                        </div>
                    }

                    // Variant cards
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

                            // Get per-emitter render data for this variant
                            let emitter_data = self.loaded_gldf.as_ref()
                                .map(|g| gldf_rs::get_variant_emitter_data(&g.gldf, &v.id))
                                .unwrap_or_default();

                            // Check if L3D data is available for this variant
                            let has_l3d = self.get_variant_l3d_ldt(&v.id).0.is_some();

                            // Check if this variant is selected
                            let is_selected = self.selected_3d_variant.as_ref() == Some(&v.id);

                            // Button callback
                            let variant_id = v.id.clone();
                            let on_view_3d = ctx.link().callback(move |_| Msg::Select3dVariant(Some(variant_id.clone())));

                            html! {
                                <div class={classes!("variant-card", "variant-card-expanded", is_selected.then_some("selected"))}>
                                    <div class="card-header-row">
                                        <span class="card-id">{ &v.id }</span>
                                        if let Some(order) = v.sort_order.filter(|&o| o > 0) {
                                            <span style="font-size: 11px; color: var(--text-tertiary);">{ format!("#{}", order) }</span>
                                        }
                                    </div>
                                    <div class="card-content">
                                        // 3D Scene button - FIRST element
                                        if has_l3d {
                                            <div style="margin-bottom: 12px;">
                                                <button
                                                    onclick={on_view_3d}
                                                    class="btn-3d-scene"
                                                    style={format!(
                                                        "background: {}; color: white; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer; font-size: 12px; display: flex; align-items: center; gap: 6px;",
                                                        if is_selected { "var(--accent-green)" } else { "var(--accent-blue)" }
                                                    )}
                                                >
                                                    <span>{ "🏠" }</span>
                                                    { if is_selected { "Viewing 3D Scene" } else { "View 3D Scene" } }
                                                </button>
                                            </div>
                                        }

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

                                        // Show emitter references with detailed data
                                        if !emitter_data.emitters.is_empty() {
                                            <div class="emitter-references" style="margin-top: 12px; padding-top: 12px; border-top: 1px solid var(--border-color);">
                                                <div style="font-size: 12px; font-weight: 500; color: var(--text-secondary); margin-bottom: 8px;">
                                                    { "Light Emitters" }
                                                </div>
                                                <div style="display: flex; flex-direction: column; gap: 8px;">
                                                    { for emitter_data.emitters.iter().map(|em| {
                                                        let flux_text = em.luminous_flux.map(|f| format!("{} lm", f)).unwrap_or_else(|| "-".to_string());
                                                        let temp_text = em.color_temperature.map(|t| format!("{} K", t)).unwrap_or_else(|| "-".to_string());
                                                        let emergency_badge = em.emergency_behavior.as_ref().map(|eb| {
                                                            let color = if eb == "EmergencyOnly" { "var(--accent-orange)" } else { "var(--accent-green)" };
                                                            html! { <span style={format!("background: {}; color: white; padding: 1px 4px; border-radius: 3px; font-size: 9px; margin-left: 4px;", color)}>{ eb }</span> }
                                                        });
                                                        html! {
                                                            <div style="background: var(--bg-secondary); padding: 8px 10px; border-radius: 6px;">
                                                                <div style="display: flex; align-items: center; gap: 8px; font-size: 11px; margin-bottom: 4px;">
                                                                    <span style="font-family: var(--font-mono); font-weight: 500;">
                                                                        { &em.leo_name }
                                                                    </span>
                                                                    <span style="color: var(--text-tertiary);">{ "→" }</span>
                                                                    <span style="color: var(--accent-blue);">{ &em.emitter_id }</span>
                                                                    { emergency_badge }
                                                                </div>
                                                                <div style="display: flex; gap: 16px; font-size: 10px; color: var(--text-secondary);">
                                                                    <span style="display: flex; align-items: center; gap: 4px;">
                                                                        <span style="color: var(--accent-yellow);">{ "☀" }</span>
                                                                        { flux_text }
                                                                    </span>
                                                                    <span style="display: flex; align-items: center; gap: 4px;">
                                                                        <span style="color: var(--accent-orange);">{ "🌡" }</span>
                                                                        { temp_text }
                                                                    </span>
                                                                </div>
                                                            </div>
                                                        }
                                                    })}
                                                </div>
                                            </div>
                                        }

                                        // Mountings section - edit controls only in Editor mode
                                        if self.mode == AppMode::Editor {
                                            { self.render_mountings_editor(ctx, v) }
                                        } else {
                                            { self.render_mountings_readonly(v) }
                                        }
                                    </div>
                                </div>
                            }
                        })}
                    </div>
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
            console::log!(format!(
                "[GLDF->L3D] Rendering L3D: {}, content_len: {}",
                l3d_name,
                l3d_content.len()
            ));
            if l3d_content.len() > 20 {
                console::log!(format!(
                    "[GLDF->L3D] First 20 bytes: {:?}",
                    &l3d_content[..20]
                ));
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
                        if !buf_file.content.clone().unwrap_or_default().is_empty() {
                            <img src={format!("data:image/jpeg;base64,{}", BASE64_STANDARD.encode(buf_file.clone().content.unwrap_or_default()))} />
                        } else {
                            <p style="color: var(--text-tertiary);">{ "JPG image (external URL)" }</p>
                        }
                    }
                    else if buf_file.name.clone().unwrap_or_default().to_lowercase().ends_with(".png") {
                        if !buf_file.content.clone().unwrap_or_default().is_empty() {
                            <img src={format!("data:image/png;base64,{}", BASE64_STANDARD.encode(buf_file.clone().content.unwrap_or_default()))} />
                        } else {
                            <p style="color: var(--text-tertiary);">{ "PNG image (external URL)" }</p>
                        }
                    }
                    else if buf_file.name.clone().unwrap_or_default().to_lowercase().ends_with(".ldt") {
                        if !buf_file.content.clone().unwrap_or_default().is_empty() {
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
                                <textarea readonly=true value={format!(r"{}", String::from_utf8_lossy(content.as_slice()))} />
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
            let url_files: Vec<_> = files.iter().filter(|f| f.type_attr == "url").collect();
            if url_files.is_empty() {
                return html! {};
            }
            html! {
                <div class="url-files">
                    <p style="font-size: 13px; font-weight: 500; color: var(--text-secondary); margin-bottom: 8px;">{ "External URL Resources:" }</p>
                    { for url_files.iter().map(|f| {
                        html! {
                            <div class="url-file-entry">
                                <div class="url-file-header">
                                    <span class="file-id">{ &f.id }</span>
                                    <span class="content-type">{ &f.content_type }</span>
                                </div>
                                <UrlFileViewer
                                    url={f.file_name.clone()}
                                    content_type={f.content_type.clone()}
                                    file_id={f.id.clone()}
                                />
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
            let file_type_label = if file_name_lower.ends_with(".ies") {
                "IES"
            } else {
                "LDT/Eulumdat"
            };
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
                        <textarea readonly=true value={format!(r#"{{"product": {}}}"#, loaded.gldf.to_pretty_json().expect("REASON").as_str())} />
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
