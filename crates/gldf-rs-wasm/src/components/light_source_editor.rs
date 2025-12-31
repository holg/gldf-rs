//! Light source editor component for GLDF files

use crate::components::spectrum_viewer::SpectrumViewer;
use crate::state::use_gldf;
use yew::prelude::*;

/// Light source editor component
#[function_component(LightSourceEditor)]
pub fn light_source_editor() -> Html {
    let gldf = use_gldf();
    let light_sources = gldf.product.general_definitions.light_sources.as_ref();

    let active_tab = use_state(|| "fixed".to_string());

    let on_tab_click = {
        let active_tab = active_tab.clone();
        Callback::from(move |tab: String| {
            active_tab.set(tab);
        })
    };

    let fixed_count = light_sources
        .map(|ls| ls.fixed_light_source.len())
        .unwrap_or(0);
    let changeable_count = light_sources
        .map(|ls| ls.changeable_light_source.len())
        .unwrap_or(0);

    html! {
        <div class="editor-section light-source-editor">
            <h2>{ "Light Sources" }</h2>
            <p class="section-description">
                { "Define light sources for your luminaire. Fixed light sources are integrated into the product, while changeable light sources can be replaced." }
            </p>

            <div class="tabs">
                <button
                    class={classes!("tab", (*active_tab == "fixed").then_some("active"))}
                    onclick={{
                        let on_tab_click = on_tab_click.clone();
                        Callback::from(move |_| on_tab_click.emit("fixed".to_string()))
                    }}
                >
                    { format!("Fixed Light Sources ({})", fixed_count) }
                </button>
                <button
                    class={classes!("tab", (*active_tab == "changeable").then_some("active"))}
                    onclick={{
                        let on_tab_click = on_tab_click.clone();
                        Callback::from(move |_| on_tab_click.emit("changeable".to_string()))
                    }}
                >
                    { format!("Changeable Light Sources ({})", changeable_count) }
                </button>
            </div>

            <div class="tab-content">
                if *active_tab == "fixed" {
                    <FixedLightSourcesPanel />
                } else {
                    <ChangeableLightSourcesPanel />
                }
            </div>
        </div>
    }
}

/// Panel for fixed light sources
#[function_component(FixedLightSourcesPanel)]
fn fixed_light_sources_panel() -> Html {
    let gldf = use_gldf();
    let fixed_sources = gldf
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .map(|ls| ls.fixed_light_source.as_slice())
        .unwrap_or(&[]);

    // Get spectrums for lookup
    let spectrums = gldf
        .product
        .general_definitions
        .spectrums
        .as_ref()
        .map(|s| &s.spectrum[..])
        .unwrap_or(&[]);

    // State for expanded spectrum view
    let expanded_spectrum = use_state(|| None::<String>);

    html! {
        <div class="light-sources-panel">
            if fixed_sources.is_empty() {
                <p class="empty-message">{ "No fixed light sources defined." }</p>
            } else {
                <div class="light-source-cards">
                    { for fixed_sources.iter().map(|source| {
                        let name = source.name.locale.first()
                            .map(|l| l.value.clone())
                            .unwrap_or_else(|| "(No name)".to_string());
                        let description = source.description.as_ref()
                            .and_then(|d| d.locale.first())
                            .map(|l| l.value.clone())
                            .unwrap_or_default();

                        // Get TM-30 values from color information
                        let tm30 = source.color_information.as_ref()
                            .and_then(|ci| ci.iestm3015.as_ref())
                            .map(|tm| (tm.rf, tm.rg));

                        // Get spectrum reference
                        let spectrum_ref = source.spectrum_reference.as_ref()
                            .map(|sr| sr.spectrum_id.clone());

                        // Find the actual spectrum data
                        let spectrum_data = spectrum_ref.as_ref()
                            .and_then(|id| spectrums.iter().find(|s| &s.id == id));

                        // Check if this spectrum is expanded
                        let is_expanded = spectrum_ref.as_ref()
                            .map(|id| expanded_spectrum.as_ref() == Some(id))
                            .unwrap_or(false);

                        let on_toggle_spectrum = {
                            let expanded_spectrum = expanded_spectrum.clone();
                            let spectrum_id = spectrum_ref.clone();
                            Callback::from(move |_: MouseEvent| {
                                if let Some(id) = &spectrum_id {
                                    if expanded_spectrum.as_ref() == Some(id) {
                                        expanded_spectrum.set(None);
                                    } else {
                                        expanded_spectrum.set(Some(id.clone()));
                                    }
                                }
                            })
                        };

                        html! {
                            <div class="light-source-card" key={source.id.clone()}>
                                <div class="card-header">
                                    <span class="card-id">{ &source.id }</span>
                                    <span class="card-type">{ "Fixed" }</span>
                                </div>
                                <div class="card-body">
                                    <h4>{ name }</h4>
                                    if !description.is_empty() {
                                        <p class="description">{ description }</p>
                                    }
                                    if let Some(manufacturer) = &source.manufacturer {
                                        <div class="detail">
                                            <span class="label">{ "Manufacturer:" }</span>
                                            <span class="value">{ manufacturer }</span>
                                        </div>
                                    }
                                    if let Some(power) = &source.rated_input_power {
                                        <div class="detail">
                                            <span class="label">{ "Rated Power:" }</span>
                                            <span class="value">{ format!("{} W", power) }</span>
                                        </div>
                                    }
                                    if let Some(gtin) = &source.gtin {
                                        <div class="detail">
                                            <span class="label">{ "GTIN:" }</span>
                                            <span class="value">{ gtin }</span>
                                        </div>
                                    }
                                    if let Some(color_info) = &source.color_information {
                                        if let Some(cri) = &color_info.color_rendering_index {
                                            <div class="detail">
                                                <span class="label">{ "CRI:" }</span>
                                                <span class="value">{ *cri }</span>
                                            </div>
                                        }
                                        if let Some(cct) = &color_info.correlated_color_temperature {
                                            <div class="detail">
                                                <span class="label">{ "CCT:" }</span>
                                                <span class="value">{ format!("{} K", cct) }</span>
                                            </div>
                                        }
                                        // TM-30 values
                                        if let Some((rf, rg)) = tm30 {
                                            <div class="detail tm30-values">
                                                <span class="label">{ "TM-30:" }</span>
                                                <span class="value tm30">
                                                    <span class="rf" title="Fidelity Index">{ format!("Rf {}", rf) }</span>
                                                    <span class="rg" title="Gamut Index">{ format!("Rg {}", rg) }</span>
                                                </span>
                                            </div>
                                        }
                                        // CIE chromaticity
                                        if let Some(coords) = &color_info.rated_chromacity_coordinate_values {
                                            <div class="detail">
                                                <span class="label">{ "CIE 1931:" }</span>
                                                <span class="value">{ format!("x={:.4}, y={:.4}", coords.x, coords.y) }</span>
                                            </div>
                                        }
                                        // Melanopic factor
                                        if let Some(melanopic) = &color_info.melanopic_factor {
                                            <div class="detail">
                                                <span class="label">{ "Melanopic:" }</span>
                                                <span class="value">{ format!("{:.2}", melanopic) }</span>
                                            </div>
                                        }
                                    }
                                    // Spectrum reference with toggle button
                                    if let Some(ref spec_id) = spectrum_ref {
                                        <div class="detail spectrum-ref">
                                            <span class="label">{ "Spectrum:" }</span>
                                            <button
                                                class={classes!("spectrum-toggle", is_expanded.then_some("expanded"))}
                                                onclick={on_toggle_spectrum}
                                                title="Click to view spectral power distribution"
                                            >
                                                { spec_id }
                                                <span class="toggle-icon">{ if is_expanded { "▼" } else { "▶" } }</span>
                                            </button>
                                        </div>
                                    }
                                    // Expanded spectrum viewer
                                    if is_expanded {
                                        if let Some(spectrum) = spectrum_data {
                                            <div class="spectrum-viewer-container">
                                                <SpectrumViewer
                                                    spectrum={spectrum.clone()}
                                                    width={400.0}
                                                    height={250.0}
                                                    rf={tm30.map(|(rf, _)| rf)}
                                                    rg={tm30.map(|(_, rg)| rg)}
                                                    cct={source.color_information.as_ref().and_then(|ci| ci.correlated_color_temperature)}
                                                    cri={source.color_information.as_ref().and_then(|ci| ci.color_rendering_index)}
                                                />
                                            </div>
                                        }
                                    }
                                </div>
                                <div class="card-actions">
                                    <button class="btn-edit" disabled=true title="Editing coming soon">{ "Edit" }</button>
                                </div>
                            </div>
                        }
                    })}
                </div>
            }
            <button class="btn-add" disabled=true title="Adding coming soon">{ "+ Add Fixed Light Source" }</button>
        </div>
    }
}

/// Panel for changeable light sources
#[function_component(ChangeableLightSourcesPanel)]
fn changeable_light_sources_panel() -> Html {
    let gldf = use_gldf();
    let changeable_sources = gldf
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .map(|ls| ls.changeable_light_source.as_slice())
        .unwrap_or(&[]);

    html! {
        <div class="light-sources-panel">
            if changeable_sources.is_empty() {
                <p class="empty-message">{ "No changeable light sources defined." }</p>
            } else {
                <div class="light-source-cards">
                    { for changeable_sources.iter().map(|source| {
                        let name = source.name.value.clone();
                        let description = source.description.as_ref()
                            .map(|d| d.value.clone())
                            .unwrap_or_default();

                        html! {
                            <div class="light-source-card" key={source.id.clone()}>
                                <div class="card-header">
                                    <span class="card-id">{ &source.id }</span>
                                    <span class="card-type changeable">{ "Changeable" }</span>
                                </div>
                                <div class="card-body">
                                    <h4>{ name }</h4>
                                    if !description.is_empty() {
                                        <p class="description">{ description }</p>
                                    }
                                    if let Some(manufacturer) = &source.manufacturer {
                                        <div class="detail">
                                            <span class="label">{ "Manufacturer:" }</span>
                                            <span class="value">{ manufacturer }</span>
                                        </div>
                                    }
                                    if let Some(photometry_ref) = &source.photometry_reference {
                                        <div class="detail">
                                            <span class="label">{ "Photometry:" }</span>
                                            <span class="value">{ &photometry_ref.photometry_id }</span>
                                        </div>
                                    }
                                </div>
                                <div class="card-actions">
                                    <button class="btn-edit" disabled=true title="Editing coming soon">{ "Edit" }</button>
                                </div>
                            </div>
                        }
                    })}
                </div>
            }
            <button class="btn-add" disabled=true title="Adding coming soon">{ "+ Add Changeable Light Source" }</button>
        </div>
    }
}
