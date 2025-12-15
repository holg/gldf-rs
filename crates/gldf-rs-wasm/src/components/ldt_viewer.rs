//! LDT/Eulumdat diagram viewer component

use eulumdat::{
    diagram::{CartesianDiagram, PolarDiagram, SvgTheme},
    Eulumdat, IesParser,
};
use gloo::console::log;
#[allow(unused_imports)]
use wasm_bindgen::JsCast;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub enum ViewType {
    Polar,
    Cartesian,
}

#[derive(Properties, PartialEq)]
pub struct LdtViewerProps {
    pub ldt_data: Vec<u8>,
    #[prop_or(400.0)]
    pub width: f64,
    #[prop_or(400.0)]
    pub height: f64,
    /// Optional override for luminous flux (from emitter's rated_luminous_flux)
    #[prop_or_default]
    pub flux_override: Option<i32>,
}

#[function_component(LdtViewer)]
pub fn ldt_viewer(props: &LdtViewerProps) -> Html {
    let view_type = use_state(|| ViewType::Polar);
    let svg_container = use_node_ref();

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

    let svg_content = {
        let view_type = (*view_type).clone();
        generate_ldt_svg(&ldt_data, props.width, props.height, view_type)
    };

    // Use effect to set innerHTML for SVG rendering
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

    let on_polar_click = {
        let view_type = view_type.clone();
        Callback::from(move |_| {
            view_type.set(ViewType::Polar);
        })
    };

    let on_cart_click = {
        let view_type = view_type.clone();
        Callback::from(move |_| {
            view_type.set(ViewType::Cartesian);
        })
    };

    let polar_class = if *view_type == ViewType::Polar {
        "toggle-btn active"
    } else {
        "toggle-btn"
    };

    let cart_class = if *view_type == ViewType::Cartesian {
        "toggle-btn active"
    } else {
        "toggle-btn"
    };

    html! {
        <div class="ldt-viewer">
            <div class="ldt-viewer-controls">
                <button onclick={on_polar_click} class={polar_class}>
                    {"Polar"}
                </button>
                <button onclick={on_cart_click} class={cart_class}>
                    {"Cartesian"}
                </button>
            </div>
            <div
                ref={svg_container}
                class="ldt-diagram"
                style={format!("width: {}px; height: {}px;", props.width, props.height)}
            />
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

    match view_type {
        ViewType::Polar => {
            let diagram = PolarDiagram::from_eulumdat(&ldt);
            diagram.to_svg(width, height, &theme)
        }
        ViewType::Cartesian => {
            let diagram = CartesianDiagram::from_eulumdat(&ldt, width, height, 8);
            diagram.to_svg(width, height, &theme)
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
