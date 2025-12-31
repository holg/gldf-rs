//! Dynamic Plugin Viewer Component
//!
//! A generic viewer that adapts to any loaded WASM plugin's capabilities.
//! Discovers functions, examples, and output formats from the plugin manifest.
//! Supports code editing, execution, and multiple output formats (SVG, JSON, KiCad, etc.)

use crate::plugins::{has_plugin, Plugin, PluginFunction};
use gloo::console::log;
use wasm_bindgen::JsCast;
use web_sys::HtmlTextAreaElement;
use yew::prelude::*;

// Re-export the LispViewer for legacy usage
pub use super::lisp_viewer::LispViewer;

#[derive(Clone, PartialEq)]
pub enum PluginLoadState {
    NotLoaded,
    Loading,
    Loaded,
    Error(String),
}

#[derive(Clone, PartialEq)]
pub enum OutputFormat {
    Svg,
    Json,
    KicadSym,
    KicadMod,
    Text,
    Custom(String),
}

impl OutputFormat {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "svg" => Self::Svg,
            "json" => Self::Json,
            "kicad_sym" => Self::KicadSym,
            "kicad_mod" => Self::KicadMod,
            _ => Self::Custom(s.to_string()),
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::Svg => "svg",
            Self::Json => "json",
            Self::KicadSym => "kicad_sym",
            Self::KicadMod => "kicad_mod",
            Self::Text => "text",
            Self::Custom(s) => s,
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct DynamicPluginViewerProps {
    /// Plugin ID to use (e.g., "acadlisp", "typst")
    pub plugin_id: String,
    /// Initial code to display
    #[prop_or_default]
    pub initial_code: String,
    /// Title override (uses plugin name if not set)
    #[prop_or_default]
    pub title: Option<String>,
    /// Width of the viewer
    #[prop_or(800)]
    pub width: u32,
    /// Height of the viewer
    #[prop_or(600)]
    pub height: u32,
    /// Whether to auto-run on load
    #[prop_or(false)]
    pub auto_run: bool,
}

/// Dynamic plugin viewer that discovers and uses plugin capabilities
#[function_component(DynamicPluginViewer)]
pub fn dynamic_plugin_viewer(props: &DynamicPluginViewerProps) -> Html {
    let plugin_id = props.plugin_id.clone();
    let load_state = use_state(|| {
        if has_plugin(&plugin_id) {
            PluginLoadState::Loaded
        } else {
            PluginLoadState::NotLoaded
        }
    });

    let code = use_state(|| props.initial_code.clone());
    let result = use_state(String::new);
    let output = use_state(String::new);
    let svg_content = use_state(String::new);
    let json_content = use_state(String::new);
    let output_format = use_state(|| OutputFormat::Svg);
    let entity_count = use_state(|| 0_usize);
    let available_formats = use_state(Vec::<String>::new);
    let available_examples = use_state(Vec::<String>::new);
    let available_functions = use_state(Vec::<PluginFunction>::new);

    // Get plugin info when loaded
    {
        let plugin_id = plugin_id.clone();
        let load_state = load_state.clone();
        let available_formats = available_formats.clone();
        let available_examples = available_examples.clone();
        let available_functions = available_functions.clone();

        use_effect_with((*load_state).clone(), move |state| {
            if *state == PluginLoadState::Loaded {
                if let Some(plugin) = Plugin::get(&plugin_id) {
                    available_formats.set(plugin.capabilities.output_formats.clone());
                    available_examples.set(plugin.capabilities.examples.clone());
                    available_functions.set(plugin.capabilities.functions.clone());
                    log!(format!(
                        "[PluginViewer] Loaded plugin {} with {} functions, {} examples",
                        plugin_id,
                        plugin.capabilities.functions.len(),
                        plugin.capabilities.examples.len()
                    ));
                }
            }
            || ()
        });
    }

    // Run code
    let run_code = {
        let plugin_id = plugin_id.clone();
        let code = code.clone();
        let result = result.clone();
        let output = output.clone();
        let svg_content = svg_content.clone();
        let json_content = json_content.clone();
        let entity_count = entity_count.clone();

        Callback::from(move |_| {
            if let Some(plugin) = Plugin::get(&plugin_id) {
                let code_str = (*code).clone();
                log!(format!("[PluginViewer] Running code: {} chars", code_str.len()));

                // Evaluate code
                match plugin.eval(&code_str) {
                    Ok(res) => result.set(res),
                    Err(e) => result.set(format!("Error: {}", e)),
                }

                // Get output buffer
                if let Ok(out) = plugin.get_output() {
                    output.set(out);
                }

                // Get SVG
                if let Ok(svg) = plugin.get_svg() {
                    svg_content.set(svg);
                }

                // Get JSON
                if let Ok(json) = plugin.get_json() {
                    json_content.set(json);
                }

                // Get entity count
                entity_count.set(plugin.entity_count());
            }
        })
    };

    // Clear
    let clear = {
        let plugin_id = plugin_id.clone();
        let result = result.clone();
        let output = output.clone();
        let svg_content = svg_content.clone();
        let json_content = json_content.clone();
        let entity_count = entity_count.clone();

        Callback::from(move |_| {
            if let Some(plugin) = Plugin::get(&plugin_id) {
                let _ = plugin.clear();
                result.set(String::new());
                output.set(String::new());
                svg_content.set(String::new());
                json_content.set(String::new());
                entity_count.set(0);
            }
        })
    };

    // Load example
    let load_example = {
        let plugin_id = plugin_id.clone();
        let code = code.clone();

        Callback::from(move |name: String| {
            if let Some(plugin) = Plugin::get(&plugin_id) {
                if let Some(example_code) = plugin.get_example(&name) {
                    code.set(example_code);
                }
            }
        })
    };

    // Handle code input
    let on_code_input = {
        let code = code.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(target) = e.target() {
                if let Ok(textarea) = target.dyn_into::<HtmlTextAreaElement>() {
                    code.set(textarea.value());
                }
            }
        })
    };

    // Switch output format
    let set_format = {
        let output_format = output_format.clone();
        Callback::from(move |format: OutputFormat| {
            output_format.set(format);
        })
    };

    // Get title
    let title = props.title.clone().unwrap_or_else(|| {
        Plugin::get(&plugin_id)
            .map(|p| p.capabilities.name.clone())
            .unwrap_or_else(|| plugin_id.clone())
    });

    // Get plugin info for header
    let plugin_info = Plugin::get(&plugin_id);
    let ui_icon = plugin_info
        .as_ref()
        .map(|p| p.capabilities.ui.icon.clone())
        .unwrap_or_default();
    let ui_color = plugin_info
        .as_ref()
        .map(|p| p.capabilities.ui.color.clone())
        .unwrap_or_else(|| "#00ff88".to_string());

    // Render output based on format
    let render_output = {
        let format = (*output_format).clone();
        let svg = (*svg_content).clone();
        let json = (*json_content).clone();

        match format {
            OutputFormat::Svg => {
                if svg.is_empty() {
                    html! { <div class="plugin-output-placeholder">{"No output yet"}</div> }
                } else {
                    html! {
                        <div
                            class="plugin-svg-output"
                            style="background: white; border-radius: 4px; padding: 8px;"
                        >
                            { Html::from_html_unchecked(AttrValue::from(svg)) }
                        </div>
                    }
                }
            }
            OutputFormat::Json => {
                html! {
                    <pre class="plugin-json-output" style="font-size: 11px; max-height: 300px; overflow: auto;">
                        { json }
                    </pre>
                }
            }
            _ => {
                html! {
                    <pre class="plugin-text-output" style="font-size: 11px;">
                        { &*result }
                    </pre>
                }
            }
        }
    };

    // Style based on plugin UI config
    let header_style = format!(
        "background: {}22; border-left: 3px solid {}; padding: 8px 12px; margin-bottom: 12px; border-radius: 0 4px 4px 0;",
        ui_color, ui_color
    );

    html! {
        <div
            class="dynamic-plugin-viewer"
            style={format!("width: {}px; font-family: system-ui, sans-serif;", props.width)}
        >
            // Header
            <div style={header_style}>
                <div style="display: flex; align-items: center; gap: 8px;">
                    if !ui_icon.is_empty() {
                        <span style={format!("font-size: 24px; color: {};", ui_color)}>
                            { &ui_icon }
                        </span>
                    }
                    <div>
                        <div style="font-weight: 600; font-size: 14px;">{ &title }</div>
                        if let Some(ref p) = plugin_info {
                            <div style="font-size: 11px; color: var(--text-secondary, #666);">
                                { format!("v{} - {}", p.capabilities.version, p.capabilities.description) }
                            </div>
                        }
                    </div>
                </div>
            </div>

            // Examples bar
            if !available_examples.is_empty() {
                <div style="margin-bottom: 12px; display: flex; flex-wrap: wrap; gap: 4px;">
                    <span style="font-size: 11px; color: var(--text-tertiary, #888); margin-right: 4px;">
                        {"Examples:"}
                    </span>
                    { for available_examples.iter().map(|name| {
                        let name_clone = name.clone();
                        let load = load_example.clone();
                        html! {
                            <button
                                onclick={Callback::from(move |_| load.emit(name_clone.clone()))}
                                style="
                                    font-size: 10px;
                                    padding: 2px 6px;
                                    border: 1px solid var(--border-color, #ddd);
                                    border-radius: 3px;
                                    background: var(--bg-secondary, #f5f5f5);
                                    cursor: pointer;
                                "
                            >
                                { name }
                            </button>
                        }
                    })}
                </div>
            }

            // Code editor
            <div style="margin-bottom: 12px;">
                <textarea
                    value={(*code).clone()}
                    oninput={on_code_input}
                    placeholder="Enter code here..."
                    style={format!(
                        "width: 100%;
                        height: 200px;
                        font-family: 'SF Mono', Monaco, monospace;
                        font-size: 12px;
                        padding: 12px;
                        border: 1px solid var(--border-color, #ddd);
                        border-radius: 4px;
                        resize: vertical;
                        background: var(--bg-secondary, #1e1e1e);
                        color: var(--text-primary, #d4d4d4);
                        box-sizing: border-box;"
                    )}
                />
            </div>

            // Action buttons
            <div style="display: flex; gap: 8px; margin-bottom: 12px;">
                <button
                    onclick={run_code}
                    style={format!(
                        "padding: 8px 16px;
                        background: {};
                        color: black;
                        border: none;
                        border-radius: 4px;
                        cursor: pointer;
                        font-weight: 500;",
                        ui_color
                    )}
                >
                    {"▶ Run"}
                </button>
                <button
                    onclick={clear}
                    style="
                        padding: 8px 16px;
                        background: var(--bg-tertiary, #333);
                        color: var(--text-primary, #ddd);
                        border: 1px solid var(--border-color, #555);
                        border-radius: 4px;
                        cursor: pointer;
                    "
                >
                    {"Clear"}
                </button>

                // Output format selector
                <div style="margin-left: auto; display: flex; gap: 4px;">
                    { for available_formats.iter().map(|fmt| {
                        let format = OutputFormat::from_str(fmt);
                        let is_active = format.as_str() == output_format.as_str();
                        let set_fmt = set_format.clone();
                        let fmt_clone = format.clone();
                        html! {
                            <button
                                onclick={Callback::from(move |_| set_fmt.emit(fmt_clone.clone()))}
                                style={format!(
                                    "padding: 4px 8px;
                                    font-size: 11px;
                                    border: 1px solid {};
                                    border-radius: 3px;
                                    background: {};
                                    color: {};
                                    cursor: pointer;",
                                    if is_active { &ui_color } else { "var(--border-color, #555)" },
                                    if is_active { format!("{}33", ui_color) } else { "transparent".to_string() },
                                    if is_active { &ui_color } else { "var(--text-secondary, #888)" }
                                )}
                            >
                                { fmt.to_uppercase() }
                            </button>
                        }
                    })}
                </div>
            </div>

            // Result/output area
            if !result.is_empty() {
                <div style="margin-bottom: 8px; padding: 8px; background: var(--bg-tertiary, #2d2d2d); border-radius: 4px; font-size: 11px;">
                    <span style="color: var(--text-tertiary, #888);">{"Result: "}</span>
                    <code style="color: var(--text-primary, #ddd);">{ &*result }</code>
                </div>
            }

            // Console output
            if !output.is_empty() {
                <div style="margin-bottom: 12px;">
                    <div style="font-size: 11px; color: var(--text-tertiary, #888); margin-bottom: 4px;">
                        {"Console Output:"}
                    </div>
                    <pre style="
                        padding: 8px;
                        background: var(--bg-tertiary, #1a1a1a);
                        border-radius: 4px;
                        font-size: 11px;
                        max-height: 100px;
                        overflow: auto;
                        margin: 0;
                    ">
                        { &*output }
                    </pre>
                </div>
            }

            // Main output
            <div style="min-height: 200px; border: 1px solid var(--border-color, #333); border-radius: 4px; overflow: hidden;">
                { render_output }
            </div>

            // Entity count
            if *entity_count > 0 {
                <div style="margin-top: 8px; font-size: 11px; color: var(--text-tertiary, #888);">
                    { format!("{} entities", *entity_count) }
                </div>
            }

            // Functions reference (collapsible)
            if !available_functions.is_empty() {
                <details style="margin-top: 16px;">
                    <summary style="cursor: pointer; font-size: 12px; color: var(--text-secondary, #888);">
                        { format!("Available Functions ({})", available_functions.len()) }
                    </summary>
                    <div style="margin-top: 8px; font-size: 11px;">
                        { for available_functions.iter().map(|f| {
                            html! {
                                <div style="padding: 4px 0; border-bottom: 1px solid var(--border-color, #333);">
                                    <code style="color: var(--accent, #00ff88);">{ &f.name }</code>
                                    if !f.args.is_empty() {
                                        <span style="color: var(--text-tertiary, #666);">
                                            { format!("({})", f.args.join(", ")) }
                                        </span>
                                    }
                                    if !f.returns.is_empty() {
                                        <span style="color: var(--text-tertiary, #666);">
                                            { format!(" → {}", f.returns) }
                                        </span>
                                    }
                                    if !f.description.is_empty() {
                                        <div style="color: var(--text-tertiary, #666); margin-left: 16px;">
                                            { &f.description }
                                        </div>
                                    }
                                </div>
                            }
                        })}
                    </div>
                </details>
            }
        </div>
    }
}

// Legacy PluginViewer props/component for backwards compatibility
#[derive(Properties, PartialEq)]
pub struct PluginViewerProps {
    /// Plugin ID to use (e.g., "acadlisp")
    pub plugin_id: String,
    /// Initial code to display
    #[prop_or_default]
    pub initial_code: String,
    /// Title override (uses plugin name if not set)
    #[prop_or_default]
    pub title: Option<String>,
    #[prop_or(800)]
    pub width: u32,
    #[prop_or(600)]
    pub height: u32,
}

/// Legacy plugin viewer - delegates to DynamicPluginViewer or LispViewer
#[function_component(PluginViewer)]
pub fn plugin_viewer(props: &PluginViewerProps) -> Html {
    // If the new plugin system has this plugin, use DynamicPluginViewer
    if has_plugin(&props.plugin_id) {
        return html! {
            <DynamicPluginViewer
                plugin_id={props.plugin_id.clone()}
                initial_code={props.initial_code.clone()}
                title={props.title.clone()}
                width={props.width}
                height={props.height}
            />
        };
    }

    // Fall back to LispViewer for acadlisp when not using plugin system
    let title = props.title.clone().unwrap_or_else(|| {
        if let Some(plugin) = Plugin::get(&props.plugin_id) {
            plugin.capabilities.name.clone()
        } else {
            props.plugin_id.clone()
        }
    });

    html! {
        <div class="plugin-viewer-wrapper">
            <LispViewer
                initial_code={props.initial_code.clone()}
                title={title}
                width={props.width}
                height={props.height}
            />
        </div>
    }
}

/// Check if the plugin system is available
pub fn is_plugin_system_ready() -> bool {
    has_plugin("acadlisp") || has_plugin("bevy") || has_plugin("typst")
}
