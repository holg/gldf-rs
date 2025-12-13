//! LDT/Eulumdat diagram viewer component with multiple visualization types

use eulumdat::{
    diagram::{ButterflyDiagram, CartesianDiagram, HeatmapDiagram, PolarDiagram, SvgTheme},
    BugDiagram, Eulumdat, IesParser,
};
use gloo::console::log;
use web_sys::Element;
use yew::prelude::*;

/// Available diagram visualization types
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DiagramType {
    Polar,
    Cartesian,
    Butterfly,
    Heatmap,
    Bug,
}

impl DiagramType {
    fn label(&self) -> &'static str {
        match self {
            DiagramType::Polar => "Polar",
            DiagramType::Cartesian => "Cartesian",
            DiagramType::Butterfly => "Butterfly",
            DiagramType::Heatmap => "Heatmap",
            DiagramType::Bug => "BUG Rating",
        }
    }

    fn all() -> &'static [DiagramType] {
        &[
            DiagramType::Polar,
            DiagramType::Cartesian,
            DiagramType::Butterfly,
            DiagramType::Heatmap,
            DiagramType::Bug,
        ]
    }
}

/// Available themes
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ThemeType {
    Dark,
    Light,
}

impl ThemeType {
    fn label(&self) -> &'static str {
        match self {
            ThemeType::Dark => "Dark",
            ThemeType::Light => "Light",
        }
    }

    fn to_svg_theme(&self) -> SvgTheme {
        match self {
            ThemeType::Dark => SvgTheme::dark(),
            ThemeType::Light => SvgTheme::light(),
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct LdtViewerProps {
    pub ldt_data: Vec<u8>,
    #[prop_or(400.0)]
    pub width: f64,
    #[prop_or(400.0)]
    pub height: f64,
    /// Show diagram type selector
    #[prop_or(true)]
    pub show_controls: bool,
    /// Initial diagram type
    #[prop_or(DiagramType::Polar)]
    pub initial_type: DiagramType,
}

#[function_component(LdtViewer)]
pub fn ldt_viewer(props: &LdtViewerProps) -> Html {
    let diagram_type = use_state(|| props.initial_type);
    let theme = use_state(|| ThemeType::Dark);
    let svg_ref = use_node_ref();

    // Parse the LDT/IES data once
    let parsed_ldt = use_memo(props.ldt_data.clone(), |data| {
        parse_photometric_data(data)
    });

    // Generate SVG based on current diagram type and theme
    let svg_content = match &*parsed_ldt {
        Ok(ldt) => generate_diagram_svg(
            ldt,
            props.width,
            props.height,
            *diagram_type,
            theme.to_svg_theme(),
        ),
        Err(error_msg) => error_svg(props.width, props.height, error_msg),
    };

    // Use effect to set innerHTML when svg_content changes
    {
        let svg_ref = svg_ref.clone();
        let svg_content = svg_content.clone();
        use_effect_with(svg_content, move |svg| {
            if let Some(element) = svg_ref.cast::<Element>() {
                element.set_inner_html(svg);
            }
            || ()
        });
    }

    // Get background color based on theme
    let bg_class = match *theme {
        ThemeType::Dark => "bg-gray-900",
        ThemeType::Light => "bg-gray-100",
    };

    html! {
        <div class="ldt-viewer">
            if props.show_controls {
                <div class="ldt-controls">
                    // Diagram type buttons
                    <div class="diagram-type-selector">
                        { for DiagramType::all().iter().map(|dt| {
                            let dt = *dt;
                            let diagram_type = diagram_type.clone();
                            let is_active = *diagram_type == dt;
                            let class = if is_active {
                                "btn btn-sm btn-primary"
                            } else {
                                "btn btn-sm btn-secondary"
                            };
                            html! {
                                <button
                                    class={class}
                                    onclick={Callback::from(move |_| diagram_type.set(dt))}
                                >
                                    { dt.label() }
                                </button>
                            }
                        })}
                    </div>

                    // Theme toggle
                    <div class="theme-selector">
                        <button
                            class={if *theme == ThemeType::Dark { "btn btn-sm btn-primary" } else { "btn btn-sm btn-secondary" }}
                            onclick={let theme = theme.clone(); Callback::from(move |_| theme.set(ThemeType::Dark))}
                        >
                            { "Dark" }
                        </button>
                        <button
                            class={if *theme == ThemeType::Light { "btn btn-sm btn-primary" } else { "btn btn-sm btn-secondary" }}
                            onclick={let theme = theme.clone(); Callback::from(move |_| theme.set(ThemeType::Light))}
                        >
                            { "Light" }
                        </button>
                    </div>
                </div>
            }

            <div
                ref={svg_ref.clone()}
                class={classes!("ldt-diagram", "rounded", bg_class)}
                style={format!("width: {}px; height: {}px;", props.width, props.height)}
            />

            // Show parsed info
            if let Ok(ldt) = &*parsed_ldt {
                <div class="ldt-info">
                    <span class="ldt-info-item">
                        { format!("C-planes: {}", ldt.c_angles.len()) }
                    </span>
                    <span class="ldt-info-item">
                        { format!("Gamma: {}", ldt.g_angles.len()) }
                    </span>
                    <span class="ldt-info-item">
                        { format!("Symmetry: {:?}", ldt.symmetry) }
                    </span>
                </div>
            }
        </div>
    }
}

/// Parse LDT or IES data
fn parse_photometric_data(buffer: &[u8]) -> Result<Eulumdat, String> {
    let content = std::str::from_utf8(buffer)
        .map_err(|e| format!("Invalid UTF-8 encoding: {:?}", e))?;

    // Detect file type and parse accordingly
    if content.trim_start().starts_with("IESNA") {
        IesParser::parse(content).map_err(|e| format!("IES parse error: {:?}", e))
    } else {
        Eulumdat::parse(content).map_err(|e| format!("LDT parse error: {:?}", e))
    }
}

/// Generate SVG for the specified diagram type
fn generate_diagram_svg(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    diagram_type: DiagramType,
    theme: SvgTheme,
) -> String {
    match diagram_type {
        DiagramType::Polar => {
            let diagram = PolarDiagram::from_eulumdat(ldt);
            diagram.to_svg(width, height, &theme)
        }
        DiagramType::Cartesian => {
            let diagram = CartesianDiagram::from_eulumdat(ldt, width, height, 8);
            diagram.to_svg(width, height, &theme)
        }
        DiagramType::Butterfly => {
            let diagram = ButterflyDiagram::from_eulumdat(ldt, width, height, 60.0);
            diagram.to_svg(width, height, &theme)
        }
        DiagramType::Heatmap => {
            let diagram = HeatmapDiagram::from_eulumdat(ldt, width, height);
            diagram.to_svg(width, height, &theme)
        }
        DiagramType::Bug => {
            let diagram = BugDiagram::from_eulumdat(ldt);
            diagram.to_svg(width, height, &theme)
        }
    }
}

/// Generate error SVG
fn error_svg(width: f64, height: f64, message: &str) -> String {
    log!(format!("LDT Viewer Error: {}", message));
    let bg_color = "#1a1a2e";
    let text_color = "#ff6b6b";
    format!(
        r#"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
            <rect width="100%" height="100%" fill="{}"/>
            <text x="50%" y="45%" text-anchor="middle" fill="{}" font-size="14" font-family="system-ui">
                Error
            </text>
            <text x="50%" y="55%" text-anchor="middle" fill="{}" font-size="12" font-family="system-ui">
                {}
            </text>
        </svg>"#,
        width, height, bg_color, text_color, text_color, message
    )
}

// Legacy compatibility - keep the old ViewType enum for existing code
#[derive(Clone, PartialEq)]
pub enum ViewType {
    Polar,
    Cartesian,
}
