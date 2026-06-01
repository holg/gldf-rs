//! Overview page — the user's "what is this file?" answer.
//!
//! Mirrors `view_overview` + `view_files_overview` + `view_light_sources_overview`
//! + `view_variants_overview` from the Yew side.

use crate::components::LocaleFooView;
use crate::state::use_gldf;
use leptos::prelude::*;

#[component]
pub fn OverviewPage() -> impl IntoView {
    let state = use_gldf();

    view! {
        {move || state.with(|s| {
            let header = &s.product.header;
            let files_count = s.embedded_files.len();
            let ls = s.product.general_definitions.light_sources.as_ref();
            let fixed_ls_count = ls.map(|l| l.fixed_light_source.len()).unwrap_or(0);
            let changeable_ls_count = ls.map(|l| l.changeable_light_source.len()).unwrap_or(0);
            let variants_count = s
                .product
                .product_definitions
                .variants
                .as_ref()
                .map(|v| v.variant.len())
                .unwrap_or(0);
            let photometries_count = s
                .product
                .general_definitions
                .photometries
                .as_ref()
                .map(|p| p.photometry.len())
                .unwrap_or(0);

            let manufacturer = if header.manufacturer.is_empty() {
                "Unknown Manufacturer".to_string()
            } else {
                header.manufacturer.clone()
            };
            let author = if header.author.is_empty() { "—".to_string() } else { header.author.clone() };
            let created_with = if header.created_with_application.is_empty() {
                "—".to_string()
            } else {
                header.created_with_application.clone()
            };
            let creation_time = if header.creation_time_code.is_empty() {
                "—".to_string()
            } else {
                header.creation_time_code.clone()
            };
            let format_version = format!("{:?}", header.format_version);

            view! {
                <>
                    <div class="card product-info-card">
                        <div class="card-body">
                            <div class="product-info-header">
                                <span class="icon">{ "💡" }</span>
                                <div>
                                    <h2>{ manufacturer }</h2>
                                    <p class="format-version">{ format!("GLDF Format {}", format_version.clone()) }</p>
                                </div>
                            </div>
                            <div class="info-grid">
                                <div class="info-row">
                                    <span class="label">{ "Author" }</span>
                                    <span class="value">{ author }</span>
                                </div>
                                <div class="info-row">
                                    <span class="label">{ "Created With" }</span>
                                    <span class="value">{ created_with }</span>
                                </div>
                                <div class="info-row">
                                    <span class="label">{ "Creation Date" }</span>
                                    <span class="value">{ creation_time }</span>
                                </div>
                                <div class="info-row">
                                    <span class="label">{ "Format Version" }</span>
                                    <span class="value">{ format_version }</span>
                                </div>
                            </div>
                        </div>
                    </div>

                    <div class="stats-grid">
                        <div class="stat-card">
                            <div class="icon blue">{ "📁" }</div>
                            <div class="value">{ files_count }</div>
                            <div class="label">{ "Files" }</div>
                        </div>
                        <div class="stat-card">
                            <div class="icon yellow">{ "💡" }</div>
                            <div class="value">{ fixed_ls_count + changeable_ls_count }</div>
                            <div class="label">{ "Light Sources" }</div>
                        </div>
                        <div class="stat-card">
                            <div class="icon purple">{ "📦" }</div>
                            <div class="value">{ variants_count }</div>
                            <div class="label">{ "Variants" }</div>
                        </div>
                        <div class="stat-card">
                            <div class="icon orange">{ "☀️" }</div>
                            <div class="value">{ photometries_count }</div>
                            <div class="label">{ "Photometries" }</div>
                        </div>
                    </div>

                    <div class="content-columns">
                        <div class="content-column">
                            <FilesOverviewCard />
                            <LightSourcesOverviewCard />
                        </div>
                        <div class="content-column">
                            <VariantsOverviewCard />
                        </div>
                    </div>
                </>
            }
        })}
    }
}

#[component]
fn FilesOverviewCard() -> impl IntoView {
    let state = use_gldf();

    view! {
        {move || state.with(|s| {
            // The embedded_files key is the <File id="..."> id. To classify by
            // type, we walk product.general_definitions.files.file (the XML
            // declarations) and use their declared file_name.
            let files = &s.product.general_definitions.files.file;
            let photometric: Vec<String> = files
                .iter()
                .filter(|f| f.file_name.ends_with(".ldt") || f.file_name.ends_with(".ies"))
                .map(|f| f.file_name.clone())
                .collect();
            let images: Vec<String> = files
                .iter()
                .filter(|f| f.file_name.ends_with(".jpg") || f.file_name.ends_with(".png") || f.file_name.ends_with(".jpeg"))
                .map(|f| f.file_name.clone())
                .collect();
            let geometry: Vec<String> = files
                .iter()
                .filter(|f| f.file_name.ends_with(".l3d"))
                .map(|f| f.file_name.clone())
                .collect();
            let total = files.len();

            view! {
                <div class="collapsible open">
                    <div class="collapsible-header">
                        <span class="icon blue">{ "📁" }</span>
                        <h4>{ "Files" }</h4>
                        <span class="count">{ total }</span>
                        <span class="chevron">{ "▶" }</span>
                    </div>
                    <div class="collapsible-content">
                        <FileCategorySection title="Photometric" icon="☀️" color="var(--accent-orange)" names=photometric />
                        <FileCategorySection title="Images" icon="🖼" color="var(--accent-blue)" names=images />
                        <FileCategorySection title="Geometry" icon="🧊" color="var(--accent-green)" names=geometry />
                    </div>
                </div>
            }
        })}
    }
}

#[component]
fn FileCategorySection(
    title: &'static str,
    icon: &'static str,
    color: &'static str,
    names: Vec<String>,
) -> impl IntoView {
    if names.is_empty() {
        return view! { <></> }.into_any();
    }
    let total = names.len();
    let visible = names.iter().take(3).cloned().collect::<Vec<_>>();
    let extra = if total > 3 {
        Some(format!("+ {} more...", total - 3))
    } else {
        None
    };
    view! {
        <div class="file-category">
            <div class="file-category-title">{ title }</div>
            {visible.into_iter().map(|name| view! {
                <div class="file-item">
                    <span class="icon" style=format!("color: {};", color)>{ icon }</span>
                    <span class="name">{ name }</span>
                </div>
            }).collect::<Vec<_>>()}
            {extra.map(|e| view! { <div class="more-items">{ e }</div> })}
        </div>
    }.into_any()
}

#[component]
fn LightSourcesOverviewCard() -> impl IntoView {
    let state = use_gldf();

    view! {
        {move || state.with(|s| {
            let ls = s.product.general_definitions.light_sources.as_ref();
            let fixed = ls
                .map(|l| l.fixed_light_source.clone())
                .unwrap_or_default();
            let changeable = ls
                .map(|l| l.changeable_light_source.clone())
                .unwrap_or_default();
            let total = fixed.len() + changeable.len();

            view! {
                <div class="collapsible open">
                    <div class="collapsible-header">
                        <span class="icon yellow">{ "💡" }</span>
                        <h4>{ "Light Sources" }</h4>
                        <span class="count">{ total }</span>
                        <span class="chevron">{ "▶" }</span>
                    </div>
                    <div class="collapsible-content">
                        {if !fixed.is_empty() {
                            let take = fixed.iter().take(5).cloned().collect::<Vec<_>>();
                            let extra = if fixed.len() > 5 {
                                Some(format!("+ {} more...", fixed.len() - 5))
                            } else { None };
                            view! {
                                <div class="file-category-title">{ "Fixed" }</div>
                                {take.into_iter().map(|ls| view! {
                                    <div class="light-source-item">
                                        <span class="icon" style="color: var(--accent-yellow);">{ "💡" }</span>
                                        <div class="info">
                                            <div class="name">
                                                <LocaleFooView value=ls.name compact=true />
                                            </div>
                                            <div class="id">{ format!("ID: {}", ls.id) }</div>
                                        </div>
                                    </div>
                                }).collect::<Vec<_>>()}
                                {extra.map(|e| view! { <div class="more-items">{ e }</div> })}
                            }.into_any()
                        } else { ().into_any() }}
                        {if !changeable.is_empty() {
                            let take = changeable.iter().take(5).cloned().collect::<Vec<_>>();
                            let extra = if changeable.len() > 5 {
                                Some(format!("+ {} more...", changeable.len() - 5))
                            } else { None };
                            view! {
                                <div class="file-category-title" style="margin-top: 12px;">{ "Changeable" }</div>
                                {take.into_iter().map(|ls| view! {
                                    <div class="light-source-item">
                                        <span class="icon" style="color: var(--accent-orange);">{ "💡" }</span>
                                        <div class="info">
                                            <div class="name">
                                                <LocaleFooView value=ls.name compact=true />
                                            </div>
                                            <div class="id">{ format!("ID: {}", ls.id) }</div>
                                        </div>
                                    </div>
                                }).collect::<Vec<_>>()}
                                {extra.map(|e| view! { <div class="more-items">{ e }</div> })}
                            }.into_any()
                        } else { ().into_any() }}
                        {if total == 0 {
                            view! { <p style="color: var(--text-tertiary); font-size: 13px;">{ "No light sources defined" }</p> }.into_any()
                        } else { ().into_any() }}
                    </div>
                </div>
            }
        })}
    }
}

#[component]
fn VariantsOverviewCard() -> impl IntoView {
    let state = use_gldf();

    view! {
        {move || state.with(|s| {
            let variants = s
                .product
                .product_definitions
                .variants
                .as_ref()
                .map(|v| v.variant.clone())
                .unwrap_or_default();
            let count = variants.len();
            let take = variants.iter().take(10).cloned().collect::<Vec<_>>();
            let extra = if count > 10 {
                Some(format!("+ {} more...", count - 10))
            } else { None };

            view! {
                <div class="collapsible open">
                    <div class="collapsible-header">
                        <span class="icon purple">{ "📦" }</span>
                        <h4>{ "Variants" }</h4>
                        <span class="count">{ count }</span>
                        <span class="chevron">{ "▶" }</span>
                    </div>
                    <div class="collapsible-content">
                        {if variants.is_empty() {
                            view! { <p style="color: var(--text-tertiary); font-size: 13px;">{ "No variants defined" }</p> }.into_any()
                        } else {
                            view! {
                                {take.into_iter().map(|v| {
                                    let id_str = v.id.clone();
                                    let name = v.name.clone();
                                    let desc = v.description.clone();
                                    view! {
                                        <div class="variant-item">
                                            <div class="variant-item-header">
                                                <span class="icon">{ "📦" }</span>
                                                <span class="name">
                                                    {match name {
                                                        Some(n) => view! { <LocaleFooView value=n compact=true /> }.into_any(),
                                                        None => view! { <span>{ id_str.clone() }</span> }.into_any(),
                                                    }}
                                                </span>
                                            </div>
                                            {desc.map(|d| view! {
                                                <div class="description">
                                                    <LocaleFooView value=d compact=true />
                                                </div>
                                            })}
                                            <div class="id">{ format!("ID: {}", id_str) }</div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                                {extra.map(|e| view! { <div class="more-items">{ e }</div> })}
                            }.into_any()
                        }}
                    </div>
                </div>
            }
        })}
    }
}
