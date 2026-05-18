#![recursion_limit = "256"]
//! GLDF WASM Editor Application

extern crate base64;
extern crate gldf_rs;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use gldf_rs::convert::ldt_to_gldf;
use gldf_rs::gldf::GldfProduct;
use gldf_rs::ifc::{GldfToIfc, IfcImporter};
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
mod i18n;
mod state;
mod utils;
// XSD-enum validator moved to gldf-rs-lib so FFI/Python/server consumers
// can use it without dragging in the WASM viewer. Reach it as
// `gldf_rs::validation::{validate, Violation}` from this crate.

use components::{
    BevySceneViewer, EditorTabs, EmitterConfig, IfcViewer, L3dViewer, LanguageBanner, LdtViewer,
    MountingConfig, UrlFileViewer,
};
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
    Plugins,
    IfcViewer,
    Header,
    Electrical,
    Mechanical,
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
    ExportIfc,
    SetDragging(bool),
    LoadDemo(String),
    DemoLoaded(Result<Vec<u8>, String>, String),
    LoadFromUrl(String),
    UrlLoaded(Result<Vec<u8>, String>, String),
    Select3dVariant(Option<String>),
    SelectFile(Option<String>),
    /// Download a single file from the loaded GLDF by its `<File>` element id.
    DownloadFile(String),
    /// Download a per-variant photometry file in the chosen format. The
    /// LDT/IES bytes are patched to carry the variant-resolved
    /// `RatedLuminousFlux` and `RatedInputPower` so downstream calc tools
    /// see the right lm/W. The variant id is the disambiguator (one
    /// photometry can be referenced by multiple variants).
    DownloadVariantPhotometry {
        variant_id: String,
        photometry_id: String,
        format: PhotometryExportFormat,
    },
    /// Cross-page jump: navigate to the Variants sidebar item, expand the
    /// given variant, and scroll its card into view. Used by aggregation
    /// chips on Electrical / Mechanical / Marketing pages so the user can
    /// click a SKU and land on its detail card without manually navigating.
    FocusVariant(String),
    /// Switch the viewer's UI translation language. Persists to
    /// localStorage; triggers re-render of every XSD-enum label.
    SetUiLanguage(String),
    /// Dismiss the XSD-violations banner for the current file. The
    /// banner reappears on the next load if the new file has any.
    DismissXsdViolations,
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
    /// Viewer UI translation language. Drives XSD-enum label rendering
    /// (LightDistribution etc.). Independent of any document locale.
    ui_language: String,
    show_help: bool,
    #[allow(dead_code)]
    pdf_enabled: bool,
    loading_url: Option<String>,
    /// Closed-XSD-enum violations found in the most recently loaded
    /// file. Populated by `gldf_rs::validation::validate()` after every
    /// successful load; cleared when the user dismisses the banner or
    /// loads a new file. Empty for clean files.
    xsd_violations: Vec<gldf_rs::validation::Violation>,
    /// Whether the user has dismissed the violations banner for the
    /// current file. Resets on each new load.
    xsd_violations_dismissed: bool,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        // Parse URL query parameters
        let (url_param, pdf_enabled) = Self::parse_url_params();

        // If URL parameter is present, trigger loading
        if let Some(ref url) = url_param {
            let url_clone = url.clone();
            let link = ctx.link().clone();
            // Use spawn_local to send the message after create completes
            wasm_bindgen_futures::spawn_local(async move {
                link.send_message(Msg::LoadFromUrl(url_clone));
            });
        }

        Self {
            readers: HashMap::default(),
            files: Vec::default(),
            mode: AppMode::Viewer,
            loaded_gldf: None,
            nav_item: NavItem::Overview,
            is_dragging: false,
            selected_3d_variant: None,
            selected_file: None,
            ui_language: load_ui_language_from_storage(),
            show_help: false,
            pdf_enabled,
            loading_url: url_param,
            xsd_violations: Vec::new(),
            xsd_violations_dismissed: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Loaded(file_name, file_type, data) => {
                console::log!("Got Files:", file_type.as_str());

                let file_name_lower = file_name.to_lowercase();

                // Try to parse GLDF
                if file_name_lower.ends_with(".gldf") {
                    // Clear stale Bevy viewer data before loading new GLDF
                    crate::components::clear_l3d_data();
                    if let Ok(gldf) = WasmGldfProduct::load_gldf_from_buf_all(data.clone()) {
                        self.loaded_gldf = Some(gldf);
                        self.refresh_xsd_violations();
                    }
                }
                // Handle IFC files - convert luminaire data to GLDF
                else if file_name_lower.ends_with(".ifc") {
                    console::log!("Converting IFC luminaire to GLDF...");
                    match String::from_utf8(data.clone()) {
                        Ok(ifc_content) => {
                            // Check if this IFC contains luminaire types
                            if !ifc_content.to_uppercase().contains("IFCLIGHTFIXTURETYPE") {
                                console::log!("IFC file does not contain IFCLIGHTFIXTURETYPE - not a luminaire IFC");
                                console::log!("This tool only supports IFC files exported from GLDF or containing luminaire definitions");
                            } else {
                                match gldf_rs::ifc_to_gldf(&ifc_content) {
                                    Ok(gldf_bytes) => {
                                        console::log!(
                                            "IFC converted to GLDF successfully, size:",
                                            gldf_bytes.len()
                                        );
                                        match WasmGldfProduct::load_gldf_from_buf_all(gldf_bytes) {
                                            Ok(gldf) => {
                                                console::log!(
                                                    "GLDF loaded successfully from IFC conversion"
                                                );
                                                console::log!(
                                                    "Files in converted GLDF:",
                                                    gldf.files.len()
                                                );

                                                // Clear stale Bevy viewer data before loading new GLDF
                                                crate::components::clear_l3d_data();

                                                // Clear existing files and add files from converted GLDF
                                                self.files.clear();
                                                for buf_file in &gldf.files {
                                                    if let (Some(name), Some(content)) =
                                                        (&buf_file.name, &buf_file.content)
                                                    {
                                                        // Determine file type from extension/name
                                                        let file_type = if name.ends_with(".xml") {
                                                            "application/xml"
                                                        } else if name.ends_with(".ldt") {
                                                            "ldc/eulumdat"
                                                        } else if name.ends_with(".l3d") {
                                                            "geo/l3d"
                                                        } else if name.ends_with(".png") {
                                                            "image/png"
                                                        } else if name.ends_with(".jpg")
                                                            || name.ends_with(".jpeg")
                                                        {
                                                            "image/jpeg"
                                                        } else {
                                                            "application/octet-stream"
                                                        };
                                                        self.files.push(FileDetails {
                                                            data: content.clone(),
                                                            file_type: file_type.to_string(),
                                                            name: name.clone(),
                                                        });
                                                        console::log!(
                                                            "  Added file:",
                                                            name.as_str()
                                                        );
                                                    }
                                                }

                                                self.loaded_gldf = Some(gldf);
                                                self.refresh_xsd_violations();
                                                self.readers.remove(&file_name);
                                                return true;
                                            }
                                            Err(e) => {
                                                console::log!(
                                                    "Failed to load converted GLDF:",
                                                    format!("{}", e).as_str()
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        console::log!(
                                            "IFC conversion failed:",
                                            format!("{}", e).as_str()
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            console::log!("IFC file encoding error:", format!("{}", e).as_str());
                        }
                    }
                    // Don't add IFC to files list - either it converted or we show an error
                    self.readers.remove(&file_name);
                    return true;
                }
                // Handle ULD (DIALux) files - convert to GLDF
                #[cfg(feature = "uld")]
                if file_name_lower.ends_with(".uld") {
                    console::log!("Converting ULD to GLDF...");
                    crate::components::clear_l3d_data();
                    match uld_rs::uld_to_gldf_named(&data, Some(&file_name)) {
                        Ok(gldf_bytes) => {
                            console::log!("ULD converted to GLDF");
                            match gldf_rs::GldfProduct::load_gldf_from_buf_all(gldf_bytes) {
                                Ok(gldf) => {
                                    self.loaded_gldf = Some(gldf);
                                    self.refresh_xsd_violations();
                                    self.files.push(FileDetails {
                                        data,
                                        file_type: file_type.clone(),
                                        name: file_name.clone(),
                                    });
                                }
                                Err(e) => {
                                    console::log!(&format!(
                                        "Failed to parse converted GLDF: {}",
                                        e
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            console::log!(&format!("ULD conversion failed: {}", e));
                        }
                    }
                    self.readers.remove(&file_name);
                    return true;
                }
                #[cfg(not(feature = "uld"))]
                if file_name_lower.ends_with(".uld") {
                    console::log!("ULD file conversion is not available in this build. Enable the 'uld' feature.");
                    self.readers.remove(&file_name);
                    return true;
                }
                // Handle ROLF (Relux) files - not yet supported
                if file_name_lower.ends_with(".rolf") {
                    console::log!("ROLF file conversion is not yet supported.");
                    self.readers.remove(&file_name);
                    return true;
                }
                // Handle LDT/IES files - convert to minimal GLDF
                if file_name_lower.ends_with(".ldt") || file_name_lower.ends_with(".ies") {
                    console::log!("Converting LDT/IES to GLDF...");
                    // Clear stale Bevy viewer data before loading new GLDF
                    crate::components::clear_l3d_data();
                    match ldt_to_gldf(&data, &file_name) {
                        Ok(gldf) => {
                            console::log!("LDT/IES converted to GLDF structure");
                            self.loaded_gldf = Some(gldf);
                            self.refresh_xsd_violations();
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
                        trigger_text_download(&json, "product.json", "application/json");
                    }
                }
                false
            }
            Msg::ExportXml => {
                if let Some(ref gldf) = self.loaded_gldf {
                    if let Ok(xml) = gldf.gldf.to_xml() {
                        trigger_text_download(&xml, "product.xml", "application/xml");
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
            Msg::ExportIfc => {
                if let Some(ref gldf) = self.loaded_gldf {
                    // Extract L3D mesh data for geometry export
                    let mesh_data = GldfToIfc::extract_mesh_from_files(&gldf.gldf, &gldf.files);
                    if let Some(ref m) = mesh_data {
                        console::log!(
                            "Mesh extracted:",
                            m.vertices.len(),
                            "vertices,",
                            m.triangles.len(),
                            "triangles"
                        );
                    } else {
                        console::log!("No mesh extracted from L3D files");
                    }
                    match GldfToIfc::export_with_mesh_and_files(
                        &gldf.gldf,
                        mesh_data.as_ref(),
                        &gldf.files,
                    ) {
                        Ok(ifc_content) => {
                            console::log!("Exported IFC:", ifc_content.len(), "chars");

                            // Create blob and trigger download
                            let bytes = ifc_content.as_bytes();
                            let uint8arr = js_sys::Uint8Array::new(
                                &unsafe { js_sys::Uint8Array::view(bytes) }.into(),
                            );
                            let array = js_sys::Array::new();
                            array.push(&uint8arr.buffer());
                            let opts = web_sys::BlobPropertyBag::new();
                            opts.set_type("application/x-step");
                            if let Ok(blob) =
                                web_sys::Blob::new_with_u8_array_sequence_and_options(&array, &opts)
                            {
                                if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                                    let window = web_sys::window().unwrap();
                                    let document = window.document().unwrap();
                                    if let Ok(a) = document.create_element("a") {
                                        let _ = a.set_attribute("href", &url);
                                        let _ = a.set_attribute("download", "exported.ifc");
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
                        Err(e) => {
                            console::error!("IFC export error:", e.to_string());
                        }
                    }
                }
                false
            }
            Msg::SetDragging(dragging) => {
                self.is_dragging = dragging;
                true
            }
            Msg::LoadDemo(filename) => {
                let link = ctx.link().clone();
                let url = format!("/{}", filename);
                let fname = filename.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let result = gloo::net::http::Request::get(&url)
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

                    link.send_message(Msg::DemoLoaded(data, fname));
                });
                false
            }
            Msg::DemoLoaded(result, filename) => {
                match result {
                    Ok(data) => {
                        console::log!("Demo loaded:", data.len(), "bytes");
                        // Clear stale Bevy viewer data before loading new GLDF
                        crate::components::clear_l3d_data();
                        if let Ok(gldf) = WasmGldfProduct::load_gldf_from_buf_all(data.clone()) {
                            self.loaded_gldf = Some(gldf);
                            self.refresh_xsd_violations();
                        }
                        self.files.push(FileDetails {
                            data,
                            file_type: "application/gldf".to_string(),
                            name: filename,
                        });
                    }
                    Err(e) => {
                        console::log!("Failed to load demo:", e.as_str());
                    }
                }
                true
            }
            Msg::LoadFromUrl(url) => {
                console::log!("Loading from URL:", url.as_str());
                self.loading_url = Some(url.clone());
                let link = ctx.link().clone();
                let url_clone = url.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let result = gloo::net::http::Request::get(&url_clone)
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

                    link.send_message(Msg::UrlLoaded(data, url_clone));
                });
                true // Re-render to show loading state
            }
            Msg::UrlLoaded(result, url) => {
                self.loading_url = None;
                match result {
                    Ok(data) => {
                        // Extract filename from URL
                        let file_name = url
                            .split('/')
                            .next_back()
                            .unwrap_or("file.gldf")
                            .split('?')
                            .next()
                            .unwrap_or("file.gldf")
                            .to_string();
                        console::log!("URL loaded:", file_name.as_str(), data.len(), "bytes");

                        // Try to parse GLDF
                        if file_name.to_lowercase().ends_with(".gldf") {
                            // Clear stale Bevy viewer data before loading new GLDF
                            crate::components::clear_l3d_data();
                            if let Ok(gldf) = WasmGldfProduct::load_gldf_from_buf_all(data.clone())
                            {
                                self.loaded_gldf = Some(gldf);
                                self.refresh_xsd_violations();
                            }
                        }

                        self.files.push(FileDetails {
                            data,
                            file_type: "application/gldf".to_string(),
                            name: file_name,
                        });
                    }
                    Err(e) => {
                        console::log!("Failed to load URL:", e.as_str());
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
            Msg::FocusVariant(variant_id) => {
                self.nav_item = NavItem::Variants;
                self.selected_3d_variant = Some(variant_id.clone());
                // Schedule the scroll-into-view for the next animation frame
                // so Yew's render commits the new nav target first.
                schedule_scroll_to_variant(&variant_id);
                true
            }
            Msg::SetUiLanguage(lang) => {
                if crate::state::SUPPORTED_UI_LANGUAGES
                    .iter()
                    .any(|l| **l == lang)
                {
                    save_ui_language_to_storage(&lang);
                    self.ui_language = lang;
                    true
                } else {
                    false
                }
            }
            Msg::DismissXsdViolations => {
                self.xsd_violations_dismissed = true;
                true
            }
            Msg::DownloadFile(file_id) => {
                if let Some(ref gldf) = self.loaded_gldf {
                    // Resolve the <File> element by id, then look up its content
                    // in the BufFile list (which is keyed by ZIP-entry path).
                    let file_def = gldf
                        .gldf
                        .general_definitions
                        .files
                        .file
                        .iter()
                        .find(|f| f.id == file_id);
                    if let Some(def) = file_def {
                        if let Some(buf) = gldf.files.iter().find(|bf| {
                            bf.name
                                .as_ref()
                                .map(|n| {
                                    let stored = n.rsplit('/').next().unwrap_or(n);
                                    stored.eq_ignore_ascii_case(&def.file_name)
                                })
                                .unwrap_or(false)
                        }) {
                            if let Some(ref content) = buf.content {
                                trigger_binary_download(content, &def.file_name, &def.content_type);
                            }
                        }
                    }
                }
                false
            }
            Msg::DownloadVariantPhotometry {
                variant_id,
                photometry_id,
                format,
            } => {
                let Some(ref gldf) = self.loaded_gldf else {
                    return false;
                };
                // Find variant + walk its emitter chain to pick the
                // resolution that matches the requested photometry id.
                let variant = gldf
                    .gldf
                    .product_definitions
                    .variants
                    .as_ref()
                    .and_then(|vs| vs.variant.iter().find(|v| v.id == variant_id));
                let Some(variant) = variant else {
                    return false;
                };
                let resolutions = resolve_variant_photometries(
                    &gldf.gldf.product_definitions,
                    variant,
                    &gldf.gldf.general_definitions,
                );
                let res = resolutions
                    .into_iter()
                    .find(|r| r.photometry_id == photometry_id);
                let Some(res) = res else {
                    return false;
                };

                // Resolve the photometry's source file → bytes.
                let file_id_to_filename: std::collections::HashMap<String, String> = gldf
                    .gldf
                    .general_definitions
                    .files
                    .file
                    .iter()
                    .map(|f| (f.id.clone(), f.file_name.clone()))
                    .collect();
                let phot = gldf
                    .gldf
                    .general_definitions
                    .photometries
                    .as_ref()
                    .and_then(|ps| ps.photometry.iter().find(|p| p.id == photometry_id));
                let Some(phot) = phot else {
                    return false;
                };
                let Some(file_id) = phot
                    .photometry_file_reference
                    .as_ref()
                    .map(|fr| &fr.file_id)
                else {
                    return false;
                };
                let Some(filename) = file_id_to_filename.get(file_id) else {
                    return false;
                };
                let buf = gldf.files.iter().find(|bf| {
                    bf.name
                        .as_ref()
                        .map(|n| {
                            let stored = n.rsplit('/').next().unwrap_or(n);
                            stored.eq_ignore_ascii_case(filename)
                        })
                        .unwrap_or(false)
                });
                let Some(buf) = buf else {
                    return false;
                };
                let Some(ref raw) = buf.content else {
                    return false;
                };

                // Parse, patch, then export in the requested format. Patching
                // happens whether or not the user requested LDT — IES and
                // ATLA exports also need the corrected lumens/watts.
                let s = match std::str::from_utf8(raw) {
                    Ok(s) => s,
                    Err(_) => return false,
                };
                let mut ldt = match eulumdat::Eulumdat::parse(s) {
                    Ok(l) => l,
                    Err(_) => match eulumdat::IesParser::parse(s) {
                        Ok(l) => l,
                        Err(_) => return false,
                    },
                };
                if let Some(set) = ldt.lamp_sets.first_mut() {
                    if let Some(w) = res.watts {
                        set.wattage_with_ballast = w;
                    }
                    if let Some(lm) = res.lumens {
                        set.total_luminous_flux = lm as f64;
                    }
                }

                let Some(bytes) = export_photometry(&ldt, format) else {
                    return false;
                };

                // Filename: <variant_id>_<source_basename>.<ext>. Drop the
                // source extension so we don't end up with foo.ldt.json.
                let source_stem = std::path::Path::new(filename)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("photometry");
                let download_name =
                    format!("{}_{}.{}", variant_id, source_stem, format.extension());
                trigger_binary_download(&bytes, &download_name, format.mime());
                false
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

/// Trigger a text file download in the browser
fn trigger_text_download(content: &str, filename: &str, mime_type: &str) {
    let array = js_sys::Array::new();
    array.push(&wasm_bindgen::JsValue::from_str(content));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime_type);
    if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&array, &opts) {
        if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            if let Ok(a) = document.create_element("a") {
                let _ = a.set_attribute("href", &url);
                let _ = a.set_attribute("download", filename);
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

/// Aggregation result for a single field across `ProductMetaData` defaults
/// and per-`Variant` overrides. Used by Electrical / Mechanical / Marketing
/// viewer pages to summarise "what does this file say about IPCode" etc.
///
/// Variants without an explicit value are treated as "inheriting" — they
/// don't change the aggregation, but they're still valid SKUs.
#[derive(Debug, Clone)]
enum FieldAggregate {
    /// No source has the value. Caller should hide the row entirely.
    Empty,
    /// All sources agree. Single value to display.
    Uniform(String),
    /// Variants disagree. One entry per distinct value; the inner Vec lists
    /// variant ids that hold that value (sorted by appearance).
    Varies(Vec<(String, Vec<String>)>),
}

/// Aggregate a single Electrical (or any DescriptiveAttributes) field over
/// the product default plus every variant's override.
///
/// `default_value` is the ProductMetaData/.../Electrical/<field> value;
/// `variant_values` is `(variant_id, Option<value>)` for every variant.
/// `None` for a variant means "inherit / unset", contributing nothing to
/// the distinct-values set.
fn aggregate_field(
    default_value: Option<String>,
    variant_values: Vec<(String, Option<String>)>,
) -> FieldAggregate {
    use std::collections::BTreeMap;

    // Group variant ids by their explicit value. Variants that don't
    // override (None) are considered "inheriting from default" and tracked
    // separately so we can surface the default value when relevant.
    let mut by_value: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut inheriting: Vec<String> = Vec::new();

    for (vid, val) in variant_values {
        match val {
            Some(v) => by_value.entry(v).or_default().push(vid),
            None => inheriting.push(vid),
        }
    }

    // If there's a default, the inheriting variants effectively carry it.
    if let Some(ref d) = default_value {
        if !inheriting.is_empty() {
            by_value
                .entry(d.clone())
                .or_default()
                .extend(inheriting.iter().cloned());
        }
    }

    match by_value.len() {
        0 => {
            // No variant has the value, but ProductMetaData might still.
            match default_value {
                Some(d) => FieldAggregate::Uniform(d),
                None => FieldAggregate::Empty,
            }
        }
        1 => {
            let (v, _) = by_value.into_iter().next().unwrap();
            FieldAggregate::Uniform(v)
        }
        _ => {
            let buckets: Vec<(String, Vec<String>)> = by_value.into_iter().collect();
            FieldAggregate::Varies(buckets)
        }
    }
}

/// Resolve a display label for a variant chip. Uses the product number
/// (most user-recognisable — typically the article number or GTIN) if any
/// locale entry has a non-empty value, falling back to the variant id.
fn variant_chip_label(variant: &gldf_rs::gldf::product_definitions::Variant) -> String {
    if let Some(ref pn) = variant.product_number {
        if let Some(loc) = pn.locale.iter().find(|l| !l.value.is_empty()) {
            return loc.value.clone();
        }
    }
    variant.id.clone()
}

/// Resolved photometry for a single variant — what `lumens / watts` actually
/// works out to, given the per-variant overrides scattered across the GLDF
/// document tree.
///
/// The XSD chain is:
///   Variant.geometry.{simple|model|bare}_emitter_reference[].emitter_id
///     → GeneralDefinitions.emitters[].emitter (matched by id)
///       → Emitter.fixed_light_emitter[] / changeable_light_emitter[]
///         → photometry_reference.photometry_id  (which LDT shape)
///         → light_source_reference.fixed_light_source_id  (FixedLight only)
///         → rated_luminous_flux  (variant-level lumens override)
///       → GeneralDefinitions.light_sources.fixed_light_source[]  (by id)
///         → rated_input_power  (the wattage we want)
///
/// `lumens` / `watts` may be `None` when the file simply doesn't carry them
/// at the resolved level. In that case the LDT-native value remains the
/// fallback (handled by callers).
#[derive(Debug, Clone)]
struct VariantPhotometryResolution {
    /// Photometry id this emitter resolves to (the LDT shape).
    photometry_id: String,
    /// Variant-level rated luminous flux, if `FixedLightEmitter.RatedLuminousFlux`
    /// or `ChangeableLightEmitter`'s nominal flux is set.
    ///
    /// For `EmergencyOnly` emitters the XSD's `RatedLuminousFlux` is the
    /// emergency-mode flux (because those emitters have no normal mode), so
    /// we don't surface it as a "normal-operation" lumens override —
    /// `lumens` stays `None` for that case to avoid claiming `100 lm / 81 W
    /// = 1.2 lm/W` as the variant's efficacy.
    lumens: Option<i32>,
    /// Variant-level rated input power, walked through the
    /// `LightSourceReference` to the `FixedLightSource.RatedInputPower`.
    /// Suppressed for `EmergencyOnly` emitters (same reason as `lumens`).
    watts: Option<f64>,
    /// Light source id that supplied the wattage (for display / tooltip).
    light_source_id: Option<String>,
    /// `FixedLightEmitter.@emergencyBehaviour` — `None` (no attribute),
    /// `"None"` (explicit normal), `"Combined"` (normal + emergency), or
    /// `"EmergencyOnly"`. Drives a tag in the viewer card and suppresses
    /// the calc/patch for `EmergencyOnly`.
    emergency_behaviour: Option<String>,
    /// `FixedLightEmitter.ControlGearReference.@controlGearId` — points at
    /// the driver in `GeneralDefinitions/ControlGears`. Used by the
    /// Electrical viewer to render driver details (standby power, dimming,
    /// interfaces) per variant.
    control_gear_id: Option<String>,
}

impl VariantPhotometryResolution {
    /// `true` when this emitter only operates during emergency conditions.
    /// The viewer skips the calc column and the patch for these — emergency
    /// flux against full-load wattage produces a meaningless ratio.
    fn is_emergency_only(&self) -> bool {
        matches!(self.emergency_behaviour.as_deref(), Some("EmergencyOnly"))
    }
}

/// Resolve every emitter chain on a single variant to its
/// `(photometry_id, lumens, watts)` tuple. A variant can carry multiple
/// emitters (multi-emitter floodlights, mixed direct/indirect, etc.), which
/// is why the return is a `Vec`.
///
/// Walks all three emitter-reference branches the XSD allows:
/// `Variant > Geometry > EmitterReference` (bare, no 3D),
/// `... > SimpleGeometryReference` (with @emitterId),
/// `... > ModelGeometryReference > EmitterReference` (the nested form).
fn resolve_variant_photometries(
    product: &gldf_rs::gldf::product_definitions::ProductDefinitions,
    variant: &gldf_rs::gldf::product_definitions::Variant,
    general: &gldf_rs::gldf::general_definitions::GeneralDefinitions,
) -> Vec<VariantPhotometryResolution> {
    let _ = product; // Reserved for future use (Equipment lookups, etc.)

    let mut emitter_ids: Vec<String> = Vec::new();
    if let Some(ref geom) = variant.geometry {
        // Bare branch
        for er in &geom.emitter_reference {
            emitter_ids.push(er.emitter_id.clone());
        }
        // Simple geometry branch
        if let Some(ref sgr) = geom.simple_geometry_reference {
            emitter_ids.push(sgr.emitter_id.clone());
        }
        // Model geometry branch (nested EmitterReference list)
        if let Some(ref mgr) = geom.model_geometry_reference {
            for er in &mgr.emitter_reference {
                emitter_ids.push(er.emitter_id.clone());
            }
        }
    }

    let emitters = match general.emitters.as_ref() {
        Some(e) => &e.emitter,
        None => return Vec::new(),
    };
    let light_sources = general.light_sources.as_ref();

    let mut out = Vec::new();
    for eid in &emitter_ids {
        let Some(emitter) = emitters.iter().find(|e| &e.id == eid) else {
            continue;
        };
        // FixedLightEmitter: variant-level lumens + light_source_reference for watts.
        for fle in &emitter.fixed_light_emitter {
            let photometry_id = fle.photometry_reference.photometry_id.clone();
            let is_emergency_only =
                matches!(fle.emergency_behaviour.as_deref(), Some("EmergencyOnly"));
            // For EmergencyOnly emitters, RatedLuminousFlux is the emergency
            // flux and there is no normal-mode wattage on the
            // light_source_reference (the LightSource it points at carries
            // the *full-load* W, not the emergency W). Pairing those two
            // gives a meaningless ratio (100 lm / 81 W ≈ 1.2 lm/W on the
            // SLV Tria 2). Skip the override entirely; the LDT-native calc
            // is similarly off but the viewer suppresses Calc for these.
            let lumens = if is_emergency_only {
                None
            } else {
                fle.rated_luminous_flux
            };
            let (light_source_id, watts) = if is_emergency_only {
                (
                    fle.light_source_reference.fixed_light_source_id.clone(),
                    None,
                )
            } else {
                match (
                    fle.light_source_reference.fixed_light_source_id.as_ref(),
                    light_sources,
                ) {
                    (Some(ls_id), Some(ls)) => {
                        let watts = ls
                            .fixed_light_source
                            .iter()
                            .find(|s| &s.id == ls_id)
                            .and_then(|s| s.rated_input_power);
                        (Some(ls_id.clone()), watts)
                    }
                    _ => (None, None),
                }
            };
            out.push(VariantPhotometryResolution {
                photometry_id,
                lumens,
                watts,
                light_source_id,
                emergency_behaviour: fle.emergency_behaviour.clone(),
                control_gear_id: fle
                    .control_gear_reference
                    .as_ref()
                    .map(|cgr| cgr.control_gear_id.clone()),
            });
        }
        // ChangeableLightEmitter has no light_source_reference at the emitter
        // level (the XSD pairs Changeable bulbs with control gear via
        // Equipment, not at the emitter). For phase 1 we surface them with
        // photometry_id only — wattage shows as None and the calc falls back
        // to the LDT value, which is the right behaviour for replaceable bulbs.
        for cle in &emitter.changeable_light_emitter {
            out.push(VariantPhotometryResolution {
                photometry_id: cle.photometry_reference.photometry_id.clone(),
                lumens: None,
                watts: None,
                light_source_id: None,
                emergency_behaviour: cle.emergency_behaviour.clone(),
                // ChangeableLightEmitter has no ControlGearReference at
                // the emitter level (the XSD wires gear via Equipment for
                // replaceable bulbs). Phase 1 leaves this None.
                control_gear_id: None,
            });
        }
    }
    out
}

/// Aggregated electrical info for a single variant — the headline numbers
/// an electrician needs at install time. Built by walking the same emitter
/// chain the photometry resolver uses, plus the LightSource's voltage /
/// power-range / energy-label metadata.
///
/// `EmergencyOnly` emitters are excluded from the watts/lumens totals (their
/// RatedLuminousFlux is the emergency flux and the LightSource power is the
/// normal-mode rating, so summing them would over-count). They're surfaced
/// separately as a count + emergency flux total.
#[derive(Debug, Clone, Default)]
struct VariantElectricalSummary {
    /// Total connected load (sum of `RatedInputPower` across normal-mode
    /// emitters' light sources) in watts. `None` when no FixedLightSource
    /// in the variant chain has a `RatedInputPower` declared.
    total_watts: Option<f64>,
    /// Total normal-mode flux in lumens (sum of `RatedLuminousFlux` across
    /// all FixedLightEmitters that are not EmergencyOnly).
    total_lumens: Option<i32>,
    /// `total_lumens / total_watts` — only computed when both are present
    /// and watts > 0.
    efficacy_lm_w: Option<f64>,
    /// All distinct voltage rails this variant carries. Most fixtures have
    /// exactly one (e.g. `230V AC 50Hz`). Multi-rail luminaires (a 230V
    /// driver with a 24V auxiliary, etc.) get more than one entry.
    voltages: Vec<String>,
    /// Energy labels from FixedLightSource.EnergyLabels — typically one per
    /// region (`A++/EU`, `A/RU`, etc.).
    energy_labels: Vec<(String, String)>,
    /// Distinct ControlGear ids referenced by this variant's emitters,
    /// resolved against `GeneralDefinitions/ControlGears` for display.
    control_gear_ids: Vec<String>,
    /// Per-emitter dimming envelope from `FixedLightSource.PowerRange`.
    /// `(lower_w, upper_w, default_w)` triples, one per distinct light
    /// source in the variant. Useful for "this variant dims 5–81 W".
    power_ranges: Vec<(f64, f64, f64)>,
    /// Number of `EmergencyOnly` emitters on this variant — surfaced as a
    /// badge ("3 emergency emitters") so the user knows the variant
    /// supports emergency lighting without conflating its W with the load.
    emergency_only_count: usize,
}

/// Roll up a variant's emitter chain into one electrical summary. Pulls
/// voltage, power range, and energy label from the FixedLightSource each
/// emitter resolves to (deduped by light source id so a variant referencing
/// the same LightSource twice doesn't double-count voltage).
fn summarise_variant_electrical(
    variant: &gldf_rs::gldf::product_definitions::Variant,
    general: &gldf_rs::gldf::general_definitions::GeneralDefinitions,
    product: &gldf_rs::gldf::product_definitions::ProductDefinitions,
) -> VariantElectricalSummary {
    let resolutions = resolve_variant_photometries(product, variant, general);
    let light_sources = general.light_sources.as_ref();

    let mut summary = VariantElectricalSummary::default();
    let mut sum_watts = 0.0_f64;
    let mut watts_seen = false;
    let mut sum_lumens = 0_i32;
    let mut lumens_seen = false;
    let mut seen_ls_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for res in &resolutions {
        if res.is_emergency_only() {
            summary.emergency_only_count += 1;
            continue;
        }
        if let Some(w) = res.watts {
            sum_watts += w;
            watts_seen = true;
        }
        if let Some(lm) = res.lumens {
            sum_lumens += lm;
            lumens_seen = true;
        }
        if let Some(cg_id) = &res.control_gear_id {
            if !summary.control_gear_ids.iter().any(|id| id == cg_id) {
                summary.control_gear_ids.push(cg_id.clone());
            }
        }

        // Dedup voltage / power range / energy labels by light source id.
        // Two emitters on the same LightSource share these — counting them
        // twice would mislead the electrician.
        if let (Some(ls_id), Some(ls)) = (&res.light_source_id, light_sources) {
            if seen_ls_ids.insert(ls_id.clone()) {
                if let Some(fls) = ls.fixed_light_source.iter().find(|s| &s.id == ls_id) {
                    if let Some(ref v) = fls.rated_input_voltage {
                        let s = format_voltage(v);
                        if !summary.voltages.iter().any(|x| x == &s) {
                            summary.voltages.push(s);
                        }
                    }
                    if let Some(ref pr) = fls.power_range {
                        summary.power_ranges.push((
                            pr.lower,
                            pr.upper,
                            pr.default_light_source_power,
                        ));
                    }
                    if let Some(ref labels) = fls.energy_labels {
                        for el in &labels.energy_label {
                            let pair = (el.region.clone(), el.value.clone());
                            if !summary.energy_labels.iter().any(|p| p == &pair) {
                                summary.energy_labels.push(pair);
                            }
                        }
                    }
                }
            }
        }
    }

    if watts_seen {
        summary.total_watts = Some(sum_watts);
    }
    if lumens_seen {
        summary.total_lumens = Some(sum_lumens);
    }
    if let (Some(lm), Some(w)) = (summary.total_lumens, summary.total_watts) {
        if w > 0.0 {
            summary.efficacy_lm_w = Some(lm as f64 / w);
        }
    }
    summary
}

/// Format a `Voltage` block into the kind of one-liner an electrician
/// expects to see on a label: `"230V AC 50Hz"`, `"24V DC"`, `"100–240V AC
/// 50/60Hz"`. Falls back to the type/frequency only when neither fixed nor
/// range is set (rare — usually a malformed file).
fn format_voltage(v: &gldf_rs::gldf::general_definitions::electrical::Voltage) -> String {
    let current = match v.type_attr {
        gldf_rs::gldf::general_definitions::electrical::CurrentType::AC => "AC",
        gldf_rs::gldf::general_definitions::electrical::CurrentType::DC => "DC",
        gldf_rs::gldf::general_definitions::electrical::CurrentType::UC => "UC",
    };
    let freq = match v.frequency {
        gldf_rs::gldf::general_definitions::electrical::Frequency::Hz50 => "50Hz",
        gldf_rs::gldf::general_definitions::electrical::Frequency::Hz60 => "60Hz",
        gldf_rs::gldf::general_definitions::electrical::Frequency::Hz50_60 => "50/60Hz",
        gldf_rs::gldf::general_definitions::electrical::Frequency::Hz400 => "400Hz",
    };
    let level = if let Some(fixed) = v.fixed_voltage {
        format!("{:.0}V", fixed)
    } else if let Some(ref r) = v.voltage_range {
        if (r.min - r.max).abs() < f64::EPSILON {
            format!("{:.0}V", r.min)
        } else {
            format!("{:.0}–{:.0}V", r.min, r.max)
        }
    } else {
        return format!("{} {}", current, freq);
    };
    // DC doesn't carry a meaningful frequency; suppress the Hz on DC rails.
    if matches!(
        v.type_attr,
        gldf_rs::gldf::general_definitions::electrical::CurrentType::DC
    ) {
        format!("{} {}", level, current)
    } else {
        format!("{} {} {}", level, current, freq)
    }
}

/// Resolve the variant's `control_gear_ids` against
/// `GeneralDefinitions/ControlGears` to a list of full `ControlGear`
/// records. Drops ids that don't resolve (silent — they'd be schema gaps,
/// not user errors).
fn control_gears_for_summary<'a>(
    summary: &VariantElectricalSummary,
    general: &'a gldf_rs::gldf::general_definitions::GeneralDefinitions,
) -> Vec<&'a gldf_rs::gldf::general_definitions::electrical::ControlGear> {
    let Some(ref cgs) = general.control_gears else {
        return Vec::new();
    };
    summary
        .control_gear_ids
        .iter()
        .filter_map(|id| cgs.control_gear.iter().find(|cg| &cg.id == id))
        .collect()
}

/// Render a single ControlGear as a compact chip with the data points an
/// electrician scans for: standby power, dimmable Y/N, color-controllable
/// Y/N, supported interfaces, energy label.
fn render_control_gear_chip(
    cg: &gldf_rs::gldf::general_definitions::electrical::ControlGear,
) -> Html {
    let name = cg
        .name
        .locale
        .iter()
        .find(|l| !l.value.is_empty())
        .map(|l| l.value.clone())
        .unwrap_or_else(|| cg.id.clone());
    let voltage = cg.nominal_voltage.as_ref().map(format_voltage);
    let interfaces: Vec<String> = cg
        .interfaces
        .as_ref()
        .map(|i| i.interface.clone())
        .unwrap_or_default();

    html! {
        <div class="control-gear-chip">
            <div class="control-gear-chip-header">
                <span class="control-gear-chip-name">{ name }</span>
                <span class="control-gear-chip-id">{ &cg.id }</span>
            </div>
            <div class="control-gear-chip-body">
                if let Some(v) = voltage {
                    <span class="control-gear-meta voltage">{ v }</span>
                }
                if let Some(sp) = cg.standby_power {
                    <span class="control-gear-meta standby" title="Standby power consumption (phantom load)">
                        { format!("standby {:.2} W", sp) }
                    </span>
                }
                if cg.dimmable.unwrap_or(false) {
                    <span class="control-gear-meta dimmable">{ "dimmable" }</span>
                }
                if cg.color_controllable.unwrap_or(false) {
                    <span class="control-gear-meta colorctrl">{ "color-controllable" }</span>
                }
                if !interfaces.is_empty() {
                    <span class="control-gear-meta interfaces" title="Control interfaces">
                        { interfaces.join(" · ") }
                    </span>
                }
                if let Some(ref labels) = cg.energy_labels {
                    { for labels.energy_label.iter().map(|el| html! {
                        <span class="control-gear-meta energy">
                            { format!("{} ({})", el.value, el.region) }
                        </span>
                    }) }
                }
            </div>
        </div>
    }
}

/// Render a single variant's mounting installation card for the Mechanical
/// viewer. Walks all four `Mountings` branches (Ceiling / Wall / Ground /
/// WorkingPlane) and surfaces the data points an installer scans for:
/// recessed cutout dimensions (the hole-saw target), mounting height,
/// pendant length, pole height. Returns `None` when the variant carries an
/// empty `<Mountings/>` so callers can skip it cleanly.
fn render_variant_mounting_card(
    ctx: &Context<App>,
    variant: &gldf_rs::gldf::product_definitions::Variant,
    mountings: &gldf_rs::gldf::product_definitions::Mountings,
) -> Option<Html> {
    let label = variant_chip_label(variant);
    let mut sections: Vec<Html> = Vec::new();

    // Helper: format a Recessed cutout (rectangular or circular) into the
    // installer's "hole" line. Returns None when no cutout is declared.
    let format_recessed = |r: &gldf_rs::gldf::product_definitions::Recessed| -> Html {
        let cutout: Html = if let Some(ref rc) = r.rectangular_cutout {
            html! {
                <span class="cutout-pill rectangular" title="Rectangular cutout (W × L × Depth, mm)">
                    { format!("⬛ {} × {} × {} mm", rc.width, rc.length, rc.depth) }
                </span>
            }
        } else if let Some(ref cc) = r.circular_cutout {
            html! {
                <span class="cutout-pill circular" title="Circular cutout (⌀ × Depth, mm)">
                    { format!("⌀ {} × {} mm depth", cc.diameter, cc.depth) }
                </span>
            }
        } else if r.recessed_depth > 0 {
            // Some files only declare the recessed depth without a cutout
            // (the housing snaps into a generic hole the installer cuts).
            html! {
                <span class="cutout-pill" title="Recessed depth (mm). No cutout dimensions declared.">
                    { format!("{} mm depth", r.recessed_depth) }
                </span>
            }
        } else {
            html! {}
        };
        html! {
            <span class="mount-detail">
                <span class="mount-detail-label">{ "recessed" }</span>
                { cutout }
            </span>
        }
    };

    if let Some(ref ceiling) = mountings.ceiling {
        let mut details: Vec<Html> = Vec::new();
        if let Some(ref r) = ceiling.recessed {
            details.push(format_recessed(r));
        }
        if ceiling.surface_mounted.is_some() {
            details.push(html! {
                <span class="mount-detail"><span class="mount-detail-label">{ "surface mounted" }</span></span>
            });
        }
        if let Some(ref p) = ceiling.pendant {
            details.push(html! {
                <span class="mount-detail">
                    <span class="mount-detail-label">{ "pendant" }</span>
                    <span class="cutout-pill" title="Pendant cable / rod length (mm)">
                        { format!("{:.0} mm drop", p.pendant_length) }
                    </span>
                </span>
            });
        }
        if !details.is_empty() {
            sections.push(html! {
                <div class="mount-section">
                    <span class="mount-section-icon">{ "🔝" }</span>
                    <span class="mount-section-name">{ "Ceiling" }</span>
                    <span class="mount-section-details">{ for details.into_iter() }</span>
                </div>
            });
        }
    }
    if let Some(ref wall) = mountings.wall {
        let mut details: Vec<Html> = Vec::new();
        if wall.mounting_height > 0 {
            details.push(html! {
                <span class="mount-detail">
                    <span class="mount-detail-label">{ "mounting height" }</span>
                    <span class="cutout-pill">{ format!("{} mm", wall.mounting_height) }</span>
                </span>
            });
        }
        if let Some(ref r) = wall.recessed {
            details.push(format_recessed(r));
        }
        if wall.surface_mounted.is_some() {
            details.push(html! {
                <span class="mount-detail"><span class="mount-detail-label">{ "surface mounted" }</span></span>
            });
        }
        if !details.is_empty() {
            sections.push(html! {
                <div class="mount-section">
                    <span class="mount-section-icon">{ "🧱" }</span>
                    <span class="mount-section-name">{ "Wall" }</span>
                    <span class="mount-section-details">{ for details.into_iter() }</span>
                </div>
            });
        }
    }
    if let Some(ref ground) = mountings.ground {
        let mut details: Vec<Html> = Vec::new();
        if let Some(ref pt) = ground.pole_top {
            if let Some(h) = pt.get_pole_height() {
                details.push(html! {
                    <span class="mount-detail">
                        <span class="mount-detail-label">{ "pole top" }</span>
                        <span class="cutout-pill" title="Pole height (mm)">
                            { format!("{} mm pole", h) }
                        </span>
                    </span>
                });
            } else {
                details.push(html! {
                    <span class="mount-detail"><span class="mount-detail-label">{ "pole top" }</span></span>
                });
            }
        }
        if let Some(ref pi) = ground.pole_integrated {
            if let Some(h) = pi.get_pole_height() {
                details.push(html! {
                    <span class="mount-detail">
                        <span class="mount-detail-label">{ "pole integrated" }</span>
                        <span class="cutout-pill" title="Pole height (mm)">
                            { format!("{} mm pole", h) }
                        </span>
                    </span>
                });
            } else {
                details.push(html! {
                    <span class="mount-detail"><span class="mount-detail-label">{ "pole integrated" }</span></span>
                });
            }
        }
        if ground.free_standing.is_some() {
            details.push(html! {
                <span class="mount-detail"><span class="mount-detail-label">{ "free standing" }</span></span>
            });
        }
        if ground.surface_mounted.is_some() {
            details.push(html! {
                <span class="mount-detail"><span class="mount-detail-label">{ "surface mounted" }</span></span>
            });
        }
        if let Some(ref r) = ground.recessed {
            details.push(format_recessed(r));
        }
        if !details.is_empty() {
            sections.push(html! {
                <div class="mount-section">
                    <span class="mount-section-icon">{ "🏞️" }</span>
                    <span class="mount-section-name">{ "Ground" }</span>
                    <span class="mount-section-details">{ for details.into_iter() }</span>
                </div>
            });
        }
    }
    if let Some(ref wp) = mountings.working_plane {
        if wp.free_standing.is_some() {
            sections.push(html! {
                <div class="mount-section">
                    <span class="mount-section-icon">{ "🪑" }</span>
                    <span class="mount-section-name">{ "Working plane" }</span>
                    <span class="mount-section-details">
                        <span class="mount-detail"><span class="mount-detail-label">{ "free standing" }</span></span>
                    </span>
                </div>
            });
        }
    }

    if sections.is_empty() {
        return None;
    }

    let target = variant.id.clone();
    let on_click = ctx
        .link()
        .callback(move |_| Msg::FocusVariant(target.clone()));
    Some(html! {
        <div class="electrical-variant-card">
            <div class="electrical-variant-header">
                <button
                    type="button"
                    class="electrical-variant-name"
                    title={format!("Open variant {} on the Variants page", variant.id)}
                    onclick={on_click}
                >
                    { label }
                </button>
                <span class="electrical-variant-id">{ &variant.id }</span>
            </div>
            <div class="mount-sections">
                { for sections.into_iter() }
            </div>
        </div>
    })
}

/// Export format for per-variant photometry downloads. LDT/IES are the
/// industry-standard photometric formats; ATLA JSON/XML carry richer
/// metadata (lossless-ish round-trip via the eulumdat-rs atla module). The
/// dropdown in the photometry card defaults to whichever matches the source
/// file's extension — note in the UI that round-tripping LDT→IES (or
/// vice-versa) loses some details, so prefer the source format when the
/// downstream tool accepts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhotometryExportFormat {
    Ldt,
    Ies,
    AtlaJson,
    AtlaXml,
}

impl PhotometryExportFormat {
    fn extension(self) -> &'static str {
        match self {
            PhotometryExportFormat::Ldt => "ldt",
            PhotometryExportFormat::Ies => "ies",
            PhotometryExportFormat::AtlaJson => "json",
            PhotometryExportFormat::AtlaXml => "xml",
        }
    }
    fn mime(self) -> &'static str {
        match self {
            PhotometryExportFormat::Ldt | PhotometryExportFormat::Ies => "text/plain",
            PhotometryExportFormat::AtlaJson => "application/json",
            PhotometryExportFormat::AtlaXml => "application/xml",
        }
    }
    fn label(self) -> &'static str {
        match self {
            PhotometryExportFormat::Ldt => "LDT (EULUMDAT)",
            PhotometryExportFormat::Ies => "IES",
            PhotometryExportFormat::AtlaJson => "ATLA JSON",
            PhotometryExportFormat::AtlaXml => "ATLA XML",
        }
    }
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "ldt" => Some(Self::Ldt),
            "ies" => Some(Self::Ies),
            "atla_json" => Some(Self::AtlaJson),
            "atla_xml" => Some(Self::AtlaXml),
            _ => None,
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::Ldt => "ldt",
            Self::Ies => "ies",
            Self::AtlaJson => "atla_json",
            Self::AtlaXml => "atla_xml",
        }
    }
    /// Smart default based on the source filename's extension. LDT input →
    /// LDT output; IES input → IES output. JSON/XML never auto-default
    /// (they're explicit user choices when the input is one of the
    /// industry formats).
    fn default_for_source(filename: Option<&str>) -> Self {
        let lower = filename.map(|n| n.to_lowercase()).unwrap_or_default();
        if lower.ends_with(".ies") {
            Self::Ies
        } else {
            Self::Ldt
        }
    }
}

/// Render the chosen format from a parsed `Eulumdat`. Returns the bytes and
/// the recommended filename suffix (without leading dot).
///
/// All four formats round-trip the *patched* `Eulumdat` object, so a 50W
/// variant downloaded as IES carries the 50W wattage even when the source
/// LDT had 0 W. ATLA JSON/XML are particularly attractive when the
/// downstream tool can consume them — they preserve fields LDT/IES drop.
fn export_photometry(ldt: &eulumdat::Eulumdat, format: PhotometryExportFormat) -> Option<Vec<u8>> {
    match format {
        PhotometryExportFormat::Ldt => Some(ldt.to_ldt().into_bytes()),
        PhotometryExportFormat::Ies => Some(eulumdat::IesExporter::export(ldt).into_bytes()),
        PhotometryExportFormat::AtlaJson => {
            let doc = eulumdat::atla::LuminaireOpticalData::from(ldt);
            eulumdat::atla::json::write(&doc)
                .ok()
                .map(String::into_bytes)
        }
        PhotometryExportFormat::AtlaXml => {
            let doc = eulumdat::atla::LuminaireOpticalData::from(ldt);
            eulumdat::atla::xml::write(&doc)
                .ok()
                .map(String::into_bytes)
        }
    }
}

/// Re-emit an LDT with the variant-resolved lumens / watts patched onto
/// `lamp_sets[0]`. This is what makes the embedded LdtViewer (and any
/// downstream eulumdat consumer) report the correct lm/W instead of the
/// LDT-native 0.0 lm/W.
///
/// Why patch `[0]` only: most LDT fixtures carry one lamp_set; multi-set
/// LDTs (stacked driver options) are rare and the GLDF variant model
/// already disambiguates them via per-variant references, so each variant
/// effectively pins one lamp_set anyway.
///
/// Returns `None` when the bytes don't parse as LDT, or when neither
/// override is supplied (no work to do — caller should keep the original).
fn patch_ldt_for_variant(
    raw_bytes: &[u8],
    lumens: Option<i32>,
    watts: Option<f64>,
) -> Option<Vec<u8>> {
    if lumens.is_none() && watts.is_none() {
        return None;
    }
    let s = std::str::from_utf8(raw_bytes).ok()?;
    let mut ldt = eulumdat::Eulumdat::parse(s).ok()?;
    if let Some(set) = ldt.lamp_sets.first_mut() {
        if let Some(w) = watts {
            set.wattage_with_ballast = w;
        }
        if let Some(lm) = lumens {
            set.total_luminous_flux = lm as f64;
        }
    }
    Some(ldt.to_ldt().into_bytes())
}

/// `localStorage` key used by both the toolbar picker (App level) and the
/// editor-side state. Keep in sync with `state::UI_LANGUAGE_STORAGE_KEY`.
const UI_LANGUAGE_STORAGE_KEY: &str = "gldf.ui_language";

/// Read the user's chosen UI language at startup. Falls back to "en" on
/// missing or unsupported values.
fn load_ui_language_from_storage() -> String {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(UI_LANGUAGE_STORAGE_KEY).ok().flatten())
        .filter(|l| {
            crate::state::SUPPORTED_UI_LANGUAGES
                .iter()
                .any(|s| **s == *l)
        })
        .unwrap_or_else(|| "en".to_string())
}

/// Persist the chosen UI language. Silent no-op outside the browser.
fn save_ui_language_to_storage(lang: &str) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(UI_LANGUAGE_STORAGE_KEY, lang);
        }
    }
}

/// Schedule a scroll-into-view for a variant card after Yew commits the
/// next render. Cards are expected to carry `id="variant-{id}"`. Uses a
/// double-`requestAnimationFrame` to land after Yew's effects flush — a
/// single frame can race with Yew's DOM mutation.
fn schedule_scroll_to_variant(variant_id: &str) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let anchor = format!("variant-{}", variant_id);
    let Some(window) = web_sys::window() else {
        return;
    };

    // Two-frame delay: paint after Yew render, then scroll.
    // `scroll_into_view_with_bool(true)` aligns the element's top edge with
    // the scroll container — good enough; smooth-scroll options need a
    // larger web-sys feature set than the build currently enables.
    let raf2 = Closure::once_into_js(move || {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        if let Some(el) = document.get_element_by_id(&anchor) {
            el.scroll_into_view_with_bool(true);
        }
    });

    let raf1 = Closure::once_into_js(move || {
        let Some(window) = web_sys::window() else {
            return;
        };
        let _ = window.request_animation_frame(raf2.as_ref().unchecked_ref());
    });

    let _ = window.request_animation_frame(raf1.as_ref().unchecked_ref());
}

/// Trigger a binary file download in the browser. The GLDF content_type is
/// used as the MIME type if it looks plausible; otherwise we fall back to
/// `application/octet-stream`. The browser will save the file with the given
/// filename, preserving binary integrity.
fn trigger_binary_download(content: &[u8], filename: &str, content_type: &str) {
    // GLDF content types like `geometry/l3d`, `ldc/eulumdat`, `image/png` —
    // anything containing "/" is good enough as a hint to the browser.
    let mime_type = if content_type.contains('/') {
        content_type
    } else {
        "application/octet-stream"
    };

    let uint8 = js_sys::Uint8Array::new_with_length(content.len() as u32);
    uint8.copy_from(content);
    let array = js_sys::Array::new();
    array.push(&uint8.buffer());
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime_type);
    if let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&array, &opts) {
        if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Ok(a) = document.create_element("a") {
                        let _ = a.set_attribute("href", &url);
                        let _ = a.set_attribute("download", filename);
                        let _ = a.set_attribute("style", "display: none");
                        if let Some(body) = document.body() {
                            let _ = body.append_child(&a);
                            if let Some(html_a) = a.dyn_ref::<web_sys::HtmlElement>() {
                                html_a.click();
                            }
                            let _ = body.remove_child(&a);
                        }
                    }
                }
            }
            let _ = web_sys::Url::revoke_object_url(&url);
        }
    }
}

fn parse_file_for_gldf(file: &FileDetails) -> FileBufGldf {
    WasmGldfProduct::load_gldf_from_buf_all(file.data.clone()).unwrap()
}

/// Render small inline badges summarising what `DescriptiveAttributes` blocks
/// a variant carries (Mechanical / Electrical / Emergency / Marketing /
/// CustomProperties). Visual cue only — the actual values live in the
/// dedicated sidebar pages, no need to duplicate them here.
fn render_variant_attr_badges(
    attrs: &gldf_rs::gldf::product_definitions::DescriptiveAttributes,
) -> Html {
    let mut badges: Vec<Html> = Vec::new();
    if attrs.mechanical.is_some() {
        badges.push(html! { <span class="variant-attr-badge">{ "Mechanical" }</span> });
    }
    if attrs.electrical.is_some() {
        badges.push(html! { <span class="variant-attr-badge">{ "Electrical" }</span> });
    }
    if attrs.emergency.is_some() {
        badges.push(html! { <span class="variant-attr-badge">{ "Emergency" }</span> });
    }
    if attrs.marketing.is_some() {
        badges.push(html! { <span class="variant-attr-badge">{ "Marketing" }</span> });
    }
    if let Some(ref cp) = attrs.custom_properties {
        let count = cp.property.len();
        if count > 0 {
            badges.push(html! {
                <span class="variant-attr-badge custom-properties">
                    { format!("Custom Properties ({})", count) }
                </span>
            });
        }
    }
    if badges.is_empty() {
        return html! {};
    }
    html! {
        <div class="variant-attr-badges" style="margin-top: 10px; display: flex; flex-wrap: wrap; gap: 6px;">
            <span class="label" style="font-size: 11px; color: var(--text-secondary); margin-right: 4px;">{ "Has:" }</span>
            { for badges }
        </div>
    }
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
    /// Render the XSD-violations banner. Called by `view_content` only
    /// when there are violations and the user hasn't dismissed them yet.
    fn view_xsd_violations_banner(&self, ctx: &Context<Self>) -> Html {
        const VISIBLE_LIMIT: usize = 6;
        let total = self.xsd_violations.len();
        let visible: Vec<&gldf_rs::validation::Violation> =
            self.xsd_violations.iter().take(VISIBLE_LIMIT).collect();
        let hidden = total.saturating_sub(VISIBLE_LIMIT);

        let on_dismiss = ctx.link().callback(|_| Msg::DismissXsdViolations);

        html! {
            <div class="xsd-violations-banner">
                <div class="xsd-violations-header">
                    <span class="xsd-violations-icon">{ "🚩" }</span>
                    <span class="xsd-violations-title">
                        { format!("{} schema violation{}: closed-enum value{} outside the XSD definition.",
                            total,
                            if total == 1 { "" } else { "s" },
                            if total == 1 { "" } else { "s" },
                        ) }
                    </span>
                    <button
                        type="button"
                        class="xsd-violations-dismiss"
                        title="Dismiss for this file"
                        onclick={on_dismiss}
                    >
                        { "✕" }
                    </button>
                </div>
                <ul class="xsd-violations-list">
                    { for visible.iter().map(|v| html! {
                        <li class="xsd-violation-row" title={v.hint.clone()}>
                            <code class="xsd-violation-field">{ v.field }</code>
                            <span class="xsd-violation-value">{ format!("\u{201c}{}\u{201d}", v.value) }</span>
                            <span class="xsd-violation-path">{ &v.path }</span>
                        </li>
                    }) }
                    if hidden > 0 {
                        <li class="xsd-violation-more">
                            { format!("+{} more (open the browser console for the full list)", hidden) }
                        </li>
                    }
                </ul>
            </div>
        }
    }

    /// Run the closed-XSD-enum validator against the loaded GLDF and
    /// stash the result. Called from every load path; resets the
    /// dismissed flag so a fresh load always shows the banner if there
    /// are violations.
    fn refresh_xsd_violations(&mut self) {
        self.xsd_violations = match self.loaded_gldf.as_ref() {
            Some(buf) => gldf_rs::validation::validate(&buf.gldf),
            None => Vec::new(),
        };
        self.xsd_violations_dismissed = false;
        if !self.xsd_violations.is_empty() {
            console::log!(format!(
                "GLDF XSD validation: {} closed-enum violation(s)",
                self.xsd_violations.len()
            ));
        }
    }

    /// Parse URL query parameters (?url=... and ?pdf=1)
    fn parse_url_params() -> (Option<String>, bool) {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return (None, false),
        };
        let location = match window.location().search() {
            Ok(s) => s,
            Err(_) => return (None, false),
        };

        if location.is_empty() {
            return (None, false);
        }

        // Parse query string manually (simple implementation)
        let query = location.trim_start_matches('?');
        let mut url_param = None;
        let mut pdf_enabled = false;

        for part in query.split('&') {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "url" => {
                        // URL decode the value
                        url_param = Some(
                            js_sys::decode_uri_component(value)
                                .map(|s| s.as_string().unwrap_or_default())
                                .unwrap_or_else(|_| value.to_string()),
                        );
                    }
                    "pdf" => {
                        pdf_enabled = value == "1" || value.to_lowercase() == "true";
                    }
                    _ => {}
                }
            }
        }

        console::log!(
            "URL params - url:",
            url_param.as_deref().unwrap_or("none"),
            "pdf:",
            pdf_enabled
        );
        (url_param, pdf_enabled)
    }

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
            NavItem::Mechanical => (
                "Mechanical",
                vec![
                    ("IK Rating", "Impact protection per IEC 62262 (IK00–IK10)."),
                    (
                        "Product Form",
                        "Round, Square, Linear, etc. (closed XSD enumeration).",
                    ),
                    ("Product Size", "Length, Width, Height in millimetres."),
                    ("Weight", "Total weight in kilograms."),
                    (
                        "Per-variant",
                        "Outdoor/indoor SKUs may override IK or sealing — pick a variant from the scope picker.",
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
        // Local closure for translating sidebar labels in the active UI
        // language. Declared once per render so the calls below stay
        // compact (`t("gldf.nav.overview")`).
        let ui_lang = self.ui_language.clone();
        let t = |key: &str| crate::i18n::ui::t(key, &ui_lang);

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
        let plugins_count = self
            .loaded_gldf
            .as_ref()
            .map(|g| gldf_rs::plugin::PluginManager::discover_plugins(&g.files).len())
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
                        { self.nav_item(ctx, NavItem::Overview, "📊", &t("gldf.nav.overview"), None, has_file) }
                        { self.nav_item(ctx, NavItem::RawData, "{ }", &t("gldf.nav.raw_data"), None, has_file) }
                        { self.nav_item(ctx, NavItem::FileViewer, "👁", &t("gldf.nav.file_viewer"), Some(self.files.len()), has_file) }
                        { self.nav_item(ctx, NavItem::IfcViewer, "🏗️", &t("gldf.nav.ifc_viewer"), None, has_file) }
                        if plugins_count > 0 {
                            { self.nav_item(ctx, NavItem::Plugins, "🔌", &t("gldf.nav.plugins"), Some(plugins_count), has_file) }
                        }
                    </ul>
                </div>

                <div class="sidebar-section">
                    <div class="sidebar-section-title">{ "Document" }</div>
                    <ul class="sidebar-nav">
                        { self.nav_item(ctx, NavItem::Header, "📄", &t("gldf.nav.header"), None, has_file) }
                        { self.nav_item(ctx, NavItem::Electrical, "⚡", &t("gldf.nav.electrical"), None, has_file) }
                        { self.nav_item(ctx, NavItem::Mechanical, "🔩", &t("gldf.nav.mechanical"), None, has_file) }
                        { self.nav_item(ctx, NavItem::Applications, "🏢", &t("gldf.nav.applications"), None, has_file) }
                        { self.nav_item(ctx, NavItem::Photometry, "🔬", &t("gldf.nav.photometry"), None, has_file) }
                        { self.nav_item(ctx, NavItem::Statistics, "📈", &t("gldf.nav.statistics"), None, has_file) }
                    </ul>
                </div>

                <div class="sidebar-section">
                    <div class="sidebar-section-title">{ "Definitions" }</div>
                    <ul class="sidebar-nav">
                        { self.nav_item(ctx, NavItem::Files, "📁", &t("gldf.nav.files"), Some(files_count), has_file) }
                        { self.nav_item(ctx, NavItem::LightSources, "💡", &t("gldf.nav.light_sources"), Some(light_sources_count), has_file) }
                        { self.nav_item(ctx, NavItem::Emitters, "🔆", &t("gldf.nav.emitters"), Some(emitters_count), has_file) }
                        { self.nav_item(ctx, NavItem::Variants, "📦", &t("gldf.nav.variants"), Some(variants_count), has_file) }
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
        // Local closure for translating welcome-screen labels in the
        // active UI language.
        let ui_lang = self.ui_language.clone();
        let t = |key: &str| crate::i18n::ui::t(key, &ui_lang);

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
                    <label for="file-upload" class="btn btn-primary">{ t("gldf.welcome.open_file") }</label>
                    <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadDemo("slv_tria_2.gldf".into()))}>
                        { t("gldf.welcome.load_slv_tria_2") }
                    </button>
                    <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadDemo("FLOODLIGHT_MAX_1200W_757_SYM_10_WAL_enriched.gldf".into()))}>
                        { t("gldf.welcome.load_floodlight_max") }
                    </button>
                    <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadDemo("LEDVANCE_FLOODLIGHT_MAX.gldf".into()))}>
                        { t("gldf.welcome.load_floodlight_combined") }
                    </button>
                    <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadDemo("biolux_hcl.gldf".into()))}>
                        { t("gldf.welcome.load_biolux") }
                    </button>
                </div>

                <input
                    id="file-upload"
                    type="file"
                    accept=".gldf,.ldt,.ies,.ifc,.uld,.rolf"
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
        // Local closure for translating UI chrome (toolbar buttons, content
        // header title) in the active UI language.
        let ui_lang = self.ui_language.clone();
        let t = |key: &str| crate::i18n::ui::t(key, &ui_lang);

        let title = match self.nav_item {
            NavItem::Overview => t("gldf.nav.overview"),
            NavItem::RawData => t("gldf.nav.raw_data"),
            NavItem::FileViewer => t("gldf.nav.file_viewer"),
            NavItem::Plugins => t("gldf.nav.plugins"),
            NavItem::IfcViewer => t("gldf.nav.ifc_viewer"),
            NavItem::Header => t("gldf.nav.header"),
            NavItem::Electrical => t("gldf.nav.electrical"),
            NavItem::Mechanical => t("gldf.nav.mechanical"),
            NavItem::Applications => t("gldf.nav.applications"),
            NavItem::Photometry => t("gldf.nav.photometry"),
            NavItem::Statistics => t("gldf.nav.statistics"),
            NavItem::Files => t("gldf.nav.files"),
            NavItem::LightSources => t("gldf.nav.light_sources"),
            NavItem::Emitters => t("gldf.nav.emitters"),
            NavItem::Variants => t("gldf.nav.variants"),
        };

        html! {
            <>
                // Content Header with toolbar
                <div class="content-header">
                    <h1 class="content-title">{ title }</h1>

                    <div class="toolbar-actions">
                        if self.loaded_gldf.is_some() {
                            <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::ToggleEditor)}>
                                {
                                    if self.mode == AppMode::Viewer {
                                        t("gldf.toolbar.edit_mode")
                                    } else {
                                        t("gldf.toolbar.view_mode")
                                    }
                                }
                            </button>
                            <button class="btn btn-success" onclick={ctx.link().callback(|_| Msg::ExportGldf)}>
                                { t("gldf.toolbar.export_gldf") }
                            </button>
                            <button class="btn btn-success" onclick={ctx.link().callback(|_| Msg::ExportJson)}>
                                { t("gldf.toolbar.export_json") }
                            </button>
                            <button class="btn btn-success" onclick={ctx.link().callback(|_| Msg::ExportXml)}>
                                { t("gldf.toolbar.export_xml") }
                            </button>
                            <button class="btn btn-info" onclick={ctx.link().callback(|_| Msg::ExportIfc)} title={t("gldf.toolbar.export_ifc_tooltip")}>
                                { t("gldf.toolbar.export_ifc") }
                            </button>
                        }
                        // UI translation language picker. Always visible
                        // alongside Clear/Help so users can verify the effect
                        // even on pages where labels haven't been translated.
                        // Independent of any document locale; persists to
                        // localStorage["gldf.ui_language"].
                        <select
                            class="ui-language-toolbar-select"
                            title={t("gldf.toolbar.ui_lang_tooltip")}
                            value={self.ui_language.clone()}
                            onchange={ctx.link().callback(|e: Event| {
                                let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
                                Msg::SetUiLanguage(select.value())
                            })}
                        >
                            { for crate::state::SUPPORTED_UI_LANGUAGES.iter().map(|l| {
                                let is_selected = *l == self.ui_language;
                                let label = crate::state::ui_language_native_name(l).unwrap_or(*l);
                                html! {
                                    <option value={(*l).to_string()} selected={is_selected}>
                                        { format!("🖥 {}", label) }
                                    </option>
                                }
                            }) }
                        </select>
                        // Always show Clear and Help buttons
                        <button class="btn btn-outline" onclick={ctx.link().callback(|_| Msg::ClearAll)} title={t("gldf.toolbar.clear_tooltip")}>
                            { t("gldf.toolbar.clear") }
                        </button>
                        <button class="btn btn-outline" onclick={ctx.link().callback(|_| Msg::ToggleHelp)} title={t("gldf.toolbar.help_tooltip")}>
                            { t("gldf.toolbar.help") }
                        </button>
                    </div>
                </div>

                // Content Body — wrapped in a GldfProvider when a GLDF is
                // loaded so every per-section view (and any nested
                // LocaleFooView / LocaleFooEditor) sees the same active
                // language and product state. The `LanguageBar` rendered just
                // inside the provider is a viewer-mode affordance: it appears
                // automatically whenever the document has more than one locale
                // and silently hides itself otherwise.
                <div class="content-body">
                    // XSD-violation banner — surfaces every closed-enum
                    // field whose value isn't in the schema's
                    // <xs:enumeration> set. Stays visible until the user
                    // dismisses it (or loads a new file). The list is
                    // capped at 6 visible rows; longer lists collapse to
                    // a "+N more" line so the banner doesn't dominate the
                    // viewport.
                    if !self.xsd_violations.is_empty() && !self.xsd_violations_dismissed {
                        { self.view_xsd_violations_banner(ctx) }
                    }
                    {{
                        let body = match self.nav_item {
                            NavItem::Overview => self.view_overview(),
                            NavItem::RawData => self.view_raw_data(),
                            NavItem::FileViewer => self.view_file_viewer(ctx),
                            NavItem::Plugins => self.view_plugins(),
                            NavItem::IfcViewer => self.view_ifc_viewer(),
                            NavItem::Header => self.view_header_editor(ctx),
                            NavItem::Electrical => self.view_electrical_editor(ctx),
                            NavItem::Mechanical => self.view_mechanical_editor(ctx),
                            NavItem::Applications => self.view_applications_editor(ctx),
                            NavItem::Photometry => self.view_photometry_editor(ctx),
                            NavItem::Statistics => self.view_statistics(),
                            NavItem::Files => self.view_files_list(ctx),
                            NavItem::LightSources => self.view_light_sources(),
                            NavItem::Emitters => self.view_emitters(),
                            NavItem::Variants => self.view_variants(ctx),
                        };
                        // The GldfProvider stays at the workspace level so
                        // every per-section view shares one state tree
                        // (active_language, edits, etc.). The
                        // `LanguageBanner` itself moved out of here — it now
                        // renders only on pages that surface multi-locale
                        // content (Variants, Light Sources, Identity, etc.),
                        // so pages like Electrical / Files / Photometry /
                        // Header dashboard don't show a banner that has
                        // nothing to do.
                        if let Some(ref gldf) = self.loaded_gldf {
                            html! {
                                <GldfProviderWithData gldf={gldf.gldf.clone()}>
                                    { body }
                                </GldfProviderWithData>
                            }
                        } else {
                            body
                        }
                    }}
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
                    <LanguageBanner />
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
                            { for fixed.iter().take(5).map(|ls| html! {
                                <div class="light-source-item">
                                    <span class="icon" style="color: var(--accent-yellow);">{ "💡" }</span>
                                    <div class="info">
                                        <div class="name">
                                            <crate::components::LocaleFooView value={ls.name.clone()} compact=true />
                                        </div>
                                        <div class="id">{ format!("ID: {}", ls.id) }</div>
                                    </div>
                                </div>
                            })}
                            if fixed.len() > 5 {
                                <div class="more-items">{ format!("+ {} more...", fixed.len() - 5) }</div>
                            }
                        }
                        if !changeable.is_empty() {
                            <div class="file-category-title" style="margin-top: 12px;">{ "Changeable" }</div>
                            { for changeable.iter().take(5).map(|ls| html! {
                                <div class="light-source-item">
                                    <span class="icon" style="color: var(--accent-orange);">{ "💡" }</span>
                                    <div class="info">
                                        <div class="name">
                                            <crate::components::LocaleFooView value={ls.name.clone()} compact=true />
                                        </div>
                                        <div class="id">{ format!("ID: {}", ls.id) }</div>
                                    </div>
                                </div>
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
                                let name_value = v.name.clone();
                                let desc_value = v.description.clone();
                                html! {
                                    <div class="variant-item">
                                        <div class="variant-item-header">
                                            <span class="icon">{ "📦" }</span>
                                            <span class="name">{
                                                if let Some(n) = name_value.clone() {
                                                    html! { <crate::components::LocaleFooView value={n} compact=true /> }
                                                } else {
                                                    html! { { &v.id } }
                                                }
                                            }</span>
                                        </div>
                                        if let Some(d) = desc_value {
                                            <div class="description">
                                                <crate::components::LocaleFooView value={d} compact=true />
                                            </div>
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

    fn view_plugins(&self) -> Html {
        use crate::components::PluginViewer;

        if let Some(ref gldf) = self.loaded_gldf {
            let plugins = gldf_rs::plugin::PluginManager::discover_plugins(&gldf.files);

            if plugins.is_empty() {
                return html! {
                    <div class="empty-state">
                        <div class="icon">{ "🔌" }</div>
                        <h3>{ "No Plugins Found" }</h3>
                        <p>{ "This GLDF file does not contain any embedded viewer plugins." }</p>
                    </div>
                };
            }

            html! {
                <div class="plugins-container">
                    <div class="plugins-info">
                        <p>{ format!("Found {} plugin(s) in this GLDF file:", plugins.len()) }</p>
                        <ul class="plugin-list">
                            { for plugins.iter().map(|p| html! {
                                <li class="plugin-item">
                                    <strong>{ &p.manifest.name }</strong>
                                    <span class="plugin-version">{ format!(" v{}", p.manifest.version) }</span>
                                    <p class="plugin-description">{ &p.manifest.description }</p>
                                </li>
                            })}
                        </ul>
                    </div>
                    <div class="plugin-viewer-container">
                        <PluginViewer files={gldf.files.clone()} width={800} height={600} />
                    </div>
                </div>
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="icon">{ "🔌" }</div>
                    <h3>{ "No File Loaded" }</h3>
                    <p>{ "Load a GLDF file to view embedded plugins." }</p>
                </div>
            }
        }
    }

    fn view_ifc_viewer(&self) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            // Convert GLDF to IFC and then import it back to get ImportedLuminaire
            // Extract mesh data if available
            let mesh_data = GldfToIfc::extract_mesh_from_files(&gldf.gldf, &gldf.files);
            match GldfToIfc::export_with_mesh_and_files(&gldf.gldf, mesh_data.as_ref(), &gldf.files)
            {
                Ok(ifc_content) => match IfcImporter::from_str(&ifc_content) {
                    Ok(importer) => match importer.import() {
                        Ok(luminaire) => {
                            html! {
                                <IfcViewer luminaire={luminaire} />
                            }
                        }
                        Err(e) => {
                            html! {
                                <div class="empty-state error">
                                    <div class="icon">{ "⚠️" }</div>
                                    <h3>{ "IFC Import Error" }</h3>
                                    <p>{ format!("Failed to import IFC: {}", e) }</p>
                                </div>
                            }
                        }
                    },
                    Err(e) => {
                        html! {
                            <div class="empty-state error">
                                <div class="icon">{ "⚠️" }</div>
                                <h3>{ "IFC Parse Error" }</h3>
                                <p>{ format!("Failed to parse IFC: {}", e) }</p>
                            </div>
                        }
                    }
                },
                Err(e) => {
                    html! {
                        <div class="empty-state error">
                            <div class="icon">{ "⚠️" }</div>
                            <h3>{ "IFC Export Error" }</h3>
                            <p>{ format!("Failed to export to IFC: {}", e) }</p>
                        </div>
                    }
                }
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="icon">{ "🏗️" }</div>
                    <h3>{ "No File Loaded" }</h3>
                    <p>{ "Load a GLDF file to view IFC representation with 3D geometry." }</p>
                </div>
            }
        }
    }

    fn view_header_editor(&self, ctx: &Context<Self>) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            if self.mode == AppMode::Editor {
                // The outer `view_content` already wraps everything in a
                // `GldfProviderWithData` (so the LanguageBanner and the
                // viewer/editor share one state tree). Don't create a nested
                // provider here — that produced two disconnected reducers
                // (banner wrote to the outer one, child reads from a fresh
                // inner one), which is what made the language switch silent
                // and Edit Mode appear empty until the inner Load fired.
                html! { <EditorTabs /> }
            } else {
                // Document dashboard: a tight key/value list of header fields
                // plus a few derived facts (file size, language coverage, inner
                // file count) and a Download GLDF action. The Header section in
                // the XSD only carries six identity fields; the user's natural
                // question on opening a file is "what is this?" — we expand the
                // page to answer that.
                let header = &gldf.gldf.header;

                let gldf_outer_file = self
                    .files
                    .iter()
                    .find(|f| f.name.to_lowercase().ends_with(".gldf"));
                let gldf_size_str = gldf_outer_file.map(|f| {
                    let bytes = f.data.len();
                    if bytes >= 1_048_576 {
                        format!("{:.2} MB ({} bytes)", bytes as f64 / 1_048_576.0, bytes)
                    } else if bytes >= 1024 {
                        format!("{:.1} KB ({} bytes)", bytes as f64 / 1024.0, bytes)
                    } else {
                        format!("{} bytes", bytes)
                    }
                });

                // GLDF declares <File> entries inside <GeneralDefinitions/Files>.
                // The ZIP wrapper also contains product.xml and the directory
                // entries — those don't count as "files in this GLDF".
                let inner_file_count = gldf.gldf.general_definitions.files.file.len();

                let langs = crate::state::collect_languages(&gldf.gldf);
                let lang_summary = if langs.is_empty() {
                    "(none)".to_string()
                } else if langs.len() == 1 {
                    format!("1 language: {}", langs[0])
                } else {
                    format!("{} languages: {}", langs.len(), langs.join(", "))
                };

                let schema_url = if gldf.gldf.xsnonamespaceschemalocation.is_empty() {
                    None
                } else {
                    Some(gldf.gldf.xsnonamespaceschemalocation.clone())
                };

                // Download button reuses the same export pipeline as the toolbar.
                let on_download_gldf = ctx.link().callback(|_| Msg::ExportGldf);

                html! {
                    <div class="document-dashboard">
                        <div class="dashboard-card">
                            <div class="dashboard-card-header">
                                <h3>{ "Document Identity" }</h3>
                            </div>
                            <dl class="dashboard-list">
                                <dt>{ "Manufacturer" }</dt>
                                <dd>{ if header.manufacturer.is_empty() { "—".to_string() } else { header.manufacturer.clone() } }</dd>
                                <dt>{ "Author" }</dt>
                                <dd>{ if header.author.is_empty() { "—".to_string() } else { header.author.clone() } }</dd>
                                <dt>{ "Created With" }</dt>
                                <dd>{ if header.created_with_application.is_empty() { "—".to_string() } else { header.created_with_application.clone() } }</dd>
                                <dt>{ "Creation Time" }</dt>
                                <dd>{ if header.creation_time_code.is_empty() { "—".to_string() } else { header.creation_time_code.clone() } }</dd>
                                <dt>{ "Format Version" }</dt>
                                <dd class="code">{ header.format_version.to_version_string() }</dd>
                                <dt>{ "Default Language" }</dt>
                                <dd>{ header.default_language.clone().unwrap_or_else(|| "(unset)".to_string()) }</dd>
                            </dl>
                        </div>

                        <div class="dashboard-card">
                            <div class="dashboard-card-header">
                                <h3>{ "Document Stats" }</h3>
                            </div>
                            <dl class="dashboard-list">
                                if let Some(name) = gldf_outer_file.map(|f| f.name.clone()) {
                                    <dt>{ "Filename" }</dt>
                                    <dd class="code">{ name }</dd>
                                }
                                if let Some(size) = gldf_size_str {
                                    <dt>{ "Size" }</dt>
                                    <dd>{ size }</dd>
                                }
                                <dt>{ "Inner Files" }</dt>
                                <dd>{ format!("{} (declared in GeneralDefinitions/Files)", inner_file_count) }</dd>
                                <dt>{ "Locale Coverage" }</dt>
                                <dd>{ lang_summary }</dd>
                                if let Some(url) = schema_url {
                                    <dt>{ "Schema" }</dt>
                                    <dd>
                                        <a href={url.clone()} target="_blank" rel="noopener noreferrer" class="schema-link">
                                            { url }
                                        </a>
                                    </dd>
                                }
                            </dl>
                        </div>

                        <div class="dashboard-actions">
                            <button class="btn btn-primary" onclick={on_download_gldf}>
                                { "⬇ Download GLDF" }
                            </button>
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

    fn view_electrical_editor(&self, ctx: &Context<Self>) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            if self.mode == AppMode::Editor {
                // Inherit the outer provider; see note in view_header_editor.
                html! { <components::ElectricalEditor /> }
            } else {
                // Read-only view. Electrical lives at TWO levels in the XSD:
                // ProductMetaData/.../Electrical (defaults applied to every
                // variant) and Variant/.../Electrical (per-SKU overrides).
                // The viewer aggregates both into one row per field, showing
                // either a single value (when uniform) or a "varies" row with
                // clickable variant chips that focus the relevant SKU on the
                // Variants page.
                let default_electrical = gldf
                    .gldf
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|d| d.electrical.as_ref());

                let variants: Vec<&gldf_rs::gldf::product_definitions::Variant> = gldf
                    .gldf
                    .product_definitions
                    .variants
                    .as_ref()
                    .map(|vs| vs.variant.iter().collect())
                    .unwrap_or_default();

                // Helper: gather (variant_id, Option<value>) for a single
                // field from every variant, given the per-variant accessor.
                let gather = |access: &dyn Fn(
                    &gldf_rs::gldf::product_definitions::Electrical,
                ) -> Option<String>|
                 -> Vec<(String, Option<String>)> {
                    variants
                        .iter()
                        .map(|v| {
                            let val = v
                                .descriptive_attributes
                                .as_ref()
                                .and_then(|a| a.electrical.as_ref())
                                .and_then(access);
                            (v.id.clone(), val)
                        })
                        .collect()
                };

                let safety = aggregate_field(
                    default_electrical
                        .and_then(|e| e.electrical_safety_class.clone())
                        .map(|s| format!("Class {}", s)),
                    gather(&|e| {
                        e.electrical_safety_class
                            .clone()
                            .map(|s| format!("Class {}", s))
                    }),
                );
                let ip = aggregate_field(
                    default_electrical.and_then(|e| e.ingress_protection_ip_code.clone()),
                    gather(&|e| e.ingress_protection_ip_code.clone()),
                );
                let pf = aggregate_field(
                    default_electrical
                        .and_then(|e| e.power_factor)
                        .map(|v| format!("{:.2}", v)),
                    gather(&|e| e.power_factor.map(|v| format!("{:.2}", v))),
                );
                let clo = aggregate_field(
                    default_electrical
                        .and_then(|e| e.constant_light_output)
                        .map(|b| {
                            if b {
                                "Yes".to_string()
                            } else {
                                "No".to_string()
                            }
                        }),
                    gather(&|e| {
                        e.constant_light_output.map(|b| {
                            if b {
                                "Yes".to_string()
                            } else {
                                "No".to_string()
                            }
                        })
                    }),
                );
                // LightDistribution aggregates on the canonical XSD value
                // (so two variants with "Direct" group together regardless of
                // UI language), then we translate the displayed buckets via
                // the i18n table for the active UI language.
                let dist_canonical = aggregate_field(
                    default_electrical.and_then(|e| e.light_distribution.clone()),
                    gather(&|e| e.light_distribution.clone()),
                );
                let ui_lang = gldf.gldf.product_definitions.product_meta_data.as_ref(); // unused here, keep ui_lang from state instead
                let _ = ui_lang;
                let ui_lang_str = self.ui_language.clone();
                let dist = match dist_canonical {
                    FieldAggregate::Empty => FieldAggregate::Empty,
                    FieldAggregate::Uniform(v) => FieldAggregate::Uniform(
                        crate::i18n::light_distribution::display_label(&v, &ui_lang_str),
                    ),
                    FieldAggregate::Varies(buckets) => FieldAggregate::Varies(
                        buckets
                            .into_iter()
                            .map(|(v, ids)| {
                                (
                                    crate::i18n::light_distribution::display_label(
                                        &v,
                                        &ui_lang_str,
                                    ),
                                    ids,
                                )
                            })
                            .collect(),
                    ),
                };
                let sc = aggregate_field(
                    default_electrical.and_then(|e| e.switching_capacity.clone()),
                    gather(&|e| e.switching_capacity.clone()),
                );

                // Build a quick lookup for chip labels.
                let label_of: std::collections::HashMap<String, String> = variants
                    .iter()
                    .map(|v| (v.id.clone(), variant_chip_label(v)))
                    .collect();

                let render_row = |label: &str, agg: &FieldAggregate| -> Html {
                    match agg {
                        FieldAggregate::Empty => html! {},
                        FieldAggregate::Uniform(v) => html! {
                            <div class="aggregate-row">
                                <span class="aggregate-row-label">{ label }</span>
                                <span class="aggregate-row-value">{ v }</span>
                            </div>
                        },
                        FieldAggregate::Varies(buckets) => html! {
                            <div class="aggregate-row varies">
                                <span class="aggregate-row-label">
                                    { label }
                                    <span class="varies-badge" title="This field varies between variants">
                                        { "varies" }
                                    </span>
                                </span>
                                <div class="aggregate-row-buckets">
                                    { for buckets.iter().map(|(value, vids)| html! {
                                        <div class="aggregate-bucket">
                                            <span class="bucket-value">{ value }</span>
                                            <span class="bucket-chips">
                                                { for vids.iter().map(|vid| {
                                                    let display = label_of.get(vid).cloned().unwrap_or_else(|| vid.clone());
                                                    let target = vid.clone();
                                                    let on_click = ctx.link().callback(move |_| Msg::FocusVariant(target.clone()));
                                                    let title = format!("Open variant {} on the Variants page", vid);
                                                    html! {
                                                        <button
                                                            type="button"
                                                            class="variant-chip"
                                                            title={title}
                                                            onclick={on_click}
                                                        >
                                                            { display }
                                                        </button>
                                                    }
                                                }) }
                                            </span>
                                        </div>
                                    }) }
                                </div>
                            </div>
                        },
                    }
                };

                // Per-variant electrical summary — drives both the new
                // aggregate rows (Wattage / Lumens / Efficacy / Voltage)
                // and the standalone "Power & supply" section beneath the
                // grid. Computed once per variant; reused by both renderers.
                let summaries: Vec<(
                    &gldf_rs::gldf::product_definitions::Variant,
                    VariantElectricalSummary,
                )> = variants
                    .iter()
                    .map(|v| {
                        (
                            *v,
                            summarise_variant_electrical(
                                v,
                                &gldf.gldf.general_definitions,
                                &gldf.gldf.product_definitions,
                            ),
                        )
                    })
                    .collect();

                // Wattage row across variants. Aggregated like SafetyClass:
                // single value when all variants agree (or there's only
                // one), per-variant chips when they differ.
                let watts_agg = aggregate_field(
                    None,
                    summaries
                        .iter()
                        .map(|(v, s)| (v.id.clone(), s.total_watts.map(|w| format!("{:.1} W", w))))
                        .collect(),
                );
                let lumens_agg = aggregate_field(
                    None,
                    summaries
                        .iter()
                        .map(|(v, s)| (v.id.clone(), s.total_lumens.map(|lm| format!("{} lm", lm))))
                        .collect(),
                );
                let efficacy_agg = aggregate_field(
                    None,
                    summaries
                        .iter()
                        .map(|(v, s)| {
                            (
                                v.id.clone(),
                                s.efficacy_lm_w.map(|e| format!("{:.1} lm/W", e)),
                            )
                        })
                        .collect(),
                );
                // Voltage: render each variant's distinct voltage rails
                // joined with " + ". Most variants have one; multi-rail
                // luminaires show e.g. "230V AC 50Hz + 24V DC".
                let voltage_agg = aggregate_field(
                    None,
                    summaries
                        .iter()
                        .map(|(v, s)| {
                            let v_str = if s.voltages.is_empty() {
                                None
                            } else {
                                Some(s.voltages.join(" + "))
                            };
                            (v.id.clone(), v_str)
                        })
                        .collect(),
                );

                let any_value = !matches!(safety, FieldAggregate::Empty)
                    || !matches!(ip, FieldAggregate::Empty)
                    || !matches!(pf, FieldAggregate::Empty)
                    || !matches!(clo, FieldAggregate::Empty)
                    || !matches!(dist, FieldAggregate::Empty)
                    || !matches!(sc, FieldAggregate::Empty)
                    || !matches!(watts_agg, FieldAggregate::Empty)
                    || !matches!(lumens_agg, FieldAggregate::Empty)
                    || !matches!(efficacy_agg, FieldAggregate::Empty)
                    || !matches!(voltage_agg, FieldAggregate::Empty);

                // Pre-build the "Power & supply" per-variant cards. One
                // card per variant that has any per-variant electrical
                // info (W, V, energy label, dimming range, gear, emergency).
                let power_cards: Vec<Html> = summaries
                    .iter()
                    .filter(|(_, s)| {
                        s.total_watts.is_some()
                            || !s.voltages.is_empty()
                            || !s.energy_labels.is_empty()
                            || !s.power_ranges.is_empty()
                            || !s.control_gear_ids.is_empty()
                            || s.emergency_only_count > 0
                    })
                    .map(|(v, s)| {
                        let label = variant_chip_label(v);
                        let cgs = control_gears_for_summary(s, &gldf.gldf.general_definitions);
                        let target = v.id.clone();
                        let on_click = ctx
                            .link()
                            .callback(move |_| Msg::FocusVariant(target.clone()));
                        html! {
                            <div class="electrical-variant-card">
                                <div class="electrical-variant-header">
                                    <button
                                        type="button"
                                        class="electrical-variant-name"
                                        title={format!("Open variant {} on the Variants page", v.id)}
                                        onclick={on_click}
                                    >
                                        { label }
                                    </button>
                                    <span class="electrical-variant-id">{ &v.id }</span>
                                </div>
                                <div class="electrical-variant-body">
                                    if let Some(w) = s.total_watts {
                                        <div class="electrical-stat">
                                            <span class="electrical-stat-label">{ "Connected load" }</span>
                                            <span class="electrical-stat-value highlight">
                                                { format!("{:.1} W", w) }
                                            </span>
                                        </div>
                                    }
                                    if let Some(lm) = s.total_lumens {
                                        <div class="electrical-stat">
                                            <span class="electrical-stat-label">{ "Rated flux" }</span>
                                            <span class="electrical-stat-value">
                                                { format!("{} lm", lm) }
                                            </span>
                                        </div>
                                    }
                                    if let Some(e) = s.efficacy_lm_w {
                                        <div class="electrical-stat">
                                            <span class="electrical-stat-label">{ "Efficacy" }</span>
                                            <span class="electrical-stat-value">
                                                { format!("{:.1} lm/W", e) }
                                            </span>
                                        </div>
                                    }
                                    if !s.voltages.is_empty() {
                                        <div class="electrical-stat">
                                            <span class="electrical-stat-label">{ "Mains" }</span>
                                            <span class="electrical-stat-value">
                                                { s.voltages.join(" + ") }
                                            </span>
                                        </div>
                                    }
                                    if !s.power_ranges.is_empty() {
                                        <div class="electrical-stat">
                                            <span class="electrical-stat-label">{ "Dimming range" }</span>
                                            <span class="electrical-stat-value">
                                                { for s.power_ranges.iter().map(|(lo, hi, def)| html! {
                                                    <span class="dimming-pill" title={format!("Default: {:.1} W", def)}>
                                                        { format!("{:.1}–{:.1} W", lo, hi) }
                                                    </span>
                                                }) }
                                            </span>
                                        </div>
                                    }
                                    if !s.energy_labels.is_empty() {
                                        <div class="electrical-stat">
                                            <span class="electrical-stat-label">{ "Energy label" }</span>
                                            <span class="electrical-stat-value">
                                                { for s.energy_labels.iter().map(|(region, value)| html! {
                                                    <span class="energy-label-pill">
                                                        { format!("{} ({})", value, region) }
                                                    </span>
                                                }) }
                                            </span>
                                        </div>
                                    }
                                    if s.emergency_only_count > 0 {
                                        <div class="electrical-stat">
                                            <span class="electrical-stat-label">{ "Emergency emitters" }</span>
                                            <span class="electrical-stat-value">
                                                <span class="emergency-tag emergency-emergencyonly">
                                                    { format!("{} emitter{}",
                                                        s.emergency_only_count,
                                                        if s.emergency_only_count == 1 { "" } else { "s" }
                                                    ) }
                                                </span>
                                            </span>
                                        </div>
                                    }
                                    if !cgs.is_empty() {
                                        <div class="electrical-stat electrical-gear">
                                            <span class="electrical-stat-label">{ "Driver / control gear" }</span>
                                            <div class="electrical-gear-list">
                                                { for cgs.iter().map(|cg| render_control_gear_chip(cg)) }
                                            </div>
                                        </div>
                                    }
                                </div>
                            </div>
                        }
                    })
                    .collect();

                html! {
                    <div class="card">
                        <div class="card-body">
                            if !any_value {
                                <p class="empty-message">{ "No electrical data declared in this GLDF (neither at product level nor on any variant)." }</p>
                            } else {
                                <div class="aggregate-grid">
                                    { render_row("Connected load (W)", &watts_agg) }
                                    { render_row("Rated flux (lm)", &lumens_agg) }
                                    { render_row("Efficacy", &efficacy_agg) }
                                    { render_row("Mains voltage", &voltage_agg) }
                                    { render_row("Safety Class", &safety) }
                                    { render_row("IP Code", &ip) }
                                    { render_row("Power Factor", &pf) }
                                    { render_row("Constant Light Output", &clo) }
                                    { render_row("Light Distribution", &dist) }
                                    { render_row("Switching Capacity", &sc) }
                                </div>

                                if !power_cards.is_empty() {
                                    <h2 class="electrical-section-title">{ "Power & supply (per variant)" }</h2>
                                    <p class="hint" style="margin: 0 0 12px; font-size: 12px; color: var(--text-secondary);">
                                        { "Per-variant connected load, mains voltage, dimming envelope, and driver. Click a variant name to jump to its card on the Variants page." }
                                    </p>
                                    <div class="electrical-variants-grid">
                                        { for power_cards.into_iter() }
                                    </div>
                                }
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

    /// Mechanical viewer / editor — same scope-aware aggregation pattern as
    /// Electrical. The viewer summarises product default + every variant
    /// override into one row per field; editor mode delegates to the
    /// `MechanicalEditor` component which has its own scope picker.
    fn view_mechanical_editor(&self, ctx: &Context<Self>) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            if self.mode == AppMode::Editor {
                html! { <components::MechanicalEditor /> }
            } else {
                let default_mech = gldf
                    .gldf
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|d| d.mechanical.as_ref());

                let variants: Vec<&gldf_rs::gldf::product_definitions::Variant> = gldf
                    .gldf
                    .product_definitions
                    .variants
                    .as_ref()
                    .map(|vs| vs.variant.iter().collect())
                    .unwrap_or_default();

                let gather = |access: &dyn Fn(
                    &gldf_rs::gldf::product_definitions::Mechanical,
                ) -> Option<String>|
                 -> Vec<(String, Option<String>)> {
                    variants
                        .iter()
                        .map(|v| {
                            let val = v
                                .descriptive_attributes
                                .as_ref()
                                .and_then(|a| a.mechanical.as_ref())
                                .and_then(access);
                            (v.id.clone(), val)
                        })
                        .collect()
                };

                let ik = aggregate_field(
                    default_mech.and_then(|m| m.ik_rating.clone()),
                    gather(&|m| m.ik_rating.clone()),
                );
                let form = aggregate_field(
                    default_mech.and_then(|m| m.product_form.clone()),
                    gather(&|m| m.product_form.clone()),
                );
                let weight = aggregate_field(
                    default_mech
                        .and_then(|m| m.weight)
                        .map(|w| format!("{} kg", w)),
                    gather(&|m| m.weight.map(|w| format!("{} kg", w))),
                );
                // Size: render as "L × W × H mm" when present.
                let size_str =
                    |m: &gldf_rs::gldf::product_definitions::Mechanical| -> Option<String> {
                        m.product_size.as_ref().and_then(|s| {
                            if s.length == 0 && s.width == 0 && s.height == 0 {
                                None
                            } else {
                                Some(format!("{} × {} × {} mm", s.length, s.width, s.height))
                            }
                        })
                    };
                let size = aggregate_field(default_mech.and_then(size_str), gather(&size_str));
                // SealingMaterial is a LocaleFoo — pick the active language
                // (or the first non-empty entry) for comparison so we can
                // tell variants apart by their actual displayed text.
                let active_lang = self.ui_language.clone();
                let sealing_str =
                    |m: &gldf_rs::gldf::product_definitions::Mechanical| -> Option<String> {
                        m.sealing_material.as_ref().and_then(|lf| {
                            lf.locale
                                .iter()
                                .find(|l| l.language == active_lang)
                                .or_else(|| lf.locale.iter().find(|l| !l.value.is_empty()))
                                .map(|l| l.value.clone())
                        })
                    };
                let sealing =
                    aggregate_field(default_mech.and_then(sealing_str), gather(&sealing_str));

                // Adjustabilities — list of canonical strings (Tilt, Rotation,
                // LengthAdjustment, ...) per variant. Joined with ", " for
                // the aggregate row; the per-variant Mounting card below
                // renders them as proper chips.
                let adjustabilities_str =
                    |m: &gldf_rs::gldf::product_definitions::Mechanical| -> Option<String> {
                        m.adjustabilities.as_ref().and_then(|a| {
                            if a.adjustability.is_empty() {
                                None
                            } else {
                                Some(a.adjustability.join(", "))
                            }
                        })
                    };
                let adjustabilities = aggregate_field(
                    default_mech.and_then(adjustabilities_str),
                    gather(&adjustabilities_str),
                );

                // ProtectiveAreas — hazardous-zone classifications per variant
                // (Zone 1 / Zone 21 / ATEX). Same join pattern as
                // Adjustabilities; chips rendered per-variant below.
                let protective_areas_str =
                    |m: &gldf_rs::gldf::product_definitions::Mechanical| -> Option<String> {
                        m.protective_areas.as_ref().and_then(|p| {
                            if p.area.is_empty() {
                                None
                            } else {
                                Some(p.area.join(", "))
                            }
                        })
                    };
                let protective_areas = aggregate_field(
                    default_mech.and_then(protective_areas_str),
                    gather(&protective_areas_str),
                );

                let label_of: std::collections::HashMap<String, String> = variants
                    .iter()
                    .map(|v| (v.id.clone(), variant_chip_label(v)))
                    .collect();

                let render_row = |label: &str, agg: &FieldAggregate| -> Html {
                    match agg {
                        FieldAggregate::Empty => html! {},
                        FieldAggregate::Uniform(v) => html! {
                            <div class="aggregate-row">
                                <span class="aggregate-row-label">{ label }</span>
                                <span class="aggregate-row-value">{ v }</span>
                            </div>
                        },
                        FieldAggregate::Varies(buckets) => html! {
                            <div class="aggregate-row varies">
                                <span class="aggregate-row-label">
                                    { label }
                                    <span class="varies-badge" title="This field varies between variants">
                                        { "varies" }
                                    </span>
                                </span>
                                <div class="aggregate-row-buckets">
                                    { for buckets.iter().map(|(value, vids)| html! {
                                        <div class="aggregate-bucket">
                                            <span class="bucket-value">{ value }</span>
                                            <span class="bucket-chips">
                                                { for vids.iter().map(|vid| {
                                                    let display = label_of.get(vid).cloned().unwrap_or_else(|| vid.clone());
                                                    let target = vid.clone();
                                                    let on_click = ctx.link().callback(move |_| Msg::FocusVariant(target.clone()));
                                                    let title = format!("Open variant {} on the Variants page", vid);
                                                    html! {
                                                        <button
                                                            type="button"
                                                            class="variant-chip"
                                                            title={title}
                                                            onclick={on_click}
                                                        >
                                                            { display }
                                                        </button>
                                                    }
                                                }) }
                                            </span>
                                        </div>
                                    }) }
                                </div>
                            </div>
                        },
                    }
                };

                let any_aggregate_value = !matches!(ik, FieldAggregate::Empty)
                    || !matches!(form, FieldAggregate::Empty)
                    || !matches!(weight, FieldAggregate::Empty)
                    || !matches!(size, FieldAggregate::Empty)
                    || !matches!(sealing, FieldAggregate::Empty)
                    || !matches!(adjustabilities, FieldAggregate::Empty)
                    || !matches!(protective_areas, FieldAggregate::Empty);

                // Build per-variant Mounting cards. Each variant that has a
                // `Mountings` block (the vast majority do — that's how the
                // installer learns it goes on the ceiling/wall/ground)
                // gets a card listing every mounting branch the variant
                // declares plus per-branch dimensions. The "money number"
                // for installers is the recessed cutout — surfaced
                // prominently as a highlighted pill.
                let mount_cards: Vec<Html> = variants
                    .iter()
                    .filter_map(|v| {
                        let mountings = v.mountings.as_ref()?;
                        let card = render_variant_mounting_card(ctx, v, mountings);
                        // Suppress empty-mountings cards: a variant can carry
                        // an empty `<Mountings/>` element which produces no
                        // rendered content. `card` returns `None` in that case.
                        card
                    })
                    .collect();

                let any_value = any_aggregate_value || !mount_cards.is_empty();

                html! {
                    <div class="card">
                        <div class="card-body">
                            if !any_value {
                                <p class="empty-message">{ "No mechanical data declared in this GLDF (neither at product level nor on any variant)." }</p>
                            } else {
                                <div class="aggregate-grid">
                                    { render_row("IK Rating", &ik) }
                                    { render_row("Product Form", &form) }
                                    { render_row("Size", &size) }
                                    { render_row("Weight", &weight) }
                                    { render_row("Sealing Material", &sealing) }
                                    { render_row("Adjustabilities", &adjustabilities) }
                                    { render_row("Protective Areas", &protective_areas) }
                                </div>

                                if !mount_cards.is_empty() {
                                    <h2 class="electrical-section-title">{ "Mounting & installation (per variant)" }</h2>
                                    <p class="hint" style="margin: 0 0 12px; font-size: 12px; color: var(--text-secondary);">
                                        { "How each variant installs — ceiling/wall/ground/working-plane mountings with the cutout, mounting height, and pendant length where set. Click a variant name to jump to its card on the Variants page." }
                                    </p>
                                    <div class="electrical-variants-grid">
                                        { for mount_cards.into_iter() }
                                    </div>
                                }
                            }
                        </div>
                    </div>
                }
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="icon">{ "🔩" }</div>
                    <h3>{ "No Mechanical Data" }</h3>
                    <p>{ "Load a GLDF file to view mechanical attributes" }</p>
                </div>
            }
        }
    }

    fn view_applications_editor(&self, ctx: &Context<Self>) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            if self.mode == AppMode::Editor {
                // Inherit the outer provider; see note in view_header_editor.
                html! { <components::ApplicationsEditor /> }
            } else {
                // Read-only view, aggregated across product default + every
                // variant. Applications is a list of XSD enum strings (no
                // LocaleFoo) — same per-variant override pattern as
                // Electrical, but the field is a Vec so we render the *union*
                // across all sources and tag each leaf with the variant chips
                // that include it.
                use std::collections::BTreeSet;

                fn read_applications(
                    attrs: Option<&gldf_rs::gldf::product_definitions::DescriptiveAttributes>,
                ) -> Vec<String> {
                    attrs
                        .and_then(|a| a.marketing.as_ref())
                        .and_then(|m| m.applications.as_ref())
                        .map(|a| a.application.clone())
                        .unwrap_or_default()
                }

                let default_apps: Vec<String> = read_applications(
                    gldf.gldf
                        .product_definitions
                        .product_meta_data
                        .as_ref()
                        .and_then(|m| m.descriptive_attributes.as_ref()),
                );
                let default_set: BTreeSet<String> = default_apps.iter().cloned().collect();

                let variants: Vec<&gldf_rs::gldf::product_definitions::Variant> = gldf
                    .gldf
                    .product_definitions
                    .variants
                    .as_ref()
                    .map(|vs| vs.variant.iter().collect())
                    .unwrap_or_default();

                // Per-variant: (variant_id, set_of_applications). Variants
                // with no Applications block at all are treated as "inheriting
                // the product default" — they get the default set folded in.
                let per_variant: Vec<(String, BTreeSet<String>)> = variants
                    .iter()
                    .map(|v| {
                        let local = read_applications(v.descriptive_attributes.as_ref());
                        let set: BTreeSet<String> = if local.is_empty()
                            && v.descriptive_attributes
                                .as_ref()
                                .and_then(|a| a.marketing.as_ref())
                                .and_then(|m| m.applications.as_ref())
                                .is_none()
                        {
                            // No applications block → inherits product default.
                            default_set.clone()
                        } else {
                            local.into_iter().collect()
                        };
                        (v.id.clone(), set)
                    })
                    .collect();

                // Variants "uniform" if every variant's set equals the same
                // set (which may equal the default or be a different set
                // every variant agrees on).
                let uniform: Option<BTreeSet<String>> = if per_variant.is_empty() {
                    if default_set.is_empty() {
                        None
                    } else {
                        Some(default_set.clone())
                    }
                } else {
                    let first = &per_variant[0].1;
                    if per_variant.iter().all(|(_, s)| s == first) {
                        Some(first.clone())
                    } else {
                        None
                    }
                };

                // Union across product + every variant, for the "varies" branch.
                let mut union_set: BTreeSet<String> = default_set.clone();
                for (_, s) in &per_variant {
                    union_set.extend(s.iter().cloned());
                }

                // Variant id → display label, for chips.
                let label_of: std::collections::HashMap<String, String> = variants
                    .iter()
                    .map(|v| (v.id.clone(), variant_chip_label(v)))
                    .collect();

                // Sort applications hierarchically: major-group strings first,
                // then nested. Stable sort so equal-depth entries preserve
                // their input order.
                let mut sorted: Vec<&String> = union_set.iter().collect();
                sorted.sort_by(|a, b| {
                    let da = a.matches(':').count();
                    let db = b.matches(':').count();
                    da.cmp(&db).then_with(|| a.cmp(b))
                });

                // Translate to the active UI language. The translated form
                // preserves the `:` separator structure so the path/leaf
                // split below works the same way regardless of language.
                let ui_lang = self.ui_language.clone();
                let render_leaf = |app: &str| -> Html {
                    let translated = crate::i18n::applications::display_label(app, &ui_lang);
                    let depth = translated.matches(':').count();
                    let leaf = translated
                        .rsplit(':')
                        .next()
                        .unwrap_or(&translated)
                        .trim()
                        .to_string();
                    let path = if depth > 0 {
                        translated
                            .rsplitn(2, ':')
                            .nth(1)
                            .map(|s| s.trim().to_string())
                    } else {
                        None
                    };
                    html! {
                        <span class="application-leaf-content" style={format!("padding-left: {}px;", depth * 16)}>
                            if let Some(p) = path {
                                <span class="application-path">{ format!("{} › ", p) }</span>
                            }
                            <span class="application-leaf">{ leaf }</span>
                        </span>
                    }
                };

                let any_data = !union_set.is_empty();

                html! {
                    <div class="card">
                        <div class="card-body">
                            if !any_data {
                                <p class="empty-message">
                                    { "No applications declared (neither at product level nor on any variant)." }
                                </p>
                            } else if let Some(set) = uniform {
                                // Every source agrees — single flat tree.
                                <ul class="application-tree">
                                    { for sorted.iter().filter(|app| set.contains(**app)).map(|app| html! {
                                        <li class="application-tree-row">
                                            { render_leaf(app) }
                                        </li>
                                    }) }
                                </ul>
                            } else {
                                // Variants differ. Render the union; tag each
                                // leaf with chips for the variants that
                                // include it. Chips are the same FocusVariant
                                // click target used on the Electrical page.
                                <p class="hint" style="font-size: 12px; color: var(--text-secondary); margin-bottom: 10px;">
                                    { "Applications differ between variants. Each row shows which variants apply to that area." }
                                </p>
                                <ul class="application-tree application-tree-varies">
                                    { for sorted.iter().map(|app| {
                                        let app_str = (*app).clone();
                                        let variants_with: Vec<String> = per_variant
                                            .iter()
                                            .filter(|(_, s)| s.contains(&app_str))
                                            .map(|(id, _)| id.clone())
                                            .collect();
                                        let all_variants = variants_with.len() == per_variant.len()
                                            && !per_variant.is_empty();
                                        html! {
                                            <li class="application-tree-row varies">
                                                { render_leaf(app) }
                                                <span class="application-chips">
                                                    if all_variants {
                                                        <span class="all-badge" title="Every variant applies to this area">
                                                            { "all" }
                                                        </span>
                                                    } else {
                                                        { for variants_with.iter().map(|vid| {
                                                            let display = label_of.get(vid).cloned().unwrap_or_else(|| vid.clone());
                                                            let target = vid.clone();
                                                            let on_click = ctx.link().callback(move |_| Msg::FocusVariant(target.clone()));
                                                            let title = format!("Open variant {} on the Variants page", vid);
                                                            html! {
                                                                <button type="button" class="variant-chip" title={title} onclick={on_click}>
                                                                    { display }
                                                                </button>
                                                            }
                                                        }) }
                                                    }
                                                </span>
                                            </li>
                                        }
                                    }) }
                                </ul>
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

    /// Per-variant photometry download row. Renders a format dropdown
    /// (LDT / IES / ATLA JSON / ATLA XML) plus a Download button. The
    /// dropdown's default is set by the source file's extension via
    /// `PhotometryExportFormat::default_for_source` — LDT input → LDT
    /// default, IES input → IES default. The chosen format applies to the
    /// patched bytes (variant lumens/watts), so the downloaded file is
    /// safe to feed into a calc tool that pre-dates GLDF.
    fn render_photometry_download_bar(
        &self,
        ctx: &Context<Self>,
        variant_id: &str,
        photometry_id: &str,
        default_format: PhotometryExportFormat,
    ) -> Html {
        // Local state for the dropdown selection isn't possible in a struct
        // method that returns Html — instead, we keep it on the App via a
        // hashmap keyed by (variant_id, photometry_id). For now, the
        // dropdown is "fire-and-forget": pick a format, click Download, the
        // selected `value=` is read straight off the DOM in the click
        // handler. Avoids state plumbing and is fine for one-shot exports.
        let picker_id = format!("photo-fmt-{}-{}", variant_id, photometry_id);
        let formats = [
            PhotometryExportFormat::Ldt,
            PhotometryExportFormat::Ies,
            PhotometryExportFormat::AtlaJson,
            PhotometryExportFormat::AtlaXml,
        ];

        let variant_id_owned = variant_id.to_string();
        let photometry_id_owned = photometry_id.to_string();
        let picker_id_for_click = picker_id.clone();
        let on_click = ctx.link().callback(move |_: MouseEvent| {
            // Read the picker's current value off the DOM. Falls back to
            // `default_format` if the element disappeared or the value is
            // unrecognised (shouldn't happen in practice).
            let format = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id(&picker_id_for_click))
                .and_then(|el| el.dyn_into::<web_sys::HtmlSelectElement>().ok())
                .map(|sel| sel.value())
                .and_then(|v| PhotometryExportFormat::from_str(&v))
                .unwrap_or(default_format);
            Msg::DownloadVariantPhotometry {
                variant_id: variant_id_owned.clone(),
                photometry_id: photometry_id_owned.clone(),
                format,
            }
        });

        html! {
            <div class="photometry-download-bar">
                <label class="photometry-download-label" for={picker_id.clone()}>
                    { "Download as:" }
                </label>
                <select id={picker_id} class="photometry-download-select">
                    { for formats.iter().map(|f| {
                        let selected = *f == default_format;
                        html! {
                            <option value={f.as_str()} selected={selected}>{ f.label() }</option>
                        }
                    }) }
                </select>
                <button type="button" class="photometry-download-btn" onclick={on_click}>
                    { "⬇ Download" }
                </button>
                <span class="photometry-download-hint">
                    { "Variant lumens/watts patched into lamp_set[0]. LDT↔IES round-trips lose some metadata; ATLA preserves more." }
                </span>
            </div>
        }
    }

    fn view_photometry_editor(&self, ctx: &Context<Self>) -> Html {
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

                // Inherit the outer provider; see note in view_header_editor.
                html! {
                    <components::PhotometryEditor photometry_files={photometry_files} />
                }
            } else {
                // Read-only view. Photometry is fundamentally **per-variant**:
                // a 50W and 100W version of the same housing share the LDT
                // shape (cd/klm distribution) but have different lumens and
                // watts, so different lm/W. The XSD encodes this via the
                // emitter chain: each variant's emitter points at a
                // PhotometryReference + (for fixed sources) a
                // LightSourceReference, and the variant-level
                // `RatedLuminousFlux` overrides the LDT lumens.
                //
                // Layout: one card per variant, listing the photometries
                // that variant uses. Falls back to one card per Photometry
                // when the file has no variants (older single-fixture
                // files). Calc efficacy is recomputed from the resolved
                // variant lumens/watts — the LDT-native efficacy is wrong
                // when the LDT carries 0 W (which is common: GLDF is the
                // authoritative source for wattage at the variant level).
                let file_id_to_filename: std::collections::HashMap<String, String> = gldf
                    .gldf
                    .general_definitions
                    .files
                    .file
                    .iter()
                    .map(|f| (f.id.clone(), f.file_name.clone()))
                    .collect();

                // Resolve a photometry's referenced file to its raw bytes
                // (needed for the embedded LdtViewer) and the parsed
                // calculated values (for the GLDF/Calc dual grid).
                let resolve_photometry =
                    |file_id: &str| -> Option<(Vec<u8>, components::CalculatedPhotometry)> {
                        let filename = file_id_to_filename.get(file_id)?;
                        let buf = gldf.files.iter().find(|bf| {
                            bf.name
                                .as_ref()
                                .map(|n| {
                                    let stored = n.rsplit('/').next().unwrap_or(n);
                                    stored.eq_ignore_ascii_case(filename)
                                })
                                .unwrap_or(false)
                        })?;
                        let content = buf.content.as_ref()?.clone();
                        let ldt = components::parse_photometry_file(&content)?;
                        let calc = components::CalculatedPhotometry::from_eulumdat(&ldt);
                        Some((content, calc))
                    };

                let photometries = gldf
                    .gldf
                    .general_definitions
                    .photometries
                    .as_ref()
                    .map(|p| &p.photometry)
                    .cloned()
                    .unwrap_or_default();

                // Per-photometry filename lookup, used to label the resolved
                // file alongside its photometry id and to drive the source
                // format detection for the (future) download dropdown.
                let photometry_filename = |phot_id: &str| -> Option<String> {
                    photometries
                        .iter()
                        .find(|p| p.id == phot_id)
                        .and_then(|p| p.photometry_file_reference.as_ref())
                        .and_then(|fr| file_id_to_filename.get(&fr.file_id).cloned())
                };

                let variants: Vec<&gldf_rs::gldf::product_definitions::Variant> = gldf
                    .gldf
                    .product_definitions
                    .variants
                    .as_ref()
                    .map(|vs| vs.variant.iter().collect())
                    .unwrap_or_default();

                // Render a single property row.
                //
                // Default: one clean value. Only widen into the GLDF-vs-Calc
                // dual layout when the two disagree (string-equal after
                // formatting). When only one source has the value, just show
                // it — the user doesn't need to know the provenance unless
                // there's a discrepancy to explain.
                let row_text = |label: &str, gldf: Option<String>, calc: Option<String>| -> Html {
                    match (gldf, calc) {
                        (None, None) => html! {},
                        (Some(g), Some(c)) if g == c => html! {
                            <div class="photometry-row">
                                <span class="photometry-row-label">{ label }</span>
                                <span class="photometry-row-value">{ g }</span>
                            </div>
                        },
                        (Some(g), Some(c)) => html! {
                            <div class="photometry-row mismatch">
                                <span class="photometry-row-label">
                                    { label }
                                    <span class="photometry-mismatch-flag" title="GLDF and calculated values disagree">
                                        { " 🚩" }
                                    </span>
                                </span>
                                <span class="photometry-row-value dual">
                                    <span class="dual-pair">
                                        <span class="cell-tag gldf">{ "GLDF" }</span>
                                        <span class="cell-value">{ g }</span>
                                    </span>
                                    <span class="dual-pair">
                                        <span class="cell-tag calc">{ "Calc" }</span>
                                        <span class="cell-value">{ c }</span>
                                    </span>
                                </span>
                            </div>
                        },
                        (Some(g), None) => html! {
                            <div class="photometry-row">
                                <span class="photometry-row-label">{ label }</span>
                                <span class="photometry-row-value">{ g }</span>
                            </div>
                        },
                        (None, Some(c)) => html! {
                            <div class="photometry-row">
                                <span class="photometry-row-label">{ label }</span>
                                <span class="photometry-row-value">{ c }</span>
                            </div>
                        },
                    }
                };

                let pct = |v: f64| format!("{:.1}%", v * 100.0);
                let pct_opt = |v: Option<f64>| v.map(pct);

                // Render one fully-resolved photometry card, given a parsed
                // calc (LDT-native), an optional descriptive_photometry block
                // (the GLDF-stored values), and optional variant-level
                // lumens/watts overrides. When `override_lumens` and
                // `override_watts` are both present, recompute efficacy
                // from them — that's the point of this page.
                let render_photometry_card = |phot_id: &str,
                                              phot_filename: Option<String>,
                                              desc: Option<
                    &gldf_rs::gldf::general_definitions::photometries::DescriptivePhotometry,
                >,
                                              calc: Option<&components::CalculatedPhotometry>,
                                              ldt_bytes: Option<Vec<u8>>,
                                              override_lumens: Option<i32>,
                                              override_watts: Option<f64>,
                                              light_source_id: Option<&str>,
                                              emergency_behaviour: Option<&str>|
                 -> Html {
                    // Resolved-efficacy line. When both lumens and watts are
                    // available at the variant level, recompute (this is the
                    // headline fix for "1.0 lm/W vs 0.0 lm/W"). When only one
                    // is present, fall back to the LDT-native calc.
                    let resolved_efficacy: Option<f64> = match (override_lumens, override_watts) {
                        (Some(lm), Some(w)) if w > 0.0 => Some(lm as f64 / w),
                        _ => calc.and_then(|c| c.luminous_efficacy).filter(|v| *v > 0.0),
                    };
                    let efficacy_calc_str = resolved_efficacy.map(|v| format!("{:.1} lm/W", v));

                    // Suppress luminaire luminance when calc returns 0
                    // (placeholder — not yet implemented in eulumdat).
                    let lum_calc = calc
                        .and_then(|c| c.luminaire_luminance)
                        .filter(|v| *v > 0.0)
                        .map(|v| format!("{:.0} cd/m²", v));

                    html! {
                        <div class="photometry-card">
                            <div class="photometry-card-header">
                                <h3>{ phot_id }</h3>
                                if let Some(fn_) = phot_filename {
                                    <span class="photometry-file-badge" title="Source file in GeneralDefinitions/Files">
                                        { fn_ }
                                    </span>
                                }
                                if let Some(eb) = emergency_behaviour {
                                    if eb != "None" {
                                        <span
                                            class={classes!("emergency-tag", format!("emergency-{}", eb.to_lowercase()))}
                                            title={match eb {
                                                "EmergencyOnly" => "Emergency-only emitter — RatedLuminousFlux is the emergency flux. Calc and download are suppressed because pairing it with the LightSource's normal-mode RatedInputPower yields a meaningless ratio.",
                                                "Combined" => "Combined-mode emitter — operates normally and during emergency. Calc uses the normal-mode RatedLuminousFlux.",
                                                _ => "",
                                            }}
                                        >
                                            { match eb {
                                                "EmergencyOnly" => "⚠ emergency only",
                                                "Combined" => "+ emergency",
                                                other => other,
                                            } }
                                        </span>
                                    }
                                }
                            </div>
                            // Per-variant resolution summary: lumens, watts,
                            // light source id (when present). Only shown when
                            // we actually resolved variant-level overrides —
                            // for the no-variant fallback this row is empty.
                            if override_lumens.is_some() || override_watts.is_some() {
                                <div class="photometry-resolved">
                                    if let Some(lm) = override_lumens {
                                        <span class="photometry-resolved-pill lumens">
                                            { format!("{} lm", lm) }
                                        </span>
                                    }
                                    if let Some(w) = override_watts {
                                        <span class="photometry-resolved-pill watts" title={
                                            light_source_id
                                                .map(|id| format!("Resolved via FixedLightSource id=\"{}\"", id))
                                                .unwrap_or_else(|| "Per-variant rated input power".to_string())
                                        }>
                                            { format!("{} W", w) }
                                        </span>
                                    }
                                </div>
                            } else if calc.is_none() {
                                <p class="warning" style="margin: 0 0 8px; color: var(--text-tertiary); font-size: 11px;">
                                    { "(file not resolved — Calc column unavailable)" }
                                </p>
                            }
                            <div class="photometry-section">
                                <h4 class="photometry-section-title">{ "Flux distribution" }</h4>
                                { row_text(
                                    "CIE Flux Code",
                                    desc.and_then(|d| d.cie_flux_code.clone()),
                                    calc.and_then(|c| c.cie_flux_code.clone()),
                                ) }
                                { row_text(
                                    "DLOR",
                                    pct_opt(desc.and_then(|d| d.downward_light_output_ratio)),
                                    pct_opt(calc.and_then(|c| c.downward_light_output_ratio)),
                                ) }
                                { row_text(
                                    "ULOR",
                                    pct_opt(desc.and_then(|d| d.upward_light_output_ratio)),
                                    pct_opt(calc.and_then(|c| c.upward_light_output_ratio)),
                                ) }
                                { row_text(
                                    "Downward Flux Fraction",
                                    pct_opt(desc.and_then(|d| d.downward_flux_fraction)),
                                    pct_opt(calc.and_then(|c| c.downward_flux_fraction)),
                                ) }
                                { row_text(
                                    "Cut-off Angle",
                                    desc.and_then(|d| d.cut_off_angle).map(|v| format!("{:.1}°", v)),
                                    calc.and_then(|c| c.cut_off_angle).map(|v| format!("{:.1}°", v)),
                                ) }
                            </div>
                            <div class="photometry-section">
                                <h4 class="photometry-section-title">{ "Efficacy & rating" }</h4>
                                { row_text(
                                    "LOR",
                                    pct_opt(desc.and_then(|d| d.light_output_ratio)),
                                    pct_opt(calc.and_then(|c| c.light_output_ratio)),
                                ) }
                                { row_text(
                                    "Luminous Efficacy",
                                    desc.and_then(|d| d.luminous_efficacy).map(|v| format!("{:.1} lm/W", v)),
                                    efficacy_calc_str,
                                ) }
                                { row_text(
                                    "Luminaire Luminance",
                                    desc.and_then(|d| d.luminaire_luminance).map(|v| format!("{} cd/m²", v)),
                                    lum_calc,
                                ) }
                                { row_text(
                                    "BUG Rating",
                                    desc.and_then(|d| d.light_distribution_bug_rating.clone()),
                                    calc.and_then(|c| c.bug_rating.clone()),
                                ) }
                            </div>
                            <div class="photometry-section">
                                <h4 class="photometry-section-title">{ "Codes" }</h4>
                                { row_text(
                                    "Photometric Code",
                                    desc.and_then(|d| d.photometric_code.clone()),
                                    calc.and_then(|c| c.photometric_code.clone()),
                                ) }
                            </div>
                            // UGR cell remains GLDF-only (we don't compute UGR yet).
                            if let Some(d) = desc {
                                if let Some(ref ugr) = d.ugr4_h8_h705020_lq {
                                    if ugr.x.is_some() || ugr.y.is_some() {
                                        <div class="property">
                                            <span class="property-label">{ "UGR (4H/8H, 70/50/20, LQ)" }</span>
                                            <span class="property-value">{
                                                format!(
                                                    "X={} Y={}",
                                                    ugr.x.map(|v| format!("{:.1}", v)).unwrap_or_else(|| "—".to_string()),
                                                    ugr.y.map(|v| format!("{:.1}", v)).unwrap_or_else(|| "—".to_string())
                                                )
                                            }</span>
                                        </div>
                                    }
                                }
                            }

                            if let Some(bytes) = ldt_bytes {
                                <div class="photometry-ldt-viewer" style="margin-top: 16px; padding-top: 16px; border-top: 1px solid var(--border-color);">
                                    <h4 style="margin: 0 0 8px; font-size: 13px; color: var(--text-secondary);">
                                        { "Photometric File" }
                                    </h4>
                                    <LdtViewer ldt_data={bytes} width={640.0} height={480.0} />
                                </div>
                            }
                        </div>
                    }
                };

                if !variants.is_empty() {
                    // Variant-driven layout. One card per variant; if a
                    // variant has multiple emitters (multi-emitter
                    // floodlight, mixed direct/indirect), each gets its
                    // own photometry sub-card inside the variant card.
                    return html! {
                        <div class="card">
                            <div class="card-body">
                                <p class="hint" style="margin: 0 0 12px; font-size: 12px; color: var(--text-secondary);">
                                    { "Photometry is per-variant: each SKU's lumens and watts may differ even when sharing the same LDT shape. Calc efficacy below is recomputed from the variant-resolved RatedLuminousFlux ÷ FixedLightSource.RatedInputPower." }
                                </p>
                                { for variants.iter().map(|variant| {
                                    let vlabel = variant_chip_label(variant);
                                    let resolutions = resolve_variant_photometries(
                                        &gldf.gldf.product_definitions,
                                        variant,
                                        &gldf.gldf.general_definitions,
                                    );
                                    html! {
                                        <div class="variant-photometry-card" id={format!("variant-photo-{}", variant.id)}>
                                            <h2 class="variant-photometry-header">
                                                { vlabel }
                                                <span class="variant-id-tag">{ &variant.id }</span>
                                            </h2>
                                            if resolutions.is_empty() {
                                                <p class="empty-message">
                                                    { "This variant has no resolvable emitter chain (no Geometry → EmitterReference, or referenced emitter not found in GeneralDefinitions)." }
                                                </p>
                                            } else {
                                                { for resolutions.iter().map(|res| {
                                                    let phot = photometries.iter().find(|p| p.id == res.photometry_id);
                                                    let desc = phot.and_then(|p| p.descriptive_photometry.as_ref());
                                                    let resolved = phot
                                                        .and_then(|p| p.photometry_file_reference.as_ref())
                                                        .and_then(|fr| resolve_photometry(&fr.file_id));
                                                    let original_bytes = resolved.as_ref().map(|(b, _)| b.clone());

                                                    let emergency_only = res.is_emergency_only();

                                                    // Patch the LDT bytes with this variant's
                                                    // lumens/watts so the embedded LdtViewer and
                                                    // its derived `CalculatedPhotometry` reflect
                                                    // the variant — not the LDT-native 0 W.
                                                    // EmergencyOnly emitters skip the patch: their
                                                    // RatedLuminousFlux is the *emergency* flux
                                                    // and the LightSource carries the *normal-mode*
                                                    // wattage, so pairing them is meaningless.
                                                    let patched_bytes = if emergency_only {
                                                        None
                                                    } else {
                                                        original_bytes
                                                            .as_ref()
                                                            .and_then(|b| patch_ldt_for_variant(b, res.lumens, res.watts))
                                                    };
                                                    let display_bytes = patched_bytes.clone().or(original_bytes);

                                                    // Re-derive calc from the patched bytes when
                                                    // available, so all efficacy / DLOR / etc.
                                                    // numbers in the row block are consistent
                                                    // with the embedded viewer. Suppressed for
                                                    // emergency-only emitters (Calc column hidden;
                                                    // the GLDF column still surfaces the asserted
                                                    // value, which is what the user wants there).
                                                    let calc = if emergency_only {
                                                        None
                                                    } else {
                                                        display_bytes.as_ref().and_then(|b| {
                                                            components::parse_photometry_file(b)
                                                                .map(|ldt| components::CalculatedPhotometry::from_eulumdat(&ldt))
                                                        })
                                                    };
                                                    let phot_filename = photometry_filename(&res.photometry_id);
                                                    let card = render_photometry_card(
                                                        &res.photometry_id,
                                                        phot_filename.clone(),
                                                        desc,
                                                        calc.as_ref(),
                                                        display_bytes,
                                                        res.lumens,
                                                        res.watts,
                                                        res.light_source_id.as_deref(),
                                                        res.emergency_behaviour.as_deref(),
                                                    );
                                                    let default_format =
                                                        PhotometryExportFormat::default_for_source(phot_filename.as_deref());
                                                    let download_bar = if emergency_only {
                                                        html! {}
                                                    } else {
                                                        self.render_photometry_download_bar(
                                                            ctx,
                                                            &variant.id,
                                                            &res.photometry_id,
                                                            default_format,
                                                        )
                                                    };
                                                    html! { <>{ card }{ download_bar }</> }
                                                }) }
                                            }
                                        </div>
                                    }
                                })}
                            </div>
                        </div>
                    };
                }

                // No-variant fallback: legacy single-fixture files. One card
                // per Photometry, no resolved per-variant overrides — the
                // efficacy comes from the LDT-native calc.
                html! {
                    <div class="card">
                        <div class="card-body">
                            if photometries.is_empty() {
                                <p class="empty-message">{ "No photometry definitions" }</p>
                            } else {
                                { for photometries.iter().map(|phot| {
                                    let desc = phot.descriptive_photometry.as_ref();
                                    let resolved = phot.photometry_file_reference.as_ref()
                                        .and_then(|fr| resolve_photometry(&fr.file_id));
                                    let calc = resolved.as_ref().map(|(_, c)| c.clone());
                                    let ldt_bytes = resolved.as_ref().map(|(b, _)| b.clone());
                                    let phot_filename = photometry_filename(&phot.id);
                                    render_photometry_card(
                                        &phot.id,
                                        phot_filename,
                                        desc,
                                        calc.as_ref(),
                                        ldt_bytes,
                                        None,
                                        None,
                                        None,
                                        None,
                                    )
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
            // Callback for the file-viewer panel's download button. Captures
            // the currently-selected file id at render time; the row-level
            // download button has its own per-row callback.
            let download_selected_file_id = self.selected_file.clone().unwrap_or_default();
            let on_download_panel = ctx
                .link()
                .callback(move |_| Msg::DownloadFile(download_selected_file_id.clone()));

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
                                    <th>{ "Download" }</th>
                                </tr>
                            </thead>
                            <tbody>
                                { for files.iter().map(|f| {
                                    let file_id_select = f.id.clone();
                                    let file_id_dl = f.id.clone();
                                    let is_selected = self.selected_file.as_ref() == Some(&f.id);
                                    let on_click = ctx.link().callback(move |_| Msg::SelectFile(Some(file_id_select.clone())));

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

                                    // Download button — uses stopPropagation in CSS via a separate handler
                                    // so clicking the icon doesn't also trigger row selection.
                                    let on_download = ctx.link().callback(move |e: MouseEvent| {
                                        e.stop_propagation();
                                        Msg::DownloadFile(file_id_dl.clone())
                                    });

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
                                            <td class="file-download-cell">
                                                if has_content {
                                                    <button
                                                        class="btn-download-file"
                                                        onclick={on_download}
                                                        title={format!("Download {}", f.file_name)}
                                                        style="background: none; border: 1px solid var(--border-color); border-radius: 4px; padding: 4px 8px; cursor: pointer; color: var(--accent-blue);"
                                                    >
                                                        { "⬇" }
                                                    </button>
                                                } else {
                                                    <span style="color: var(--text-tertiary); font-size: 11px;">{ "—" }</span>
                                                }
                                            </td>
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

                    // File viewer at bottom (when file selected). The download
                    // button reuses the same Msg::DownloadFile dispatched by
                    // the table row.
                    if let Some((file_name, content_len, content_viewer)) = viewer_html {
                        <div class="file-viewer-panel" style="margin-top: 20px; background: var(--bg-card); border-radius: 8px; overflow: hidden;">
                            <div style="display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--border-color); background: var(--bg-sidebar);">
                                <div style="font-weight: 500;">
                                    { &file_name }
                                    <span style="color: var(--text-tertiary); margin-left: 8px; font-size: 12px;">
                                        { format!("({} bytes)", content_len) }
                                    </span>
                                </div>
                                <div style="display: flex; gap: 8px; align-items: center;">
                                    <button
                                        onclick={on_download_panel.clone()}
                                        title={format!("Download {}", &file_name)}
                                        style="background: none; border: 1px solid var(--border-color); border-radius: 4px; padding: 4px 10px; cursor: pointer; color: var(--accent-blue); font-size: 13px;"
                                    >
                                        { "⬇ Download" }
                                    </button>
                                    <button
                                        onclick={close_viewer}
                                        style="background: none; border: none; cursor: pointer; font-size: 18px; color: var(--text-secondary);"
                                    >
                                        { "✕" }
                                    </button>
                                </div>
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
                    <LanguageBanner />
                    if !fixed.is_empty() {
                        <h3 style="margin-bottom: 16px; color: var(--text-secondary);">{ "Fixed Light Sources" }</h3>
                        <div class="light-source-cards">
                            { for fixed.iter().map(|ls| html! {
                                <div class="light-source-card">
                                    <div class="card-header-row">
                                        <span class="card-id">{ &ls.id }</span>
                                        <span class="card-type">{ "Fixed" }</span>
                                    </div>
                                    <div class="card-content">
                                        <crate::components::LocaleFooView label="Name" value={ls.name.clone()} />
                                        if let Some(ref d) = ls.description {
                                            <crate::components::LocaleFooView label="Description" value={d.clone()} />
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
                                                    <span class="property-value code">{ gtin }</span>
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
                                            if let Some(ref pr) = ls.power_range {
                                                <div class="property">
                                                    <span class="property-label">{ "Power Range" }</span>
                                                    <span class="property-value">{ format!("{}-{} W", pr.lower, pr.upper) }</span>
                                                </div>
                                            }
                                            if let Some(ref color) = ls.color_information {
                                                if let Some(cct) = color.correlated_color_temperature {
                                                    <div class="property">
                                                        <span class="property-label">{ "Color Temperature" }</span>
                                                        <span class="property-value">{ format!("{} K", cct) }</span>
                                                    </div>
                                                }
                                                if let Some(ref range) = color.color_temperature_adjusting_range {
                                                    if let (Some(lo), Some(hi)) = (range.lower, range.upper) {
                                                        <div class="property">
                                                            <span class="property-label">{ "CCT Range (Tunable)" }</span>
                                                            <span class="property-value">{ format!("{}–{} K", lo, hi) }</span>
                                                        </div>
                                                    }
                                                }
                                                if let Some(cri) = color.color_rendering_index {
                                                    <div class="property">
                                                        <span class="property-label">{ "CRI (Ra)" }</span>
                                                        <span class="property-value">{ format!("{}", cri) }</span>
                                                    </div>
                                                }
                                            }
                                            if let Some(ref m) = ls.light_source_maintenance {
                                                if let Some(lifetime) = m.lifetime {
                                                    <div class="property">
                                                        <span class="property-label">{ "Lifetime" }</span>
                                                        <span class="property-value">{ format!("{} h", lifetime) }</span>
                                                    </div>
                                                }
                                                if let Some(ref lamp_type) = m.cie97_lamp_type {
                                                    <div class="property">
                                                        <span class="property-label">{ "Lamp Type (CIE 97)" }</span>
                                                        <span class="property-value">{ lamp_type }</span>
                                                    </div>
                                                }
                                            }
                                            if let Some(ref sr) = ls.spectrum_reference {
                                                <div class="property">
                                                    <span class="property-label">{ "Spectrum" }</span>
                                                    <span class="property-value code">{ &sr.spectrum_id }</span>
                                                </div>
                                            }
                                            if ls.energy_labels.is_some() {
                                                <div class="property">
                                                    <span class="property-label">{ "Energy Labels" }</span>
                                                    <span class="property-value">{ "present" }</span>
                                                </div>
                                            }
                                            if let Some(ref pou) = ls.light_source_position_of_usage {
                                                <div class="property">
                                                    <span class="property-label">{ "Position of Usage" }</span>
                                                    <span class="property-value">{ pou }</span>
                                                </div>
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
                            }) }
                        </div>
                    }
                    if !changeable.is_empty() {
                        <h3 style="margin: 24px 0 16px; color: var(--text-secondary);">{ "Changeable Light Sources" }</h3>
                        <div class="light-source-cards">
                            { for changeable.iter().map(|ls| html! {
                                <div class="light-source-card">
                                    <div class="card-header-row">
                                        <span class="card-id">{ &ls.id }</span>
                                        <span class="card-type changeable">{ "Changeable" }</span>
                                    </div>
                                    <div class="card-content">
                                        <crate::components::LocaleFooView label="Name" value={ls.name.clone()} />
                                        if let Some(ref d) = ls.description {
                                            <crate::components::LocaleFooView label="Description" value={d.clone()} />
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
                                                    <span class="property-value code">{ &photo_ref.photometry_id }</span>
                                                </div>
                                            }
                                            if let Some(ref m) = ls.light_source_maintenance {
                                                if let Some(lifetime) = m.lifetime {
                                                    <div class="property">
                                                        <span class="property-label">{ "Lifetime" }</span>
                                                        <span class="property-value">{ format!("{} h", lifetime) }</span>
                                                    </div>
                                                }
                                                if let Some(ref lamp_type) = m.cie97_lamp_type {
                                                    <div class="property">
                                                        <span class="property-label">{ "Lamp Type (CIE 97)" }</span>
                                                        <span class="property-value">{ lamp_type }</span>
                                                    </div>
                                                }
                                            }
                                        </div>
                                    </div>
                                </div>
                            }) }
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
                                let def_basename = file_def
                                    .file_name
                                    .rsplit('/')
                                    .next()
                                    .unwrap_or(&file_def.file_name);
                                stored.eq_ignore_ascii_case(def_basename)
                                    || n.eq_ignore_ascii_case(&file_def.file_name)
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

                                // Sensor Emitters
                                if !emitter.sensor_emitter.is_empty() {
                                    <div class="sensors" style="margin-top: 12px;">
                                        <h4 style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">{ "Sensor Emitters" }</h4>
                                        <div style="display: flex; flex-wrap: wrap; gap: 8px;">
                                            { for emitter.sensor_emitter.iter().map(|s| {
                                                html! {
                                                    <span style="background: var(--bg-primary); padding: 4px 8px; border-radius: 4px; font-size: 11px;">
                                                        { &s.sensor_reference.sensor_id }
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

        // Find L3D content: match by file_id first, then by basename
        let l3d_content = gldf
            .files
            .iter()
            .find(|f| {
                // Try file_id match first (most reliable)
                if f.file_id.as_ref() == Some(&mapping.l3d_file_id) {
                    return true;
                }
                // Fall back to basename comparison
                if let Some(ref name) = f.name {
                    let stored_basename = name.rsplit('/').next().unwrap_or(name);
                    mapping
                        .l3d_file_name
                        .as_ref()
                        .map(|n| {
                            let mapping_basename = n.rsplit('/').next().unwrap_or(n);
                            stored_basename.eq_ignore_ascii_case(mapping_basename)
                        })
                        .unwrap_or(false)
                } else {
                    false
                }
            })
            .and_then(|f| f.content.clone());

        // Find LDT content: match by basename (strip directory from both sides)
        let ldt_content = mapping.ldt_file_name.as_ref().and_then(|ldt_name| {
            let ldt_basename = ldt_name.rsplit('/').next().unwrap_or(ldt_name);
            gldf.files
                .iter()
                .find(|f| {
                    if let Some(ref name) = f.name {
                        let stored_basename = name.rsplit('/').next().unwrap_or(name);
                        stored_basename.eq_ignore_ascii_case(ldt_basename)
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
                    // Extract mounting config from variant
                    let mounting = &emitter_data.mounting;
                    let mounting_config = MountingConfig {
                        mounting_type: mounting.mounting_type.as_ref().map(|t| match t {
                            gldf_rs::MountingType::Ceiling => "ceiling".to_string(),
                            gldf_rs::MountingType::Wall => "wall".to_string(),
                            gldf_rs::MountingType::Ground => "ground".to_string(),
                            gldf_rs::MountingType::WorkingPlane => "working_plane".to_string(),
                        }),
                        recessed_depth_mm: mounting.recessed_depth_mm,
                        pendant_length_mm: mounting.pendant_length_mm.map(|f| f as i32),
                        mounting_height_mm: mounting.mounting_height_mm,
                        pole_height_mm: mounting.pole_height_mm,
                        is_surface_mounted: mounting.is_surface_mounted,
                        is_free_standing: mounting.is_free_standing,
                    };
                    (
                        l3d,
                        ldt_data,
                        emitter_config,
                        mounting_config,
                        variant_id.clone(),
                    )
                })
            });

            // Close button callback
            let close_viewer = ctx.link().callback(|_| Msg::Select3dVariant(None));

            html! {
                <div class="variants-container">
                    <LanguageBanner />
                    // 3D Scene Viewer (when variant selected)
                    if let Some((l3d, ldt_data, emitter_config, mounting_config, variant_id)) = viewer_data {
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
                                    mounting_config={mounting_config}
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
                                <div
                                    id={format!("variant-{}", v.id)}
                                    class={classes!("variant-card", "variant-card-expanded", is_selected.then_some("selected"))}
                                >
                                    <div class="card-header-row">
                                        <span class="card-id">{ &v.id }</span>
                                        if let Some(order) = v.sort_order.filter(|&o| o > 0) {
                                            <span style="font-size: 11px; color: var(--text-tertiary);">{ format!("#{}", order) }</span>
                                        }
                                        if let Some(ref gtin) = v.gtin {
                                            <span class="card-gtin" style="font-family: var(--font-mono); font-size: 11px; color: var(--text-secondary); margin-left: auto;">
                                                { format!("GTIN {}", gtin) }
                                            </span>
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

                                        // Multi-locale name / description / product number
                                        if let Some(ref name) = v.name {
                                            <crate::components::LocaleFooView label="Name" value={name.clone()} />
                                        }
                                        if let Some(ref desc) = v.description {
                                            <crate::components::LocaleFooView label="Description" value={desc.clone()} />
                                        }
                                        if let Some(ref pn) = v.product_number {
                                            <crate::components::LocaleFooView label="Product Number" value={pn.clone()} compact=true />
                                        }
                                        if let Some(ref tt) = v.tender_text {
                                            <crate::components::LocaleFooView label="Tender Text" value={tt.clone()} />
                                        }

                                        // ProductSeries link (per-variant)
                                        if let Some(ref series) = v.product_series {
                                            { for series.product_serie.iter().map(|s| html! {
                                                <div class="variant-series" style="margin-top: 8px; padding: 6px 8px; background: var(--bg-secondary); border-radius: 4px;">
                                                    <span class="label" style="font-size: 11px; color: var(--text-secondary);">{ "Series:" }</span>
                                                    <span class="code" style="font-family: var(--font-mono); margin-left: 6px;">{ &s.id }</span>
                                                    if let Some(ref n) = s.name {
                                                        <crate::components::LocaleFooView value={n.clone()} compact=true />
                                                    }
                                                </div>
                                            }) }
                                        }

                                        // Per-variant Pictures (file references)
                                        if let Some(ref pics) = v.pictures {
                                            if !pics.image.is_empty() {
                                                <div class="variant-pictures" style="margin-top: 8px;">
                                                    <span class="label" style="font-size: 11px; color: var(--text-secondary);">{ "Pictures:" }</span>
                                                    <ul style="list-style: none; padding-left: 12px; margin: 4px 0;">
                                                        { for pics.image.iter().map(|img| html! {
                                                            <li style="font-size: 11px;">
                                                                <span class="code" style="font-family: var(--font-mono);">{ &img.file_id }</span>
                                                                <span style="color: var(--text-tertiary); margin-left: 6px;">{ format!("({})", img.image_type) }</span>
                                                            </li>
                                                        }) }
                                                    </ul>
                                                </div>
                                            }
                                        }

                                        // Per-variant DescriptiveAttributes summary (badges only — full details
                                        // live in the dedicated sidebar pages).
                                        if let Some(ref attrs) = v.descriptive_attributes {
                                            { render_variant_attr_badges(attrs) }
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

        // Handle L3D geometry files
        if file_name_lower.ends_with(".l3d") || file.file_type.contains("geo/l3d") {
            return html! {
                <div class="preview-tile">
                    <div class="preview-header">
                        <span class="icon">{ "🔷" }</span>
                        <span class="preview-name">{ &file.name }</span>
                        <span class="preview-type">{ "L3D Geometry" }</span>
                    </div>
                    <div class="preview-media">
                        <L3dViewer l3d_data={file.data.clone()} width={500} height={500} />
                    </div>
                </div>
            };
        }

        // Handle XML files (product.xml)
        if file_name_lower.ends_with(".xml") {
            let xml_content = String::from_utf8_lossy(&file.data).to_string();
            return html! {
                <div class="preview-tile">
                    <div class="preview-header">
                        <span class="icon">{ "📋" }</span>
                        <span class="preview-name">{ &file.name }</span>
                        <span class="preview-type">{ "XML" }</span>
                    </div>
                    <div class="preview-media">
                        <textarea readonly=true value={xml_content} style="width: 100%; height: 400px; font-family: monospace; font-size: 12px;" />
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
