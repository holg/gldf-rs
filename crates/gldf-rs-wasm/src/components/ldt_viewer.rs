//! LDT/Eulumdat diagram viewer component with multiple diagram types
//!
//! Supports:
//! - EULUMDAT (.ldt) files
//! - IES (.ies) files
//! - TM-33/IESXML (.iesxml, .xml) files via atla crate

use eulumdat::{
    diagram::{ButterflyDiagram, CartesianDiagram, HeatmapDiagram, PolarDiagram, SvgTheme},
    BugDiagram, Eulumdat, IesParser, PhotometricSummary,
};
use gloo::console::log;
#[allow(unused_imports)]
use wasm_bindgen::JsCast;
use yew::prelude::*;

/// Available diagram view types
#[derive(Clone, PartialEq, Copy, Debug)]
pub enum ViewType {
    Polar,
    Cartesian,
    Heatmap,
    Butterfly,
    Bug,
    Lcs,
    Spectrum,
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
            ViewType::Spectrum => "SPD",
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
            ViewType::Spectrum => "🌈",
        }
    }

    /// All view types (without SPD - shown separately when available)
    fn all_angular() -> &'static [ViewType] {
        &[
            ViewType::Polar,
            ViewType::Cartesian,
            ViewType::Heatmap,
            ViewType::Butterfly,
            ViewType::Bug,
            ViewType::Lcs,
        ]
    }

    /// All view types including SPD for TM-33 files
    fn all_with_spectrum() -> &'static [ViewType] {
        &[
            ViewType::Polar,
            ViewType::Cartesian,
            ViewType::Heatmap,
            ViewType::Butterfly,
            ViewType::Bug,
            ViewType::Lcs,
            ViewType::Spectrum,
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
    /// Default view type (Polar, Spectrum, etc.)
    #[prop_or(ViewType::Polar)]
    pub default_view: ViewType,
}

#[function_component(LdtViewer)]
pub fn ldt_viewer(props: &LdtViewerProps) -> Html {
    let view_type = use_state(|| props.default_view);
    let is_zoomed = use_state(|| false);
    let svg_container = use_node_ref();
    let zoomed_container = use_node_ref();

    // Log incoming data
    log!(format!(
        "LdtViewer: received {} bytes, first 100 chars: {:?}",
        props.ldt_data.len(),
        String::from_utf8_lossy(&props.ldt_data[..props.ldt_data.len().min(100)])
    ));

    // Detect if this is a TM-33/ATLA XML file (has spectral data)
    let is_tm33 = {
        let content = String::from_utf8_lossy(&props.ldt_data);
        let trimmed = content.trim_start();
        trimmed.starts_with("<IESTM33")
            || trimmed.starts_with("<LuminaireOpticalData")
            || (trimmed.starts_with("<?xml")
                && (trimmed.contains("<IESTM33") || trimmed.contains("<LuminaireOpticalData")))
    };

    // Apply flux override if provided
    let ldt_data = if let Some(flux) = props.flux_override {
        apply_flux_override(&props.ldt_data, flux)
    } else {
        props.ldt_data.clone()
    };

    // Generate small SVG for inline view
    let svg_content = {
        let view_type = *view_type;
        generate_ldt_svg(&ldt_data, props.width, props.height, view_type, is_tm33)
    };

    // Generate larger SVG for zoomed modal
    let zoomed_svg_content = {
        let view_type = *view_type;
        // Use larger dimensions for zoomed view
        let zoomed_width = 800.0;
        let zoomed_height = 650.0;
        generate_ldt_svg(&ldt_data, zoomed_width, zoomed_height, view_type, is_tm33)
    };

    // Choose available view types based on file type
    let available_views: &[ViewType] = if is_tm33 {
        ViewType::all_with_spectrum()
    } else {
        ViewType::all_angular()
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
                { for available_views.iter().map(|vt| {
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
                                { for available_views.iter().map(|vt| {
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

fn generate_ldt_svg(buffer: &[u8], width: f64, height: f64, view_type: ViewType, is_tm33: bool) -> String {
    // Try to parse as string first
    let content = match std::str::from_utf8(buffer) {
        Ok(s) => s,
        Err(e) => {
            log!(format!("Error decoding file as UTF-8: {:?}", e));
            return error_svg(width, height, "Invalid file encoding");
        }
    };

    let trimmed = content.trim_start();

    // Check if this is an XML-based photometry file (TM-33-23 or ATLA S001)
    let is_xml_photometry = trimmed.starts_with("<IESTM33")
        || trimmed.starts_with("<LuminaireOpticalData")
        || (trimmed.starts_with("<?xml")
            && (trimmed.contains("<IESTM33") || trimmed.contains("<LuminaireOpticalData")));

    // For Spectrum view, we need the raw atla document to get spectral data
    if view_type == ViewType::Spectrum && is_tm33 {
        match atla::parse(content) {
            Ok(doc) => {
                return generate_spectrum_svg_from_atla(&doc, width, height);
            }
            Err(e) => {
                log!(format!("Error parsing TM-33/ATLA file for spectrum: {:?}", e));
                return error_svg(width, height, "No spectral data available");
            }
        }
    }

    // Try to detect file type and parse accordingly
    let ldt: Eulumdat = if is_xml_photometry {
        // TM-33-23 or ATLA S001 file - parse with atla crate
        log!(format!("Detected TM-33/ATLA XML format"));
        match atla::parse(content) {
            Ok(doc) => {
                log!(format!(
                    "Parsed TM-33/ATLA: manufacturer={:?}, emitters={}",
                    doc.header.manufacturer,
                    doc.emitters.len()
                ));
                let eulumdat = doc.to_eulumdat();
                log!(format!(
                    "Converted to Eulumdat: c_angles={}, g_angles={}, intensities={}",
                    eulumdat.c_angles.len(),
                    eulumdat.g_angles.len(),
                    eulumdat.intensities.len()
                ));
                eulumdat
            }
            Err(e) => {
                log!(format!("Error parsing TM-33/ATLA file: {:?}", e));
                return error_svg(width, height, "Error parsing TM-33/ATLA file");
            }
        }
    } else if trimmed.starts_with("IESNA") {
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
        ViewType::Spectrum => {
            // Spectrum view for non-TM33 files (shouldn't happen, but fallback)
            error_svg(width, height, "No spectral data (LDT/IES)")
        }
    }
}

/// Generate SPD (Spectral Power Distribution) SVG from ATLA/TM-33 document
fn generate_spectrum_svg_from_atla(doc: &atla::LuminaireOpticalData, width: f64, height: f64) -> String {
    // Try to find spectral data from the first emitter
    let spectral = doc.emitters.iter()
        .find_map(|e| e.spectral_distribution.as_ref());

    let Some(spd) = spectral else {
        return error_svg(width, height, "No spectral data in TM-33 file");
    };

    if spd.wavelengths.is_empty() || spd.values.is_empty() {
        return error_svg(width, height, "Empty spectral data");
    }

    // Use atla's SpectralDiagram
    let theme = atla::SpectralTheme::dark();
    let diagram = atla::SpectralDiagram::from_spectral(spd);
    diagram.to_svg(width, height, &theme)
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
