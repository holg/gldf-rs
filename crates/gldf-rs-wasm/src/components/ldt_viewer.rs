//! LDT/Eulumdat diagram viewer component with multiple diagram types

use eulumdat::{
    diagram::{ButterflyDiagram, CartesianDiagram, HeatmapDiagram, PolarDiagram, SvgTheme},
    BugDiagram, Eulumdat, IesParser, PhotometricSummary,
};
use gloo::console::log;
#[allow(unused_imports)]
use wasm_bindgen::JsCast;
use yew::prelude::*;

/// Available diagram view types
#[derive(Clone, PartialEq, Copy)]
pub enum ViewType {
    Polar,
    Cartesian,
    Heatmap,
    Butterfly,
    Bug,
    Lcs,
}

impl ViewType {
    fn label(&self) -> &'static str {
        match self {
            ViewType::Polar => "Polar",
            ViewType::Cartesian => "Cartesian",
            ViewType::Heatmap => "Heatmap",
            ViewType::Butterfly => "3D",
            ViewType::Bug => "BUG",
            ViewType::Lcs => "LCS",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            ViewType::Polar => "◉",
            ViewType::Cartesian => "📊",
            ViewType::Heatmap => "🔥",
            ViewType::Butterfly => "🦋",
            ViewType::Bug => "🔦",
            ViewType::Lcs => "📐",
        }
    }

    fn all() -> &'static [ViewType] {
        &[
            ViewType::Polar,
            ViewType::Cartesian,
            ViewType::Heatmap,
            ViewType::Butterfly,
            ViewType::Bug,
            ViewType::Lcs,
        ]
    }
}

#[derive(Properties, PartialEq)]
pub struct LdtViewerProps {
    pub ldt_data: Vec<u8>,
    #[prop_or(500.0)]
    pub width: f64,
    #[prop_or(400.0)]
    pub height: f64,
    /// Optional override for luminous flux (from emitter's rated_luminous_flux)
    #[prop_or_default]
    pub flux_override: Option<i32>,
    /// Show compact tabs (icons only)
    #[prop_or(false)]
    pub compact: bool,
}

#[function_component(LdtViewer)]
pub fn ldt_viewer(props: &LdtViewerProps) -> Html {
    let view_type = use_state(|| ViewType::Polar);
    let is_zoomed = use_state(|| false);
    let svg_container = use_node_ref();
    let zoomed_container = use_node_ref();

    // Log incoming data
    log!(format!(
        "LdtViewer: received {} bytes, first 100 chars: {:?}",
        props.ldt_data.len(),
        String::from_utf8_lossy(&props.ldt_data[..props.ldt_data.len().min(100)])
    ));

    // Apply flux override if provided
    let ldt_data = if let Some(flux) = props.flux_override {
        apply_flux_override(&props.ldt_data, flux)
    } else {
        props.ldt_data.clone()
    };

    // Generate small SVG for inline view
    let svg_content = {
        let view_type = *view_type;
        generate_ldt_svg(&ldt_data, props.width, props.height, view_type)
    };

    // Generate larger SVG for zoomed modal
    let zoomed_svg_content = {
        let view_type = *view_type;
        // Use larger dimensions for zoomed view
        let zoomed_width = 800.0;
        let zoomed_height = 650.0;
        generate_ldt_svg(&ldt_data, zoomed_width, zoomed_height, view_type)
    };

    // Use effect to set innerHTML for inline SVG rendering
    {
        let svg_container = svg_container.clone();
        let svg_content = svg_content.clone();
        use_effect_with((svg_content,), move |(svg_content,)| {
            if let Some(element) = svg_container.cast::<web_sys::HtmlElement>() {
                element.set_inner_html(svg_content);
            }
            || ()
        });
    }

    // Use effect to set innerHTML for zoomed SVG rendering
    {
        let zoomed_container = zoomed_container.clone();
        let zoomed_svg_content = zoomed_svg_content.clone();
        let is_zoomed_val = *is_zoomed;
        use_effect_with(
            (zoomed_svg_content, is_zoomed_val),
            move |(zoomed_svg_content, is_zoomed_val)| {
                if *is_zoomed_val {
                    if let Some(element) = zoomed_container.cast::<web_sys::HtmlElement>() {
                        element.set_inner_html(zoomed_svg_content);
                    }
                }
                || ()
            },
        );
    }

    let compact = props.compact;

    // Click handler to open zoom
    let on_diagram_click = {
        let is_zoomed = is_zoomed.clone();
        Callback::from(move |_: MouseEvent| {
            is_zoomed.set(true);
        })
    };

    // Click handler to close zoom (on overlay)
    let on_overlay_click = {
        let is_zoomed = is_zoomed.clone();
        Callback::from(move |_: MouseEvent| {
            is_zoomed.set(false);
        })
    };

    // Prevent click propagation on modal content
    let on_modal_click = Callback::from(|e: MouseEvent| {
        e.stop_propagation();
    });

    // Close button handler
    let on_close_click = {
        let is_zoomed = is_zoomed.clone();
        Callback::from(move |_: MouseEvent| {
            is_zoomed.set(false);
        })
    };

    html! {
        <div class="ldt-viewer">
            <div class="ldt-viewer-tabs">
                { for ViewType::all().iter().map(|vt| {
                    let is_active = *view_type == *vt;
                    let vt_clone = *vt;
                    let view_type = view_type.clone();
                    let onclick = Callback::from(move |_| {
                        view_type.set(vt_clone);
                    });
                    let class = if is_active { "ldt-tab active" } else { "ldt-tab" };

                    html! {
                        <button {onclick} {class} title={vt.label()}>
                            <span class="tab-icon">{ vt.icon() }</span>
                            if !compact {
                                <span class="tab-label">{ vt.label() }</span>
                            }
                        </button>
                    }
                })}
            </div>
            <div
                ref={svg_container}
                class="ldt-diagram clickable"
                style={format!("max-width: {}px; max-height: {}px; cursor: zoom-in;", props.width, props.height)}
                onclick={on_diagram_click}
                title="Click to enlarge"
            />

            // Zoomed modal overlay
            if *is_zoomed {
                <div class="ldt-zoom-overlay" onclick={on_overlay_click}>
                    <div class="ldt-zoom-modal" onclick={on_modal_click}>
                        <div class="ldt-zoom-header">
                            <div class="ldt-zoom-tabs">
                                { for ViewType::all().iter().map(|vt| {
                                    let is_active = *view_type == *vt;
                                    let vt_clone = *vt;
                                    let view_type = view_type.clone();
                                    let onclick = Callback::from(move |_| {
                                        view_type.set(vt_clone);
                                    });
                                    let class = if is_active { "ldt-tab active" } else { "ldt-tab" };

                                    html! {
                                        <button {onclick} {class} title={vt.label()}>
                                            <span class="tab-icon">{ vt.icon() }</span>
                                            <span class="tab-label">{ vt.label() }</span>
                                        </button>
                                    }
                                })}
                            </div>
                            <button class="ldt-zoom-close" onclick={on_close_click} title="Close (Esc)">
                                { "✕" }
                            </button>
                        </div>
                        <div
                            ref={zoomed_container}
                            class="ldt-zoom-content"
                        />
                    </div>
                </div>
            }
        </div>
    }
}

fn generate_ldt_svg(buffer: &[u8], width: f64, height: f64, view_type: ViewType) -> String {
    // Try to parse as string first
    let content = match std::str::from_utf8(buffer) {
        Ok(s) => s,
        Err(e) => {
            log!(format!("Error decoding file as UTF-8: {:?}", e));
            return error_svg(width, height, "Invalid file encoding");
        }
    };

    // Try to detect file type and parse accordingly
    let ldt = if content.trim_start().starts_with("IESNA") {
        // IES file
        match IesParser::parse(content) {
            Ok(ldt) => ldt,
            Err(e) => {
                log!(format!("Error parsing IES file: {:?}", e));
                return error_svg(width, height, "Error parsing IES file");
            }
        }
    } else {
        // LDT file
        match Eulumdat::parse(content) {
            Ok(ldt) => ldt,
            Err(e) => {
                log!(format!("Error parsing LDT file: {:?}", e));
                return error_svg(width, height, "Error parsing LDT file");
            }
        }
    };

    // Use dark theme for the dark background
    let theme = SvgTheme::dark();

    // Calculate photometric summary for enhanced views
    let summary = PhotometricSummary::from_eulumdat(&ldt);

    match view_type {
        ViewType::Polar => {
            let diagram = PolarDiagram::from_eulumdat(&ldt);
            diagram.to_svg_with_summary(width, height, &theme, &summary)
        }
        ViewType::Cartesian => {
            let diagram = CartesianDiagram::from_eulumdat(&ldt, width, height, 8);
            diagram.to_svg_with_summary(width, height, &theme, &summary)
        }
        ViewType::Heatmap => {
            let diagram = HeatmapDiagram::from_eulumdat(&ldt, width, height);
            diagram.to_svg_with_summary(width, height, &theme, &summary)
        }
        ViewType::Butterfly => {
            let diagram = ButterflyDiagram::from_eulumdat(&ldt, width, height, 60.0);
            diagram.to_svg(width, height, &theme)
        }
        ViewType::Bug => {
            let bug = BugDiagram::from_eulumdat(&ldt);
            bug.to_svg_with_details(width.max(550.0), height, &theme)
        }
        ViewType::Lcs => {
            let bug = BugDiagram::from_eulumdat(&ldt);
            bug.to_lcs_svg(width.max(510.0), height, &theme)
        }
    }
}

fn error_svg(width: f64, height: f64, message: &str) -> String {
    let bg_color = "#1a1a2e";
    let text_color = "#ff6b6b";
    format!(
        r#"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
            <rect width="100%" height="100%" fill="{}"/>
            <text x="50%" y="50%" text-anchor="middle" fill="{}" font-size="14">
                {}
            </text>
        </svg>"#,
        width, height, bg_color, text_color, message
    )
}

/// Apply luminous flux override to LDT/IES data
/// TODO: Implement actual flux scaling in eulumdat-rs crate
fn apply_flux_override(data: &[u8], _flux: i32) -> Vec<u8> {
    // For now, just return the original data
    // Future implementation should scale the intensity values
    // based on the ratio of the override flux to the file's original flux
    data.to_vec()
}
