//! Photometry editor component for GLDF files
//!
//! Displays and edits DescriptivePhotometry values (CIE Flux Code, LOR, UGR, etc.)
//! Shows both GLDF values (blue) and calculated values from LDT/IES (orange).

use crate::state::{use_gldf, GldfAction};
use eulumdat::{Eulumdat, GldfPhotometricData, IesParser};
use gloo::console::log;
use yew::prelude::*;

/// Value source indicator for styling
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq)]
pub enum ValueSource {
    /// Value from GLDF file
    Gldf,
    /// Value calculated from Eulumdat/IES
    Calculated,
    /// No value available
    Empty,
}

/// Calculated photometric values from LDT/IES file
#[derive(Clone, Default)]
pub struct CalculatedPhotometry {
    pub cie_flux_code: Option<String>,
    pub light_output_ratio: Option<f64>,
    pub luminous_efficacy: Option<f64>,
    pub downward_flux_fraction: Option<f64>,
    pub downward_light_output_ratio: Option<f64>,
    pub upward_light_output_ratio: Option<f64>,
    pub cut_off_angle: Option<f64>,
    pub luminaire_luminance: Option<f64>,
    pub photometric_code: Option<String>,
    pub bug_rating: Option<String>,
}

impl CalculatedPhotometry {
    /// Calculate photometric values from Eulumdat data
    pub fn from_eulumdat(ldt: &Eulumdat) -> Self {
        let gldf_data = GldfPhotometricData::from_eulumdat(ldt);
        Self {
            cie_flux_code: Some(gldf_data.cie_flux_code),
            light_output_ratio: Some(gldf_data.light_output_ratio / 100.0), // Convert to fraction
            luminous_efficacy: Some(gldf_data.luminous_efficacy),
            downward_flux_fraction: Some(gldf_data.downward_flux_fraction / 100.0),
            downward_light_output_ratio: Some(gldf_data.downward_light_output_ratio / 100.0),
            upward_light_output_ratio: Some(gldf_data.upward_light_output_ratio / 100.0),
            cut_off_angle: Some(gldf_data.cut_off_angle),
            luminaire_luminance: Some(gldf_data.luminaire_luminance),
            photometric_code: Some(gldf_data.photometric_code),
            bug_rating: Some(gldf_data.light_distribution_bug_rating),
        }
    }
}

/// Parse LDT or IES file content
fn parse_photometry_file(content: &[u8]) -> Option<Eulumdat> {
    // Convert bytes to string
    let content_str = std::str::from_utf8(content).ok()?;

    // Try parsing as LDT first
    if let Ok(ldt) = Eulumdat::parse(content_str) {
        return Some(ldt);
    }
    // Try parsing as IES (IesParser::parse returns Result<Eulumdat>)
    if let Ok(ies) = IesParser::parse(content_str) {
        return Some(ies);
    }
    None
}

/// Render a single photometry field with both GLDF and calculated values
#[allow(clippy::too_many_arguments)]
fn render_dual_field(
    label: &str,
    gldf_value: Option<String>,
    calc_value: Option<String>,
    unit: Option<&str>,
    help: Option<&str>,
    input_type: &str,
    step: Option<&str>,
    onchange: Callback<String>,
) -> Html {
    let has_gldf = gldf_value.is_some();
    let has_calc = calc_value.is_some();

    let gldf_display = gldf_value.clone().unwrap_or_default();
    let calc_display = calc_value.clone().unwrap_or_default();

    // Use GLDF value for input, or calculated as fallback display
    let input_value = if has_gldf {
        gldf_display.clone()
    } else {
        String::new()
    };

    let label = label.to_string();
    let input_type = input_type.to_string();
    let step = step.map(|s| s.to_string()).unwrap_or_default();
    let unit = unit.map(|u| u.to_string());
    let help = help.map(|h| h.to_string());

    let on_input = Callback::from(move |e: Event| {
        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
        onchange.emit(input.value());
    });

    html! {
        <div class="photometry-field-dual">
            <div class="field-header">
                <label>{ &label }</label>
            </div>
            <div class="field-values-row">
                // GLDF value (editable)
                <div class={classes!("field-value-box", if has_gldf { "value-source-gldf" } else { "value-source-empty" })}>
                    <span class="value-source-badge">{ "GLDF" }</span>
                    <div class="field-input-group">
                        <input
                            type={input_type.clone()}
                            value={input_value}
                            onchange={on_input}
                            step={step.clone()}
                            class="photometry-input"
                            placeholder={if has_calc { calc_display.clone() } else { "-".to_string() }}
                        />
                        if let Some(ref u) = unit {
                            <span class="field-unit">{ u }</span>
                        }
                    </div>
                </div>
                // Calculated value (read-only)
                <div class={classes!("field-value-box", if has_calc { "value-source-calculated" } else { "value-source-empty" })}>
                    <span class="value-source-badge">{ "Calc" }</span>
                    <div class="field-display-group">
                        <span class="calculated-value">{ if has_calc { &calc_display } else { "-" } }</span>
                        if let Some(ref u) = unit {
                            <span class="field-unit">{ u }</span>
                        }
                    </div>
                </div>
            </div>
            if let Some(ref h) = help {
                <small class="field-help">{ h }</small>
            }
        </div>
    }
}

/// Props for PhotometryEditor
#[derive(Properties, Clone, PartialEq)]
pub struct PhotometryEditorProps {
    /// Pre-parsed photometry files as (file_id, content) pairs
    /// We use strings to avoid BufFile not implementing PartialEq
    #[prop_or_default]
    pub photometry_files: Vec<(String, Vec<u8>)>,
}

/// Photometry editor component
#[function_component(PhotometryEditor)]
pub fn photometry_editor(props: &PhotometryEditorProps) -> Html {
    let gldf = use_gldf();
    let selected_index = use_state(|| 0usize);

    // Get photometries
    let photometries = gldf
        .product
        .general_definitions
        .photometries
        .as_ref()
        .map(|p| &p.photometry)
        .cloned()
        .unwrap_or_default();

    let photometry_count = photometries.len();

    // Get current photometry's descriptive data
    let current_photometry = photometries.get(*selected_index);
    let desc = current_photometry.and_then(|p| p.descriptive_photometry.as_ref());

    // Get the file reference for this photometry
    let file_id = current_photometry
        .and_then(|p| p.photometry_file_reference.as_ref())
        .map(|f| f.file_id.clone());

    // Find and parse the LDT/IES file
    let calculated = use_memo(
        (file_id.clone(), props.photometry_files.clone()),
        |(file_id, files)| {
            log!(format!(
                "PhotometryEditor: Looking for file_id={:?}, available files: {:?}",
                file_id,
                files.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>()
            ));

            if let Some(ref fid) = file_id {
                // Find file in the container - try exact match first, then partial match
                for (id, content) in files {
                    let matches = id == fid || id.contains(fid) || fid.contains(id);
                    if matches {
                        log!(format!(
                            "PhotometryEditor: Found matching file id={} for fid={}",
                            id, fid
                        ));
                        if let Some(ldt) = parse_photometry_file(content) {
                            log!(format!(
                                "PhotometryEditor: Successfully parsed LDT, calculating values..."
                            ));
                            return CalculatedPhotometry::from_eulumdat(&ldt);
                        } else {
                            log!(format!("PhotometryEditor: Failed to parse file as LDT/IES"));
                        }
                    }
                }
                log!(format!(
                    "PhotometryEditor: No matching file found for fid={}",
                    fid
                ));
            }
            CalculatedPhotometry::default()
        },
    );

    // Extract GLDF values
    let cie_flux_code = desc.and_then(|d| d.cie_flux_code.clone());
    let lor = desc.and_then(|d| d.light_output_ratio);
    let efficacy = desc.and_then(|d| d.luminous_efficacy);
    let dff = desc.and_then(|d| d.downward_flux_fraction);
    let dlor = desc.and_then(|d| d.downward_light_output_ratio);
    let ulor = desc.and_then(|d| d.upward_light_output_ratio);
    let cut_off = desc.and_then(|d| d.cut_off_angle);
    let luminance = desc.and_then(|d| d.luminaire_luminance);
    let ugr = desc.and_then(|d| d.ugr4_h8_h705020_lq.as_ref());
    let photometric_code = desc.and_then(|d| d.photometric_code.clone());
    let bug_rating = desc.and_then(|d| d.light_distribution_bug_rating.clone());

    // Callbacks for editing
    let index = *selected_index;

    let on_cie_flux_change = {
        let gldf = gldf.clone();
        Callback::from(move |value: String| {
            gldf.dispatch(GldfAction::SetPhotometryCieFluxCode {
                index,
                value: if value.is_empty() { None } else { Some(value) },
            });
        })
    };

    let on_lor_change = {
        let gldf = gldf.clone();
        Callback::from(move |value: String| {
            gldf.dispatch(GldfAction::SetPhotometryLightOutputRatio {
                index,
                value: value.parse::<f64>().ok().map(|v| v / 100.0),
            });
        })
    };

    let on_efficacy_change = {
        let gldf = gldf.clone();
        Callback::from(move |value: String| {
            gldf.dispatch(GldfAction::SetPhotometryLuminousEfficacy {
                index,
                value: value.parse().ok(),
            });
        })
    };

    let on_dff_change = {
        let gldf = gldf.clone();
        Callback::from(move |value: String| {
            gldf.dispatch(GldfAction::SetPhotometryDownwardFluxFraction {
                index,
                value: value.parse::<f64>().ok().map(|v| v / 100.0),
            });
        })
    };

    let on_dlor_change = {
        let gldf = gldf.clone();
        Callback::from(move |value: String| {
            gldf.dispatch(GldfAction::SetPhotometryDownwardLOR {
                index,
                value: value.parse::<f64>().ok().map(|v| v / 100.0),
            });
        })
    };

    let on_ulor_change = {
        let gldf = gldf.clone();
        Callback::from(move |value: String| {
            gldf.dispatch(GldfAction::SetPhotometryUpwardLOR {
                index,
                value: value.parse::<f64>().ok().map(|v| v / 100.0),
            });
        })
    };

    let on_cutoff_change = {
        let gldf = gldf.clone();
        Callback::from(move |value: String| {
            gldf.dispatch(GldfAction::SetPhotometryCutOffAngle {
                index,
                value: value.parse().ok(),
            });
        })
    };

    let on_luminance_change = {
        let gldf = gldf.clone();
        Callback::from(move |value: String| {
            gldf.dispatch(GldfAction::SetPhotometryLuminaireLuminance {
                index,
                value: value.parse().ok(),
            });
        })
    };

    let on_ugr_x_change = {
        let gldf = gldf.clone();
        Callback::from(move |value: String| {
            gldf.dispatch(GldfAction::SetPhotometryUgrX {
                index,
                value: value.parse().ok(),
            });
        })
    };

    let on_ugr_y_change = {
        let gldf = gldf.clone();
        Callback::from(move |value: String| {
            gldf.dispatch(GldfAction::SetPhotometryUgrY {
                index,
                value: value.parse().ok(),
            });
        })
    };

    let on_photometric_code_change = {
        let gldf = gldf.clone();
        Callback::from(move |value: String| {
            gldf.dispatch(GldfAction::SetPhotometryPhotometricCode {
                index,
                value: if value.is_empty() { None } else { Some(value) },
            });
        })
    };

    let on_bug_rating_change = {
        let gldf = gldf.clone();
        Callback::from(move |value: String| {
            gldf.dispatch(GldfAction::SetPhotometryBugRating {
                index,
                value: if value.is_empty() { None } else { Some(value) },
            });
        })
    };

    // Photometry selector
    let on_select_photometry = {
        let selected_index = selected_index.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            if let Ok(idx) = select.value().parse::<usize>() {
                selected_index.set(idx);
            }
        })
    };

    // Check if we have calculated values
    let has_calculated = calculated.cie_flux_code.is_some();

    html! {
        <div class="editor-section photometry-editor">
            <h2>{ "Photometry Data" }</h2>

            if photometry_count == 0 {
                <p class="empty-message">{ "No photometry definitions found" }</p>
            } else {
                // Photometry selector
                if photometry_count > 1 {
                    <div class="form-group">
                        <label>{ "Select Photometry" }</label>
                        <select onchange={on_select_photometry} class="photometry-select">
                            { for photometries.iter().enumerate().map(|(i, p)| {
                                let file_ref = p.photometry_file_reference.as_ref()
                                    .map(|f| f.file_id.clone())
                                    .unwrap_or_else(|| format!("Photometry {}", i + 1));
                                html! {
                                    <option value={i.to_string()} selected={i == *selected_index}>
                                        { format!("{}: {}", p.id, file_ref) }
                                    </option>
                                }
                            })}
                        </select>
                    </div>
                }

                // Current photometry info
                if let Some(phot) = current_photometry {
                    <div class="photometry-info">
                        <span class="photometry-id">{ format!("ID: {}", phot.id) }</span>
                        if let Some(ref file_ref) = phot.photometry_file_reference {
                            <span class="photometry-file">{ format!("File: {}", file_ref.file_id) }</span>
                        }
                    </div>
                }

                <div class="value-source-legend">
                    <span class="legend-item">
                        <span class="legend-badge value-source-gldf">{ "GLDF" }</span>
                        { " = Value from GLDF file (editable)" }
                    </span>
                    <span class="legend-item">
                        <span class="legend-badge value-source-calculated">{ "Calc" }</span>
                        { if has_calculated { " = Calculated from LDT/IES" } else { " = No LDT/IES file" } }
                    </span>
                </div>

                // Primary metrics
                <fieldset class="photometry-fieldset">
                    <legend>{ "Light Output" }</legend>
                    <div class="photometry-grid-dual">
                        { render_dual_field(
                            "CIE Flux Code",
                            cie_flux_code.clone(),
                            calculated.cie_flux_code.clone(),
                            None,
                            Some("5-digit code (e.g., 45 72 95 100 100)"),
                            "text",
                            None,
                            on_cie_flux_change,
                        )}
                        { render_dual_field(
                            "Light Output Ratio",
                            lor.map(|v| format!("{:.1}", v * 100.0)),
                            calculated.light_output_ratio.map(|v| format!("{:.1}", v * 100.0)),
                            Some("%"),
                            Some("LOR - Total efficiency"),
                            "number",
                            Some("0.1"),
                            on_lor_change,
                        )}
                        { render_dual_field(
                            "Luminous Efficacy",
                            efficacy.map(|v| format!("{:.1}", v)),
                            calculated.luminous_efficacy.map(|v| format!("{:.1}", v)),
                            Some("lm/W"),
                            None,
                            "number",
                            Some("0.1"),
                            on_efficacy_change,
                        )}
                    </div>
                </fieldset>

                // Light distribution
                <fieldset class="photometry-fieldset">
                    <legend>{ "Light Distribution" }</legend>
                    <div class="photometry-grid-dual">
                        { render_dual_field(
                            "Downward Flux Fraction",
                            dff.map(|v| format!("{:.1}", v * 100.0)),
                            calculated.downward_flux_fraction.map(|v| format!("{:.1}", v * 100.0)),
                            Some("%"),
                            Some("DFF"),
                            "number",
                            Some("0.1"),
                            on_dff_change,
                        )}
                        { render_dual_field(
                            "Downward LOR",
                            dlor.map(|v| format!("{:.1}", v * 100.0)),
                            calculated.downward_light_output_ratio.map(|v| format!("{:.1}", v * 100.0)),
                            Some("%"),
                            Some("DLOR"),
                            "number",
                            Some("0.1"),
                            on_dlor_change,
                        )}
                        { render_dual_field(
                            "Upward LOR",
                            ulor.map(|v| format!("{:.1}", v * 100.0)),
                            calculated.upward_light_output_ratio.map(|v| format!("{:.1}", v * 100.0)),
                            Some("%"),
                            Some("ULOR"),
                            "number",
                            Some("0.1"),
                            on_ulor_change,
                        )}
                        { render_dual_field(
                            "Cut-off Angle",
                            cut_off.map(|v| format!("{:.0}", v)),
                            calculated.cut_off_angle.map(|v| format!("{:.0}", v)),
                            Some("°"),
                            None,
                            "number",
                            Some("1"),
                            on_cutoff_change,
                        )}
                    </div>
                </fieldset>

                // Glare & luminance
                <fieldset class="photometry-fieldset">
                    <legend>{ "Glare & Luminance" }</legend>
                    <div class="photometry-grid-dual">
                        { render_dual_field(
                            "Luminaire Luminance",
                            luminance.map(|v| format!("{:.0}", v)),
                            calculated.luminaire_luminance.map(|v| format!("{:.0}", v)),
                            Some("cd/m²"),
                            None,
                            "number",
                            None,
                            on_luminance_change,
                        )}
                        { render_dual_field(
                            "UGR (X)",
                            ugr.and_then(|u| u.x).map(|v| format!("{:.1}", v)),
                            None, // UGR calculation is complex, skip for now
                            None,
                            Some("Unified Glare Rating X"),
                            "number",
                            Some("0.1"),
                            on_ugr_x_change,
                        )}
                        { render_dual_field(
                            "UGR (Y)",
                            ugr.and_then(|u| u.y).map(|v| format!("{:.1}", v)),
                            None, // UGR calculation is complex, skip for now
                            None,
                            Some("Unified Glare Rating Y"),
                            "number",
                            Some("0.1"),
                            on_ugr_y_change,
                        )}
                    </div>
                </fieldset>

                // Classification codes
                <fieldset class="photometry-fieldset">
                    <legend>{ "Classification" }</legend>
                    <div class="photometry-grid-dual">
                        { render_dual_field(
                            "Photometric Code",
                            photometric_code.clone(),
                            calculated.photometric_code.clone(),
                            None,
                            None,
                            "text",
                            None,
                            on_photometric_code_change,
                        )}
                        { render_dual_field(
                            "BUG Rating",
                            bug_rating.clone(),
                            calculated.bug_rating.clone(),
                            None,
                            Some("Backlight-Uplight-Glare"),
                            "text",
                            None,
                            on_bug_rating_change,
                        )}
                    </div>
                </fieldset>
            }
        </div>
    }
}
