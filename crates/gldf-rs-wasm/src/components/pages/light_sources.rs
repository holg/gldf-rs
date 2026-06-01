//! Light Sources page — fixed + changeable light source cards.
//!
//! Mirrors `view_light_sources` from the Yew side. Card per source with
//! id, type badge, name/description, and a property grid covering
//! manufacturer, gtin, power, voltage, color (CCT / CRI), maintenance
//! (lifetime / lamp type), spectrum reference, energy labels, position
//! of usage, Zhaga flag.
//!
//! The spectrum row is interactive: a dropdown picks one of the declared
//! `<Spectrum>` ids and a drop-zone embeds a fresh `.spd` and links it to
//! this light source. This is the spot in the GLDF model where the
//! photometry/spectrum/light-source triangle is actually wired up.

use crate::components::pages::link_hint::LinkHint;
use crate::components::{EditModal, IdRenameChip, LocaleFooView};
use crate::state::{use_gldf, GldfAction};
use std::sync::Arc;
use gldf_rs::gldf::general_definitions::electrical::Voltage;
use gldf_rs::gldf::general_definitions::lightsources::{
    ChangeableLightSource, ColorInformation, FixedLightSource, LightSourceMaintenance,
    MultiChannelLightSource,
};
use gldf_rs::gldf::header::{Locale, LocaleFoo};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};

#[component]
pub fn LightSourcesPage(#[prop(default = false)] editable: bool) -> impl IntoView {
    let state = use_gldf();

    let on_add_fixed = move |_| {
        state.update(|s| {
            s.apply(GldfAction::AddFixedLightSource {
                payload: Box::new(FixedLightSource::default()),
            });
        });
    };
    let on_add_changeable = move |_| {
        state.update(|s| {
            s.apply(GldfAction::AddChangeableLightSource {
                payload: Box::new(ChangeableLightSource::default()),
            });
        });
    };
    let on_add_multi_channel = move |_| {
        state.update(|s| {
            s.apply(GldfAction::AddMultiChannelLightSource {
                payload: Box::new(MultiChannelLightSource::default()),
            });
        });
    };

    view! {
        {move || state.with(|s| {
            let ls = s.product.general_definitions.light_sources.as_ref();
            let fixed: Vec<FixedLightSource> = ls
                .map(|l| l.fixed_light_source.clone())
                .unwrap_or_default();
            let changeable: Vec<ChangeableLightSource> = ls
                .map(|l| l.changeable_light_source.clone())
                .unwrap_or_default();
            let multi_channel: Vec<MultiChannelLightSource> = ls
                .map(|l| l.multi_channel_light_source.clone())
                .unwrap_or_default();

            let toolbar = if editable {
                view! {
                    <div class="light-sources-toolbar" style="display: flex; gap: 8px; margin: 0 0 12px 0;">
                        <button
                            on:click=on_add_fixed
                            style="padding: 4px 10px; font-size: 12px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;"
                        >
                            { "+ New Fixed LightSource" }
                        </button>
                        <button
                            on:click=on_add_changeable
                            style="padding: 4px 10px; font-size: 12px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;"
                        >
                            { "+ New Changeable LightSource" }
                        </button>
                        <button
                            on:click=on_add_multi_channel
                            style="padding: 4px 10px; font-size: 12px; background: var(--accent-purple); color: white; border: none; border-radius: 4px; cursor: pointer;"
                        >
                            { "+ New MultiChannel LightSource" }
                        </button>
                    </div>
                }.into_any()
            } else { ().into_any() };

            if fixed.is_empty() && changeable.is_empty() && multi_channel.is_empty() {
                return view! {
                    <>
                        { toolbar }
                        <div class="empty-state">
                            <div class="icon">{ "💡" }</div>
                            <h3>{ "No Light Sources" }</h3>
                            <p>{
                                if editable { "Click + New … above to add the first one." }
                                else { "This GLDF file has no light source definitions" }
                            }</p>
                        </div>
                    </>
                }.into_any();
            }

            view! {
                <>
                    <LinkHint
                        title="This is where photometry, spectrum, and light source come together.".to_string()
                        body="A FixedLightSource / ChangeableLightSource is where the GLDF schema lets you attach a SpectrumReference. The Spectrum row below is interactive — pick one of the declared spectra or drop a .spd file to embed and link it in one step. Emitters then pair this light source with a photometry to compose the beam.".to_string()
                    />
                    { toolbar }
                    {if !fixed.is_empty() {
                        view! {
                            <h3 style="margin-bottom: 16px; color: var(--text-secondary);">{ "Fixed Light Sources" }</h3>
                            <div class="light-source-cards">
                                {fixed.into_iter().map(|ls| view! { <FixedCard ls=ls editable=editable /> }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    } else { ().into_any() }}
                    {if !changeable.is_empty() {
                        view! {
                            <h3 style="margin: 24px 0 16px; color: var(--text-secondary);">{ "Changeable Light Sources" }</h3>
                            <div class="light-source-cards">
                                {changeable.into_iter().map(|ls| view! { <ChangeableCard ls=ls editable=editable /> }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    } else { ().into_any() }}
                    {if !multi_channel.is_empty() {
                        view! {
                            <h3 style="margin: 24px 0 16px; color: var(--text-secondary);">{ "Multi-Channel Light Sources" }</h3>
                            <div class="light-source-cards">
                                {multi_channel.into_iter().map(|ls| view! { <MultiChannelCard ls=ls editable=editable /> }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    } else { ().into_any() }}
                </>
            }.into_any()
        })}
    }
}

#[component]
fn FixedCard(ls: FixedLightSource, editable: bool) -> impl IntoView {
    let state = use_gldf();
    let is_editing = RwSignal::new(false);
    let ls_id = ls.id.clone();
    let ls_for_remove = ls.id.clone();
    let ls_for_rename = ls.id.clone();

    let on_edit = move |_| is_editing.set(true);
    let on_remove = move |_| {
        let id = ls_for_remove.clone();
        let msg = format!("Remove light source '{}'? Any Emitter or Equipment reference to it will dangle.", id);
        if web_sys::window()
            .and_then(|w| w.confirm_with_message(&msg).ok())
            .unwrap_or(false)
        {
            state.update(|s| s.apply(GldfAction::RemoveLightSource { id }));
        }
    };
    let on_rename: Arc<dyn Fn(String) + Send + Sync> = {
        let old_id = ls_for_rename.clone();
        Arc::new(move |new_id| {
            state.update(|s| s.apply(GldfAction::RenameFixedLightSource {
                old_id: old_id.clone(),
                new_id,
            }));
        })
    };

    view! {
        <div class="light-source-card">
            <div class="card-header-row">
                <IdRenameChip current_id=ls_id.clone() editable=editable on_rename=on_rename />
                <span class="card-type">{ "Fixed" }</span>
                {if editable && !is_editing.get() {
                    view! {
                        <span style="margin-left: auto; display: inline-flex; gap: 6px;">
                            <button
                                on:click=on_edit
                                style="padding: 2px 8px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                            >{ "✎ Edit" }</button>
                            <button
                                on:click=on_remove
                                style="padding: 2px 8px; font-size: 11px; background: none; border: 1px solid var(--accent-orange); border-radius: 4px; cursor: pointer; color: var(--accent-orange);"
                            >{ "Remove" }</button>
                        </span>
                    }.into_any()
                } else { ().into_any() }}
            </div>
            <div class="card-content">
                <FixedReadBody ls=ls.clone() />
            </div>
            {if editable {
                let ls_for_modal = ls.clone();
                let on_close_for_modal: Arc<dyn Fn() + Send + Sync> = Arc::new(move || is_editing.set(false));
                let title = format!("Edit Fixed LightSource: {}", ls.id);
                view! {
                    <EditModal
                        open=is_editing
                        title=title
                        on_close=on_close_for_modal.clone()
                    >
                        <FixedEditForm
                            ls=ls_for_modal.clone()
                            on_done=move || is_editing.set(false)
                        />
                    </EditModal>
                }.into_any()
            } else { ().into_any() }}
        </div>
    }
}

#[component]
fn FixedReadBody(ls: FixedLightSource) -> impl IntoView {
    view! {
        <>
            <div style="margin-bottom: 4px;">
                <span style="font-size: 11px; color: var(--text-secondary);">{ "Name: " }</span>
                <LocaleFooView value=ls.name.clone() compact=true />
            </div>
            {ls.description.clone().map(|d| view! {
                <div style="margin-bottom: 4px;">
                    <span style="font-size: 11px; color: var(--text-secondary);">{ "Description: " }</span>
                    <LocaleFooView value=d compact=true />
                </div>
            })}
            <div class="properties-grid">
                {ls.manufacturer.clone().map(|m| view! {
                    <PropertyRow label="Manufacturer" value=m code=false />
                })}
                {ls.gtin.clone().map(|g| view! {
                    <PropertyRow label="GTIN" value=g code=true />
                })}
                {ls.rated_input_power.map(|p| view! {
                    <PropertyRow label="Rated Power" value=format!("{} W", p) code=false />
                })}
                {ls.rated_input_voltage.clone().map(|v| {
                    let text = if let Some(fixed) = v.fixed_voltage {
                        format!("{} V ({:?})", fixed, v.type_attr)
                    } else if let Some(ref range) = v.voltage_range {
                        format!("{}-{} V ({:?})", range.min, range.max, v.type_attr)
                    } else {
                        format!("{:?}", v.type_attr)
                    };
                    view! { <PropertyRow label="Rated Voltage" value=text code=false /> }
                })}
                {ls.power_range.clone().map(|pr| view! {
                    <PropertyRow label="Power Range" value=format!("{}-{} W", pr.lower, pr.upper) code=false />
                })}
                {ls.color_information.clone().map(|color| view! {
                    <>
                        {color.correlated_color_temperature.map(|cct| view! {
                            <PropertyRow label="Color Temperature" value=format!("{} K", cct) code=false />
                        })}
                        {color.color_temperature_adjusting_range.clone().and_then(|range| {
                            match (range.lower, range.upper) {
                                (Some(lo), Some(hi)) => Some(view! {
                                    <PropertyRow label="CCT Range (Tunable)" value=format!("{}–{} K", lo, hi) code=false />
                                }),
                                _ => None,
                            }
                        })}
                        {color.color_rendering_index.map(|cri| view! {
                            <PropertyRow label="CRI (Ra)" value=format!("{}", cri) code=false />
                        })}
                    </>
                })}
                {ls.light_source_maintenance.clone().map(|m| view! {
                    <>
                        {m.lifetime.map(|l| view! {
                            <PropertyRow label="Lifetime" value=format!("{} h", l) code=false />
                        })}
                        {m.cie97_lamp_type.clone().map(|t| view! {
                            <PropertyRow label="Lamp Type (CIE 97)" value=t code=false />
                        })}
                    </>
                })}
                {ls.spectrum_reference.clone().map(|sr| view! {
                    <PropertyRow label="Spectrum" value=sr.spectrum_id code=true />
                })}
                {if ls.energy_labels.is_some() {
                    view! { <PropertyRow label="Energy Labels" value="present".to_string() code=false /> }.into_any()
                } else { ().into_any() }}
                {ls.light_source_position_of_usage.clone().map(|pou| view! {
                    <PropertyRow label="Position of Usage" value=pou code=false />
                })}
                {ls.zhaga_standard.map(|z| view! {
                    <PropertyRow label="Zhaga" value=(if z { "Yes" } else { "No" }).to_string() code=false />
                })}
            </div>
        </>
    }
}

#[component]
fn ChangeableCard(ls: ChangeableLightSource, editable: bool) -> impl IntoView {
    let state = use_gldf();
    let is_editing = RwSignal::new(false);
    let ls_id = ls.id.clone();
    let ls_for_remove = ls.id.clone();
    let ls_for_rename = ls.id.clone();

    let on_edit = move |_| is_editing.set(true);
    let on_remove = move |_| {
        let id = ls_for_remove.clone();
        let msg = format!("Remove light source '{}'? Any Equipment reference to it will dangle.", id);
        if web_sys::window()
            .and_then(|w| w.confirm_with_message(&msg).ok())
            .unwrap_or(false)
        {
            state.update(|s| s.apply(GldfAction::RemoveLightSource { id }));
        }
    };
    let on_rename: Arc<dyn Fn(String) + Send + Sync> = {
        let old_id = ls_for_rename.clone();
        Arc::new(move |new_id| {
            state.update(|s| s.apply(GldfAction::RenameChangeableLightSource {
                old_id: old_id.clone(),
                new_id,
            }));
        })
    };

    view! {
        <div class="light-source-card">
            <div class="card-header-row">
                <IdRenameChip current_id=ls_id.clone() editable=editable on_rename=on_rename />
                <span class="card-type changeable">{ "Changeable" }</span>
                {if editable && !is_editing.get() {
                    view! {
                        <span style="margin-left: auto; display: inline-flex; gap: 6px;">
                            <button
                                on:click=on_edit
                                style="padding: 2px 8px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                            >{ "✎ Edit" }</button>
                            <button
                                on:click=on_remove
                                style="padding: 2px 8px; font-size: 11px; background: none; border: 1px solid var(--accent-orange); border-radius: 4px; cursor: pointer; color: var(--accent-orange);"
                            >{ "Remove" }</button>
                        </span>
                    }.into_any()
                } else { ().into_any() }}
            </div>
            <div class="card-content">
                <ChangeableReadBody ls=ls.clone() />
            </div>
            {if editable {
                let ls_for_modal = ls.clone();
                let on_close_for_modal: Arc<dyn Fn() + Send + Sync> = Arc::new(move || is_editing.set(false));
                let title = format!("Edit Changeable LightSource: {}", ls.id);
                view! {
                    <EditModal
                        open=is_editing
                        title=title
                        on_close=on_close_for_modal.clone()
                    >
                        <ChangeableEditForm
                            ls=ls_for_modal.clone()
                            on_done=move || is_editing.set(false)
                        />
                    </EditModal>
                }.into_any()
            } else { ().into_any() }}
        </div>
    }
}

#[component]
fn ChangeableReadBody(ls: ChangeableLightSource) -> impl IntoView {
    view! {
        <>
            <div style="margin-bottom: 4px;">
                <span style="font-size: 11px; color: var(--text-secondary);">{ "Name: " }</span>
                <LocaleFooView value=ls.name.clone() compact=true />
            </div>
            {ls.description.clone().map(|d| view! {
                <div style="margin-bottom: 4px;">
                    <span style="font-size: 11px; color: var(--text-secondary);">{ "Description: " }</span>
                    <LocaleFooView value=d compact=true />
                </div>
            })}
            <div class="properties-grid">
                {ls.manufacturer.clone().map(|m| view! {
                    <PropertyRow label="Manufacturer" value=m code=false />
                })}
                {ls.photometry_reference.clone().map(|p| view! {
                    <PropertyRow label="Photometry" value=p.photometry_id code=true />
                })}
                {ls.spectrum_reference.clone().map(|sr| view! {
                    <PropertyRow label="Spectrum" value=sr.spectrum_id code=true />
                })}
                {ls.light_source_maintenance.clone().map(|m| view! {
                    <>
                        {m.lifetime.map(|l| view! {
                            <PropertyRow label="Lifetime" value=format!("{} h", l) code=false />
                        })}
                        {m.cie97_lamp_type.clone().map(|t| view! {
                            <PropertyRow label="Lamp Type (CIE 97)" value=t code=false />
                        })}
                    </>
                })}
            </div>
        </>
    }
}

/// Card for a MultiChannelLightSource. Read-only when `editable=false`.
/// In editor mode: rename id, edit base fields via a modal, remove the
/// source, manage Channels inline (Edit + Remove per row + an Add form).
#[component]
fn MultiChannelCard(ls: MultiChannelLightSource, editable: bool) -> impl IntoView {
    let state = use_gldf();
    let ls_id = ls.id.clone();
    let ls_for_rename = ls.id.clone();
    let ls_for_remove = ls.id.clone();
    let is_editing = RwSignal::new(false);

    let on_edit = move |_| is_editing.set(true);
    let on_remove = move |_| {
        let id = ls_for_remove.clone();
        let msg = format!("Remove multi-channel light source '{}'? Any MultiChannelLightEmitter referencing it will dangle.", id);
        if web_sys::window()
            .and_then(|w| w.confirm_with_message(&msg).ok())
            .unwrap_or(false)
        {
            state.update(|s| s.apply(GldfAction::RemoveLightSource { id }));
        }
    };
    let on_rename: Arc<dyn Fn(String) + Send + Sync> = {
        let old_id = ls_for_rename.clone();
        Arc::new(move |new_id| {
            state.update(|s| s.apply(GldfAction::RenameMultiChannelLightSource {
                old_id: old_id.clone(),
                new_id,
            }));
        })
    };

    let ls_id_for_modal_title = ls.id.clone();
    let ls_for_modal = ls.clone();
    let close_modal: Arc<dyn Fn() + Send + Sync> = Arc::new(move || is_editing.set(false));

    view! {
        <div class="light-source-card">
            <div class="card-header-row">
                <IdRenameChip current_id=ls_id editable=editable on_rename=on_rename />
                <span class="card-type" style="background: var(--accent-purple);">{ "MultiChannel" }</span>
                {if editable {
                    view! {
                        <span style="margin-left: auto; display: inline-flex; gap: 6px;">
                            <button
                                on:click=on_edit
                                style="padding: 2px 8px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                            >{ "✎ Edit" }</button>
                            <button
                                on:click=on_remove
                                style="padding: 2px 8px; font-size: 11px; background: none; border: 1px solid var(--accent-orange); border-radius: 4px; cursor: pointer; color: var(--accent-orange);"
                            >{ "Remove" }</button>
                        </span>
                    }.into_any()
                } else { ().into_any() }}
            </div>
            <div class="card-content">
                <MultiChannelReadBody ls=ls.clone() editable=editable />
            </div>
            {if editable {
                let modal_title = format!("Edit MultiChannel LightSource: {}", ls_id_for_modal_title);
                view! {
                    <EditModal
                        open=is_editing
                        title=modal_title
                        on_close=close_modal.clone()
                    >
                        <MultiChannelEditForm
                            ls=ls_for_modal.clone()
                            on_done=move || is_editing.set(false)
                        />
                    </EditModal>
                }.into_any()
            } else { ().into_any() }}
        </div>
    }
}

/// Body of a MultiChannelCard. Shows the base fields and the Channels
/// table. When `editable`, each channel row gets ✎ / Remove buttons and
/// an `+ Add Channel` form appears below the table.
#[component]
fn MultiChannelReadBody(ls: MultiChannelLightSource, editable: bool) -> impl IntoView {
    // editable is no longer used here: ALL editing affordances moved into
    // the MultiChannelEditForm (which the modal renders on ✎ Edit). The
    // read body just renders the data.
    let _ = editable;
    let channel_count = ls.channels.channel.len();
    let ls_id_for_channels = ls.id.clone();

    view! {
        <>
            <div style="margin-bottom: 4px;">
                <span style="font-size: 11px; color: var(--text-secondary);">{ "Name: " }</span>
                <LocaleFooView value=ls.name.clone() compact=true />
            </div>
            {ls.description.clone().map(|d| view! {
                <div style="margin-bottom: 4px;">
                    <span style="font-size: 11px; color: var(--text-secondary);">{ "Description: " }</span>
                    <LocaleFooView value=d compact=true />
                </div>
            })}
            <div class="properties-grid">
                {ls.manufacturer.clone().map(|m| view! {
                    <PropertyRow label="Manufacturer" value=m code=false />
                })}
                {ls.gtin.clone().map(|g| view! {
                    <PropertyRow label="GTIN" value=g code=true />
                })}
                <PropertyRow label="Rated Power" value=format!("{} W", ls.rated_input_power) code=false />
                <PropertyRow label="Channels" value=format!("{}", channel_count) code=false />
            </div>
            <div style="margin-top: 8px; padding: 8px; background: var(--bg-primary); border-radius: 4px;">
                <div style="font-size: 11px; color: var(--text-secondary); margin-bottom: 6px;">{ "Channels" }</div>
                {if !ls.channels.channel.is_empty() {
                    view! {
                        <table style="width: 100%; font-size: 11px; border-collapse: collapse;">
                            <thead>
                                <tr style="text-align: left; color: var(--text-tertiary);">
                                    <th style="padding: 4px;">{ "Name" }</th>
                                    <th style="padding: 4px;">{ "Spectrum" }</th>
                                    <th style="padding: 4px;">{ "Photometry" }</th>
                                    <th style="padding: 4px;">{ "Flux (lm)" }</th>
                                </tr>
                            </thead>
                            <tbody>
                                {ls.channels.channel.clone().into_iter().enumerate().map(|(idx, ch)| {
                                    let ls_id = ls_id_for_channels.clone();
                                    view! {
                                        // Read-only on the card; the editor
                                        // modal hosts the editable ChannelRow.
                                        <ChannelRow
                                            ls_id=ls_id
                                            index=idx
                                            channel=ch
                                            editable=false
                                        />
                                    }
                                }).collect::<Vec<_>>()}
                            </tbody>
                        </table>
                    }.into_any()
                } else {
                    view! {
                        <p style="margin: 0; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                            { "No channels yet. Click ✎ Edit to add one." }
                        </p>
                    }.into_any()
                }}
            </div>
        </>
    }
}

/// One row in the Channels table. Editable mode lets the user inline-edit
/// display name + spectrum/photometry refs + flux, or remove the row.
#[component]
fn ChannelRow(
    ls_id: String,
    index: usize,
    channel: gldf_rs::gldf::general_definitions::lightsources::MultiChannelChannel,
    editable: bool,
) -> impl IntoView {
    let state = use_gldf();
    let ls_id_for_remove = ls_id.clone();
    let ls_id_for_save = ls_id.clone();
    let is_editing = RwSignal::new(false);

    let display_name = channel
        .display_name
        .locale
        .first()
        .map(|l| l.value.clone())
        .unwrap_or_default();
    let spectrum_id = channel.spectrum_reference.spectrum_id.clone();
    let photometry_id = channel.photometry_reference.photometry_id.clone();
    let flux = channel.rated_luminous_flux;
    let channel_for_save = channel.clone();

    // Per-row state for the inline edit form.
    let s_name = RwSignal::new(display_name.clone());
    let s_spectrum = RwSignal::new(spectrum_id.clone());
    let s_photo = RwSignal::new(photometry_id.clone());
    let s_flux = RwSignal::new(flux.to_string());

    // Catalogs read live from state so a newly-added Spectrum / Photometry
    // appears in the dropdown without round-tripping.
    let spectrum_ids = Signal::derive(move || {
        state.with(|s| {
            s.product
                .general_definitions
                .spectrums
                .as_ref()
                .map(|ss| ss.spectrum.iter().map(|sp| sp.id.clone()).collect::<Vec<_>>())
                .unwrap_or_default()
        })
    });
    let photometry_ids = Signal::derive(move || {
        state.with(|s| {
            s.product
                .general_definitions
                .photometries
                .as_ref()
                .map(|ps| ps.photometry.iter().map(|p| p.id.clone()).collect::<Vec<_>>())
                .unwrap_or_default()
        })
    });

    let on_edit = move |_| {
        s_name.set(display_name.clone());
        s_spectrum.set(spectrum_id.clone());
        s_photo.set(photometry_id.clone());
        s_flux.set(flux.to_string());
        is_editing.set(true);
    };
    let on_cancel = move |_| is_editing.set(false);
    let on_remove = move |_| {
        state.update(|s| s.apply(GldfAction::RemoveMultiChannelChannel {
            ls_id: ls_id_for_remove.clone(),
            index,
        }));
    };
    let on_save = move |_| {
        use gldf_rs::gldf::general_definitions::lightsources::{
            ChannelPhotometryReference, ChannelSpectrumReference,
        };
        use gldf_rs::gldf::header::{Locale, LocaleFoo};
        let mut new_channel = channel_for_save.clone();
        let name = s_name.get();
        new_channel.display_name = if name.trim().is_empty() {
            LocaleFoo::default()
        } else {
            LocaleFoo {
                locale: vec![Locale {
                    language: String::new(),
                    value: name,
                }],
            }
        };
        new_channel.spectrum_reference = ChannelSpectrumReference {
            spectrum_id: s_spectrum.get(),
        };
        new_channel.photometry_reference = ChannelPhotometryReference {
            photometry_id: s_photo.get(),
        };
        new_channel.rated_luminous_flux = s_flux.get().trim().parse::<i32>().unwrap_or(0);
        state.update(|s| s.apply(GldfAction::UpdateMultiChannelChannel {
            ls_id: ls_id_for_save.clone(),
            index,
            channel: Box::new(new_channel),
        }));
        is_editing.set(false);
    };

    let on_name_input = move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                s_name.set(input.value());
            }
        }
    };
    let on_spectrum_change = move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            if let Ok(sel) = target.dyn_into::<HtmlSelectElement>() {
                s_spectrum.set(sel.value());
            }
        }
    };
    let on_photo_change = move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            if let Ok(sel) = target.dyn_into::<HtmlSelectElement>() {
                s_photo.set(sel.value());
            }
        }
    };
    let on_flux_input = move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                s_flux.set(input.value());
            }
        }
    };

    let display_name_static = channel
        .display_name
        .locale
        .first()
        .map(|l| l.value.clone())
        .unwrap_or_default();

    view! {
        <tr style="border-top: 1px solid var(--border-color); vertical-align: top;">
            {move || if editable && is_editing.get() {
                let s_ids = spectrum_ids.get();
                let p_ids = photometry_ids.get();
                let cur_spec = s_spectrum.get();
                let cur_photo = s_photo.get();
                view! {
                    <>
                        <td style="padding: 4px;">
                            <input
                                type="text"
                                prop:value=move || s_name.get()
                                on:input=on_name_input.clone()
                                style="width: 100%; padding: 2px 4px; font-size: 11px;"
                            />
                        </td>
                        <td style="padding: 4px;">
                            <select on:change=on_spectrum_change.clone() style="width: 100%; font-size: 11px;">
                                {if s_ids.is_empty() {
                                    view! { <option value="">{ "(none)" }</option> }.into_any()
                                } else {
                                    view! {
                                        {s_ids.into_iter().map(|sid| {
                                            let selected = sid == cur_spec;
                                            let label = sid.clone();
                                            view! { <option value=sid selected=selected>{ label }</option> }
                                        }).collect::<Vec<_>>()}
                                    }.into_any()
                                }}
                            </select>
                        </td>
                        <td style="padding: 4px;">
                            <select on:change=on_photo_change.clone() style="width: 100%; font-size: 11px;">
                                {if p_ids.is_empty() {
                                    view! { <option value="">{ "(none)" }</option> }.into_any()
                                } else {
                                    view! {
                                        {p_ids.into_iter().map(|pid| {
                                            let selected = pid == cur_photo;
                                            let label = pid.clone();
                                            view! { <option value=pid selected=selected>{ label }</option> }
                                        }).collect::<Vec<_>>()}
                                    }.into_any()
                                }}
                            </select>
                        </td>
                        <td style="padding: 4px;">
                            <input
                                type="number"
                                min="0"
                                prop:value=move || s_flux.get()
                                on:input=on_flux_input.clone()
                                style="width: 80px; padding: 2px 4px; font-size: 11px;"
                            />
                        </td>
                        <td style="padding: 4px; display: flex; gap: 4px;">
                            <button
                                on:click=on_save.clone()
                                style="padding: 2px 6px; font-size: 11px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;"
                            >{ "Save" }</button>
                            <button
                                on:click=on_cancel.clone()
                                style="padding: 2px 6px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                            >{ "Cancel" }</button>
                        </td>
                    </>
                }.into_any()
            } else {
                view! {
                    <>
                        <td style="padding: 4px;">{ display_name_static.clone() }</td>
                        <td style="padding: 4px; font-family: var(--font-mono); color: var(--accent-orange);">{ channel.spectrum_reference.spectrum_id.clone() }</td>
                        <td style="padding: 4px; font-family: var(--font-mono); color: var(--accent-purple);">{ channel.photometry_reference.photometry_id.clone() }</td>
                        <td style="padding: 4px;">{ channel.rated_luminous_flux }</td>
                        {if editable {
                            view! {
                                <td style="padding: 4px; display: flex; gap: 4px;">
                                    <button
                                        on:click=on_edit.clone()
                                        title="Edit"
                                        style="padding: 1px 6px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                                    >{ "✎" }</button>
                                    <button
                                        on:click=on_remove.clone()
                                        style="padding: 1px 6px; font-size: 11px; background: none; border: 1px solid var(--accent-orange); border-radius: 4px; cursor: pointer; color: var(--accent-orange);"
                                    >{ "✕" }</button>
                                </td>
                            }.into_any()
                        } else { ().into_any() }}
                    </>
                }.into_any()
            }}
        </tr>
    }
}

/// Inline "+ Add Channel" form below the channels table. Picks Spectrum +
/// Photometry from the declared catalogs; flux as a number; an optional
/// DisplayName text.
#[component]
fn AddChannelForm(ls_id: String) -> impl IntoView {
    let state = use_gldf();
    let is_open = RwSignal::new(false);
    let s_name = RwSignal::new(String::new());
    let s_spectrum = RwSignal::new(String::new());
    let s_photo = RwSignal::new(String::new());
    let s_flux = RwSignal::new(String::from("0"));
    let ls_id_for_submit = ls_id.clone();

    let spectrum_ids = Signal::derive(move || {
        state.with(|s| {
            s.product
                .general_definitions
                .spectrums
                .as_ref()
                .map(|ss| ss.spectrum.iter().map(|sp| sp.id.clone()).collect::<Vec<_>>())
                .unwrap_or_default()
        })
    });
    let photometry_ids = Signal::derive(move || {
        state.with(|s| {
            s.product
                .general_definitions
                .photometries
                .as_ref()
                .map(|ps| ps.photometry.iter().map(|p| p.id.clone()).collect::<Vec<_>>())
                .unwrap_or_default()
        })
    });

    // Seed the pickers with the first catalog entry whenever the form
    // opens, so the user doesn't have to interact with empty selects.
    Effect::new(move |_| {
        if is_open.get() {
            if s_spectrum.with(|v| v.is_empty()) {
                if let Some(first) = spectrum_ids.get().into_iter().next() {
                    s_spectrum.set(first);
                }
            }
            if s_photo.with(|v| v.is_empty()) {
                if let Some(first) = photometry_ids.get().into_iter().next() {
                    s_photo.set(first);
                }
            }
        }
    });

    let on_open = move |_| is_open.set(true);
    let on_cancel = move |_| {
        is_open.set(false);
        s_name.set(String::new());
        s_flux.set(String::from("0"));
    };
    let on_submit = move |_| {
        use gldf_rs::gldf::general_definitions::lightsources::{
            ChannelPhotometryReference, ChannelSpectrumReference, MultiChannelChannel,
        };
        use gldf_rs::gldf::header::{Locale, LocaleFoo};
        let spec = s_spectrum.get();
        let photo = s_photo.get();
        if spec.is_empty() || photo.is_empty() {
            return;
        }
        let name = s_name.get();
        let display_name = if name.trim().is_empty() {
            LocaleFoo::default()
        } else {
            LocaleFoo {
                locale: vec![Locale {
                    language: String::new(),
                    value: name,
                }],
            }
        };
        let flux = s_flux.get().trim().parse::<i32>().unwrap_or(0);
        let channel = MultiChannelChannel {
            display_name,
            spectrum_reference: ChannelSpectrumReference { spectrum_id: spec },
            photometry_reference: ChannelPhotometryReference { photometry_id: photo },
            rated_luminous_flux: flux,
            color_information: None,
        };
        state.update(|s| s.apply(GldfAction::AddMultiChannelChannel {
            ls_id: ls_id_for_submit.clone(),
            channel: Box::new(channel),
        }));
        is_open.set(false);
        s_name.set(String::new());
        s_flux.set(String::from("0"));
    };

    let on_name_input = move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                s_name.set(input.value());
            }
        }
    };
    let on_spectrum_change = move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            if let Ok(sel) = target.dyn_into::<HtmlSelectElement>() {
                s_spectrum.set(sel.value());
            }
        }
    };
    let on_photo_change = move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            if let Ok(sel) = target.dyn_into::<HtmlSelectElement>() {
                s_photo.set(sel.value());
            }
        }
    };
    let on_flux_input = move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                s_flux.set(input.value());
            }
        }
    };

    view! {
        {move || if is_open.get() {
            let s_ids = spectrum_ids.get();
            let p_ids = photometry_ids.get();
            let no_spec = s_ids.is_empty();
            let no_photo = p_ids.is_empty();
            view! {
                <div style="padding: 8px; background: var(--bg-secondary); border: 1px dashed var(--accent-blue); border-radius: 4px; display: flex; flex-direction: column; gap: 6px;">
                    <div style="font-size: 11px; color: var(--text-secondary);">{ "Add Channel" }</div>
                    <label style="display: grid; grid-template-columns: 140px 1fr; gap: 6px; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-tertiary);">{ "Display name" }</span>
                        <input
                            type="text"
                            placeholder="e.g. Warm / Cool / Red"
                            on:input=on_name_input.clone()
                            style="padding: 2px 4px; font-size: 11px;"
                        />
                    </label>
                    <label style="display: grid; grid-template-columns: 140px 1fr; gap: 6px; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-tertiary);">{ "Spectrum" }</span>
                        <select on:change=on_spectrum_change.clone() style="font-size: 11px;">
                            {if no_spec {
                                view! { <option value="">{ "(no Spectrum declared)" }</option> }.into_any()
                            } else {
                                view! {
                                    {s_ids.into_iter().map(|sid| {
                                        let label = sid.clone();
                                        view! { <option value=sid>{ label }</option> }
                                    }).collect::<Vec<_>>()}
                                }.into_any()
                            }}
                        </select>
                    </label>
                    <label style="display: grid; grid-template-columns: 140px 1fr; gap: 6px; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-tertiary);">{ "Photometry" }</span>
                        <select on:change=on_photo_change.clone() style="font-size: 11px;">
                            {if no_photo {
                                view! { <option value="">{ "(no Photometry declared)" }</option> }.into_any()
                            } else {
                                view! {
                                    {p_ids.into_iter().map(|pid| {
                                        let label = pid.clone();
                                        view! { <option value=pid>{ label }</option> }
                                    }).collect::<Vec<_>>()}
                                }.into_any()
                            }}
                        </select>
                    </label>
                    <label style="display: grid; grid-template-columns: 140px 1fr; gap: 6px; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-tertiary);">{ "Flux (lm)" }</span>
                        <input
                            type="number"
                            min="0"
                            prop:value=move || s_flux.get()
                            on:input=on_flux_input.clone()
                            style="padding: 2px 4px; font-size: 11px; width: 120px;"
                        />
                    </label>
                    <div style="display: flex; gap: 6px;">
                        <button
                            on:click=on_submit.clone()
                            disabled=no_spec || no_photo
                            style="padding: 3px 10px; font-size: 11px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;"
                        >{ "+ Add Channel" }</button>
                        <button
                            on:click=on_cancel.clone()
                            style="padding: 3px 10px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                        >{ "Cancel" }</button>
                    </div>
                </div>
            }.into_any()
        } else {
            view! {
                <button
                    on:click=on_open
                    style="padding: 3px 10px; font-size: 11px; background: none; border: 1px dashed var(--accent-blue); border-radius: 4px; cursor: pointer; color: var(--accent-blue);"
                >{ "+ Add Channel" }</button>
            }.into_any()
        }}
    }
}

/// Inline edit form for a MultiChannelLightSource's base fields. Channels
/// stay edited inline on the card itself; this form covers the top-level
/// fields like name/manufacturer/gtin/power/position. The form runs
/// inside an EditModal.
#[component]
fn MultiChannelEditForm(
    ls: MultiChannelLightSource,
    on_done: impl Fn() + Send + Sync + 'static + Clone,
) -> impl IntoView {
    let state = use_gldf();
    let original = ls.clone();
    let id_for_save = ls.id.clone();

    let name_first = first_locale_value(&ls.name);
    let desc_first = ls
        .description
        .as_ref()
        .map(first_locale_value)
        .unwrap_or_default();
    let manufacturer = ls.manufacturer.clone().unwrap_or_default();
    let gtin = ls.gtin.clone().unwrap_or_default();
    let power = ls.rated_input_power.to_string();
    let position = ls.light_source_position_of_usage.clone().unwrap_or_default();

    let s_name = RwSignal::new(name_first);
    let s_desc = RwSignal::new(desc_first);
    let s_manuf = RwSignal::new(manufacturer);
    let s_gtin = RwSignal::new(gtin);
    let s_power = RwSignal::new(power);
    let s_pos = RwSignal::new(position);

    let on_cancel = {
        let done = on_done.clone();
        move |_| done()
    };
    let on_save = move |_| {
        let mut new_ls = original.clone();
        new_ls.name = upsert_locale_value(new_ls.name, &s_name.get());
        let desc_text = s_desc.get();
        new_ls.description = if desc_text.trim().is_empty() {
            None
        } else {
            Some(upsert_locale_value(
                new_ls.description.unwrap_or_default(),
                &desc_text,
            ))
        };
        new_ls.manufacturer = empty_to_none(s_manuf.get());
        new_ls.gtin = empty_to_none(s_gtin.get());
        new_ls.rated_input_power = s_power.get().trim().parse::<f64>().unwrap_or(0.0);
        new_ls.light_source_position_of_usage = empty_to_none(s_pos.get());

        state.update(|s| s.apply(GldfAction::UpdateMultiChannelLightSource {
            id: id_for_save.clone(),
            payload: Box::new(new_ls),
        }));
        on_done();
    };

    // Reactive view of the live channel list — the modal reads state
    // directly so per-channel edits (which dispatch their own actions
    // immediately) show up here without rebuilding the modal.
    let ls_id_for_lookup = ls.id.clone();
    let ls_id_for_rows = ls.id.clone();
    let ls_id_for_add = ls.id.clone();
    let channels_view = move || {
        let target = ls_id_for_lookup.clone();
        state.with(|s| {
            s.product
                .general_definitions
                .light_sources
                .as_ref()
                .and_then(|l| {
                    l.multi_channel_light_source
                        .iter()
                        .find(|m| m.id == target)
                })
                .map(|mc| mc.channels.channel.clone())
                .unwrap_or_default()
        })
    };

    view! {
        <div class="ls-edit-form" style="display: flex; flex-direction: column; gap: 6px;">
            <TextRow label="Name (default locale)" value=s_name />
            <TextRow label="Description" value=s_desc />
            <TextRow label="Manufacturer" value=s_manuf />
            <TextRow label="GTIN" value=s_gtin />
            <TextRow label="Rated Power (W)" value=s_power />
            <TextRow label="Position of Usage" value=s_pos />

            <div style="margin-top: 12px; padding: 8px; background: var(--bg-secondary); border-radius: 4px;">
                <div style="font-size: 12px; color: var(--text-secondary); margin-bottom: 6px; font-weight: 500;">{ "Channels" }</div>
                {move || {
                    let channels = channels_view();
                    let ls_id_for_channels = ls_id_for_rows.clone();
                    if channels.is_empty() {
                        view! {
                            <p style="margin: 0 0 8px; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                                { "No channels yet. Use + Add Channel below to create one." }
                            </p>
                        }.into_any()
                    } else {
                        view! {
                            <table style="width: 100%; font-size: 11px; border-collapse: collapse; margin-bottom: 8px;">
                                <thead>
                                    <tr style="text-align: left; color: var(--text-tertiary);">
                                        <th style="padding: 4px;">{ "Name" }</th>
                                        <th style="padding: 4px;">{ "Spectrum" }</th>
                                        <th style="padding: 4px;">{ "Photometry" }</th>
                                        <th style="padding: 4px;">{ "Flux (lm)" }</th>
                                        <th style="padding: 4px;">{ "" }</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {channels.into_iter().enumerate().map(|(idx, ch)| {
                                        let ls_id = ls_id_for_channels.clone();
                                        view! {
                                            <ChannelRow ls_id=ls_id index=idx channel=ch editable=true />
                                        }
                                    }).collect::<Vec<_>>()}
                                </tbody>
                            </table>
                        }.into_any()
                    }
                }}
                <AddChannelForm ls_id=ls_id_for_add />
            </div>

            <p style="margin: 4px 0 0; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                { "EnergyLabels, ActivePowerTable, ColorInformation, and LightSourceImages are preserved verbatim on save." }
            </p>
            <div style="display: flex; gap: 6px; margin-top: 6px;">
                <button on:click=on_save style="padding: 4px 10px; font-size: 12px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;">{ "Save base fields" }</button>
                <button on:click=on_cancel style="padding: 4px 10px; font-size: 12px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);">{ "Close" }</button>
            </div>
        </div>
    }
}

/// Inline editor for a FixedLightSource. Edits the high-value fields the
/// user typically wants to touch (name/manufacturer/gtin/power/voltage/
/// color/maintenance/position/zhaga). Less-common nested types (energy
/// labels, active power tables, light source images) are preserved verbatim
/// when we Save — the user can still see them but not edit them here.
#[component]
fn FixedEditForm(ls: FixedLightSource, on_done: impl Fn() + Send + Sync + 'static + Clone) -> impl IntoView {
    let state = use_gldf();
    let original = ls.clone();
    let id_for_save = ls.id.clone();
    let ls_id_for_pickers = ls.id.clone();
    let current_spectrum = ls.spectrum_reference.clone().map(|sr| sr.spectrum_id);

    let name_first = first_locale_value(&ls.name);
    let desc_first = ls.description.as_ref().map(first_locale_value).unwrap_or_default();
    let manufacturer = ls.manufacturer.clone().unwrap_or_default();
    let gtin = ls.gtin.clone().unwrap_or_default();
    let power = ls.rated_input_power.map(|p| p.to_string()).unwrap_or_default();
    let voltage = ls.rated_input_voltage.as_ref().and_then(|v| v.fixed_voltage).map(|v| v.to_string()).unwrap_or_default();
    let cct = ls.color_information.as_ref().and_then(|c| c.correlated_color_temperature).map(|v| v.to_string()).unwrap_or_default();
    let cri = ls.color_information.as_ref().and_then(|c| c.color_rendering_index).map(|v| v.to_string()).unwrap_or_default();
    let lifetime = ls.light_source_maintenance.as_ref().and_then(|m| m.lifetime).map(|v| v.to_string()).unwrap_or_default();
    let lamp_type = ls.light_source_maintenance.as_ref().and_then(|m| m.cie97_lamp_type.clone()).unwrap_or_default();
    let position = ls.light_source_position_of_usage.clone().unwrap_or_default();
    let zhaga = ls.zhaga_standard.unwrap_or(false);

    let s_name = RwSignal::new(name_first);
    let s_desc = RwSignal::new(desc_first);
    let s_manuf = RwSignal::new(manufacturer);
    let s_gtin = RwSignal::new(gtin);
    let s_power = RwSignal::new(power);
    let s_volt = RwSignal::new(voltage);
    let s_cct = RwSignal::new(cct);
    let s_cri = RwSignal::new(cri);
    let s_life = RwSignal::new(lifetime);
    let s_lamp = RwSignal::new(lamp_type);
    let s_pos = RwSignal::new(position);
    let s_zhaga = RwSignal::new(zhaga);

    let on_cancel = {
        let done = on_done.clone();
        move |_| done()
    };
    let on_save = move |_| {
        let mut new_ls = original.clone();
        new_ls.name = upsert_locale_value(new_ls.name, &s_name.get());
        let desc_text = s_desc.get();
        new_ls.description = if desc_text.trim().is_empty() {
            None
        } else {
            Some(upsert_locale_value(new_ls.description.unwrap_or_default(), &desc_text))
        };
        new_ls.manufacturer = empty_to_none(s_manuf.get());
        new_ls.gtin = empty_to_none(s_gtin.get());
        new_ls.rated_input_power = parse_opt_f64(&s_power.get());
        new_ls.rated_input_voltage = build_voltage(new_ls.rated_input_voltage, &s_volt.get());
        new_ls.color_information = build_color_info(new_ls.color_information, &s_cct.get(), &s_cri.get());
        new_ls.light_source_maintenance = build_maintenance(new_ls.light_source_maintenance, &s_life.get(), &s_lamp.get());
        new_ls.light_source_position_of_usage = empty_to_none(s_pos.get());
        new_ls.zhaga_standard = Some(s_zhaga.get());

        state.update(|s| {
            s.apply(GldfAction::UpdateFixedLightSource {
                id: id_for_save.clone(),
                payload: Box::new(new_ls),
            });
        });
        on_done();
    };

    view! {
        <div class="ls-edit-form" style="display: flex; flex-direction: column; gap: 6px;">
            <TextRow label="Name (default locale)" value=s_name />
            <TextRow label="Description" value=s_desc />
            <TextRow label="Manufacturer" value=s_manuf />
            <TextRow label="GTIN" value=s_gtin />
            <TextRow label="Rated Power (W)" value=s_power />
            <TextRow label="Fixed Voltage (V)" value=s_volt />
            <TextRow label="CCT (K)" value=s_cct />
            <TextRow label="CRI (Ra)" value=s_cri />
            <TextRow label="Lifetime (h)" value=s_life />
            <TextRow label="Lamp Type (CIE 97)" value=s_lamp />
            <TextRow label="Position of Usage" value=s_pos />
            <BoolRow label="Zhaga" value=s_zhaga />
            <SpectrumPicker
                light_source_id=ls_id_for_pickers.clone()
                current=current_spectrum.clone()
            />
            <p style="margin: 4px 0 0; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                { "Other fields (energy labels, active power table, light source images, …) preserved on save." }
            </p>
            <div style="display: flex; gap: 6px; margin-top: 6px;">
                <button on:click=on_save style="padding: 4px 10px; font-size: 12px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;">{ "Save" }</button>
                <button on:click=on_cancel style="padding: 4px 10px; font-size: 12px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);">{ "Cancel" }</button>
            </div>
        </div>
    }
}

/// Inline editor for a ChangeableLightSource — shorter than the Fixed form
/// because ChangeableLightSource has fewer first-class fields in the schema.
#[component]
fn ChangeableEditForm(ls: ChangeableLightSource, on_done: impl Fn() + Send + Sync + 'static + Clone) -> impl IntoView {
    let state = use_gldf();
    let original = ls.clone();
    let id_for_save = ls.id.clone();
    let ls_id_for_pickers = ls.id.clone();
    let current_photometry = ls.photometry_reference.clone().map(|p| p.photometry_id);
    let current_spectrum = ls.spectrum_reference.clone().map(|sr| sr.spectrum_id);

    let name_first = first_locale_value(&ls.name);
    let desc_first = ls.description.as_ref().map(first_locale_value).unwrap_or_default();
    let manufacturer = ls.manufacturer.clone().unwrap_or_default();
    let lifetime = ls.light_source_maintenance.as_ref().and_then(|m| m.lifetime).map(|v| v.to_string()).unwrap_or_default();
    let lamp_type = ls.light_source_maintenance.as_ref().and_then(|m| m.cie97_lamp_type.clone()).unwrap_or_default();

    let s_name = RwSignal::new(name_first);
    let s_desc = RwSignal::new(desc_first);
    let s_manuf = RwSignal::new(manufacturer);
    let s_life = RwSignal::new(lifetime);
    let s_lamp = RwSignal::new(lamp_type);

    let on_cancel = {
        let done = on_done.clone();
        move |_| done()
    };
    let on_save = move |_| {
        let mut new_ls = original.clone();
        new_ls.name = upsert_locale_value(new_ls.name, &s_name.get());
        let desc_text = s_desc.get();
        new_ls.description = if desc_text.trim().is_empty() {
            None
        } else {
            Some(upsert_locale_value(new_ls.description.unwrap_or_default(), &desc_text))
        };
        new_ls.manufacturer = empty_to_none(s_manuf.get());
        new_ls.light_source_maintenance = build_maintenance(new_ls.light_source_maintenance, &s_life.get(), &s_lamp.get());

        state.update(|s| {
            s.apply(GldfAction::UpdateChangeableLightSource {
                id: id_for_save.clone(),
                payload: Box::new(new_ls),
            });
        });
        on_done();
    };

    view! {
        <div class="ls-edit-form" style="display: flex; flex-direction: column; gap: 6px;">
            <TextRow label="Name (default locale)" value=s_name />
            <TextRow label="Description" value=s_desc />
            <TextRow label="Manufacturer" value=s_manuf />
            <TextRow label="Lifetime (h)" value=s_life />
            <TextRow label="Lamp Type (CIE 97)" value=s_lamp />
            <PhotometryPicker
                light_source_id=ls_id_for_pickers.clone()
                current=current_photometry.clone()
            />
            <SpectrumPicker
                light_source_id=ls_id_for_pickers.clone()
                current=current_spectrum.clone()
            />
            <p style="margin: 4px 0 0; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                { "Other fields (energy labels, images, …) preserved on save." }
            </p>
            <div style="display: flex; gap: 6px; margin-top: 6px;">
                <button on:click=on_save style="padding: 4px 10px; font-size: 12px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;">{ "Save" }</button>
                <button on:click=on_cancel style="padding: 4px 10px; font-size: 12px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);">{ "Cancel" }</button>
            </div>
        </div>
    }
}

#[component]
fn TextRow(label: &'static str, value: RwSignal<String>) -> impl IntoView {
    let on_input = move |e: web_sys::Event| {
        let Some(target) = e.target() else { return };
        let input: HtmlInputElement = match target.dyn_into() {
            Ok(i) => i,
            Err(_) => return,
        };
        value.set(input.value());
    };
    view! {
        <label style="display: grid; grid-template-columns: 160px 1fr; gap: 8px; align-items: center; font-size: 12px;">
            <span style="color: var(--text-secondary);">{ label }</span>
            <input
                type="text"
                prop:value=move || value.get()
                on:input=on_input
                style="padding: 3px 6px; font-size: 12px; background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: 4px; color: var(--text-primary);"
            />
        </label>
    }
}

#[component]
fn BoolRow(label: &'static str, value: RwSignal<bool>) -> impl IntoView {
    let on_change = move |e: web_sys::Event| {
        let Some(target) = e.target() else { return };
        let input: HtmlInputElement = match target.dyn_into() {
            Ok(i) => i,
            Err(_) => return,
        };
        value.set(input.checked());
    };
    view! {
        <label style="display: grid; grid-template-columns: 160px 1fr; gap: 8px; align-items: center; font-size: 12px;">
            <span style="color: var(--text-secondary);">{ label }</span>
            <input
                type="checkbox"
                prop:checked=move || value.get()
                on:change=on_change
                style="justify-self: start;"
            />
        </label>
    }
}

fn first_locale_value(lf: &LocaleFoo) -> String {
    lf.locale.first().map(|l| l.value.clone()).unwrap_or_default()
}

/// Insert/overwrite the first Locale's `value`; if no locales exist, push a
/// blank-language one. Preserves any other locale entries.
fn upsert_locale_value(mut lf: LocaleFoo, value: &str) -> LocaleFoo {
    if let Some(first) = lf.locale.first_mut() {
        first.value = value.to_string();
    } else {
        lf.locale.push(Locale {
            language: String::new(),
            value: value.to_string(),
        });
    }
    lf
}

fn empty_to_none(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

fn parse_opt_f64(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

fn parse_opt_i32(s: &str) -> Option<i32> {
    s.trim().parse::<i32>().ok()
}

/// Build a `Voltage` from the user-entered "fixed voltage" string. Preserve
/// the prior struct (type_attr, voltage_range) and just overwrite
/// `fixed_voltage`. If the field is cleared, drop the whole Voltage.
fn build_voltage(prior: Option<Voltage>, text: &str) -> Option<Voltage> {
    let fixed = parse_opt_f64(text);
    match (prior, fixed) {
        (_, None) => None,
        (Some(mut v), Some(f)) => {
            v.fixed_voltage = Some(f);
            v.voltage_range = None;
            Some(v)
        }
        (None, Some(f)) => {
            let mut v = Voltage::default();
            v.fixed_voltage = Some(f);
            Some(v)
        }
    }
}

/// Preserve prior `color_information` (tunable range, x/y coordinates,
/// MacAdam steps) and overwrite CCT + CRI fields from the form.
fn build_color_info(
    prior: Option<ColorInformation>,
    cct: &str,
    cri: &str,
) -> Option<ColorInformation> {
    let cct = parse_opt_i32(cct);
    let cri = parse_opt_i32(cri);
    let mut info = prior.unwrap_or_default();
    info.correlated_color_temperature = cct;
    info.color_rendering_index = cri;
    if info == ColorInformation::default() {
        None
    } else {
        Some(info)
    }
}

fn build_maintenance(
    prior: Option<LightSourceMaintenance>,
    lifetime: &str,
    lamp_type: &str,
) -> Option<LightSourceMaintenance> {
    let life = parse_opt_i32(lifetime);
    let lamp = empty_to_none(lamp_type.to_string());
    let mut m = prior.unwrap_or_default();
    m.lifetime = life;
    m.cie97_lamp_type = lamp;
    if m == LightSourceMaintenance::default() {
        None
    } else {
        Some(m)
    }
}

/// Interactive Photometry row on a ChangeableLightSource. Picks one of
/// the declared `<Photometry>` ids; Clear unsets the reference. There's
/// no drop-file affordance — Photometries are created via the Builder
/// (which also wraps the file in a `<Photometry>` block).
///
/// Fixed light sources don't carry a `PhotometryReference` per the XSD —
/// their photometry comes from the Emitter's FixedLightEmitter — so this
/// picker is rendered only on the Changeable card.
#[component]
fn PhotometryPicker(light_source_id: String, current: Option<String>) -> impl IntoView {
    let state = use_gldf();
    let photometry_ids = Signal::derive(move || {
        state.with(|s| {
            s.product
                .general_definitions
                .photometries
                .as_ref()
                .map(|ps| ps.photometry.iter().map(|p| p.id.clone()).collect::<Vec<_>>())
                .unwrap_or_default()
        })
    });

    let ls_id_for_pick = light_source_id.clone();
    let on_pick = move |e: web_sys::Event| {
        let Some(target) = e.target() else { return };
        let select: HtmlSelectElement = match target.dyn_into() {
            Ok(s) => s,
            Err(_) => return,
        };
        let val = select.value();
        let photometry_id = if val.is_empty() { None } else { Some(val) };
        let ls_id = ls_id_for_pick.clone();
        state.update(|s| {
            s.apply(GldfAction::SetChangeableLightSourcePhotometry {
                light_source_id: ls_id,
                photometry_id,
            });
        });
    };

    let ls_id_for_clear = light_source_id.clone();
    let on_clear = move |_| {
        let ls_id = ls_id_for_clear.clone();
        state.update(|s| {
            s.apply(GldfAction::SetChangeableLightSourcePhotometry {
                light_source_id: ls_id,
                photometry_id: None,
            });
        });
    };

    let current_for_show = current.clone();
    view! {
        <div class="property photometry-picker" style="grid-column: 1 / -1;">
            <span class="property-label">{ "Photometry" }</span>
            <div style="display: flex; flex-wrap: wrap; gap: 6px; align-items: center;">
                <select on:change=on_pick style="font-size: 12px; padding: 2px 6px;">
                    <option value="" selected=current_for_show.is_none()>{ "— none —" }</option>
                    {move || photometry_ids.get().into_iter().map(|pid| {
                        let selected = current.as_deref() == Some(pid.as_str());
                        let label = pid.clone();
                        view! {
                            <option value=pid selected=selected>{ label }</option>
                        }
                    }).collect::<Vec<_>>()}
                </select>
                {current_for_show.as_ref().map(|_| view! {
                    <button
                        on:click=on_clear
                        style="font-size: 12px; padding: 2px 8px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                    >{ "Clear" }</button>
                })}
            </div>
        </div>
    }
}

/// Interactive Spectrum row: a `<select>` picking one of the declared
/// `<Spectrum>` ids, a clear button when one is already bound, and an
/// `<input type="file">` that drops a `.spd` to embed + link in one shot.
///
/// `current` is the currently-bound spectrum id (None when unset).
#[component]
fn SpectrumPicker(light_source_id: String, current: Option<String>) -> impl IntoView {
    let state = use_gldf();
    let spectrum_ids = Signal::derive(move || {
        state.with(|s| {
            s.product
                .general_definitions
                .spectrums
                .as_ref()
                .map(|ss| ss.spectrum.iter().map(|sp| sp.id.clone()).collect::<Vec<_>>())
                .unwrap_or_default()
        })
    });

    let ls_id_for_pick = light_source_id.clone();
    let on_pick = move |e: web_sys::Event| {
        let Some(target) = e.target() else { return };
        let select: HtmlSelectElement = match target.dyn_into() {
            Ok(s) => s,
            Err(_) => return,
        };
        let val = select.value();
        let spectrum_id = if val.is_empty() { None } else { Some(val) };
        let ls_id = ls_id_for_pick.clone();
        state.update(|s| {
            s.apply(GldfAction::SetLightSourceSpectrum {
                light_source_id: ls_id,
                spectrum_id,
            });
        });
    };

    let ls_id_for_clear = light_source_id.clone();
    let on_clear = move |_| {
        let ls_id = ls_id_for_clear.clone();
        state.update(|s| {
            s.apply(GldfAction::SetLightSourceSpectrum {
                light_source_id: ls_id,
                spectrum_id: None,
            });
        });
    };

    let ls_id_for_drop = light_source_id.clone();
    let on_spd_drop = move |e: web_sys::Event| {
        let Some(target) = e.target() else { return };
        let input: HtmlInputElement = match target.dyn_into() {
            Ok(i) => i,
            Err(_) => return,
        };
        let Some(files) = input.files() else { return };
        let Some(file) = files.get(0) else { return };
        let filename = file.name();
        let gloo_file = gloo::file::File::from(file);
        let ls_id = ls_id_for_drop.clone();
        spawn_local(async move {
            match gloo::file::futures::read_as_bytes(&gloo_file).await {
                Ok(bytes) => {
                    state.update(|s| {
                        s.apply(GldfAction::AddSpectrumFile {
                            bytes,
                            filename,
                            light_source_id: ls_id,
                        });
                    });
                }
                Err(e) => log::error!("read spectrum: {}", e),
            }
        });
        input.set_value("");
    };

    let current_for_show = current.clone();
    view! {
        <div class="property spectrum-picker" style="grid-column: 1 / -1;">
            <span class="property-label">{ "Spectrum" }</span>
            <div style="display: flex; flex-wrap: wrap; gap: 6px; align-items: center;">
                <select on:change=on_pick style="font-size: 12px; padding: 2px 6px;">
                    <option value="" selected=current_for_show.is_none()>{ "— none —" }</option>
                    {move || spectrum_ids.get().into_iter().map(|sid| {
                        let selected = current.as_deref() == Some(sid.as_str());
                        let label = sid.clone();
                        view! {
                            <option value=sid selected=selected>{ label }</option>
                        }
                    }).collect::<Vec<_>>()}
                </select>
                {current_for_show.as_ref().map(|_| view! {
                    <button
                        on:click=on_clear
                        style="font-size: 12px; padding: 2px 8px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                    >{ "Clear" }</button>
                })}
                <label style="font-size: 12px; padding: 2px 8px; border: 1px dashed var(--accent-orange); border-radius: 4px; cursor: pointer; color: var(--accent-orange);">
                    { "Drop .spd…" }
                    <input
                        type="file"
                        accept=".spd"
                        on:change=on_spd_drop
                        style="display: none;"
                    />
                </label>
            </div>
        </div>
    }
}

#[component]
fn PropertyRow(label: &'static str, value: String, code: bool) -> impl IntoView {
    let value_class = if code { "property-value code" } else { "property-value" };
    view! {
        <div class="property">
            <span class="property-label">{ label }</span>
            <span class=value_class>{ value }</span>
        </div>
    }
}
