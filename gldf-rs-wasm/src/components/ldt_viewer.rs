//! LDT/Eulumdat diagram viewer component

use yew::prelude::*;
use gloo::console::log;
use eulumdat::{Eulumdat, IesParser, diagram::{PolarDiagram, CartesianDiagram, SvgTheme}};

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
}

#[function_component(LdtViewer)]
pub fn ldt_viewer(props: &LdtViewerProps) -> Html {
    let view_type = use_state(|| ViewType::Polar);

    let svg_content = {
        let view_type = (*view_type).clone();
        generate_ldt_svg(&props.ldt_data, props.width, props.height, view_type)
    };

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
        "px-3 py-1 text-sm rounded bg-blue-600 text-white"
    } else {
        "px-3 py-1 text-sm rounded bg-gray-700 text-gray-300 hover:bg-gray-600"
    };

    let cart_class = if *view_type == ViewType::Cartesian {
        "px-3 py-1 text-sm rounded bg-blue-600 text-white"
    } else {
        "px-3 py-1 text-sm rounded bg-gray-700 text-gray-300 hover:bg-gray-600"
    };

    html! {
        <div class="ldt-viewer flex flex-col items-center">
            <div class="mb-2 flex gap-2">
                <button onclick={on_polar_click} class={polar_class}>
                    {"Polar"}
                </button>
                <button onclick={on_cart_click} class={cart_class}>
                    {"Cartesian"}
                </button>
            </div>
            <div
                class="ldt-diagram bg-gray-900 rounded"
                style={format!("width: {}px; height: {}px;", props.width, props.height)}
            >
                {Html::from_html_unchecked(AttrValue::from(svg_content))}
            </div>
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
