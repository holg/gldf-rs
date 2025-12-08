//! Light source editor component for GLDF files

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
