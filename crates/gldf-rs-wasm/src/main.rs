#![recursion_limit = "256"]
//! GLDF WASM Editor Application

extern crate base64;
extern crate gldf_rs;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use gldf_rs::convert::ldt_to_gldf;
use gldf_rs::gldf::GldfProduct;
use gldf_rs::version::{BuildVersion, VersionStatus};
use gldf_rs::{BufFile, FileBufGldf};
use gloo::console;
use gloo::file::callbacks::FileReader;
use gloo::file::File;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Blob, FileList, HtmlInputElement};
use yew::prelude::*;

// Use centralized JS bindings from lib to avoid duplicate symbols
use gldf_rs_wasm::js_bindings::{
    compile_typst_to_pdf_js, has_embedded_viewer, register_embedded_viewer, save_star_sky_for_bevy,
};

/// Secret URL path/parameter that enables PDF export feature
const PDF_EXPORT_PATH: &str = "gldf_pdf_export";

/// Extract and register embedded WASM viewers from a GLDF file
fn extract_embedded_viewers(gldf: &FileBufGldf) {
    // Helper to find file content by path
    fn find_file_content(gldf: &FileBufGldf, path: &str) -> Option<Vec<u8>> {
        gldf.files.iter()
            .find(|f| f.path.as_ref().map(|p| p == path).unwrap_or(false))
            .and_then(|f| f.content.clone())
    }

    fn find_file_string(gldf: &FileBufGldf, path: &str) -> Option<String> {
        find_file_content(gldf, path).and_then(|c| String::from_utf8(c).ok())
    }

    // Extract Bevy 3D viewer
    if let Some(manifest_json) = find_file_string(gldf, "other/viewer/bevy/manifest.json") {
        if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&manifest_json) {
            let js_name = manifest.get("js").and_then(|v| v.as_str()).unwrap_or("");
            let wasm_name = manifest.get("wasm").and_then(|v| v.as_str()).unwrap_or("");

            let js_path = format!("other/viewer/bevy/{}", js_name);
            let wasm_path = format!("other/viewer/bevy/{}", wasm_name);

            if let (Some(js), Some(wasm)) = (find_file_string(gldf, &js_path), find_file_content(gldf, &wasm_path)) {
                console::log!("Registering embedded Bevy viewer:", js.len(), "JS bytes,", wasm.len(), "WASM bytes");
                register_embedded_viewer("bevy", &manifest_json, &js, &wasm);
            }
        }
    }

    // Extract AcadLISP viewer
    if let Some(manifest_json) = find_file_string(gldf, "other/viewer/acadlisp/manifest.json") {
        if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&manifest_json) {
            let js_name = manifest.get("js").and_then(|v| v.as_str()).unwrap_or("");
            let wasm_name = manifest.get("wasm").and_then(|v| v.as_str()).unwrap_or("");

            let js_path = format!("other/viewer/acadlisp/{}", js_name);
            let wasm_path = format!("other/viewer/acadlisp/{}", wasm_name);

            if let (Some(js), Some(wasm)) = (find_file_string(gldf, &js_path), find_file_content(gldf, &wasm_path)) {
                console::log!("Registering embedded AcadLISP viewer:", js.len(), "JS bytes,", wasm.len(), "WASM bytes");
                register_embedded_viewer("acadlisp", &manifest_json, &js, &wasm);

                // Also register xlisp core if present
                if let (Some(xjs), Some(xwasm)) = (
                    find_file_string(gldf, "other/viewer/acadlisp/xlisp.js"),
                    find_file_content(gldf, "other/viewer/acadlisp/xlisp.wasm")
                ) {
                    console::log!("Registering embedded XLisp core:", xjs.len(), "JS bytes,", xwasm.len(), "WASM bytes");
                    register_embedded_viewer("xlisp", "{}", &xjs, &xwasm);
                }
            }
        }
    }

    // Extract Star Sky 2D viewer (lightweight canvas-based star visualization)
    if let Some(manifest_json) = find_file_string(gldf, "other/viewer/starsky/manifest.json") {
        if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&manifest_json) {
            let js_name = manifest.get("js").and_then(|v| v.as_str()).unwrap_or("gldf_starsky_wasm.js");
            let wasm_name = manifest.get("wasm").and_then(|v| v.as_str()).unwrap_or("gldf_starsky_wasm_bg.wasm");

            let js_path = format!("other/viewer/starsky/{}", js_name);
            let wasm_path = format!("other/viewer/starsky/{}", wasm_name);

            if let (Some(js), Some(wasm)) = (find_file_string(gldf, &js_path), find_file_content(gldf, &wasm_path)) {
                console::log!("Registering embedded Star Sky viewer:", js.len(), "JS bytes,", wasm.len(), "WASM bytes");
                register_embedded_viewer("starsky", &manifest_json, &js, &wasm);
            }
        }
    }
}

/// Check if PDF export should be enabled based on URL
/// Returns true if:
/// - URL contains "gldf_pdf_export" (e.g., gldf_pdf_export.html)
/// - URL has ?pdf=1 parameter
/// - typst-loader.js is already loaded (window.compileTypstToPdf exists)
fn is_pdf_export_enabled() -> bool {
    if let Some(window) = web_sys::window() {
        // Check if typst-loader is already loaded
        if js_sys::Reflect::has(&window, &JsValue::from_str("compileTypstToPdf")).unwrap_or(false) {
            return true;
        }

        // Check URL path
        if let Ok(href) = window.location().href() {
            if href.contains(PDF_EXPORT_PATH) {
                return true;
            }
        }

        // Check query parameter ?pdf=1
        if let Ok(search) = window.location().search() {
            if search.contains("pdf=1") || search.contains("pdf=true") {
                return true;
            }
        }
    }
    false
}

/// Dynamically load typst-loader.js if not already loaded
async fn load_typst_if_needed() -> Result<(), String> {
    let window = web_sys::window().ok_or("No window object")?;

    // Check if already loaded
    if js_sys::Reflect::has(&window, &JsValue::from_str("compileTypstToPdf")).unwrap_or(false) {
        // Call preloadTypst to ensure the WASM module is ready
        if let Ok(preload_fn) = js_sys::Reflect::get(&window, &JsValue::from_str("preloadTypst")) {
            if preload_fn.is_function() {
                let func = js_sys::Function::from(preload_fn);
                if let Ok(promise) = func.call0(&window) {
                    let _ = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise))
                        .await
                        .map_err(|e| format!("Typst preload failed: {:?}", e))?;
                }
            }
        }
        return Ok(());
    }

    // Need to load typst-loader.js first
    let document = window.document().ok_or("No document object")?;

    // Create script element
    let script = document
        .create_element("script")
        .map_err(|_| "Failed to create script element")?;
    script
        .set_attribute("src", "typst-loader.js")
        .map_err(|_| "Failed to set src attribute")?;

    // Create a promise that resolves when script loads
    let (tx, rx) = futures::channel::oneshot::channel::<Result<(), String>>();
    let tx = std::rc::Rc::new(std::cell::RefCell::new(Some(tx)));

    // Set up load handler
    let tx_clone = tx.clone();
    let onload = Closure::once(Box::new(move || {
        if let Some(tx) = tx_clone.borrow_mut().take() {
            let _ = tx.send(Ok(()));
        }
    }) as Box<dyn FnOnce()>);
    script
        .add_event_listener_with_callback("load", onload.as_ref().unchecked_ref())
        .map_err(|_| "Failed to add load listener")?;
    onload.forget();

    // Set up error handler
    let tx_clone = tx;
    let onerror = Closure::once(Box::new(move || {
        if let Some(tx) = tx_clone.borrow_mut().take() {
            let _ = tx.send(Err("Failed to load typst-loader.js".to_string()));
        }
    }) as Box<dyn FnOnce()>);
    script
        .add_event_listener_with_callback("error", onerror.as_ref().unchecked_ref())
        .map_err(|_| "Failed to add error listener")?;
    onerror.forget();

    // Append to document
    document
        .head()
        .ok_or("No head element")?
        .append_child(&script)
        .map_err(|_| "Failed to append script")?;

    // Wait for script to load
    rx.await.map_err(|_| "Script load cancelled")??;

    // Now call preloadTypst to load the WASM module
    gloo::console::log!("typst-loader.js loaded, loading WASM module...");
    if let Ok(preload_fn) = js_sys::Reflect::get(&window, &JsValue::from_str("preloadTypst")) {
        if preload_fn.is_function() {
            let func = js_sys::Function::from(preload_fn);
            if let Ok(promise) = func.call0(&window) {
                wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise))
                    .await
                    .map_err(|e| format!("Typst WASM load failed: {:?}", e))?;
            }
        }
    }

    Ok(())
}

/// Generate a Typst report for GLDF product data
/// Generate spectral SVG for a photometry file (TM-33/IESXML)
fn generate_spectral_svg_for_photometry(data: &[u8]) -> Option<String> {
    // Try to parse as TM-33/ATLA
    let content = std::str::from_utf8(data).ok()?;
    if !content.contains("IES") && !content.contains("LuminaireOpticalData") {
        return None;
    }

    let doc = atla::parse(content).ok()?;

    // Find spectral data from emitters
    let spd = doc.emitters.iter()
        .find_map(|e| e.spectral_distribution.as_ref())?;

    if spd.wavelengths.is_empty() || spd.values.is_empty() {
        return None;
    }

    // Generate SVG using atla's SpectralDiagram with light theme for PDF
    let theme = atla::SpectralTheme::light();
    let diagram = atla::SpectralDiagram::from_spectral(spd);
    Some(diagram.to_svg(400.0, 200.0, &theme))
}

fn generate_gldf_typst_report(gldf: &GldfProduct, files: &[BufFile]) -> String {
    let manufacturer = &gldf.header.manufacturer;
    let product_name = gldf
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|pm| pm.name.as_ref())
        .and_then(|n| n.locale.first())
        .map(|l| l.value.as_str())
        .unwrap_or("Unknown Product");

    let description = gldf
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|pm| pm.description.as_ref())
        .and_then(|d| d.locale.first())
        .map(|l| l.value.as_str())
        .unwrap_or("");

    let variant_count = gldf
        .product_definitions
        .variants
        .as_ref()
        .map(|v| v.variant.len())
        .unwrap_or(0);

    let light_source_count = gldf
        .general_definitions
        .light_sources
        .as_ref()
        .map(|ls| ls.fixed_light_source.len() + ls.changeable_light_source.len())
        .unwrap_or(0);

    // Generate variant table rows
    let variant_rows = gldf
        .product_definitions
        .variants
        .as_ref()
        .map(|variants| {
            variants
                .variant
                .iter()
                .take(20) // Limit to first 20 variants for the report
                .map(|v| {
                    let name = v
                        .name
                        .as_ref()
                        .and_then(|n| n.locale.first())
                        .map(|l| l.value.as_str())
                        .unwrap_or(&v.id);
                    let product_number = v
                        .product_number
                        .as_ref()
                        .and_then(|pn| pn.locale.first())
                        .map(|l| l.value.as_str())
                        .unwrap_or("-");
                    format!("  [{}], [{}],", name, product_number)
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    // Generate emitters section with spectral diagrams
    let emitters_section = generate_emitters_typst_section(gldf, files);

    format!(
        r##"#set page(paper: "a4", margin: 2cm)
#set text(font: "Arial", size: 11pt)

#align(center)[
  #text(size: 24pt, weight: "bold")[GLDF Product Report]

  #v(1em)

  #text(size: 14pt)[{manufacturer}]

  #v(0.5em)

  #text(size: 16pt, weight: "bold")[{product_name}]
]

#v(2em)

= Product Overview

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt,
  inset: 8pt,
  [*Manufacturer*], [{manufacturer}],
  [*Product Name*], [{product_name}],
  [*Variants*], [{variant_count}],
  [*Light Sources*], [{light_source_count}],
)

#v(1em)

== Description

{description}

#v(1em)

#if {variant_count} > 0 [
  = Product Variants

  #table(
    columns: (1fr, 1fr),
    stroke: 0.5pt,
    inset: 6pt,
    [*Variant Name*], [*Product Number*],
{variant_rows}
  )

  #if {variant_count} > 20 [
    _... and {remaining} more variants_
  ]
]

{emitters_section}

#v(2em)

#align(center)[
  #text(size: 9pt, fill: gray)[
    Generated by GLDF Viewer (gldf.icu) using Typst
  ]
]
"##,
        manufacturer = manufacturer,
        product_name = product_name,
        description = description,
        variant_count = variant_count,
        light_source_count = light_source_count,
        variant_rows = variant_rows,
        remaining = variant_count.saturating_sub(20),
        emitters_section = emitters_section,
    )
}

/// Generate Typst section for emitters with spectral diagrams
fn generate_emitters_typst_section(gldf: &GldfProduct, files: &[BufFile]) -> String {
    let emitters = match &gldf.general_definitions.emitters {
        Some(e) => &e.emitter,
        None => return String::new(),
    };

    if emitters.is_empty() {
        return String::new();
    }

    let mut sections = Vec::new();
    sections.push("\n#pagebreak()\n\n= Emitters\n".to_string());

    for emitter in emitters.iter().take(50) {
        // Take first 50 emitters max
        let emitter_id = &emitter.id;

        // Get emitter name from fixed light emitters
        let emitter_name = emitter
            .fixed_light_emitter
            .first()
            .and_then(|fle| fle.name.as_ref())
            .and_then(|n| n.locale.first())
            .map(|l| l.value.as_str())
            .unwrap_or(emitter_id);

        // Get photometry reference
        let photo_id = emitter
            .fixed_light_emitter
            .first()
            .map(|fle| fle.photometry_reference.photometry_id.as_str());

        // Get luminous flux
        let flux = emitter
            .fixed_light_emitter
            .first()
            .and_then(|fle| fle.rated_luminous_flux);

        // Get light source reference for color temperature
        let light_source_id = emitter
            .fixed_light_emitter
            .first()
            .and_then(|fle| fle.light_source_reference.fixed_light_source_id.as_ref())
            .map(|s| s.as_str());

        // Look up color temperature from light source
        let color_temp = light_source_id.and_then(|ls_id| {
            gldf.general_definitions
                .light_sources
                .as_ref()
                .and_then(|ls| {
                    ls.fixed_light_source
                        .iter()
                        .find(|fls| fls.id == ls_id)
                        .and_then(|fls| {
                            fls.color_information
                                .as_ref()
                                .and_then(|ci| ci.correlated_color_temperature)
                        })
                })
        });

        // Build emitter entry
        let mut entry = format!(
            r#"
== {}

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt,
  inset: 6pt,
  [*Emitter ID*], [{}],
"#,
            emitter_name, emitter_id
        );

        if let Some(flux_val) = flux {
            entry.push_str(&format!("  [*Luminous Flux*], [{} lm],\n", flux_val));
        }

        if let Some(temp) = color_temp {
            entry.push_str(&format!("  [*Color Temperature*], [{} K],\n", temp));
        }

        if let Some(pid) = photo_id {
            entry.push_str(&format!("  [*Photometry*], [{}],\n", pid));

            // Try to find and generate spectral SVG
            if let Some(svg) = find_and_generate_spectral_svg(gldf, pid, files) {
                entry.push_str(")\n\n");
                entry.push_str("=== Spectral Distribution\n\n");
                // Embed SVG as raw content (Typst supports SVG)
                let svg_escaped = svg
                    .replace("\\", "\\\\")
                    .replace("\"", "\\\"")
                    .replace("#", "\\#");
                entry.push_str(&format!(
                    "#image.decode(\"{}\", width: 100%)\n",
                    svg_escaped
                ));
            } else {
                entry.push_str(")\n");
            }
        } else {
            entry.push_str(")\n");
        }

        sections.push(entry);
    }

    sections.join("\n")
}

/// Find photometry file and generate spectral SVG
fn find_and_generate_spectral_svg(
    gldf: &GldfProduct,
    photometry_id: &str,
    files: &[BufFile],
) -> Option<String> {
    // Find photometry definition to get file reference
    let photometry = gldf
        .general_definitions
        .photometries
        .as_ref()?
        .photometry
        .iter()
        .find(|p| p.id == photometry_id)?;

    let file_id = photometry.photometry_file_reference.as_ref()?.file_id.as_str();

    // Find the actual file content
    let file_content = files.iter().find(|f| {
        f.file_id.as_deref() == Some(file_id)
            || f.name.as_deref().map(|n| n.contains(file_id)).unwrap_or(false)
    })?;

    let content = file_content.content.as_ref()?;
    generate_spectral_svg_for_photometry(content)
}

// Use library modules
use gldf_rs_wasm::components;
use gldf_rs_wasm::state;
use gldf_rs_wasm::utils;

use components::{BevySceneViewer, EditorTabs, EmitterConfig, L3dViewer, LdtViewer, LispViewer, StarSkyViewer, UrlFileViewer, ViewType};
use state::{use_gldf, GldfAction, GldfProvider};
use utils::{check_version_status, is_localhost};

/// Fetch WASM viewer files from URLs
/// Returns a list of (filename, content) tuples
async fn fetch_wasm_viewer_files(viewer_id: &str) -> Result<Vec<(String, Vec<u8>)>, String> {
    use gloo::net::http::Request;

    // Define which files to fetch for each viewer type
    let base_path = match viewer_id {
        "bevy" => "/bevy",
        "acadlisp" => "/acadlisp",
        "typst" => "/typst",
        "starsky" => "/starsky",
        _ => return Err(format!("Unknown viewer: {}", viewer_id)),
    };

    // First, try to fetch the manifest.json to get the list of files
    let manifest_url = format!("{}/manifest.json", base_path);
    let manifest_response = Request::get(&manifest_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch manifest: {}", e))?;

    if !manifest_response.ok() {
        return Err(format!("Manifest not found at {}", manifest_url));
    }

    let manifest_text = manifest_response.text().await
        .map_err(|e| format!("Failed to read manifest: {}", e))?;

    // Parse manifest to get file list
    let manifest: serde_json::Value = serde_json::from_str(&manifest_text)
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;

    let mut files = Vec::new();

    // Add the manifest itself
    files.push(("manifest.json".to_string(), manifest_text.into_bytes()));

    // Get files from manifest - try different formats
    let mut files_to_fetch: Vec<String> = Vec::new();

    // Format 1: "files" array
    if let Some(file_list) = manifest.get("files").and_then(|f| f.as_array()) {
        for file_entry in file_list {
            if let Some(filename) = file_entry.as_str()
                .or_else(|| file_entry.get("name").and_then(|n| n.as_str()))
            {
                files_to_fetch.push(filename.to_string());
            }
        }
    }
    // Format 2: "js" and "wasm" keys (e.g., bevy manifest)
    if let Some(js_file) = manifest.get("js").and_then(|f| f.as_str()) {
        files_to_fetch.push(js_file.to_string());
    }
    if let Some(wasm_file) = manifest.get("wasm").and_then(|f| f.as_str()) {
        files_to_fetch.push(wasm_file.to_string());
    }
    // Format 3: "js_file" and "wasm_file" keys
    if let Some(js_file) = manifest.get("js_file").and_then(|f| f.as_str()) {
        files_to_fetch.push(js_file.to_string());
    }
    if let Some(wasm_file) = manifest.get("wasm_file").and_then(|f| f.as_str()) {
        files_to_fetch.push(wasm_file.to_string());
    }

    // Fetch all files
    for filename in files_to_fetch {
        let file_url = format!("{}/{}", base_path, filename);
        gloo::console::log!("[WasmViewer] Fetching:", &file_url);

        let response = Request::get(&file_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch {}: {}", filename, e))?;

        if !response.ok() {
            gloo::console::log!("[WasmViewer] Warning: Failed to fetch", &filename);
            continue;
        }

        let content = response.binary().await
            .map_err(|e| format!("Failed to read {}: {}", filename, e))?;

        gloo::console::log!("[WasmViewer] Fetched:", &filename, content.len(), "bytes");
        files.push((filename, content));
    }

    if files.len() <= 1 {
        return Err("No files found in manifest".to_string());
    }

    Ok(files)
}

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
    /// AutoLISP code viewer (for CAD export)
    Lisp,
    /// Star Sky 2D viewer (for Astral Sky demo - Tribute to Astrophysics)
    StarSky,
}

/// Demo file options
#[derive(Clone, Copy, PartialEq, Default)]
pub enum DemoFile {
    AecGa15,
    #[default]
    SlvTria2,
    /// Astral Sky demo (star catalogue visualization) - tribute to lidrs/astronomy community
    AstralSky,
}

impl DemoFile {
    fn filename(&self) -> &'static str {
        match self {
            DemoFile::AecGa15 => "aec_ga15.gldf",
            DemoFile::SlvTria2 => "slv_tria_2.gldf",
            DemoFile::AstralSky => "astral_sky_lüdinghausen.gldf",
        }
    }
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
    ExportPdf,
    PdfExported(Result<Vec<u8>, String>),
    SetDragging(bool),
    LoadDemo(DemoFile),
    DemoLoaded(DemoFile, Result<Vec<u8>, String>),
    LoadUrl(String),
    UrlLoaded(String, Result<Vec<u8>, String>),
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
    // Sync edited product back to loaded_gldf
    SyncProduct(GldfProduct),
    // Version check
    VersionChecked(Result<(BuildVersion, VersionStatus), String>),
    // Star sky data
    LoadStarSky(String),
    StarSkyLoaded(Result<String, String>),
    // WASM viewer embedding
    EmbedWasmViewer(String), // viewer_id: bevy, acadlisp, typst, starsky
    WasmViewerFetched(String, Result<Vec<(String, Vec<u8>)>, String>), // viewer_id, files (path, content)
    RemoveWasmViewer(String), // viewer_id
}

/// Mode of the application
#[derive(Clone, PartialEq)]
enum AppMode {
    Viewer,
    Editor,
}

/// Version check result
#[derive(Clone)]
pub struct VersionInfo {
    pub local: BuildVersion,
    pub status: VersionStatus,
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
    version_info: Option<VersionInfo>,
    pdf_exporting: bool,
    /// Star sky JSON data for astral sky visualization
    star_sky_json: Option<String>,
    /// Sky location from URL parameters (city, lat, lng)
    sky_location: Option<(String, f64, f64)>,
    /// Default view type for LDT diagrams (from URL ?view= parameter)
    default_ldt_view: ViewType,
    /// Initial nav item from URL ?view= parameter (applied after file load)
    initial_nav_item: Option<NavItem>,
    /// Currently loading WASM viewer (for embedding)
    loading_wasm_viewer: Option<String>,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        // Check URL for auto-load parameter
        // ?url=/path/to/file.gldf -> load from URL
        // aec.html or ?demo=aec -> AEC GA15
        // index.html or default -> no auto-load (user clicks button)
        if let Some(url) = Self::get_url_param() {
            let link = ctx.link().clone();
            // Defer the load to after component is mounted
            gloo::timers::callback::Timeout::new(100, move || {
                link.send_message(Msg::LoadUrl(url));
            })
            .forget();
        } else if let Some(demo) = Self::get_auto_demo() {
            let link = ctx.link().clone();
            // Defer the load to after component is mounted
            gloo::timers::callback::Timeout::new(100, move || {
                link.send_message(Msg::LoadDemo(demo));
            })
            .forget();
        }

        // Check version in the background
        {
            let link = ctx.link().clone();
            spawn_local(async move {
                let result = check_version_status().await;
                link.send_message(Msg::VersionChecked(result));
            });
        }

        // Parse ?view= parameter for initial navigation and view type
        let (initial_nav_item, default_ldt_view) = Self::get_view_param();

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
            version_info: None,
            pdf_exporting: false,
            star_sky_json: None,
            sky_location: Self::get_sky_location_params(),
            default_ldt_view,
            initial_nav_item,
            loading_wasm_viewer: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Loaded(file_name, file_type, data) => {
                console::log!("Got Files:", file_type.as_str());

                let file_name_lower = file_name.to_lowercase();

                // Try to parse GLDF
                if file_name_lower.ends_with(".gldf") {
                    match WasmGldfProduct::load_gldf_from_buf_all(data.clone()) {
                        Ok(gldf) => {
                            console::log!("GLDF parsed successfully");
                            // Check if this is an astral sky GLDF (by unique_id or filename)
                            let is_astral_sky = file_name_lower.contains("astral_sky")
                                || gldf.gldf.header.unique_gldf_id.as_ref().map(|id| id.contains("astral-sky")).unwrap_or(false);
                            if is_astral_sky {
                                // Look for embedded sky_data.json in other/ folder
                                let sky_json = gldf.files.iter()
                                    .find(|f| f.path.as_ref().map(|p| p == "other/sky_data.json").unwrap_or(false))
                                    .and_then(|f| f.content.as_ref())
                                    .and_then(|content| String::from_utf8(content.clone()).ok());

                                if let Some(json) = sky_json {
                                    console::log!("Found embedded sky_data.json:", json.len(), "chars");
                                    // Save to localStorage immediately for Bevy 3D viewer
                                    save_star_sky_for_bevy(&json);
                                    self.star_sky_json = Some(json);
                                } else {
                                    // Fallback: try to load external JSON file
                                    let json_filename = file_name.replace(".gldf", ".json");
                                    console::log!("No embedded sky_data.json, trying external:", json_filename.as_str());
                                    ctx.link().send_message(Msg::LoadStarSky(json_filename));
                                }
                            } else {
                                // Clear star sky data for non-astral GLDFs
                                self.star_sky_json = None;
                            }
                            // Extract embedded WASM viewers before storing
                            extract_embedded_viewers(&gldf);
                            self.loaded_gldf = Some(gldf);
                            // Apply initial nav item from ?view= parameter
                            if let Some(nav) = self.initial_nav_item.take() {
                                console::log!("Applying initial nav item:", format!("{:?}", nav).as_str());
                                self.nav_item = nav;
                            } else {
                                // Auto-navigate to Star Sky if this is an astral sky file
                                if self.has_sky_data() && file_name_lower.contains("astral_sky") {
                                    console::log!("Auto-navigating to Star Sky view for astral sky file");
                                    self.nav_item = NavItem::StarSky;
                                } else {
                                    console::log!("No initial nav item to apply");
                                }
                            }
                        }
                        Err(e) => {
                            console::log!("GLDF parse error:", format!("{:?}", e).as_str());
                        }
                    }
                }
                // Handle ULD (DIALux) and ROLF (Relux) files - not supported yet
                else if file_name_lower.ends_with(".uld") || file_name_lower.ends_with(".rolf") {
                    console::log!("ULD/ROLF file conversion is not yet available in this version.");
                }
                // Handle LDT/IES files - convert to minimal GLDF
                else if file_name_lower.ends_with(".ldt") || file_name_lower.ends_with(".ies") {
                    console::log!("Converting LDT/IES to GLDF...");
                    match ldt_to_gldf(&data, &file_name) {
                        Ok(gldf) => {
                            console::log!("LDT/IES converted to GLDF structure");
                            extract_embedded_viewers(&gldf);
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
            Msg::ExportPdf => {
                if let Some(ref gldf) = self.loaded_gldf {
                    self.pdf_exporting = true;

                    // Generate Typst source for GLDF report with embedded files
                    let typst_source = generate_gldf_typst_report(&gldf.gldf, &gldf.files);

                    let link = ctx.link().clone();
                    spawn_local(async move {
                        // First, ensure typst-loader.js is loaded
                        let load_result = load_typst_if_needed().await;
                        if let Err(e) = load_result {
                            link.send_message(Msg::PdfExported(Err(e)));
                            return;
                        }

                        let result = match compile_typst_to_pdf_js(&typst_source).await {
                            Ok(js_val) => {
                                let array = js_sys::Uint8Array::new(&js_val);
                                Ok(array.to_vec())
                            }
                            Err(e) => {
                                let msg = e
                                    .as_string()
                                    .unwrap_or_else(|| "Unknown error".to_string());
                                Err(msg)
                            }
                        };
                        link.send_message(Msg::PdfExported(result));
                    });
                }
                true
            }
            Msg::PdfExported(result) => {
                self.pdf_exporting = false;
                match result {
                    Ok(pdf_bytes) => {
                        console::log!("PDF generated:", pdf_bytes.len(), "bytes");

                        // Create blob and trigger download
                        let uint8arr = js_sys::Uint8Array::new(
                            &unsafe { js_sys::Uint8Array::view(&pdf_bytes) }.into(),
                        );
                        let array = js_sys::Array::new();
                        array.push(&uint8arr.buffer());
                        let opts = web_sys::BlobPropertyBag::new();
                        opts.set_type("application/pdf");
                        if let Ok(blob) =
                            web_sys::Blob::new_with_u8_array_sequence_and_options(&array, &opts)
                        {
                            if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                                let window = web_sys::window().unwrap();
                                let document = window.document().unwrap();
                                if let Ok(a) = document.create_element("a") {
                                    let _ = a.set_attribute("href", &url);
                                    let _ = a.set_attribute("download", "gldf_report.pdf");
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
                        console::log!("PDF generation failed:", e.as_str());
                    }
                }
                true
            }
            Msg::SetDragging(dragging) => {
                self.is_dragging = dragging;
                true
            }
            Msg::LoadDemo(demo) => {
                let link = ctx.link().clone();
                let url = format!("/{}", demo.filename());
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

                    link.send_message(Msg::DemoLoaded(demo, data));
                });
                false
            }
            Msg::DemoLoaded(demo, result) => {
                match result {
                    Ok(data) => {
                        console::log!("Demo loaded:", demo.filename(), data.len(), "bytes");
                        let file_name = demo.filename().to_string();
                        let file_name_lower = file_name.to_lowercase();

                        if let Ok(gldf) = WasmGldfProduct::load_gldf_from_buf_all(data.clone()) {
                            // Check if this is an astral sky GLDF
                            let is_astral_sky = file_name_lower.contains("astral_sky")
                                || gldf.gldf.header.unique_gldf_id.as_ref().map(|id| id.contains("astral-sky")).unwrap_or(false);
                            if is_astral_sky {
                                // Look for embedded sky_data.json in other/ folder
                                let sky_json = gldf.files.iter()
                                    .find(|f| f.path.as_ref().map(|p| p == "other/sky_data.json").unwrap_or(false))
                                    .and_then(|f| f.content.as_ref())
                                    .and_then(|content| String::from_utf8(content.clone()).ok());

                                if let Some(json) = sky_json {
                                    console::log!("Found embedded sky_data.json in demo:", json.len(), "chars");
                                    // Save to localStorage immediately for Bevy 3D viewer
                                    save_star_sky_for_bevy(&json);
                                    self.star_sky_json = Some(json);
                                } else {
                                    // Fallback: try to load external JSON file
                                    let json_filename = file_name.replace(".gldf", ".json");
                                    console::log!("No embedded sky_data.json, trying external:", json_filename.as_str());
                                    ctx.link().send_message(Msg::LoadStarSky(json_filename));
                                }
                            }
                            // Extract embedded WASM viewers
                            extract_embedded_viewers(&gldf);
                            self.loaded_gldf = Some(gldf);

                            // Apply initial nav item from ?view= parameter
                            if let Some(nav) = self.initial_nav_item.take() {
                                console::log!("Applying initial nav item from demo load:", format!("{:?}", nav).as_str());
                                self.nav_item = nav;
                            }
                        }
                        self.files.push(FileDetails {
                            data,
                            file_type: "application/gldf".to_string(),
                            name: file_name,
                        });
                    }
                    Err(e) => {
                        console::log!("Failed to load demo:", e.as_str());
                    }
                }
                true
            }
            Msg::LoadUrl(url) => {
                let link = ctx.link().clone();
                let url_clone = url.clone();
                console::log!("Loading GLDF from URL:", &url);
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

                    link.send_message(Msg::UrlLoaded(url_clone, data));
                });
                false
            }
            Msg::UrlLoaded(url, result) => {
                // Extract filename from URL for display
                let filename = url
                    .rsplit('/')
                    .next()
                    .unwrap_or(&url)
                    .to_string();

                match result {
                    Ok(data) => {
                        console::log!("URL loaded:", &filename, data.len(), "bytes");
                        if let Ok(gldf) = WasmGldfProduct::load_gldf_from_buf_all(data.clone()) {
                            extract_embedded_viewers(&gldf);
                            self.loaded_gldf = Some(gldf);
                        }
                        self.files.push(FileDetails {
                            data,
                            file_type: "application/gldf".to_string(),
                            name: filename,
                        });
                    }
                    Err(e) => {
                        console::log!("Failed to load URL:", &url, e.as_str());
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
            Msg::SyncProduct(product) => {
                if let Some(ref mut gldf) = self.loaded_gldf {
                    gldf.gldf = product;
                }
                false // No re-render needed, just sync the data
            }
            Msg::VersionChecked(result) => {
                match result {
                    Ok((local, status)) => {
                        console::log!("Version check:", status.message().as_str());
                        self.version_info = Some(VersionInfo { local, status });
                    }
                    Err(e) => {
                        console::log!("Version check failed:", e.as_str());
                    }
                }
                true
            }
            Msg::LoadStarSky(json_url) => {
                console::log!("Loading star sky JSON from:", json_url.as_str());
                let link = ctx.link().clone();
                spawn_local(async move {
                    let result = utils::fetch_text_data(&json_url).await;
                    link.send_message(Msg::StarSkyLoaded(result));
                });
                false
            }
            Msg::StarSkyLoaded(result) => {
                match result {
                    Ok(json) => {
                        console::log!("Star sky JSON loaded:", json.len(), "chars");
                        self.star_sky_json = Some(json);
                    }
                    Err(e) => {
                        console::log!("Star sky JSON load failed:", e.as_str());
                    }
                }
                true
            }
            Msg::EmbedWasmViewer(viewer_id) => {
                console::log!("Embedding WASM viewer:", viewer_id.as_str());
                self.loading_wasm_viewer = Some(viewer_id.clone());
                let link = ctx.link().clone();
                let vid = viewer_id.clone();
                spawn_local(async move {
                    let result = fetch_wasm_viewer_files(&vid).await;
                    link.send_message(Msg::WasmViewerFetched(vid, result));
                });
                true // Show loading state
            }
            Msg::WasmViewerFetched(viewer_id, result) => {
                self.loading_wasm_viewer = None;
                match result {
                    Ok(files) => {
                        console::log!("WASM viewer files fetched:", viewer_id.as_str(), files.len(), "files");
                        if let Some(ref mut gldf) = self.loaded_gldf {
                            // Add each file to the GLDF
                            for (path, content) in files {
                                let full_path = format!("other/viewer/{}/{}", viewer_id, path);
                                console::log!("Adding file:", full_path.as_str(), content.len(), "bytes");

                                // Check if file already exists, if so update it
                                if let Some(existing) = gldf.files.iter_mut().find(|f| {
                                    f.name.as_ref().map(|n| n == &full_path).unwrap_or(false)
                                }) {
                                    existing.content = Some(content);
                                } else {
                                    // Add new file
                                    gldf.files.push(BufFile {
                                        name: Some(full_path),
                                        content: Some(content),
                                        file_id: None,
                                        path: None,
                                    });
                                }
                            }
                            console::log!("WASM viewer embedded successfully:", viewer_id.as_str());
                        }
                    }
                    Err(e) => {
                        console::log!("WASM viewer fetch failed:", e.as_str());
                    }
                }
                true
            }
            Msg::RemoveWasmViewer(viewer_id) => {
                console::log!("Removing WASM viewer:", viewer_id.as_str());
                if let Some(ref mut gldf) = self.loaded_gldf {
                    let prefix = format!("other/viewer/{}/", viewer_id);
                    gldf.files.retain(|f| {
                        !f.name.as_ref().map(|n| n.starts_with(&prefix)).unwrap_or(false)
                    });
                    console::log!("WASM viewer removed:", viewer_id.as_str());
                }
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
    /// Check URL for auto-load demo parameter
    /// Returns Some(DemoFile) if auto-load is requested
    fn get_auto_demo() -> Option<DemoFile> {
        let window = web_sys::window()?;
        let location = window.location();

        // Check pathname for special pages
        if let Ok(pathname) = location.pathname() {
            if pathname.contains("aec") {
                return Some(DemoFile::AecGa15);
            }
            if pathname.contains("lidrs") || pathname.contains("stars") || pathname.contains("astral") {
                return Some(DemoFile::AstralSky);
            }
        }

        // Check query string for ?demo=aec, ?demo=slv, ?demo=lidrs/stars
        if let Ok(search) = location.search() {
            if search.contains("demo=aec") {
                return Some(DemoFile::AecGa15);
            }
            if search.contains("demo=slv") {
                return Some(DemoFile::SlvTria2);
            }
            if search.contains("demo=lidrs") || search.contains("demo=stars") || search.contains("demo=astral") {
                return Some(DemoFile::AstralSky);
            }
        }

        None
    }

    /// Check URL for ?url= parameter to load GLDF from a URL
    /// Returns Some(url_string) if ?url= parameter is present
    /// Example: ?url=/aec_ga15_enriched.gldf or ?url=demo/my_file.gldf
    fn get_url_param() -> Option<String> {
        let window = web_sys::window()?;
        let location = window.location();

        if let Ok(search) = location.search() {
            // Parse query string to find url= parameter
            // Handle both ?url=... and &url=...
            if let Some(url_start) = search.find("url=") {
                let url_value = &search[url_start + 4..]; // Skip "url="
                // Find the end of the value (next & or end of string)
                let url_end = url_value.find('&').unwrap_or(url_value.len());
                let url = &url_value[..url_end];

                // URL decode the value (handle %20, etc.)
                if let Ok(decoded) = js_sys::decode_uri_component(url) {
                    let decoded_str: String = decoded.into();
                    if !decoded_str.is_empty() {
                        return Some(decoded_str);
                    }
                } else if !url.is_empty() {
                    // Fallback if decode fails
                    return Some(url.to_string());
                }
            }
        }

        None
    }

    /// Parse ?view= parameter to get initial nav item and view type
    /// Format: view=<nav_item>__<view_type>
    /// Examples: view=emitters__spectral, view=files, view=photometry__polar
    fn get_view_param() -> (Option<NavItem>, ViewType) {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return (None, ViewType::Polar),
        };
        let location = window.location();

        if let Ok(search) = location.search() {
            if let Some(view_start) = search.find("view=") {
                let view_value = &search[view_start + 5..]; // Skip "view="
                let view_end = view_value.find('&').unwrap_or(view_value.len());
                let view_str = &view_value[..view_end].to_lowercase();

                // Split by __ to get nav_item and optional view_type
                let parts: Vec<&str> = view_str.split("__").collect();

                // Parse nav item
                let nav_item = match parts.first().map(|s| *s) {
                    Some("overview") => Some(NavItem::Overview),
                    Some("rawdata") | Some("raw") => Some(NavItem::RawData),
                    Some("fileviewer") | Some("viewer") => Some(NavItem::FileViewer),
                    Some("header") => Some(NavItem::Header),
                    Some("electrical") => Some(NavItem::Electrical),
                    Some("applications") => Some(NavItem::Applications),
                    Some("photometry") => Some(NavItem::Photometry),
                    Some("statistics") | Some("stats") => Some(NavItem::Statistics),
                    Some("files") => Some(NavItem::Files),
                    Some("lightsources") | Some("sources") => Some(NavItem::LightSources),
                    Some("emitters") => Some(NavItem::Emitters),
                    Some("variants") => Some(NavItem::Variants),
                    Some("lisp") | Some("autolisp") | Some("cad") => Some(NavItem::Lisp),
                    Some("starsky") | Some("stars") | Some("sky") | Some("astral") => Some(NavItem::StarSky),
                    _ => None,
                };

                // Parse view type (for LDT diagrams)
                let view_type = match parts.get(1).map(|s| *s) {
                    Some("spectral") | Some("spectrum") | Some("spd") => ViewType::Spectrum,
                    Some("polar") => ViewType::Polar,
                    Some("cartesian") | Some("cart") => ViewType::Cartesian,
                    Some("heatmap") | Some("heat") => ViewType::Heatmap,
                    Some("butterfly") | Some("3d") => ViewType::Butterfly,
                    Some("bug") => ViewType::Bug,
                    Some("lcs") => ViewType::Lcs,
                    _ => ViewType::Polar,
                };

                console::log!("Parsed view param:", format!("nav={:?}, view_type={:?}", nav_item, view_type).as_str());
                return (nav_item, view_type);
            }
        }

        (None, ViewType::Polar)
    }

    /// Parse URL parameters for sky location
    /// Supports both query string (?city=...) and hash (#city=...)
    /// Format: city=CityName&lat=51.77&lng=7.44
    /// Returns (city_name, latitude, longitude) if all present
    fn get_sky_location_params() -> Option<(String, f64, f64)> {
        let window = web_sys::window()?;
        let location = window.location();

        // Try hash first (#city=...), then query string (?city=...)
        let params_str = location.hash().ok()
            .filter(|h| h.len() > 1)
            .map(|h| h[1..].to_string()) // Remove leading #
            .or_else(|| location.search().ok().map(|s| s.trim_start_matches('?').to_string()))?;

        // Parse parameters
        let mut city: Option<String> = None;
        let mut lat: Option<f64> = None;
        let mut lng: Option<f64> = None;

        for param in params_str.split('&') {
            let parts: Vec<&str> = param.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }
            let key = parts[0].to_lowercase();
            let value = parts[1];

            match key.as_str() {
                "city" | "location" | "name" => {
                    // URL decode the city name
                    city = js_sys::decode_uri_component(value).ok().map(|s| s.into());
                }
                "lat" | "latitude" => {
                    lat = value.parse().ok();
                }
                "lng" | "lon" | "longitude" => {
                    lng = value.parse().ok();
                }
                _ => {}
            }
        }

        // Return only if we have at least lat/lng (city can default)
        match (lat, lng) {
            (Some(latitude), Some(longitude)) => {
                let city_name = city.unwrap_or_else(|| format!("{:.2}°N, {:.2}°E", latitude, longitude));
                console::log!("Sky location from URL:", format!("{} ({}, {})", city_name, latitude, longitude).as_str());
                Some((city_name, latitude, longitude))
            }
            _ => None
        }
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
                        { self.view_version_info() }
                        <a href="https://github.com/holg/gldf-rs" target="_blank" class="help-link">{ "GitHub" }</a>
                    </div>
                </div>
            </div>
        }
    }

    fn view_version_info(&self) -> Html {
        match &self.version_info {
            Some(info) => {
                let version_text = format!("GLDF Viewer v{}", info.local.version);
                let (status_class, status_icon) = match &info.status {
                    VersionStatus::Current => ("version-current", ""),
                    VersionStatus::Outdated { .. } => ("version-outdated", " (update available)"),
                    VersionStatus::Newer { .. } => ("version-dev", " (dev)"),
                    VersionStatus::Unknown => ("version-unknown", " (?)"),
                };

                // Only show localhost indicator when on localhost
                let localhost_indicator = if is_localhost() {
                    html! { <span class="localhost-badge">{ "localhost" }</span> }
                } else {
                    html! {}
                };

                html! {
                    <span class={classes!("help-version", status_class)}>
                        { localhost_indicator }
                        { version_text }
                        { status_icon }
                    </span>
                }
            }
            None => {
                html! {
                    <span class="help-version version-loading">
                        { "GLDF Viewer" }
                        { " (loading...)" }
                    </span>
                }
            }
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

                // Only show Tools section when a GLDF with relevant content is loaded
                { self.render_tools_section(ctx, has_file) }

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

    /// Render the Tools section (AutoLISP, Star Sky) only when relevant content exists
    fn render_tools_section(&self, ctx: &Context<Self>, has_file: bool) -> Html {
        let show_lisp = has_file && (self.has_lisp_code() || has_embedded_viewer("acadlisp"));
        let show_starsky = has_file && (has_embedded_viewer("starsky") || self.has_sky_data());

        if !show_lisp && !show_starsky {
            return html! {};
        }

        html! {
            <div class="sidebar-section">
                <div class="sidebar-section-title">{ "Tools" }</div>
                <ul class="sidebar-nav">
                    { self.nav_item(ctx, NavItem::Lisp, "λ", "AutoLISP", None, show_lisp) }
                    { self.nav_item(ctx, NavItem::StarSky, "★", "Star Sky", None, show_starsky) }
                </ul>
            </div>
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
                    <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadDemo(DemoFile::SlvTria2))}>
                        { "Load Demo" }
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
            NavItem::Lisp => "AutoLISP",
            NavItem::StarSky => "Star Sky",
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
                            // PDF export (hidden feature, requires typst-loader.js)
                            if is_pdf_export_enabled() {
                                <button
                                    class="btn btn-primary"
                                    onclick={ctx.link().callback(|_| Msg::ExportPdf)}
                                    disabled={self.pdf_exporting}
                                    title="Export as PDF report (compiles in browser)"
                                >
                                    { if self.pdf_exporting { "Generating PDF..." } else { "Export PDF" } }
                                </button>
                            }
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
                            NavItem::Header => self.view_header_editor(ctx),
                            NavItem::Electrical => self.view_electrical_editor(ctx),
                            NavItem::Applications => self.view_applications_editor(ctx),
                            NavItem::Photometry => self.view_photometry_editor(ctx),
                            NavItem::Statistics => self.view_statistics(),
                            NavItem::Files => self.view_files_list(ctx),
                            NavItem::LightSources => self.view_light_sources(),
                            NavItem::Emitters => self.view_emitters(),
                            NavItem::Variants => self.view_variants(ctx),
                            NavItem::Lisp => self.view_lisp(),
                            NavItem::StarSky => self.view_star_sky(ctx),
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

                    // Star Sky Viewer (for astral_sky GLDFs)
                    {
                        if let Some(ref star_json) = self.star_sky_json {
                            // Use URL location if available, otherwise fall back to header manufacturer
                            let location_name = self.sky_location.as_ref()
                                .map(|(city, _, _)| city.clone())
                                .unwrap_or_else(|| header.manufacturer.clone());
                            let location_info = self.sky_location.as_ref()
                                .map(|(city, lat, lng)| format!("{} ({:.2}°N, {:.2}°E)", city, lat, lng));
                            html! {
                                <div class="star-sky-section" style="margin: 20px 0;">
                                    <h3 style="margin-bottom: 10px;">
                                        { "⭐ Star Sky View" }
                                        if let Some(ref info) = location_info {
                                            <span style="font-size: 14px; font-weight: normal; color: var(--text-secondary); margin-left: 8px;">
                                                { info }
                                            </span>
                                        }
                                    </h3>
                                    <StarSkyViewer
                                        star_json={star_json.clone()}
                                        location_name={location_name}
                                        width={800}
                                        height={500}
                                    />
                                </div>
                            }
                        } else {
                            html! {}
                        }
                    }

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

    fn view_header_editor(&self, ctx: &Context<Self>) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            if self.mode == AppMode::Editor {
                let on_change = ctx.link().callback(Msg::SyncProduct);
                html! {
                    <GldfProviderWithData gldf={gldf.gldf.clone()} on_change={on_change}>
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

    fn view_electrical_editor(&self, ctx: &Context<Self>) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            if self.mode == AppMode::Editor {
                let on_change = ctx.link().callback(Msg::SyncProduct);
                html! {
                    <GldfProviderWithData gldf={gldf.gldf.clone()} on_change={on_change}>
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

    fn view_applications_editor(&self, ctx: &Context<Self>) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            if self.mode == AppMode::Editor {
                let on_change = ctx.link().callback(Msg::SyncProduct);
                html! {
                    <GldfProviderWithData gldf={gldf.gldf.clone()} on_change={on_change}>
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

    fn view_photometry_editor(&self, ctx: &Context<Self>) -> Html {
        if let Some(ref gldf) = self.loaded_gldf {
            if self.mode == AppMode::Editor {
                let on_change = ctx.link().callback(Msg::SyncProduct);
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
                    <GldfProviderWithData gldf={gldf.gldf.clone()} on_change={on_change}>
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

                // Get spectrums if available
                let spectrums = gldf
                    .gldf
                    .general_definitions
                    .spectrums
                    .as_ref()
                    .map(|s| &s.spectrum[..])
                    .unwrap_or(&[]);

                html! {
                    <div class="card">
                        <div class="card-body">
                            <h2>{ "Photometries" }</h2>
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

                            // Spectrums section
                            if !spectrums.is_empty() {
                                <h2 style="margin-top: 2rem;">{ "Spectrums (TM-30/TM-33)" }</h2>
                                <p class="section-description">
                                    { format!("{} spectrum definitions with spectral power distribution data", spectrums.len()) }
                                </p>
                                <div class="spectrum-grid">
                                    { for spectrums.iter().map(|spectrum| {
                                        let intensity_count = spectrum.intensity.len();
                                        let file_id = &spectrum.spectrum_file_reference.file_id;
                                        let has_file_ref = !file_id.is_empty();
                                        html! {
                                            <div class="spectrum-card">
                                                <h4>{ &spectrum.id }</h4>
                                                if has_file_ref {
                                                    <p class="file-ref">{ format!("File: {}", file_id) }</p>
                                                }
                                                <div class="spectrum-info">
                                                    <span class="info-item">
                                                        { format!("{} wavelength bins", intensity_count) }
                                                    </span>
                                                    if has_file_ref {
                                                        <span class="info-item tm33-badge">{ "TM-33" }</span>
                                                    }
                                                </div>
                                                // Mini spectrum preview
                                                <div class="spectrum-preview">
                                                    <components::SpectrumViewer
                                                        spectrum={spectrum.clone()}
                                                        width={350.0}
                                                        height={200.0}
                                                    />
                                                </div>
                                            </div>
                                        }
                                    })}
                                </div>
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
        } else if fname_lower.ends_with(".ldt")
            || fname_lower.ends_with(".ies")
            || fname_lower.ends_with(".iesxml")
        {
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

            // Group files by content type
            let mut grouped: std::collections::BTreeMap<String, Vec<_>> = std::collections::BTreeMap::new();
            for f in files.iter() {
                // Extract category from content type (e.g., "ldc" from "ldc/iesxml")
                let category = f.content_type.split('/').next().unwrap_or("other").to_string();
                grouped.entry(category).or_default().push(f);
            }

            // Pretty names for categories
            let category_name = |cat: &str| -> &'static str {
                match cat {
                    "ldc" => "Photometric Data (LDC)",
                    "geo" => "Geometry Files",
                    "image" => "Images",
                    "document" => "Documents",
                    "spectrum" => "Spectrum Files",
                    "sensor" => "Sensor Data",
                    "other" => "Other Files",
                    _ => "Other Files",
                }
            };

            let category_icon = |cat: &str| -> &'static str {
                match cat {
                    "ldc" => "💡",
                    "geo" => "📐",
                    "image" => "🖼️",
                    "document" => "📄",
                    "spectrum" => "🌈",
                    "sensor" => "📡",
                    _ => "📁",
                }
            };

            html! {
                <div class="files-container">
                    // Summary bar
                    <div class="card" style="margin-bottom: 16px; padding: 12px 16px;">
                        <div style="display: flex; flex-wrap: wrap; gap: 16px; align-items: center;">
                            <span style="font-weight: 500;">{ format!("{} files total", files.len()) }</span>
                            <span style="color: var(--text-tertiary);">{ "|" }</span>
                            { for grouped.iter().map(|(cat, cat_files)| {
                                html! {
                                    <span style="display: flex; align-items: center; gap: 4px; font-size: 13px;">
                                        <span>{ category_icon(cat) }</span>
                                        <span class={classes!("content-type-badge", content_type_class(cat))}>
                                            { format!("{}: {}", category_name(cat), cat_files.len()) }
                                        </span>
                                    </span>
                                }
                            })}
                        </div>
                    </div>

                    // Grouped files
                    { for grouped.iter().map(|(cat, cat_files)| {
                        let is_expanded = cat_files.len() <= 10; // Auto-expand small groups
                        html! {
                            <details class="file-group card" style="margin-bottom: 12px;" open={is_expanded}>
                                <summary style="cursor: pointer; padding: 12px 16px; background: var(--bg-sidebar); border-radius: 8px 8px 0 0; display: flex; align-items: center; gap: 8px; user-select: none;">
                                    <span style="font-size: 16px;">{ category_icon(cat) }</span>
                                    <span style="font-weight: 500;">{ category_name(cat) }</span>
                                    <span class={classes!("content-type-badge", content_type_class(cat))} style="margin-left: auto;">
                                        { format!("{} files", cat_files.len()) }
                                    </span>
                                </summary>
                                <div style="max-height: 400px; overflow-y: auto;">
                                    <table class="data-table files-table-clickable" style="margin: 0;">
                                        <thead>
                                            <tr>
                                                <th style="width: 25%;">{ "ID" }</th>
                                                <th style="width: 40%;">{ "File Name" }</th>
                                                <th style="width: 20%;">{ "Content Type" }</th>
                                                <th style="width: 15%;">{ "Type" }</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            { for cat_files.iter().map(|f| {
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
                                                        <td class="file-id" style="font-size: 11px;">{ &f.id }</td>
                                                        <td class="file-name">{ &f.file_name }</td>
                                                        <td>
                                                            <span class={classes!("content-type-badge", content_type_class(&f.content_type))} style="font-size: 10px;">
                                                                { &f.content_type }
                                                            </span>
                                                        </td>
                                                        <td class="file-type" style="font-size: 11px;">{ &f.type_attr }</td>
                                                    </tr>
                                                }
                                            })}
                                        </tbody>
                                    </table>
                                </div>
                            </details>
                        }
                    })}

                    if self.selected_file.is_none() && files.len() > 10 {
                        <p style="text-align: center; color: var(--text-tertiary); margin-top: 16px; font-size: 13px;">
                            { "Click on a file to preview its content" }
                        </p>
                    }

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

                    // Embedded WASM Viewers section
                    <div class="card" style="margin-top: 20px;">
                        <components::WasmViewersSection
                            on_embed={ctx.link().callback(Msg::EmbedWasmViewer)}
                            on_remove={ctx.link().callback(Msg::RemoveWasmViewer)}
                            loading_viewer={self.loading_wasm_viewer.clone()}
                            has_gldf={true}
                        />
                    </div>
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
                                                            <LdtViewer ldt_data={data} width={400.0} height={300.0} default_view={self.get_default_emitter_view()} />
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

            // Check if we have star sky data (for astral sky GLDFs)
            let star_sky_json = self.star_sky_json.clone();

            // Get selected variant name for star sky viewer
            let selected_variant_name = self.selected_3d_variant.as_ref().and_then(|vid| {
                variants.iter().find(|v| &v.id == vid).and_then(|v| {
                    v.name.as_ref()
                        .and_then(|n| n.locale.first())
                        .map(|l| l.value.clone())
                })
            });

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
                                    onclick={close_viewer.clone()}
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
                    } else if let (Some(ref json), Some(ref variant_id)) = (&star_sky_json, &self.selected_3d_variant) {
                        // Star Sky Viewer (for astral sky GLDFs without L3D)
                        // The variant name is the star name to highlight
                        <div class="variant-3d-viewer" style="margin-bottom: 20px; background: var(--bg-secondary); border-radius: 8px; overflow: hidden;">
                            <div style="display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--border-color);">
                                <div style="font-weight: 500;">
                                    { "⭐ Star Sky: " }
                                    <span style="color: var(--accent-purple);">{ selected_variant_name.as_deref().unwrap_or(variant_id) }</span>
                                </div>
                                <button
                                    onclick={close_viewer}
                                    style="background: none; border: none; cursor: pointer; font-size: 18px; color: var(--text-secondary);"
                                >
                                    { "✕" }
                                </button>
                            </div>
                            <div style="padding: 0;">
                                <StarSkyViewer
                                    star_json={json.clone()}
                                    location_name={selected_variant_name.clone().unwrap_or_else(|| variant_id.clone())}
                                    highlight_star={selected_variant_name.clone()}
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

                            // Check if this is an astral sky GLDF (no L3D but has star sky data)
                            let has_star_sky = self.star_sky_json.is_some();

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
                                                    onclick={on_view_3d.clone()}
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
                                        } else if has_star_sky {
                                            // For star sky variants, show "View in Sky" button
                                            <div style="margin-bottom: 12px;">
                                                <button
                                                    onclick={on_view_3d}
                                                    class="btn-3d-scene"
                                                    style={format!(
                                                        "background: {}; color: white; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer; font-size: 12px; display: flex; align-items: center; gap: 6px;",
                                                        if is_selected { "var(--accent-purple)" } else { "var(--accent-indigo, #6366f1)" }
                                                    )}
                                                >
                                                    <span>{ "⭐" }</span>
                                                    { if is_selected { "Viewing in Sky" } else { "View in Sky" } }
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

    /// AutoLISP code viewer/editor with SVG/DXF output
    fn view_lisp(&self) -> Html {
        let lisp_code = self.get_lisp_code().unwrap_or_default();
        let has_embedded_code = !lisp_code.is_empty();
        let star_count = if has_embedded_code {
            // Count draw-star calls in the code
            lisp_code.matches("draw-star").count()
        } else {
            0
        };

        html! {
            <div class="lisp-container" style="padding: 0 20px;">
                <div class="info-callout" style="margin-bottom: 20px;">
                    <div class="info-icon">{ "λ" }</div>
                    <div class="info-content">
                        <strong>{ "AutoLISP Interpreter" }</strong>
                        <p style="margin: 4px 0 0; color: var(--text-secondary);">
                            { "Run AutoLISP code and generate SVG/DXF output for CAD integration. " }
                            { "Powered by " }
                            <a href="https://acadlisp.de" target="_blank" style="color: var(--accent-purple);">{ "acadlisp" }</a>
                            { " WASM engine." }
                        </p>
                        {
                            if has_embedded_code {
                                html! {
                                    <p style="margin: 8px 0 0; color: var(--accent-purple);">
                                        { format!("📄 Loaded star_sky.lsp from GLDF ({} stars, {} bytes)", star_count, lisp_code.len()) }
                                    </p>
                                }
                            } else {
                                html! {}
                            }
                        }
                    </div>
                </div>

                <LispViewer
                    initial_code={lisp_code}
                    title="AutoLISP Editor"
                    width={900}
                    height={700}
                />
            </div>
        }
    }

    /// Check if the current GLDF has sky_data.json embedded
    fn has_sky_data(&self) -> bool {
        if let Some(ref gldf) = self.loaded_gldf {
            return gldf.files.iter().any(|f| {
                f.path.as_ref().map(|p| p.contains("sky_data.json")).unwrap_or(false)
            });
        }
        false
    }

    /// Get the sky_data.json content from the GLDF
    fn get_sky_data_json(&self) -> Option<String> {
        if let Some(ref gldf) = self.loaded_gldf {
            return gldf.files.iter()
                .find(|f| f.path.as_ref().map(|p| p.contains("sky_data.json")).unwrap_or(false))
                .and_then(|f| f.content.clone())
                .and_then(|c| String::from_utf8(c).ok());
        }
        None
    }

    /// Check if the current GLDF has AutoLISP code embedded
    fn has_lisp_code(&self) -> bool {
        if let Some(ref gldf) = self.loaded_gldf {
            return gldf.files.iter().any(|f| {
                f.path.as_ref().map(|p| p.ends_with(".lsp") || p.contains("autolisp")).unwrap_or(false)
            });
        }
        false
    }

    /// Get the AutoLISP code from the GLDF (first .lsp file found)
    fn get_lisp_code(&self) -> Option<String> {
        if let Some(ref gldf) = self.loaded_gldf {
            return gldf.files.iter()
                .find(|f| f.path.as_ref().map(|p| p.ends_with(".lsp")).unwrap_or(false))
                .and_then(|f| f.content.clone())
                .and_then(|c| String::from_utf8(c).ok());
        }
        None
    }

    /// Get a custom property from the GLDF's ProductMetaData
    fn get_custom_property(&self, property_id: &str) -> Option<String> {
        self.loaded_gldf.as_ref().and_then(|gldf| {
            gldf.gldf
                .product_definitions
                .product_meta_data
                .as_ref()
                .and_then(|m| m.descriptive_attributes.as_ref())
                .and_then(|d| d.custom_properties.as_ref())
                .and_then(|cp| {
                    cp.property.iter()
                        .find(|p| p.id == property_id)
                        .map(|p| p.value.clone())
                        .filter(|v| !v.is_empty())
                })
        })
    }

    /// Get the default emitter view type from GLDF custom properties
    /// Priority: GLDF custom property > URL parameter > default (Polar)
    fn get_default_emitter_view(&self) -> ViewType {
        // First check GLDF custom property
        if let Some(v) = self.get_custom_property("default_emitter_view") {
            return match v.to_lowercase().as_str() {
                "spectral" | "spectrum" | "spd" => ViewType::Spectrum,
                "polar" => ViewType::Polar,
                "cartesian" | "cart" => ViewType::Cartesian,
                "heatmap" | "heat" => ViewType::Heatmap,
                "butterfly" | "3d" => ViewType::Butterfly,
                "bug" => ViewType::Bug,
                "lcs" => ViewType::Lcs,
                _ => ViewType::Polar,
            };
        }
        // Fall back to URL parameter or default
        self.default_ldt_view
    }

    /// View Star Sky - 2D polar projection of visible stars
    /// Tribute to Astrophysics!
    fn view_star_sky(&self, _ctx: &Context<Self>) -> Html {
        let has_embedded = has_embedded_viewer("starsky");
        let sky_data = self.get_sky_data_json();

        // Parse star count from sky data
        let star_info = sky_data.as_ref().and_then(|json| {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|data| {
                let location = data.get("location")
                    .and_then(|l| l.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("Unknown");
                let star_count = data.get("stars")
                    .and_then(|s| s.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                (location.to_string(), star_count)
            })
        });

        html! {
            <div class="star-sky-container" style="padding: 0 20px;">
                <div class="info-callout" style="margin-bottom: 20px; background: linear-gradient(135deg, #0a0a1a 0%, #1a1a3a 100%); border: 1px solid #4da6ff;">
                    <div class="info-icon" style="font-size: 32px;">{ "★" }</div>
                    <div class="info-content">
                        <strong style="color: #ffd700;">{ "Star Sky Viewer" }</strong>
                        <p style="margin: 4px 0 0; color: #ccc;">
                            { "2D polar projection of the night sky. " }
                            if let Some((location, count)) = &star_info {
                                { format!("{} visible stars from {}. ", count, location) }
                            }
                            <br />
                            <em style="color: #888;">{ "Tribute to Astrophysics!" }</em>
                        </p>
                    </div>
                </div>

                if has_embedded && sky_data.is_some() {
                    <div class="star-sky-viewer" style="position: relative;">
                        <canvas
                            id="star-sky-canvas"
                            style="width: 100%; height: 600px; background: #0a0a1a; border-radius: 8px;"
                        />
                        <script>
                            { format!(r#"
                                (async function() {{
                                    if (window.hasEmbeddedViewer && window.hasEmbeddedViewer('starsky')) {{
                                        const skyData = {};
                                        await window.loadEmbeddedStarSky('star-sky-canvas', JSON.stringify(skyData));
                                    }}
                                }})();
                            "#, sky_data.as_ref().unwrap()) }
                        </script>
                    </div>
                } else if sky_data.is_some() {
                    // Fallback: use the existing astral_sky.html approach
                    <div class="star-sky-fallback" style="text-align: center; padding: 40px;">
                        <p style="color: var(--text-secondary);">
                            { "Star data is embedded in this GLDF. " }
                            <a href="/astral_sky.html" target="_blank" style="color: var(--accent-blue);">
                                { "Open in Astral Sky Demo" }
                            </a>
                        </p>
                    </div>
                } else {
                    <div class="star-sky-empty" style="text-align: center; padding: 40px;">
                        <p style="color: var(--text-secondary);">
                            { "No star data found in this GLDF file." }
                            <br />
                            { "Load an Astral Sky GLDF to visualize the night sky." }
                        </p>
                    </div>
                }

                // Info panel with star list
                if let Some(json) = &sky_data {
                    <div class="star-list-panel" style="margin-top: 20px;">
                        <details>
                            <summary style="cursor: pointer; color: var(--accent-blue);">
                                { "Show star data (JSON)" }
                            </summary>
                            <pre style="max-height: 400px; overflow: auto; background: #1a1a2e; padding: 12px; border-radius: 4px; font-size: 11px;">
                                { json.chars().take(5000).collect::<String>() }
                                if json.len() > 5000 {
                                    { format!("\n... ({} more characters)", json.len() - 5000) }
                                }
                            </pre>
                        </details>
                    </div>
                }
            </div>
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
    #[prop_or_default]
    pub on_change: Option<Callback<GldfProduct>>,
}

#[function_component(GldfProviderWithData)]
pub fn gldf_provider_with_data(props: &GldfProviderWithDataProps) -> Html {
    html! {
        <GldfProvider>
            <GldfInitializer gldf={props.gldf.clone()} on_change={props.on_change.clone()}>
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
    #[prop_or_default]
    pub on_change: Option<Callback<GldfProduct>>,
}

#[function_component(GldfInitializer)]
pub fn gldf_initializer(props: &GldfInitializerProps) -> Html {
    let state = use_gldf();
    let initialized = use_state(|| false);

    // Initialize state on first render
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

    // Sync changes back to parent when state is modified
    {
        let on_change = props.on_change.clone();
        let product = state.product.clone();
        let is_modified = state.is_modified;
        use_effect_with((product.clone(), is_modified), move |(product, is_modified)| {
            if *is_modified {
                if let Some(ref callback) = on_change {
                    callback.emit(product.clone());
                }
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
