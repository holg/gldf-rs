//! LISP Viewer Component
//!
//! Provides an AutoLISP code editor and SVG/DXF output viewer.
//! Uses the acadlisp WASM module loaded from acadlisp.de

use gloo::console::log;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew::virtual_dom::VNode;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadAcadLisp)]
    fn load_acad_lisp() -> js_sys::Promise;

    #[wasm_bindgen(js_name = isAcadLispLoaded)]
    fn is_acad_lisp_loaded() -> bool;

    #[wasm_bindgen(js_name = evalLisp)]
    fn eval_lisp(code: &str) -> String;

    #[wasm_bindgen(js_name = getLispSvg)]
    fn get_lisp_svg() -> String;

    #[wasm_bindgen(js_name = getLispDxf)]
    fn get_lisp_dxf() -> String;

    #[wasm_bindgen(js_name = getLispEntitiesJson)]
    fn get_lisp_entities_json() -> String;

    #[wasm_bindgen(js_name = getLispOutput)]
    fn get_lisp_output() -> String;

    #[wasm_bindgen(js_name = getLispEntityCount)]
    fn get_lisp_entity_count() -> usize;

    #[wasm_bindgen(js_name = clearLispDrawing)]
    fn clear_lisp_drawing();

    #[wasm_bindgen(js_name = getLispExample)]
    fn get_lisp_example(name: &str) -> String;

    #[wasm_bindgen(js_name = getLispExampleNames)]
    fn get_lisp_example_names() -> String;
}

#[derive(Clone, PartialEq)]
enum LoadState {
    NotLoaded,
    Loading,
    Loaded,
    Error(String),
}

#[derive(Clone, PartialEq)]
enum OutputMode {
    Svg,
    Dxf,
    Json,
}

#[derive(Properties, PartialEq)]
pub struct LispViewerProps {
    /// Initial LISP code to display
    #[prop_or_default]
    pub initial_code: String,
    /// Title for the viewer
    #[prop_or("AutoLISP Viewer".to_string())]
    pub title: String,
    #[prop_or(800)]
    pub width: u32,
    #[prop_or(600)]
    pub height: u32,
}

#[function_component(LispViewer)]
pub fn lisp_viewer(props: &LispViewerProps) -> Html {
    let load_state = use_state(|| {
        if is_acad_lisp_loaded() {
            LoadState::Loaded
        } else {
            LoadState::NotLoaded
        }
    });

    let code = use_state(|| props.initial_code.clone());

    let result = use_state(String::new);
    let output = use_state(String::new);
    let svg_content = use_state(String::new);
    let output_mode = use_state(|| OutputMode::Svg);
    let entity_count = use_state(|| 0_usize);

    // Load LISP engine
    let start_loading = {
        let load_state = load_state.clone();
        Callback::from(move |_| {
            if *load_state != LoadState::NotLoaded {
                return;
            }

            load_state.set(LoadState::Loading);

            let load_state = load_state.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let promise = load_acad_lisp();
                let result = wasm_bindgen_futures::JsFuture::from(promise).await;

                match result {
                    Ok(_) => {
                        if is_acad_lisp_loaded() {
                            load_state.set(LoadState::Loaded);
                        } else {
                            load_state.set(LoadState::Error("Failed to initialize".to_string()));
                        }
                    }
                    Err(e) => {
                        let msg = format!("{:?}", e);
                        log!(format!("[LISP] Load error: {}", msg));
                        load_state.set(LoadState::Error(msg));
                    }
                }
            });
        })
    };

    // Run LISP code
    let run_code = {
        let code = code.clone();
        let result = result.clone();
        let output = output.clone();
        let svg_content = svg_content.clone();
        let entity_count = entity_count.clone();
        Callback::from(move |_| {
            let code_str = (*code).clone();
            log!(format!("[LISP] Running code: {} chars", code_str.len()));

            let eval_result = eval_lisp(&code_str);
            result.set(eval_result);

            output.set(get_lisp_output());
            svg_content.set(get_lisp_svg());
            entity_count.set(get_lisp_entity_count());
        })
    };

    // Clear drawing
    let clear = {
        let result = result.clone();
        let output = output.clone();
        let svg_content = svg_content.clone();
        let entity_count = entity_count.clone();
        Callback::from(move |_| {
            clear_lisp_drawing();
            result.set(String::new());
            output.set(String::new());
            svg_content.set(String::new());
            entity_count.set(0);
        })
    };

    // Update code from textarea
    let on_code_change = {
        let code = code.clone();
        Callback::from(move |e: InputEvent| {
            let target: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            code.set(target.value());
        })
    };

    // Output mode buttons
    let set_svg_mode = {
        let output_mode = output_mode.clone();
        Callback::from(move |_| output_mode.set(OutputMode::Svg))
    };
    let set_dxf_mode = {
        let output_mode = output_mode.clone();
        Callback::from(move |_| output_mode.set(OutputMode::Dxf))
    };
    let set_json_mode = {
        let output_mode = output_mode.clone();
        Callback::from(move |_| output_mode.set(OutputMode::Json))
    };

    // Download DXF
    let download_dxf = Callback::from(move |_| {
        let dxf = get_lisp_dxf();
        download_file("drawing.dxf", &dxf, "application/dxf");
    });

    html! {
        <div class="lisp-viewer" style={format!("width: {}px;", props.width)}>
            <div class="lisp-header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;">
                <h3 style="margin: 0; display: flex; align-items: center; gap: 8px;">
                    <span>{ "λ" }</span>
                    { &props.title }
                </h3>
                <div class="lisp-controls" style="display: flex; gap: 8px;">
                    {
                        match &*load_state {
                            LoadState::NotLoaded => html! {
                                <button
                                    onclick={start_loading}
                                    class="btn btn-primary"
                                    style="background: var(--accent-purple); border: none; padding: 6px 12px; border-radius: 4px; color: white; cursor: pointer;"
                                >
                                    { "Load LISP Engine" }
                                </button>
                            },
                            LoadState::Loading => html! {
                                <span style="color: var(--accent-yellow);">{ "Loading acadlisp..." }</span>
                            },
                            LoadState::Loaded => html! {
                                <>
                                    <button
                                        onclick={run_code}
                                        style="background: var(--accent-green); border: none; padding: 6px 12px; border-radius: 4px; color: white; cursor: pointer;"
                                    >
                                        { "▶ Run" }
                                    </button>
                                    <button
                                        onclick={clear}
                                        style="background: var(--accent-orange); border: none; padding: 6px 12px; border-radius: 4px; color: white; cursor: pointer;"
                                    >
                                        { "Clear" }
                                    </button>
                                    <button
                                        onclick={download_dxf}
                                        style="background: var(--accent-blue); border: none; padding: 6px 12px; border-radius: 4px; color: white; cursor: pointer;"
                                    >
                                        { "⬇ DXF" }
                                    </button>
                                </>
                            },
                            LoadState::Error(msg) => html! {
                                <span style="color: var(--accent-red);">{ format!("Error: {}", msg) }</span>
                            },
                        }
                    }
                </div>
            </div>

            <div class="lisp-content" style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
                // Code editor
                <div class="lisp-editor" style="display: flex; flex-direction: column;">
                    <div style="font-size: 11px; color: var(--text-secondary); margin-bottom: 4px;">{ "LISP Code" }</div>
                    <textarea
                        value={(*code).clone()}
                        oninput={on_code_change}
                        style="flex: 1; min-height: 300px; background: #1a1a2e; color: #00ff88; border: 1px solid var(--border-color); border-radius: 4px; padding: 12px; font-family: 'Courier New', monospace; font-size: 12px; resize: vertical;"
                        spellcheck="false"
                    />
                </div>

                // Output panel
                <div class="lisp-output" style="display: flex; flex-direction: column;">
                    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px;">
                        <div style="font-size: 11px; color: var(--text-secondary);">
                            { format!("Output ({} entities)", *entity_count) }
                        </div>
                        <div style="display: flex; gap: 4px;">
                            <button
                                onclick={set_svg_mode}
                                style={format!(
                                    "background: {}; border: none; padding: 2px 8px; border-radius: 3px; color: white; cursor: pointer; font-size: 10px;",
                                    if *output_mode == OutputMode::Svg { "var(--accent-blue)" } else { "var(--bg-tertiary)" }
                                )}
                            >
                                { "SVG" }
                            </button>
                            <button
                                onclick={set_dxf_mode}
                                style={format!(
                                    "background: {}; border: none; padding: 2px 8px; border-radius: 3px; color: white; cursor: pointer; font-size: 10px;",
                                    if *output_mode == OutputMode::Dxf { "var(--accent-blue)" } else { "var(--bg-tertiary)" }
                                )}
                            >
                                { "DXF" }
                            </button>
                            <button
                                onclick={set_json_mode}
                                style={format!(
                                    "background: {}; border: none; padding: 2px 8px; border-radius: 3px; color: white; cursor: pointer; font-size: 10px;",
                                    if *output_mode == OutputMode::Json { "var(--accent-blue)" } else { "var(--bg-tertiary)" }
                                )}
                            >
                                { "JSON" }
                            </button>
                        </div>
                    </div>
                    <div style="flex: 1; min-height: 300px; height: 400px; background: #1a1a2e; border: 1px solid var(--border-color); border-radius: 4px; overflow: auto;">
                        {
                            match &*output_mode {
                                OutputMode::Svg => {
                                    if svg_content.is_empty() {
                                        html! {
                                            <div style="display: flex; align-items: center; justify-content: center; height: 100%; color: var(--text-tertiary);">
                                                { "Run LISP code to see SVG output" }
                                            </div>
                                        }
                                    } else {
                                        // Strip XML declaration if present (not valid in HTML context)
                                        let svg_html = if svg_content.starts_with("<?xml") {
                                            svg_content.split_once("?>")
                                                .map(|(_, rest)| rest.trim())
                                                .unwrap_or(&svg_content)
                                                .to_string()
                                        } else {
                                            (*svg_content).clone()
                                        };
                                        // Wrap in a div with proper styling
                                        let wrapper_html = format!(
                                            r#"<div style="width: 100%; height: 100%; min-height: 380px;">{}</div>"#,
                                            svg_html
                                        );
                                        // Use Html::from_html_unchecked to render raw SVG
                                        VNode::from_html_unchecked(wrapper_html.into())
                                    }
                                },
                                OutputMode::Dxf => {
                                    let dxf = get_lisp_dxf();
                                    html! {
                                        <pre style="padding: 12px; color: #88ff88; font-size: 11px; margin: 0; white-space: pre-wrap;">
                                            { dxf }
                                        </pre>
                                    }
                                },
                                OutputMode::Json => {
                                    let json = get_lisp_entities_json();
                                    html! {
                                        <pre style="padding: 12px; color: #88ff88; font-size: 11px; margin: 0; white-space: pre-wrap;">
                                            { json }
                                        </pre>
                                    }
                                },
                            }
                        }
                    </div>
                </div>
            </div>

            // Console output
            if !output.is_empty() || !result.is_empty() {
                <div class="lisp-console" style="margin-top: 12px;">
                    <div style="font-size: 11px; color: var(--text-secondary); margin-bottom: 4px;">{ "Console" }</div>
                    <pre style="background: #0a0a1a; border: 1px solid var(--border-color); border-radius: 4px; padding: 12px; color: #00ff88; font-size: 11px; margin: 0; max-height: 150px; overflow: auto;">
                        { (*output).clone() }
                        { if !result.is_empty() { format!("\n> {}", *result) } else { String::new() } }
                    </pre>
                </div>
            }
        </div>
    }
}

/// Download a file with given content
fn download_file(filename: &str, content: &str, mime_type: &str) {
    use wasm_bindgen::JsCast;

    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };

    // Create blob
    let uint8arr = js_sys::Uint8Array::from(content.as_bytes());
    let array = js_sys::Array::new();
    array.push(&uint8arr.buffer());
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime_type);
    let blob = match web_sys::Blob::new_with_u8_array_sequence_and_options(&array, &opts) {
        Ok(b) => b,
        Err(_) => return,
    };

    // Create download link
    let url = match web_sys::Url::create_object_url_with_blob(&blob) {
        Ok(u) => u,
        Err(_) => return,
    };

    let link = match document.create_element("a") {
        Ok(e) => e.dyn_into::<web_sys::HtmlAnchorElement>().unwrap(),
        Err(_) => return,
    };
    link.set_href(&url);
    link.set_download(filename);
    link.click();

    let _ = web_sys::Url::revoke_object_url(&url);
}
