//! Spectrum viewer component for displaying TM-30/TM-33 spectral data
//!
//! This component renders a spectral power distribution (SPD) chart from GLDF spectrum data.

use gldf_rs::gldf::general_definitions::photometries::Spectrum;
use yew::prelude::*;

/// Properties for the spectrum viewer
#[derive(Properties, PartialEq, Clone)]
pub struct SpectrumViewerProps {
    /// The spectrum data to display
    pub spectrum: Spectrum,
    #[prop_or(500.0)]
    pub width: f64,
    #[prop_or(300.0)]
    pub height: f64,
    /// Optional TM-30 Rf value to display
    #[prop_or_default]
    pub rf: Option<i32>,
    /// Optional TM-30 Rg value to display
    #[prop_or_default]
    pub rg: Option<i32>,
    /// Optional CCT value to display
    #[prop_or_default]
    pub cct: Option<i32>,
    /// Optional CRI value to display
    #[prop_or_default]
    pub cri: Option<i32>,
}

#[function_component(SpectrumViewer)]
pub fn spectrum_viewer(props: &SpectrumViewerProps) -> Html {
    let svg_content = generate_spectrum_svg(
        &props.spectrum,
        props.width,
        props.height,
        props.rf,
        props.rg,
        props.cct,
        props.cri,
    );

    let container_ref = use_node_ref();

    // Use effect to set innerHTML for SVG rendering
    {
        let container_ref = container_ref.clone();
        let svg_content = svg_content.clone();
        use_effect_with(svg_content, move |svg_content| {
            if let Some(element) = container_ref.cast::<web_sys::HtmlElement>() {
                element.set_inner_html(svg_content);
            }
            || ()
        });
    }

    html! {
        <div
            ref={container_ref}
            class="spectrum-viewer"
            style={format!("max-width: {}px; max-height: {}px;", props.width, props.height)}
        />
    }
}

/// Generate SVG for spectrum visualization
fn generate_spectrum_svg(
    spectrum: &Spectrum,
    width: f64,
    height: f64,
    rf: Option<i32>,
    rg: Option<i32>,
    cct: Option<i32>,
    cri: Option<i32>,
) -> String {
    // Theme colors (dark theme)
    let bg_color = "#1a1a2e";
    let grid_color = "#2a2a4e";
    let text_color = "#e0e0e0";
    let axis_color = "#4a4a6e";

    // Chart margins
    let margin_left = 60.0;
    let margin_right = 20.0;
    let margin_top = 40.0;
    let margin_bottom = 60.0;
    let info_height = if rf.is_some() || cct.is_some() { 40.0 } else { 0.0 };

    let chart_width = width - margin_left - margin_right;
    let chart_height = height - margin_top - margin_bottom - info_height;

    // Get intensity data
    let intensities: Vec<(f64, f64)> = spectrum
        .intensity
        .iter()
        .filter_map(|i| {
            let wl = i.wavelength? as f64;
            let val = i.value?;
            Some((wl, val))
        })
        .collect();

    if intensities.is_empty() {
        return error_svg(width, height, "No spectral data available");
    }

    // Find min/max wavelength and max intensity
    let min_wl = intensities.iter().map(|(w, _)| *w).fold(f64::INFINITY, f64::min);
    let max_wl = intensities.iter().map(|(w, _)| *w).fold(f64::NEG_INFINITY, f64::max);
    let max_intensity = intensities.iter().map(|(_, i)| *i).fold(f64::NEG_INFINITY, f64::max);

    // Scale functions
    let scale_x = |wl: f64| -> f64 {
        margin_left + ((wl - min_wl) / (max_wl - min_wl)) * chart_width
    };
    let scale_y = |intensity: f64| -> f64 {
        margin_top + chart_height - (intensity / max_intensity) * chart_height
    };

    // Generate path data for the spectrum curve
    let mut path_data = String::new();
    for (i, (wl, intensity)) in intensities.iter().enumerate() {
        let x = scale_x(*wl);
        let y = scale_y(*intensity);
        if i == 0 {
            path_data.push_str(&format!("M {:.1} {:.1}", x, y));
        } else {
            path_data.push_str(&format!(" L {:.1} {:.1}", x, y));
        }
    }

    // Generate filled area path (for gradient fill)
    let mut area_path = path_data.clone();
    if let Some((last_wl, _)) = intensities.last() {
        let last_x = scale_x(*last_wl);
        area_path.push_str(&format!(
            " L {:.1} {:.1} L {:.1} {:.1} Z",
            last_x,
            margin_top + chart_height,
            margin_left,
            margin_top + chart_height
        ));
    }

    // Generate wavelength color stops for gradient
    let color_stops = generate_wavelength_gradient_stops();

    // Generate grid lines and labels
    let mut grid_lines = String::new();
    let mut labels = String::new();

    // Wavelength axis labels (every 50nm)
    for wl in (400..=750).step_by(50) {
        let x = scale_x(wl as f64);
        grid_lines.push_str(&format!(
            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-opacity="0.3"/>"#,
            x, margin_top, x, margin_top + chart_height, grid_color
        ));
        labels.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" fill="{}" font-size="10">{}</text>"#,
            x,
            margin_top + chart_height + 15.0,
            text_color,
            wl
        ));
    }

    // Intensity axis labels
    for i in 0..=4 {
        let intensity = (i as f64) * 0.25;
        let y = scale_y(intensity);
        grid_lines.push_str(&format!(
            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-opacity="0.3"/>"#,
            margin_left, y, margin_left + chart_width, y, grid_color
        ));
        labels.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="end" fill="{}" font-size="10">{:.0}%</text>"#,
            margin_left - 5.0,
            y + 3.0,
            text_color,
            intensity * 100.0
        ));
    }

    // Color metrics info panel
    let info_panel = if rf.is_some() || cct.is_some() || cri.is_some() {
        let mut info_items = Vec::new();
        if let Some(cct_val) = cct {
            info_items.push(format!(r##"<tspan fill="#ffcc00">CCT: {}K</tspan>"##, cct_val));
        }
        if let Some(cri_val) = cri {
            info_items.push(format!(r##"<tspan fill="#66ff66">CRI: {}</tspan>"##, cri_val));
        }
        if let Some(rf_val) = rf {
            info_items.push(format!(r##"<tspan fill="#ff9966">Rf: {}</tspan>"##, rf_val));
        }
        if let Some(rg_val) = rg {
            info_items.push(format!(r##"<tspan fill="#66ccff">Rg: {}</tspan>"##, rg_val));
        }

        let info_text = info_items.join(r##"<tspan fill="#666">  |  </tspan>"##);
        format!(
            r##"<text x="{:.1}" y="{:.1}" text-anchor="middle" fill="{}" font-size="12">{}</text>"##,
            width / 2.0,
            height - 10.0,
            text_color,
            info_text
        )
    } else {
        String::new()
    };

    // Wavelength color bar at the bottom
    let color_bar_height = 8.0;
    let color_bar_y = margin_top + chart_height + 25.0;
    let color_bar = format!(
        r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="url(#wavelengthGradient)" rx="2"/>"#,
        margin_left, color_bar_y, chart_width, color_bar_height
    );

    format!(
        r##"<svg width="{width}" height="{height}" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="wavelengthGradient" x1="0%" y1="0%" x2="100%" y2="0%">
      {color_stops}
    </linearGradient>
    <linearGradient id="spectrumFill" x1="0%" y1="0%" x2="100%" y2="0%">
      {color_stops}
    </linearGradient>
    <clipPath id="chartArea">
      <rect x="{margin_left}" y="{margin_top}" width="{chart_width}" height="{chart_height}"/>
    </clipPath>
  </defs>

  <!-- Background -->
  <rect width="100%" height="100%" fill="{bg_color}"/>

  <!-- Title -->
  <text x="{title_x}" y="25" text-anchor="middle" fill="{text_color}" font-size="14" font-weight="bold">
    Spectral Power Distribution
  </text>

  <!-- Grid lines -->
  {grid_lines}

  <!-- Axes -->
  <line x1="{margin_left}" y1="{axis_y1}" x2="{axis_x2}" y2="{axis_y1}" stroke="{axis_color}" stroke-width="1"/>
  <line x1="{margin_left}" y1="{margin_top}" x2="{margin_left}" y2="{axis_y1}" stroke="{axis_color}" stroke-width="1"/>

  <!-- Filled area with wavelength gradient -->
  <path d="{area_path}" fill="url(#spectrumFill)" fill-opacity="0.3" clip-path="url(#chartArea)"/>

  <!-- Spectrum curve -->
  <path d="{path_data}" fill="none" stroke="url(#wavelengthGradient)" stroke-width="2" clip-path="url(#chartArea)"/>

  <!-- Axis labels -->
  {labels}

  <!-- Axis titles -->
  <text x="{xlabel_x}" y="{xlabel_y}" text-anchor="middle" fill="{text_color}" font-size="11">Wavelength (nm)</text>
  <text x="15" y="{ylabel_y}" text-anchor="middle" fill="{text_color}" font-size="11" transform="rotate(-90, 15, {ylabel_y})">Relative Intensity</text>

  <!-- Color bar -->
  {color_bar}

  <!-- Info panel -->
  {info_panel}
</svg>"##,
        width = width,
        height = height,
        color_stops = color_stops,
        bg_color = bg_color,
        text_color = text_color,
        grid_lines = grid_lines,
        axis_color = axis_color,
        margin_left = margin_left,
        margin_top = margin_top,
        chart_width = chart_width,
        chart_height = chart_height,
        axis_y1 = margin_top + chart_height,
        axis_x2 = margin_left + chart_width,
        area_path = area_path,
        path_data = path_data,
        labels = labels,
        title_x = width / 2.0,
        xlabel_x = margin_left + chart_width / 2.0,
        xlabel_y = margin_top + chart_height + 45.0,
        ylabel_y = margin_top + chart_height / 2.0,
        color_bar = color_bar,
        info_panel = info_panel,
    )
}

/// Generate gradient color stops for wavelength visualization
fn generate_wavelength_gradient_stops() -> String {
    // Standard visible spectrum colors
    let colors = [
        (380.0, "#7600ed"), // Violet
        (420.0, "#4b0082"), // Indigo
        (450.0, "#0000ff"), // Blue
        (495.0, "#00ffff"), // Cyan
        (520.0, "#00ff00"), // Green
        (565.0, "#ffff00"), // Yellow
        (590.0, "#ff7f00"), // Orange
        (620.0, "#ff0000"), // Red
        (700.0, "#8b0000"), // Dark red
        (780.0, "#3d0000"), // Near-IR (dim)
    ];

    let min_wl = 380.0;
    let max_wl = 780.0;
    let range = max_wl - min_wl;

    colors
        .iter()
        .map(|(wl, color)| {
            let offset = ((wl - min_wl) / range) * 100.0;
            format!(r#"<stop offset="{:.1}%" stop-color="{}"/>"#, offset, color)
        })
        .collect::<Vec<_>>()
        .join("\n      ")
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

/// Simple component to display spectrum info without full chart
#[derive(Properties, PartialEq, Clone)]
pub struct SpectrumInfoProps {
    pub spectrum_id: String,
    #[prop_or_default]
    pub rf: Option<i32>,
    #[prop_or_default]
    pub rg: Option<i32>,
}

#[function_component(SpectrumInfo)]
pub fn spectrum_info(props: &SpectrumInfoProps) -> Html {
    html! {
        <div class="spectrum-info">
            <span class="spectrum-id">{ format!("Spectrum: {}", props.spectrum_id) }</span>
            if let Some(rf) = props.rf {
                <span class="tm30-rf" title="TM-30 Fidelity Index">
                    { format!("Rf: {}", rf) }
                </span>
            }
            if let Some(rg) = props.rg {
                <span class="tm30-rg" title="TM-30 Gamut Index">
                    { format!("Rg: {}", rg) }
                </span>
            }
        </div>
    }
}
