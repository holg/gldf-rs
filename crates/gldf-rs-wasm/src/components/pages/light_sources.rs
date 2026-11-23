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
use crate::components::pages::variants::resolve_raw_photometry_bytes;
use crate::components::{
    EditModal, IdRenameChip, LdtPanel, LightSourceExportMenu, LocaleFooView, SpdPanel,
};
use crate::i18n::ui::t as ui_t;
use crate::state::{use_gldf, GldfAction};
use gldf_rs::gldf::general_definitions::electrical::Voltage;
use gldf_rs::gldf::general_definitions::lightsources::{
    ChangeableLightSource, ColorInformation, FixedLightSource, LightSourceMaintenance,
    MultiChannelLightSource,
};
use gldf_rs::gldf::header::{Locale, LocaleFoo};
use leptos::prelude::*;
use std::sync::Arc;

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
            let lang = s.ui_language.clone();
            let tr = |key: &str| ui_t(key, &lang);
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
                            data-testid="ls-add-fixed"
                            style="padding: 4px 10px; font-size: 12px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;"
                        >
                            { tr("gldf.light_sources.add_fixed") }
                        </button>
                        <button
                            on:click=on_add_changeable
                            style="padding: 4px 10px; font-size: 12px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;"
                        >
                            { tr("gldf.light_sources.add_changeable") }
                        </button>
                        <button
                            on:click=on_add_multi_channel
                            style="padding: 4px 10px; font-size: 12px; background: var(--accent-purple); color: white; border: none; border-radius: 4px; cursor: pointer;"
                        >
                            { tr("gldf.light_sources.add_multi_channel") }
                        </button>
                    </div>
                }.into_any()
            } else { ().into_any() };

            if fixed.is_empty() && changeable.is_empty() && multi_channel.is_empty() {
                let empty_msg = if editable {
                    tr("gldf.light_sources.empty_editor_hint")
                } else {
                    tr("gldf.light_sources.empty")
                };
                return view! {
                    <>
                        { toolbar }
                        <div class="empty-state">
                            <div class="icon">{ "💡" }</div>
                            <h3>{ tr("gldf.light_sources.no_sources") }</h3>
                            <p>{ empty_msg }</p>
                        </div>
                    </>
                }.into_any();
            }

            view! {
                <>
                    <LinkHint
                        title=tr("gldf.light_sources.hint_title")
                        body=tr("gldf.light_sources.description")
                    />
                    { toolbar }
                    {if !fixed.is_empty() {
                        view! {
                            <h3 style="margin-bottom: 16px; color: var(--text-secondary);">{ tr("gldf.light_sources.fixed_title") }</h3>
                            <div class="light-source-cards">
                                {fixed.into_iter().map(|ls| view! { <FixedCard ls=ls editable=editable /> }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    } else { ().into_any() }}
                    {if !changeable.is_empty() {
                        view! {
                            <h3 style="margin: 24px 0 16px; color: var(--text-secondary);">{ tr("gldf.light_sources.changeable_title") }</h3>
                            <div class="light-source-cards">
                                {changeable.into_iter().map(|ls| view! { <ChangeableCard ls=ls editable=editable /> }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    } else { ().into_any() }}
                    {if !multi_channel.is_empty() {
                        view! {
                            <h3 style="margin: 24px 0 16px; color: var(--text-secondary);">{ tr("gldf.light_sources.multi_channel_title") }</h3>
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
        let msg = format!(
            "Remove light source '{}'? Any Emitter or Equipment reference to it will dangle.",
            id
        );
        if crate::components::edit_util::confirm(&msg) {
            state.update(|s| s.apply(GldfAction::RemoveLightSource { id }));
        }
    };
    let on_rename: Arc<dyn Fn(String) + Send + Sync> = {
        let old_id = ls_for_rename.clone();
        Arc::new(move |new_id| {
            state.update(|s| {
                s.apply(GldfAction::RenameFixedLightSource {
                    old_id: old_id.clone(),
                    new_id,
                })
            });
        })
    };

    view! {
        <div class="light-source-card">
            <div class="card-header-row">
                <IdRenameChip current_id=ls_id.clone() editable=editable on_rename=on_rename />
                <span class="card-type">{ "Fixed" }</span>
                {if editable && !is_editing.get() {
                    let ls_id_for_testid = ls_id.clone();
                    let ls_id_for_export = ls_id.clone();
                    view! {
                        <span style="margin-left: auto; display: inline-flex; gap: 6px;">
                            <button
                                on:click=on_edit
                                data-testid=format!("ls-edit-{}", ls_id_for_testid)
                                style="padding: 2px 8px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                            >{ "✎ Edit" }</button>
                            <LightSourceExportMenu light_source_id=ls_id_for_export />
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
            <FixedLightSourceRenders light_source_id=ls.id.clone() />
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
                        {color.tlci.map(|t| view! {
                            <PropertyRow label="TLCI" value=format!("{}", t) code=false />
                        })}
                        {color.melanopic_factor.map(|m| view! {
                            <PropertyRow label="Melanopic factor" value=format!("{:.3}", m) code=false />
                        })}
                        {color.initial_color_tolerance.clone().map(|t| view! {
                            <PropertyRow label="Initial color tolerance" value=t code=false />
                        })}
                        {color.maintained_color_tolerance.clone().map(|t| view! {
                            <PropertyRow label="Maintained color tolerance" value=t code=false />
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
        let msg = format!(
            "Remove light source '{}'? Any Equipment reference to it will dangle.",
            id
        );
        if crate::components::edit_util::confirm(&msg) {
            state.update(|s| s.apply(GldfAction::RemoveLightSource { id }));
        }
    };
    let on_rename: Arc<dyn Fn(String) + Send + Sync> = {
        let old_id = ls_for_rename.clone();
        Arc::new(move |new_id| {
            state.update(|s| {
                s.apply(GldfAction::RenameChangeableLightSource {
                    old_id: old_id.clone(),
                    new_id,
                })
            });
        })
    };

    view! {
        <div class="light-source-card">
            <div class="card-header-row">
                <IdRenameChip current_id=ls_id.clone() editable=editable on_rename=on_rename />
                <span class="card-type changeable">{ "Changeable" }</span>
                {if editable && !is_editing.get() {
                    let ls_id_for_export = ls_id.clone();
                    view! {
                        <span style="margin-left: auto; display: inline-flex; gap: 6px;">
                            <button
                                on:click=on_edit
                                style="padding: 2px 8px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                            >{ "✎ Edit" }</button>
                            <LightSourceExportMenu light_source_id=ls_id_for_export />
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
                        {color.tlci.map(|t| view! {
                            <PropertyRow label="TLCI" value=format!("{}", t) code=false />
                        })}
                        {color.melanopic_factor.map(|m| view! {
                            <PropertyRow label="Melanopic factor" value=format!("{:.3}", m) code=false />
                        })}
                        {color.initial_color_tolerance.clone().map(|t| view! {
                            <PropertyRow label="Initial color tolerance" value=t code=false />
                        })}
                        {color.maintained_color_tolerance.clone().map(|t| view! {
                            <PropertyRow label="Maintained color tolerance" value=t code=false />
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
    let ls_for_rename = ls.id.clone();
    let ls_for_remove = ls.id.clone();
    let is_editing = RwSignal::new(false);

    let on_edit = move |_| is_editing.set(true);
    let on_remove = move |_| {
        let id = ls_for_remove.clone();
        let msg = format!("Remove multi-channel light source '{}'? Any MultiChannelLightEmitter referencing it will dangle.", id);
        if crate::components::edit_util::confirm(&msg) {
            state.update(|s| s.apply(GldfAction::RemoveLightSource { id }));
        }
    };
    let on_rename: Arc<dyn Fn(String) + Send + Sync> = {
        let old_id = ls_for_rename.clone();
        Arc::new(move |new_id| {
            state.update(|s| {
                s.apply(GldfAction::RenameMultiChannelLightSource {
                    old_id: old_id.clone(),
                    new_id,
                })
            });
        })
    };

    let ls_id_for_modal_title = ls.id.clone();
    let ls_id_for_export = ls.id.clone();
    let ls_for_modal = ls.clone();
    let close_modal: Arc<dyn Fn() + Send + Sync> = Arc::new(move || is_editing.set(false));

    view! {
        <div class="light-source-card">
            <div class="card-header-row">
                <IdRenameChip current_id=ls.id.clone() editable=editable on_rename=on_rename />
                <span class="card-type" style="background: var(--accent-purple);">{ "MultiChannel" }</span>
                {if editable {
                    let ls_id_for_export = ls_id_for_export.clone();
                    view! {
                        <span style="margin-left: auto; display: inline-flex; gap: 6px;">
                            <button
                                on:click=on_edit
                                style="padding: 2px 8px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                            >{ "✎ Edit" }</button>
                            <LightSourceExportMenu light_source_id=ls_id_for_export />
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
                .map(|ss| {
                    ss.spectrum
                        .iter()
                        .map(|sp| sp.id.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    });
    let photometry_ids = Signal::derive(move || {
        state.with(|s| {
            s.product
                .general_definitions
                .photometries
                .as_ref()
                .map(|ps| {
                    ps.photometry
                        .iter()
                        .map(|p| p.id.clone())
                        .collect::<Vec<_>>()
                })
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
        state.update(|s| {
            s.apply(GldfAction::RemoveMultiChannelChannel {
                ls_id: ls_id_for_remove.clone(),
                index,
            })
        });
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
        state.update(|s| {
            s.apply(GldfAction::UpdateMultiChannelChannel {
                ls_id: ls_id_for_save.clone(),
                index,
                channel: Box::new(new_channel),
            })
        });
        is_editing.set(false);
    };

    let on_name_input = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = crate::components::edit_util::event_value(&e) {
            s_name.set(v);
        }
    };
    let on_spectrum_change = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = crate::components::edit_util::event_value(&e) {
            s_spectrum.set(v);
        }
    };
    let on_photo_change = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = crate::components::edit_util::event_value(&e) {
            s_photo.set(v);
        }
    };
    let on_flux_input = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = crate::components::edit_util::event_value(&e) {
            s_flux.set(v);
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
                                on:input=on_name_input
                                style="width: 100%; padding: 2px 4px; font-size: 11px;"
                            />
                        </td>
                        <td style="padding: 4px;">
                            <select on:change=on_spectrum_change style="width: 100%; font-size: 11px;">
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
                            <select on:change=on_photo_change style="width: 100%; font-size: 11px;">
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
                                on:input=on_flux_input
                                style="width: 80px; padding: 2px 4px; font-size: 11px;"
                            />
                        </td>
                        <td style="padding: 4px; display: flex; gap: 4px;">
                            <button
                                on:click=on_save.clone()
                                style="padding: 2px 6px; font-size: 11px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;"
                            >{ "Save" }</button>
                            <button
                                on:click=on_cancel
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
                .map(|ss| {
                    ss.spectrum
                        .iter()
                        .map(|sp| sp.id.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    });
    let photometry_ids = Signal::derive(move || {
        state.with(|s| {
            s.product
                .general_definitions
                .photometries
                .as_ref()
                .map(|ps| {
                    ps.photometry
                        .iter()
                        .map(|p| p.id.clone())
                        .collect::<Vec<_>>()
                })
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
            photometry_reference: ChannelPhotometryReference {
                photometry_id: photo,
            },
            rated_luminous_flux: flux,
            color_information: None,
        };
        state.update(|s| {
            s.apply(GldfAction::AddMultiChannelChannel {
                ls_id: ls_id_for_submit.clone(),
                channel: Box::new(channel),
            })
        });
        is_open.set(false);
        s_name.set(String::new());
        s_flux.set(String::from("0"));
    };

    let on_name_input = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = crate::components::edit_util::event_value(&e) {
            s_name.set(v);
        }
    };
    let on_spectrum_change = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = crate::components::edit_util::event_value(&e) {
            s_spectrum.set(v);
        }
    };
    let on_photo_change = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = crate::components::edit_util::event_value(&e) {
            s_photo.set(v);
        }
    };
    let on_flux_input = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = crate::components::edit_util::event_value(&e) {
            s_flux.set(v);
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
                            on:input=on_name_input
                            style="padding: 2px 4px; font-size: 11px;"
                        />
                    </label>
                    <label style="display: grid; grid-template-columns: 140px 1fr; gap: 6px; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-tertiary);">{ "Spectrum" }</span>
                        <select on:change=on_spectrum_change style="font-size: 11px;">
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
                        <select on:change=on_photo_change style="font-size: 11px;">
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
                            on:input=on_flux_input
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
                            on:click=on_cancel
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
    let position = ls
        .light_source_position_of_usage
        .clone()
        .unwrap_or_default();

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

        state.update(|s| {
            s.apply(GldfAction::UpdateMultiChannelLightSource {
                id: id_for_save.clone(),
                payload: Box::new(new_ls),
            })
        });
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
                .and_then(|l| l.multi_channel_light_source.iter().find(|m| m.id == target))
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
fn FixedEditForm(
    ls: FixedLightSource,
    on_done: impl Fn() + Send + Sync + 'static + Clone,
) -> impl IntoView {
    let state = use_gldf();
    let original = ls.clone();
    let id_for_save = ls.id.clone();
    let ls_id_for_pickers = ls.id.clone();
    let current_spectrum = ls.spectrum_reference.clone().map(|sr| sr.spectrum_id);

    let name_first = first_locale_value(&ls.name);
    let desc_first = ls
        .description
        .as_ref()
        .map(first_locale_value)
        .unwrap_or_default();
    let manufacturer = ls.manufacturer.clone().unwrap_or_default();
    let gtin = ls.gtin.clone().unwrap_or_default();
    let power = ls
        .rated_input_power
        .map(|p| p.to_string())
        .unwrap_or_default();
    let voltage = ls
        .rated_input_voltage
        .as_ref()
        .and_then(|v| v.fixed_voltage)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let cct = ls
        .color_information
        .as_ref()
        .and_then(|c| c.correlated_color_temperature)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let cri = ls
        .color_information
        .as_ref()
        .and_then(|c| c.color_rendering_index)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let cct_low = ls
        .color_information
        .as_ref()
        .and_then(|c| c.color_temperature_adjusting_range.as_ref())
        .and_then(|r| r.lower)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let cct_high = ls
        .color_information
        .as_ref()
        .and_then(|c| c.color_temperature_adjusting_range.as_ref())
        .and_then(|r| r.upper)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let initial_tol = ls
        .color_information
        .as_ref()
        .and_then(|c| c.initial_color_tolerance.clone())
        .unwrap_or_default();
    let maintained_tol = ls
        .color_information
        .as_ref()
        .and_then(|c| c.maintained_color_tolerance.clone())
        .unwrap_or_default();
    let tlci = ls
        .color_information
        .as_ref()
        .and_then(|c| c.tlci)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let melanopic = ls
        .color_information
        .as_ref()
        .and_then(|c| c.melanopic_factor)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let lifetime = ls
        .light_source_maintenance
        .as_ref()
        .and_then(|m| m.lifetime)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let lamp_type = ls
        .light_source_maintenance
        .as_ref()
        .and_then(|m| m.cie97_lamp_type.clone())
        .unwrap_or_default();
    let position = ls
        .light_source_position_of_usage
        .clone()
        .unwrap_or_default();
    let zhaga = ls.zhaga_standard.unwrap_or(false);

    let s_name = RwSignal::new(name_first);
    let s_desc = RwSignal::new(desc_first);
    let s_manuf = RwSignal::new(manufacturer);
    let s_gtin = RwSignal::new(gtin);
    let s_power = RwSignal::new(power);
    let s_volt = RwSignal::new(voltage);
    let s_cct = RwSignal::new(cct);
    let s_cri = RwSignal::new(cri);
    let s_cct_low = RwSignal::new(cct_low);
    let s_cct_high = RwSignal::new(cct_high);
    let s_init_tol = RwSignal::new(initial_tol);
    let s_maint_tol = RwSignal::new(maintained_tol);
    let s_tlci = RwSignal::new(tlci);
    let s_melanopic = RwSignal::new(melanopic);
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
            Some(upsert_locale_value(
                new_ls.description.unwrap_or_default(),
                &desc_text,
            ))
        };
        new_ls.manufacturer = empty_to_none(s_manuf.get());
        new_ls.gtin = empty_to_none(s_gtin.get());
        new_ls.rated_input_power = parse_opt_f64(&s_power.get());
        new_ls.rated_input_voltage = build_voltage(new_ls.rated_input_voltage, &s_volt.get());
        new_ls.color_information = build_color_info(
            new_ls.color_information,
            &s_cct.get(),
            &s_cri.get(),
            &s_cct_low.get(),
            &s_cct_high.get(),
            &s_init_tol.get(),
            &s_maint_tol.get(),
            &s_tlci.get(),
            &s_melanopic.get(),
        );
        new_ls.light_source_maintenance = build_maintenance(
            new_ls.light_source_maintenance,
            &s_life.get(),
            &s_lamp.get(),
        );
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
            <TextRow label="Name (default locale)" value=s_name testid="ls-edit-name" />
            <TextRow label="Description" value=s_desc />
            <TextRow label="Manufacturer" value=s_manuf testid="ls-edit-manufacturer" />
            <TextRow label="GTIN" value=s_gtin />
            <TextRow label="Rated Power (W)" value=s_power testid="ls-edit-power" />
            <TextRow label="Fixed Voltage (V)" value=s_volt />
            <TextRow label="CCT (K)" value=s_cct testid="ls-edit-cct" />
            <TextRow label="CRI (Ra)" value=s_cri />
            <TextRow label="CCT range lower (K)" value=s_cct_low />
            <TextRow label="CCT range upper (K)" value=s_cct_high />
            <TextRow label="Initial color tolerance" value=s_init_tol />
            <TextRow label="Maintained color tolerance" value=s_maint_tol />
            <TextRow label="TLCI" value=s_tlci />
            <TextRow label="Melanopic factor" value=s_melanopic />
            <TextRow label="Lifetime (h)" value=s_life />
            <TextRow label="Lamp Type (CIE 97)" value=s_lamp />
            <TextRow label="Position of Usage" value=s_pos />
            <BoolRow label="Zhaga" value=s_zhaga />
            <SpectrumPicker
                light_source_id=ls_id_for_pickers.clone()
                current=current_spectrum.clone()
            />
            <FixedLightSourcePhotometryUsage light_source_id=ls_id_for_pickers.clone() />
            <p style="margin: 4px 0 0; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                { "Other fields (energy labels, active power table, light source images, …) preserved on save." }
            </p>
            <div style="display: flex; gap: 6px; margin-top: 6px;">
                <button on:click=on_save data-testid="ls-edit-save" style="padding: 4px 10px; font-size: 12px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;">{ "Save" }</button>
                <button on:click=on_cancel style="padding: 4px 10px; font-size: 12px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);">{ "Cancel" }</button>
            </div>
        </div>
    }
}

/// Inline editor for a ChangeableLightSource — shorter than the Fixed form
/// because ChangeableLightSource has fewer first-class fields in the schema.
#[component]
fn ChangeableEditForm(
    ls: ChangeableLightSource,
    on_done: impl Fn() + Send + Sync + 'static + Clone,
) -> impl IntoView {
    let state = use_gldf();
    let original = ls.clone();
    let id_for_save = ls.id.clone();
    let ls_id_for_pickers = ls.id.clone();
    let current_photometry = ls.photometry_reference.clone().map(|p| p.photometry_id);
    let current_spectrum = ls.spectrum_reference.clone().map(|sr| sr.spectrum_id);

    let name_first = first_locale_value(&ls.name);
    let desc_first = ls
        .description
        .as_ref()
        .map(first_locale_value)
        .unwrap_or_default();
    let manufacturer = ls.manufacturer.clone().unwrap_or_default();
    let cct = ls
        .color_information
        .as_ref()
        .and_then(|c| c.correlated_color_temperature)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let cri = ls
        .color_information
        .as_ref()
        .and_then(|c| c.color_rendering_index)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let cct_low = ls
        .color_information
        .as_ref()
        .and_then(|c| c.color_temperature_adjusting_range.as_ref())
        .and_then(|r| r.lower)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let cct_high = ls
        .color_information
        .as_ref()
        .and_then(|c| c.color_temperature_adjusting_range.as_ref())
        .and_then(|r| r.upper)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let initial_tol = ls
        .color_information
        .as_ref()
        .and_then(|c| c.initial_color_tolerance.clone())
        .unwrap_or_default();
    let maintained_tol = ls
        .color_information
        .as_ref()
        .and_then(|c| c.maintained_color_tolerance.clone())
        .unwrap_or_default();
    let tlci = ls
        .color_information
        .as_ref()
        .and_then(|c| c.tlci)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let melanopic = ls
        .color_information
        .as_ref()
        .and_then(|c| c.melanopic_factor)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let lifetime = ls
        .light_source_maintenance
        .as_ref()
        .and_then(|m| m.lifetime)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let lamp_type = ls
        .light_source_maintenance
        .as_ref()
        .and_then(|m| m.cie97_lamp_type.clone())
        .unwrap_or_default();

    let s_name = RwSignal::new(name_first);
    let s_desc = RwSignal::new(desc_first);
    let s_manuf = RwSignal::new(manufacturer);
    let s_cct = RwSignal::new(cct);
    let s_cri = RwSignal::new(cri);
    let s_cct_low = RwSignal::new(cct_low);
    let s_cct_high = RwSignal::new(cct_high);
    let s_init_tol = RwSignal::new(initial_tol);
    let s_maint_tol = RwSignal::new(maintained_tol);
    let s_tlci = RwSignal::new(tlci);
    let s_melanopic = RwSignal::new(melanopic);
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
            Some(upsert_locale_value(
                new_ls.description.unwrap_or_default(),
                &desc_text,
            ))
        };
        new_ls.manufacturer = empty_to_none(s_manuf.get());
        new_ls.color_information = build_color_info(
            new_ls.color_information,
            &s_cct.get(),
            &s_cri.get(),
            &s_cct_low.get(),
            &s_cct_high.get(),
            &s_init_tol.get(),
            &s_maint_tol.get(),
            &s_tlci.get(),
            &s_melanopic.get(),
        );
        new_ls.light_source_maintenance = build_maintenance(
            new_ls.light_source_maintenance,
            &s_life.get(),
            &s_lamp.get(),
        );

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
            <TextRow label="CCT (K)" value=s_cct />
            <TextRow label="CRI (Ra)" value=s_cri />
            <TextRow label="CCT range lower (K)" value=s_cct_low />
            <TextRow label="CCT range upper (K)" value=s_cct_high />
            <TextRow label="Initial color tolerance" value=s_init_tol />
            <TextRow label="Maintained color tolerance" value=s_maint_tol />
            <TextRow label="TLCI" value=s_tlci />
            <TextRow label="Melanopic factor" value=s_melanopic />
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
fn TextRow(
    label: &'static str,
    value: RwSignal<String>,
    #[prop(default = "")] testid: &'static str,
) -> impl IntoView {
    let on_input = move |e: crate::components::edit_util::EditEvent| {
        let Some(v) = crate::components::edit_util::event_value(&e) else {
            return;
        };
        value.set(v);
    };
    view! {
        <label style="display: grid; grid-template-columns: 160px 1fr; gap: 8px; align-items: center; font-size: 12px;">
            <span style="color: var(--text-secondary);">{ label }</span>
            <input
                type="text"
                prop:value=move || value.get()
                on:input=on_input
                data-testid=testid
                style="padding: 3px 6px; font-size: 12px; background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: 4px; color: var(--text-primary);"
            />
        </label>
    }
}

#[component]
fn BoolRow(label: &'static str, value: RwSignal<bool>) -> impl IntoView {
    let on_change = move |e: crate::components::edit_util::EditEvent| {
        let Some(c) = crate::components::edit_util::event_checked(&e) else {
            return;
        };
        value.set(c);
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
    lf.locale
        .first()
        .map(|l| l.value.clone())
        .unwrap_or_default()
}

/// Insert/overwrite the first Locale's `value`; if no locales exist, push a
/// blank-language one. Preserves any other locale entries.
fn upsert_locale_value(mut lf: LocaleFoo, value: &str) -> LocaleFoo {
    // Avoid emitting `<Locale language=""/>` for an empty value. The
    // self-closing form trips `quick-xml`'s `#[serde(rename = "$text")]`
    // deserializer on reload — "missing field `$text`" — so an empty
    // Locale becomes a non-round-trippable file. Drop the entry
    // entirely when the user clears the name field.
    let trimmed = value.trim();
    if trimmed.is_empty() {
        if !lf.locale.is_empty() {
            lf.locale.remove(0);
        }
        return lf;
    }
    if let Some(first) = lf.locale.first_mut() {
        first.value = trimmed.to_string();
    } else {
        lf.locale.push(Locale {
            language: String::new(),
            value: trimmed.to_string(),
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
        (None, Some(f)) => Some(Voltage {
            fixed_voltage: Some(f),
            ..Default::default()
        }),
    }
}

/// Preserve prior `color_information` (tunable range, x/y coordinates,
/// MacAdam steps) and overwrite CCT + CRI fields from the form.
#[allow(clippy::too_many_arguments)]
fn build_color_info(
    prior: Option<ColorInformation>,
    cct: &str,
    cri: &str,
    cct_low: &str,
    cct_high: &str,
    initial_tol: &str,
    maintained_tol: &str,
    tlci: &str,
    melanopic: &str,
) -> Option<ColorInformation> {
    use gldf_rs::gldf::general_definitions::lightsources::ColorTemperatureAdjustingRange;
    let mut info = prior.unwrap_or_default();
    info.correlated_color_temperature = parse_opt_i32(cct);
    info.color_rendering_index = parse_opt_i32(cri);
    let low = parse_opt_i32(cct_low);
    let high = parse_opt_i32(cct_high);
    info.color_temperature_adjusting_range = match (low, high) {
        (None, None) => None,
        (l, h) => Some(ColorTemperatureAdjustingRange { lower: l, upper: h }),
    };
    info.initial_color_tolerance = empty_to_none(initial_tol.to_string());
    info.maintained_color_tolerance = empty_to_none(maintained_tol.to_string());
    info.tlci = parse_opt_i32(tlci);
    info.melanopic_factor = parse_opt_f64(melanopic);
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
                .map(|ps| {
                    ps.photometry
                        .iter()
                        .map(|p| p.id.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    });

    let ls_id_for_pick = light_source_id.clone();
    let on_pick = move |e: crate::components::edit_util::EditEvent| {
        let Some(val) = crate::components::edit_util::event_value(&e) else {
            return;
        };
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
/// `<Spectrum>` ids and a clear button when one is bound. The actual
/// `.spd` upload happens on the Files page — dropping `.spd` there
/// auto-registers a `<Spectrum>` entry that shows up in this dropdown.
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
                .map(|ss| {
                    ss.spectrum
                        .iter()
                        .map(|sp| sp.id.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    });

    let ls_id_for_pick = light_source_id.clone();
    let on_pick = move |e: crate::components::edit_util::EditEvent| {
        let Some(val) = crate::components::edit_util::event_value(&e) else {
            return;
        };
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
                {move || {
                    let empty = spectrum_ids.with(|v| v.is_empty());
                    if empty {
                        view! {
                            <span style="font-size: 11px; color: var(--text-secondary);">
                                { "No Spectrum declared — drop a .spd on " }
                                <strong style="color: var(--accent-blue);">{ "📁 Files" }</strong>
                                { " to register one." }
                            </span>
                        }.into_any()
                    } else {
                        view! { <span /> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn PropertyRow(label: &'static str, value: String, code: bool) -> impl IntoView {
    let value_class = if code {
        "property-value code"
    } else {
        "property-value"
    };
    view! {
        <div class="property">
            <span class="property-label">{ label }</span>
            <span class=value_class>{ value }</span>
        </div>
    }
}

/// Photometry usage picker for FixedLightSource.
///
/// The XSD doesn't put a `<PhotometryReference>` directly on
/// `FixedLightSource` — it lives on the `FixedLightEmitter` that binds
/// LS↔Photometry. So instead of mirroring the `SpectrumPicker` we
/// surface every emitter wired to this LS as a row with a
/// `<select>` of the available photometries, and dispatch
/// [`GldfAction::SetFixedLightEmitterPhotometry`] on change.
///
/// Mirrors the user's request: *"editing the lightsource, still does
/// not allow us to reassign the photometry, like for Spectrum"*. They
/// can now repoint each emitter's photometry from the LS edit modal
/// without round-tripping through the Emitters page.
#[component]
fn FixedLightSourcePhotometryUsage(light_source_id: String) -> impl IntoView {
    let state = use_gldf();
    let ls_id = light_source_id.clone();

    let bindings = Signal::derive(move || {
        let ls_id = ls_id.clone();
        state.with(|s| {
            let mut out: Vec<(String, usize, String)> = Vec::new();
            let Some(emitters) = s.product.general_definitions.emitters.as_ref() else {
                return out;
            };
            for em in &emitters.emitter {
                for (idx, fle) in em.fixed_light_emitter.iter().enumerate() {
                    if fle.light_source_reference.fixed_light_source_id.as_deref()
                        == Some(ls_id.as_str())
                    {
                        out.push((
                            em.id.clone(),
                            idx,
                            fle.photometry_reference.photometry_id.clone(),
                        ));
                    }
                }
            }
            out
        })
    });

    let photometry_ids = Signal::derive(move || {
        state.with(|s| {
            s.product
                .general_definitions
                .photometries
                .as_ref()
                .map(|ps| {
                    ps.photometry
                        .iter()
                        .map(|p| p.id.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    });

    view! {
        <div class="property" style="grid-column: 1 / -1;">
            <span class="property-label">{ "Photometry bindings" }</span>
            <div style="display: flex; flex-direction: column; gap: 4px;">
                {move || {
                    let rows = bindings.get();
                    if rows.is_empty() {
                        view! {
                            <span style="font-size: 11px; color: var(--text-secondary); font-style: italic;">
                                { "No emitter binds this LightSource yet — go to " }
                                <strong style="color: var(--accent-blue);">{ "💡 Emitters" }</strong>
                                { " to wire it up." }
                            </span>
                        }.into_any()
                    } else {
                        let ids = photometry_ids.get();
                        rows.into_iter().map(|(emitter_id, idx, current)| {
                            let emitter_id_for_pick = emitter_id.clone();
                            let on_pick = move |e: crate::components::edit_util::EditEvent| {
                                let Some(val) = crate::components::edit_util::event_value(&e) else { return };
                                if val.is_empty() {
                                    return;
                                }
                                let emitter_id = emitter_id_for_pick.clone();
                                state.update(|s| {
                                    s.apply(GldfAction::SetFixedLightEmitterPhotometry {
                                        emitter_id,
                                        index: idx,
                                        photometry_id: val,
                                    });
                                });
                            };
                            let ids = ids.clone();
                            let current_for_match = current.clone();
                            view! {
                                <div style="display: flex; gap: 6px; align-items: center; font-size: 12px;">
                                    <span style="color: var(--text-secondary);">{ format!("{emitter_id}[{idx}]") }</span>
                                    <span style="color: var(--text-secondary);">{ "→" }</span>
                                    <select
                                        on:change=on_pick
                                        data-testid=format!("ls-photometry-{emitter_id}-{idx}")
                                        style="font-size: 12px; padding: 2px 6px;"
                                    >
                                        {ids.into_iter().map(|pid| {
                                            let selected = pid == current_for_match;
                                            let label = pid.clone();
                                            view! {
                                                <option value=pid selected=selected>{ label }</option>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </select>
                                </div>
                            }
                        }).collect::<Vec<_>>().into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// (emitter id, child index, photometry id, rotation, photometry bytes)
type PhotometryBinding = (String, usize, String, Option<String>, Vec<u8>);

/// Render the LightSource's photometric + spectral signatures inline on
/// the card. This is the **sole rendering point** for these in the
/// editor — the XSD's reference graph makes LightSource the natural
/// home (Spectrum is owned via `SpectrumReference`, Photometry resolves
/// via the FixedLightEmitter chain bound to this LS). Emitter cards
/// don't render because one Emitter can hold many FixedLightEmitter
/// children with their own rotations; Variant cards don't render
/// because they only compose existing chains.
///
/// What lands:
/// * One mini `<LdtPanel>` per FixedLightEmitter that references this
///   LS — labelled with `emitter_id[idx]` + the photometry id +
///   optional rotation.
/// * One `<SpdPanel>` per `SpectrumReference` — driven by the
///   embedded SPD bytes (TM-33 / Luxeon / generic SPD all handled by
///   SpdPanel's parse fallbacks).
///
/// Empty when no emitter binds this LS yet — keeps the card tidy
/// during early wizard flows.
#[component]
fn FixedLightSourceRenders(light_source_id: String) -> impl IntoView {
    let state = use_gldf();
    let ls_id = light_source_id.clone();

    view! {
        <div class="light-source-renders" style="margin-top: 10px;">
            {move || state.with(|s| {
                let mut bindings: Vec<PhotometryBinding> = Vec::new();
                if let Some(emitters) = s.product.general_definitions.emitters.as_ref() {
                    for em in &emitters.emitter {
                        for (idx, fle) in em.fixed_light_emitter.iter().enumerate() {
                            if fle.light_source_reference.fixed_light_source_id.as_deref()
                                != Some(ls_id.as_str())
                            {
                                continue;
                            }
                            let photometry_id = fle.photometry_reference.photometry_id.clone();
                            let bytes = resolve_raw_photometry_bytes(
                                &s.product,
                                &s.embedded_files,
                                &photometry_id,
                            );
                            let rot = fle.rotation.as_ref().map(|r| {
                                format!("rot {}/{}/{} G0={}", r.x, r.y, r.z, r.g0)
                            });
                            if let Some(b) = bytes {
                                bindings.push((em.id.clone(), idx, photometry_id, rot, b));
                            }
                        }
                    }
                }

                let spectrum_bytes: Option<Vec<u8>> = s
                    .product
                    .general_definitions
                    .light_sources
                    .as_ref()
                    .and_then(|lss| {
                        lss.fixed_light_source
                            .iter()
                            .find(|f| f.id == ls_id)
                            .and_then(|f| f.spectrum_reference.as_ref())
                    })
                    .and_then(|sr| {
                        s.product
                            .general_definitions
                            .spectrums
                            .as_ref()
                            .and_then(|ss| ss.spectrum.iter().find(|sp| sp.id == sr.spectrum_id))
                    })
                    .and_then(|sp| sp.spectrum_file_reference.as_ref())
                    .and_then(|sfr| s.embedded_files.get(&sfr.file_id).cloned());

                let no_photometry = bindings.is_empty();
                let no_spectrum = spectrum_bytes.is_none();
                if no_photometry && no_spectrum {
                    return view! {
                        <p style="margin: 0; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                            { "No emitter binds this LightSource yet, and no SpectrumReference is set — wire one on the Emitters page or via the Spectrum picker." }
                        </p>
                    }.into_any();
                }

                let bindings_label = bindings.len();
                let spectrum_label = if no_spectrum { "—".to_string() } else { "1".to_string() };
                view! {
                    <details open=true style="border: 1px solid var(--border-color); border-radius: 6px; padding: 6px 8px;">
                        <summary style="cursor: pointer; font-size: 12px; color: var(--text-secondary);">
                            { format!("📊 Photometry ({}) · 🌈 Spectrum ({})", bindings_label, spectrum_label) }
                        </summary>
                        <div style="display: flex; flex-direction: column; gap: 12px; margin-top: 8px;">
                            {if no_photometry {
                                view! {
                                    <p style="margin: 0; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                                        { "No emitter binds this LightSource yet." }
                                    </p>
                                }.into_any()
                            } else {
                                view! {
                                    <div style="display: flex; flex-wrap: wrap; gap: 12px;">
                                        {bindings.into_iter().map(|(em_id, idx, photometry_id, rotation, bytes)| {
                                            let label = format!("{em_id}[{idx}] → {photometry_id}");
                                            view! {
                                                <div style="flex: 0 0 300px;">
                                                    <div style="font-size: 11px; color: var(--text-secondary); margin-bottom: 4px;">
                                                        { label }
                                                        {rotation.map(|r| view! {
                                                            <span style="color: var(--text-tertiary); margin-left: 6px;">{ format!("· {r}") }</span>
                                                        })}
                                                    </div>
                                                    <LdtPanel bytes=bytes width=280.0 height=220.0 />
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_any()
                            }}
                            {spectrum_bytes.map(|sb| view! {
                                <div>
                                    <div style="font-size: 11px; color: var(--text-secondary); margin-bottom: 4px;">
                                        { "Spectrum (from SpectrumReference)" }
                                    </div>
                                    <SpdPanel bytes=sb />
                                </div>
                            })}
                        </div>
                    </details>
                }.into_any()
            })}
        </div>
    }
}
