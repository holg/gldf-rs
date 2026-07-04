//! IFC Viewer — exports the current GLDF to IFC, then re-imports the
//! resulting STEP text back via `gldf_rs::ifc::import_ifc`, and shows
//! the parsed `ImportedLuminaire` tree alongside the raw STEP. The
//! round-trip IS the test of our IFC importer: every entity that
//! survives the export→import path proves we can read what we write.
//!
//! Sources of the IFC text:
//!
//! * **Default**: `GldfToIfc::export(&product)` for whatever GLDF the
//!   editor currently holds. Refreshes on every state mutation.
//! * **Drop / file picker** (toolbar at the top of the page): replaces
//!   the source with external `.ifc` bytes so users can inspect any
//!   IFC file's structure, not just our own exports. Helpful for
//!   diagnosing what other tools emit.
//!
//! Layout: left pane is the raw STEP, right pane is the parsed tree
//! (Manufacturer → Variants → LightSources → Properties), with a
//! header strip showing manufacturer / variant count / total light
//! sources at a glance.

use crate::state::use_gldf;
use gldf_rs::ifc::{import_ifc, GldfToIfc, ImportedLuminaire};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

#[derive(Clone)]
struct IfcSource {
    /// Where the IFC came from — labelled for the user. "Exported
    /// from current GLDF" by default, filename for picked files.
    source: String,
    /// Raw STEP text. Long files (~50 KB+) get truncated for display
    /// in the left pane; the parser sees the full content.
    step_text: String,
    /// Parsed tree from `import_ifc`. `Err(msg)` when the parse
    /// failed — the left pane still shows the text so the user can
    /// see what tripped the parser.
    parsed: Result<ImportedLuminaire, String>,
}

#[component]
pub fn IfcViewerPage() -> impl IntoView {
    let state = use_gldf();
    let external: RwSignal<Option<IfcSource>> = RwSignal::new(None);

    // Default source: re-derived on every state change so the round-
    // trip view stays in lock-step with editor mutations. `Signal::derive`
    // (not `Memo`) — `IfcSource` doesn't implement PartialEq, and the
    // value would change on every export anyway because STEP timestamps
    // differ. Wasted memoization either way.
    let default_source = Signal::derive(move || {
        state.with(|s| match GldfToIfc::export(&s.product) {
            Ok(text) => {
                let parsed = import_ifc(&text).map_err(|e| e.to_string());
                IfcSource {
                    source: "Exported from current GLDF".to_string(),
                    step_text: text,
                    parsed,
                }
            }
            Err(e) => IfcSource {
                source: "Exported from current GLDF (export failed)".to_string(),
                step_text: format!("// IFC export error: {e}"),
                parsed: Err(format!("export: {e}")),
            },
        })
    });

    let active_source = Signal::derive(move || {
        external.get().unwrap_or_else(|| default_source.get())
    });

    let on_file = move |e: crate::components::edit_util::EditEvent| {
        let Some(target) = e.target() else { return };
        let input: HtmlInputElement = match target.dyn_into() {
            Ok(i) => i,
            Err(_) => return,
        };
        let Some(files) = input.files() else { return };
        let Some(file) = files.get(0) else { return };
        let filename = file.name();
        let gloo_file = gloo::file::File::from(file);
        spawn_local(async move {
            let bytes = match gloo::file::futures::read_as_bytes(&gloo_file).await {
                Ok(b) => b,
                Err(err) => {
                    log::error!("read IFC: {err}");
                    return;
                }
            };
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let parsed = import_ifc(&text).map_err(|e| e.to_string());
            external.set(Some(IfcSource {
                source: format!("Dropped: {filename}"),
                step_text: text,
                parsed,
            }));
        });
        input.set_value("");
    };

    let on_revert = move |_| external.set(None);

    view! {
        <div class="editor-section ifc-viewer">
            <h2>{ "IFC Viewer" }</h2>
            <p class="section-description">
                { "Exports the current GLDF to IFC STEP and re-imports the result — the round-trip is the test of our IFC reader. Drop a `.ifc` file to inspect any external IFC instead." }
            </p>

            <div style="display: flex; gap: 8px; align-items: center; margin-bottom: 10px; flex-wrap: wrap;">
                <label class="btn btn-secondary" style="cursor: pointer;">
                    { "📂 Open .ifc" }
                    <input type="file" accept=".ifc,application/x-step" on:change=on_file style="display: none;" />
                </label>
                {move || external.with(|e| e.is_some()).then(|| view! {
                    <button class="btn btn-secondary" on:click=on_revert>
                        { "↺ Back to export of current GLDF" }
                    </button>
                })}
                <span style="font-size: 12px; color: var(--text-tertiary);">
                    { move || active_source.with(|s| s.source.clone()) }
                </span>
            </div>

            <IfcHeaderStrip src=active_source />

            <div class="ifc-panes" style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
                <IfcStepPane src=active_source />
                <IfcTreePane src=active_source />
            </div>
        </div>
    }
}

#[component]
fn IfcHeaderStrip(src: Signal<IfcSource>) -> impl IntoView {
    view! {
        {move || src.with(|s| {
            let manufacturer = s.parsed.as_ref().ok().and_then(|p| p.manufacturer.clone())
                .unwrap_or_else(|| "—".to_string());
            let name = s.parsed.as_ref().ok().map(|p| p.name.clone())
                .unwrap_or_else(|| "—".to_string());
            let variants = s.parsed.as_ref().ok().map(|p| p.variants.len()).unwrap_or(0);
            let total_sources: usize = s.parsed.as_ref().ok()
                .map(|p| p.variants.iter().map(|v| v.light_sources.len()).sum())
                .unwrap_or(0);
            let step_bytes = s.step_text.len();
            let badge_style = "padding: 4px 10px; background: var(--bg-secondary); border-radius: 4px; font-size: 12px;";
            view! {
                <div style="display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 10px; align-items: center;">
                    <span style=badge_style><strong>{ "Name: " }</strong>{ name }</span>
                    <span style=badge_style><strong>{ "Manufacturer: " }</strong>{ manufacturer }</span>
                    <span style=badge_style><strong>{ "Variants: " }</strong>{ variants }</span>
                    <span style=badge_style><strong>{ "Light sources: " }</strong>{ total_sources }</span>
                    <span style=badge_style><strong>{ "STEP size: " }</strong>{ format!("{} bytes", step_bytes) }</span>
                </div>
            }
        })}
    }
}

#[component]
fn IfcStepPane(src: Signal<IfcSource>) -> impl IntoView {
    // Large STEP files (Bevy mesh export can produce 100+ KB) would
    // freeze the browser if we dumped them into a <pre>. Truncate at
    // 30 KB for display; the parser still sees the full content.
    const TRUNCATE_AT: usize = 30_000;
    view! {
        <div class="ifc-step-pane" style="border: 1px solid var(--border-color); border-radius: 6px; padding: 10px;">
            <h3 style="margin: 0 0 6px; font-size: 13px;">{ "Raw STEP (IFC4)" }</h3>
            {move || src.with(|s| {
                let total = s.step_text.len();
                let (body, truncated) = if total > TRUNCATE_AT {
                    (s.step_text[..TRUNCATE_AT].to_string(), Some(total - TRUNCATE_AT))
                } else {
                    (s.step_text.clone(), None)
                };
                let download_text = s.step_text.clone();
                let on_download = move |_| {
                    download_ifc_text(&download_text);
                };
                view! {
                    <div style="display: flex; gap: 6px; margin-bottom: 4px;">
                        <button class="btn btn-secondary" style="font-size: 11px; padding: 2px 8px;" on:click=on_download>
                            { "⬇ Download .ifc" }
                        </button>
                    </div>
                    <pre style="font-family: var(--font-mono); font-size: 11px; line-height: 1.35; background: var(--bg-secondary); padding: 8px; border-radius: 4px; max-height: 480px; overflow: auto; white-space: pre-wrap;">
                        { body }
                        {truncated.map(|n| view! {
                            <div style="color: var(--text-tertiary); font-style: italic; margin-top: 6px;">
                                { format!("…{n} bytes truncated. Click ⬇ Download to get the full file.") }
                            </div>
                        })}
                    </pre>
                }
            })}
        </div>
    }
}

#[component]
fn IfcTreePane(src: Signal<IfcSource>) -> impl IntoView {
    view! {
        <div class="ifc-tree-pane" style="border: 1px solid var(--border-color); border-radius: 6px; padding: 10px;">
            <h3 style="margin: 0 0 6px; font-size: 13px;">{ "Parsed (round-trip via import_ifc)" }</h3>
            {move || src.with(|s| match &s.parsed {
                Err(err) => view! {
                    <div style="padding: 10px; background: #2a1818; color: #ff8888; border-radius: 4px; font-family: var(--font-mono); font-size: 11px;">
                        { format!("Parse error: {err}") }
                    </div>
                }.into_any(),
                Ok(luminaire) => view! { <LuminaireTree lum=luminaire.clone() /> }.into_any(),
            })}
        </div>
    }
}

#[component]
fn LuminaireTree(lum: ImportedLuminaire) -> impl IntoView {
    let descr = lum.descriptive_attributes.clone();
    let variants = lum.variants.clone();
    view! {
        <div style="font-family: var(--font-mono); font-size: 11px; max-height: 480px; overflow: auto; padding: 6px;">
            <div style="margin-bottom: 6px;">
                <strong>{ "Name: " }</strong>{ lum.name.clone() }
                {lum.description.clone().map(|d| view! {
                    <div style="color: var(--text-secondary); margin-left: 6px;">{ d }</div>
                })}
            </div>
            {lum.model_reference.clone().map(|m| view! {
                <div><strong>{ "Model: " }</strong>{ m }</div>
            })}
            {descr.electrical_safety_class.clone().map(|v| view! {
                <div><strong>{ "Safety class: " }</strong>{ v }</div>
            })}
            {descr.ip_code.clone().map(|v| view! {
                <div><strong>{ "IP code: " }</strong>{ v }</div>
            })}
            {descr.median_useful_life.clone().map(|v| view! {
                <div><strong>{ "Median useful life: " }</strong>{ v }</div>
            })}
            <div style="margin-top: 10px;">
                <strong>{ format!("Variants ({}):", variants.len()) }</strong>
            </div>
            {variants.into_iter().enumerate().map(|(idx, v)| view! {
                <VariantNode idx=idx variant=v />
            }).collect::<Vec<_>>()}
            <EmbeddedFilesNode files=lum.embedded_files.clone() />
        </div>
    }
}

#[component]
fn VariantNode(idx: usize, variant: gldf_rs::ifc::ImportedVariant) -> impl IntoView {
    let ls = variant.light_sources.clone();
    let props = variant.properties.clone();
    view! {
        <details open=true style="margin: 6px 0 6px 10px; padding-left: 6px; border-left: 2px solid var(--accent-blue);">
            <summary style="cursor: pointer;">
                <strong>{ format!("Variant {idx}: ") }</strong>
                { variant.name.clone() }
                {variant.fixture_type.clone().map(|t| view! {
                    <span style="color: var(--text-tertiary); margin-left: 6px;">{ format!("[{}]", t) }</span>
                })}
            </summary>
            {variant.description.clone().map(|d| view! {
                <div style="color: var(--text-secondary); margin-left: 8px;">{ d }</div>
            })}
            {variant.gldf_variant_id.clone().map(|id| view! {
                <div style="margin-left: 8px;"><strong>{ "GLDF id: " }</strong><code>{ id }</code></div>
            })}
            <div style="margin: 4px 0 4px 8px;">
                <strong>{ format!("Light sources ({}):", ls.len()) }</strong>
            </div>
            {ls.into_iter().enumerate().map(|(li, l)| view! {
                <LightSourceNode idx=li ls=l />
            }).collect::<Vec<_>>()}
            <PropertiesNode props=props />
            <GeometryNode geom=variant.geometry.clone() />
        </details>
    }
}

#[component]
fn LightSourceNode(idx: usize, ls: gldf_rs::ifc::ImportedLightSource) -> impl IntoView {
    view! {
        <div style="margin: 2px 0 2px 16px; padding: 4px 6px; background: var(--bg-secondary); border-radius: 4px;">
            <div>{ format!("[{idx}] ") }<strong>{ ls.name.clone() }</strong></div>
            {ls.color_temperature.map(|v| view! { <div>{ format!("  CCT: {v} K") }</div> })}
            {ls.luminous_flux.map(|v| view! { <div>{ format!("  Flux: {v} lm") }</div> })}
            {ls.emission_source.clone().map(|v| view! { <div>{ format!("  Emission: {v}") }</div> })}
            {ls.photometry_filename.clone().map(|v| view! {
                <div style="color: var(--text-tertiary);">{ format!("  Photometry file: {v}") }</div>
            })}
        </div>
    }
}

#[component]
fn PropertiesNode(props: gldf_rs::ifc::ImportedProperties) -> impl IntoView {
    // ImportedProperties carries many Option<>s; show only the populated ones.
    let rows: Vec<(&'static str, String)> = [
        ("Sources",        props.number_of_sources.map(|v| v.to_string())),
        ("Total wattage",  props.total_wattage.map(|v| format!("{v} W"))),
        ("Mounting",       props.mounting_type.clone()),
        ("CCT",            props.color_temperature.map(|v| format!("{v} K"))),
    ]
    .into_iter()
    .filter_map(|(k, v)| v.map(|s| (k, s)))
    .collect();
    if rows.is_empty() {
        return ().into_any();
    }
    view! {
        <div style="margin: 4px 0 4px 8px;">
            <strong>{ "Properties:" }</strong>
            {rows.into_iter().map(|(k, v)| view! {
                <div style="margin-left: 16px;">{ format!("{k}: {v}") }</div>
            }).collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}

#[component]
fn GeometryNode(geom: Option<gldf_rs::ifc::ImportedGeometry>) -> impl IntoView {
    let Some(g) = geom else { return ().into_any() };
    view! {
        <div style="margin: 4px 0 4px 8px; color: var(--text-tertiary);">
            { format!("Geometry: {} vertices, {} triangles", g.vertices.len(), g.triangles.len()) }
        </div>
    }
    .into_any()
}

#[component]
fn EmbeddedFilesNode(files: Vec<gldf_rs::ifc::EmbeddedFile>) -> impl IntoView {
    if files.is_empty() {
        return ().into_any();
    }
    view! {
        <details style="margin-top: 8px;">
            <summary style="cursor: pointer; color: var(--text-secondary);">
                { format!("Embedded files ({})", files.len()) }
            </summary>
            <ul style="margin: 4px 0 4px 16px; padding-left: 18px;">
                {files.into_iter().map(|f| view! {
                    <li>
                        { format!("{} — {} ({} bytes, {})", f.filename, f.file_type, f.content.len(), f.content_type) }
                    </li>
                }).collect::<Vec<_>>()}
            </ul>
        </details>
    }
    .into_any()
}

/// Trigger an `<a download>` for the full IFC string. Mirrors the
/// existing helper in `exports.rs` but stays inline here since it's
/// only used by this page.
fn download_ifc_text(text: &str) {
    let uint8 = js_sys::Uint8Array::new(
        &unsafe { js_sys::Uint8Array::view(text.as_bytes()) }.into(),
    );
    let array = js_sys::Array::new();
    array.push(&uint8.buffer());
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("application/x-step");
    let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&array, &opts) else {
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Ok(a) = document.create_element("a") {
                let _ = a.set_attribute("href", &url);
                let _ = a.set_attribute("download", "exported.ifc");
                let _ = a.set_attribute("style", "display: none");
                if let Some(body) = document.body() {
                    let _ = body.append_child(&a);
                    if let Some(html_a) = a.dyn_ref::<web_sys::HtmlElement>() {
                        html_a.click();
                    }
                    let _ = body.remove_child(&a);
                }
                let _ = web_sys::Url::revoke_object_url(&url);
            }
        }
    }
}
