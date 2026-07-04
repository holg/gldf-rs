//! Emitters page — per-emitter cards.
//!
//! Mirrors `view_emitters` from the Yew side, minus the embedded `LdtViewer`
//! (which is a three-d renderer, slice 3 territory). Slice 1 surfaces the
//! emitter structure: id, fixed / changeable / sensor sub-types, photometry
//! + light-source references, emergency badges. The LDT viewer slot shows
//! the photometry file name + size so the user knows where the chart will
//! land in slice 3.
//!
//! Each emitter row also renders the resolved **Photometry · LightSource ·
//! Spectrum** triple. The spectrum is transitive — it's not a direct field
//! on the emitter but lives on the LightSource it references — so this
//! page is the natural place to see the full chain in one glance.

use crate::components::pages::link_hint::LinkHint;
use crate::components::{EditModal, EmitterDraft, EmitterTypeForm, IdRenameChip};
use crate::i18n::ui::t as ui_t;
use crate::state::{use_gldf, GldfAction, LightEmitterKind};
use gldf_rs::gldf::general_definitions::lightsources::Emitter;
use leptos::prelude::*;
use std::sync::Arc;

#[component]
pub fn EmittersPage(#[prop(default = false)] editable: bool) -> impl IntoView {
    let state = use_gldf();
    // Toggles the "+ New Emitter slot" inline form (shown above the cards).
    // Required-first-child means we can't auto-create on click — the user
    // must pick a type and fill its refs first.
    let show_new_form = RwSignal::new(false);

    let on_open_new_form = move |_| show_new_form.set(true);
    let close_new_form_arc: Arc<dyn Fn() + Send + Sync> = Arc::new(move || show_new_form.set(false));

    view! {
        {move || state.with(|s| {
            let lang = s.ui_language.clone();
            let tr = |key: &str| ui_t(key, &lang);
            let emitters: Vec<Emitter> = s
                .product
                .general_definitions
                .emitters
                .as_ref()
                .map(|e| e.emitter.clone())
                .unwrap_or_default();
            // Catalog pick-lists fed to every EmitterTypeForm instance.
            let photometry_ids: Vec<String> = s
                .product
                .general_definitions
                .photometries
                .as_ref()
                .map(|p| p.photometry.iter().map(|p| p.id.clone()).collect())
                .unwrap_or_default();
            let fixed_ls_ids: Vec<String> = s
                .product
                .general_definitions
                .light_sources
                .as_ref()
                .map(|l| l.fixed_light_source.iter().map(|l| l.id.clone()).collect())
                .unwrap_or_default();
            let multi_channel_ls_ids: Vec<String> = s
                .product
                .general_definitions
                .light_sources
                .as_ref()
                .map(|l| l.multi_channel_light_source.iter().map(|l| l.id.clone()).collect())
                .unwrap_or_default();
            let sensor_ids: Vec<String> = s
                .product
                .general_definitions
                .sensors
                .as_ref()
                .map(|ss| ss.sensor.iter().map(|s| s.id.clone()).collect())
                .unwrap_or_default();

            // Submit handler for the top-of-page new-emitter form.
            let new_form_submit: Arc<dyn Fn(EmitterDraft) + Send + Sync> = {
                let show_new_form = show_new_form;
                Arc::new(move |draft: EmitterDraft| {
                    let name = draft.child_name.clone();
                    state.update(|s| s.apply(GldfAction::AddEmitterWithChild {
                        child_name: if name.trim().is_empty() { None } else { Some(name) },
                        kind: draft.kind,
                        photometry_id: draft.photometry_id,
                        fixed_light_source_id: draft.fixed_light_source_id,
                        multi_channel_light_source_id: draft.multi_channel_light_source_id,
                        sensor_id: draft.sensor_id,
                    }));
                    show_new_form.set(false);
                })
            };

            let toolbar = if editable {
                let photometry_ids_for_form = photometry_ids.clone();
                let fixed_ls_for_form = fixed_ls_ids.clone();
                let mc_for_form = multi_channel_ls_ids.clone();
                let sensors_for_form = sensor_ids.clone();
                let close_arc = close_new_form_arc.clone();
                view! {
                    <div class="emitters-toolbar" style="display: flex; flex-direction: column; gap: 8px; margin: 0 0 12px 0;">
                        {move || if show_new_form.get() {
                            let create_title = ui_t("gldf.emitters.create_title", &state.with(|s| s.ui_language.clone()));
                            let submit_label = ui_t("gldf.emitters.create_submit", &state.with(|s| s.ui_language.clone()));
                            view! {
                                <div>
                                    <h4 style="margin: 0 0 8px 0; font-size: 13px;">{ create_title }</h4>
                                    <EmitterTypeForm
                                        show_name_field=true
                                        submit_label=submit_label
                                        available_photometries=photometry_ids_for_form.clone()
                                        available_fixed_ls=fixed_ls_for_form.clone()
                                        available_multi_channel_ls=mc_for_form.clone()
                                        available_sensors=sensors_for_form.clone()
                                        on_submit=new_form_submit.clone()
                                        on_cancel=close_arc.clone()
                                    />
                                </div>
                            }.into_any()
                        } else {
                            let new_label = ui_t("gldf.emitters.new_slot", &state.with(|s| s.ui_language.clone()));
                            view! {
                                <div>
                                    <button
                                        on:click=on_open_new_form
                                        data-testid="emitter-add"
                                        style="padding: 4px 10px; font-size: 12px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;"
                                    >{ new_label }</button>
                                </div>
                            }.into_any()
                        }}
                    </div>
                }.into_any()
            } else { ().into_any() };

            if emitters.is_empty() {
                let no_emitters = tr("gldf.emitters.no_emitters");
                let empty_msg = if editable {
                    tr("gldf.emitters.empty_editable")
                } else {
                    tr("gldf.emitters.empty_readonly")
                };
                return view! {
                    <>
                        { toolbar }
                        <div class="empty-state">
                            <div class="icon">{ "🔆" }</div>
                            <h3>{ no_emitters }</h3>
                            <p>{ empty_msg }</p>
                        </div>
                    </>
                }.into_any();
            }
            // Build a (photometry_id → file_name + bytes_len) map so each
            // emitter card can show what photometry would render in slice 3.
            let photo_info: std::collections::HashMap<String, (String, usize)> = {
                let mut map = std::collections::HashMap::new();
                if let Some(photometries) = s.product.general_definitions.photometries.as_ref() {
                    for p in &photometries.photometry {
                        if let Some(file_ref) = p.photometry_file_reference.as_ref() {
                            if let Some(file_def) = s.product.general_definitions.files.file.iter().find(|f| f.id == file_ref.file_id) {
                                let len = s.embedded_files.get(&file_ref.file_id).map(|b| b.len()).unwrap_or(0);
                                map.insert(p.id.clone(), (file_def.file_name.clone(), len));
                            }
                        }
                    }
                }
                map
            };
            // light_source_id → spectrum_id. The schema lets a
            // SpectrumReference sit on either Fixed or Changeable light
            // sources, and an emitter's `light_source_reference` is the
            // bridge — so this map walks the light sources once and looks
            // up per emitter row below.
            let ls_to_spectrum: std::collections::HashMap<String, String> = {
                let mut map = std::collections::HashMap::new();
                if let Some(ls) = s.product.general_definitions.light_sources.as_ref() {
                    for f in &ls.fixed_light_source {
                        if let Some(sr) = f.spectrum_reference.as_ref() {
                            map.insert(f.id.clone(), sr.spectrum_id.clone());
                        }
                    }
                    for c in &ls.changeable_light_source {
                        if let Some(sr) = c.spectrum_reference.as_ref() {
                            map.insert(c.id.clone(), sr.spectrum_id.clone());
                        }
                    }
                }
                map
            };
            view! {
                <div class="emitters-container">
                    <LinkHint
                        title="What is the Photometry · LightSource · Spectrum chain?".to_string()
                        body="A GLDF emitter binds a PhotometryReference (beam shape, from an .ldt/.ies file) to a LightSourceReference (the lamp). The spectrum is one step further: it sits on the light source, not on the emitter. So 'Spectrum (via lamp)' below is computed by following the light-source reference to its SpectrumReference. Edit the photometry on the Photometry page, attach a spectrum on the Light Sources page; this page is read-only resolution.".to_string()
                    />
                    { toolbar }
                    {emitters.into_iter().map(|e| view! {
                        <EmitterCard
                            emitter=e
                            info=photo_info.clone()
                            ls_to_spectrum=ls_to_spectrum.clone()
                            editable=editable
                            photometry_ids=photometry_ids.clone()
                            fixed_ls_ids=fixed_ls_ids.clone()
                            multi_channel_ls_ids=multi_channel_ls_ids.clone()
                            sensor_ids=sensor_ids.clone()
                        />
                    }).collect::<Vec<_>>()}
                </div>
            }.into_any()
        })}
    }
}

#[component]
fn EmitterCard(
    emitter: Emitter,
    info: std::collections::HashMap<String, (String, usize)>,
    ls_to_spectrum: std::collections::HashMap<String, String>,
    editable: bool,
    photometry_ids: Vec<String>,
    fixed_ls_ids: Vec<String>,
    multi_channel_ls_ids: Vec<String>,
    sensor_ids: Vec<String>,
) -> impl IntoView {
    let state = use_gldf();
    let lang = state.with(|s| s.ui_language.clone());
    let tr = move |key: &str| ui_t(key, &lang);
    let info_for_fixed = info.clone();
    let ls_to_spectrum_for_fixed = ls_to_spectrum.clone();
    let emitter_id_for_header = emitter.id.clone();
    let emitter_id_for_rename = emitter.id.clone();
    let emitter_id_for_modal = emitter.id.clone();
    let emitter_for_modal = emitter.clone();

    let is_editing = RwSignal::new(false);
    let on_open_edit = move |_| is_editing.set(true);
    let close_modal: Arc<dyn Fn() + Send + Sync> = Arc::new(move || is_editing.set(false));

    let on_rename: Arc<dyn Fn(String) + Send + Sync> = {
        let old_id = emitter_id_for_rename.clone();
        Arc::new(move |new_id| {
            state.update(|s| s.apply(GldfAction::RenameEmitter {
                old_id: old_id.clone(),
                new_id,
            }));
        })
    };

    view! {
        <div class="emitter-card" style="background: var(--bg-secondary); border-radius: 8px; padding: 16px; margin-bottom: 16px;">
            <div class="emitter-header" style="display: flex; align-items: center; gap: 8px; margin-bottom: 12px;">
                <span style="font-size: 20px;">{ "🔆" }</span>
                <IdRenameChip current_id=emitter_id_for_header editable=editable on_rename=on_rename />
                {if editable {
                    view! {
                        <button
                            on:click=on_open_edit
                            style="margin-left: auto; padding: 2px 8px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                        >{ "✎ Edit" }</button>
                    }.into_any()
                } else { ().into_any() }}
            </div>

            {if !emitter.fixed_light_emitter.is_empty() {
                view! {
                    <div class="fixed-emitters" style="margin-bottom: 16px;">
                        <h4 style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">{ tr("gldf.emitters.fixed_title") }</h4>
                        {emitter.fixed_light_emitter.iter().enumerate().map(|(idx, fle)| {
                            let photo_id = fle.photometry_reference.photometry_id.clone();
                            let ls_id_opt = fle.light_source_reference.fixed_light_source_id.clone();
                            let emergency = fle.emergency_behaviour.clone();
                            let info_lookup = info_for_fixed.get(&photo_id).cloned();
                            let spectrum_id = ls_id_opt
                                .as_ref()
                                .and_then(|id| ls_to_spectrum_for_fixed.get(id).cloned());
                            let cg_current = fle
                                .control_gear_reference
                                .as_ref()
                                .map(|cgr| cgr.control_gear_id.clone());
                            let emitter_id_for_cg = emitter.id.clone();
                            view! {
                                <div style="background: var(--bg-primary); padding: 12px; border-radius: 6px; margin-bottom: 8px;">
                                    <div style="display: flex; flex-wrap: wrap; gap: 16px; font-size: 12px;">
                                        {emergency.map(|eb| {
                                            let color = if eb == "EmergencyOnly" { "var(--accent-orange)" } else { "var(--accent-green)" };
                                            view! {
                                                <div>
                                                    <span style="color: var(--text-tertiary);">{ "Emergency: " }</span>
                                                    <span style=format!("background: {}; color: white; padding: 2px 6px; border-radius: 3px; font-size: 10px;", color)>{ eb }</span>
                                                </div>
                                            }
                                        })}
                                        <div>
                                            <span style="color: var(--text-tertiary);">{ "Photometry: " }</span>
                                            <span style="color: var(--accent-purple);">{ photo_id }</span>
                                        </div>
                                        {ls_id_opt.map(|id| view! {
                                            <div>
                                                <span style="color: var(--text-tertiary);">{ "Light Source: " }</span>
                                                <span style="color: var(--accent-blue);">{ id }</span>
                                            </div>
                                        })}
                                        <div>
                                            <span style="color: var(--text-tertiary);">{ "Spectrum (via lamp): " }</span>
                                            {match spectrum_id {
                                                Some(sid) => view! {
                                                    <span style="color: var(--accent-orange);">{ sid }</span>
                                                }.into_any(),
                                                None => view! {
                                                    <span style="color: var(--text-tertiary); font-style: italic;">{ "none — attach a spectrum to this light source on the Light Sources page" }</span>
                                                }.into_any(),
                                            }}
                                        </div>
                                    </div>
                                    <ControlGearPicker
                                        emitter_id=emitter_id_for_cg
                                        kind=LightEmitterKind::Fixed
                                        light_emitter_index=idx
                                        current=cg_current
                                        editable=editable
                                    />
                                    {info_lookup.map(|(file_name, len)| view! {
                                        <div style="margin-top: 8px; padding-top: 8px; border-top: 1px solid var(--border-color); font-size: 11px; color: var(--text-tertiary);">
                                            { format!("Photometry file: {} ({} bytes) — LdtViewer renders in slice 3", file_name, len) }
                                        </div>
                                    })}
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            } else { ().into_any() }}

            {if !emitter.changeable_light_emitter.is_empty() {
                view! {
                    <div class="changeable-emitters">
                        <h4 style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">{ tr("gldf.emitters.changeable_title") }</h4>
                        {emitter.changeable_light_emitter.iter().enumerate().map(|(_idx, cle)| {
                            let photo_id = cle.photometry_reference.photometry_id.clone();
                            let emergency = cle.emergency_behaviour.clone();
                            view! {
                                <div style="background: var(--bg-primary); padding: 12px; border-radius: 6px; margin-bottom: 8px;">
                                    <div style="display: flex; flex-wrap: wrap; gap: 16px; font-size: 12px;">
                                        {emergency.map(|eb| {
                                            let color = if eb == "EmergencyOnly" { "var(--accent-orange)" } else { "var(--accent-green)" };
                                            view! {
                                                <div>
                                                    <span style="color: var(--text-tertiary);">{ "Emergency: " }</span>
                                                    <span style=format!("background: {}; color: white; padding: 2px 6px; border-radius: 3px; font-size: 10px;", color)>{ eb }</span>
                                                </div>
                                            }
                                        })}
                                        <div>
                                            <span style="color: var(--text-tertiary);">{ "Photometry: " }</span>
                                            <span style="color: var(--accent-purple);">{ photo_id }</span>
                                        </div>
                                        <div>
                                            <span style="color: var(--text-tertiary);">{ "Light Source: " }</span>
                                            <span style="color: var(--text-tertiary); font-style: italic;">{ "chosen by user — one of the declared ChangeableLightSources (and its spectrum)" }</span>
                                        </div>
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            } else { ().into_any() }}

            {if !emitter.multi_channel_light_emitter.is_empty() {
                view! {
                    <div class="multi-channel-emitters" style="margin-top: 12px;">
                        <h4 style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">{ tr("gldf.emitters.multi_channel_title") }</h4>
                        {emitter.multi_channel_light_emitter.iter().enumerate().map(|(idx, mle)| {
                            let emergency = mle.emergency_behaviour.clone();
                            let ls_id = mle.light_source_reference.multi_channel_light_source_id.clone();
                            let cg_current = mle
                                .control_gear_reference
                                .as_ref()
                                .map(|cgr| cgr.control_gear_id.clone());
                            let emitter_id_for_cg = emitter.id.clone();
                            view! {
                                <div style="background: var(--bg-primary); padding: 12px; border-radius: 6px; margin-bottom: 8px;">
                                    <div style="display: flex; flex-wrap: wrap; gap: 16px; font-size: 12px;">
                                        {emergency.map(|eb| {
                                            let color = if eb == "EmergencyOnly" { "var(--accent-orange)" } else { "var(--accent-green)" };
                                            view! {
                                                <div>
                                                    <span style="color: var(--text-tertiary);">{ "Emergency: " }</span>
                                                    <span style=format!("background: {}; color: white; padding: 2px 6px; border-radius: 3px; font-size: 10px;", color)>{ eb }</span>
                                                </div>
                                            }
                                        })}
                                        <div>
                                            <span style="color: var(--text-tertiary);">{ "MultiChannel LightSource: " }</span>
                                            <span style="color: var(--accent-purple);">{ ls_id }</span>
                                        </div>
                                    </div>
                                    <ControlGearPicker
                                        emitter_id=emitter_id_for_cg
                                        kind=LightEmitterKind::MultiChannel
                                        light_emitter_index=idx
                                        current=cg_current
                                        editable=editable
                                    />
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            } else { ().into_any() }}

            {if !emitter.sensor_emitter.is_empty() {
                view! {
                    <div class="sensors" style="margin-top: 12px;">
                        <h4 style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">{ tr("gldf.emitters.sensor_title") }</h4>
                        <div style="display: flex; flex-wrap: wrap; gap: 8px;">
                            {emitter.sensor_emitter.iter().map(|s| view! {
                                <span style="background: var(--bg-primary); padding: 4px 8px; border-radius: 4px; font-size: 11px;">
                                    { s.sensor_reference.sensor_id.clone() }
                                </span>
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                }.into_any()
            } else { ().into_any() }}
        </div>
        {if editable {
            let modal_title = format!("Edit Emitter: {}", emitter_id_for_modal);
            view! {
                <EditModal
                    open=is_editing
                    title=modal_title
                    on_close=close_modal.clone()
                >
                    <EmitterEditForm
                        emitter=emitter_for_modal.clone()
                        photometry_ids=photometry_ids.clone()
                        fixed_ls_ids=fixed_ls_ids.clone()
                        multi_channel_ls_ids=multi_channel_ls_ids.clone()
                        sensor_ids=sensor_ids.clone()
                        on_done=close_modal.clone()
                    />
                </EditModal>
            }.into_any()
        } else { ().into_any() }}
    }
}

/// Modal-mounted Emitter editor. Hosts every editable affordance for an
/// Emitter: list each LightEmitter child with a Remove button (Fixed,
/// Changeable, MultiChannel, Sensor), the type-first `AddChildForm`, and
/// a "Remove this entire Emitter" button at the bottom. The card itself
/// stays read-only.
#[component]
fn EmitterEditForm(
    emitter: Emitter,
    photometry_ids: Vec<String>,
    fixed_ls_ids: Vec<String>,
    multi_channel_ls_ids: Vec<String>,
    sensor_ids: Vec<String>,
    on_done: Arc<dyn Fn() + Send + Sync>,
) -> impl IntoView {
    let state = use_gldf();
    let edit_lang = state.with(|s| s.ui_language.clone());
    let h_fixed = ui_t("gldf.emitters.fixed_title", &edit_lang);
    let h_change = ui_t("gldf.emitters.changeable_title", &edit_lang);
    let h_mc = ui_t("gldf.emitters.multi_channel_title", &edit_lang);
    let h_sensor = ui_t("gldf.emitters.sensor_title", &edit_lang);
    let emitter_id = emitter.id.clone();
    let emitter_id_for_remove = emitter.id.clone();
    let emitter_id_for_add = emitter.id.clone();
    let emitter_id_for_fixed = emitter.id.clone();
    let emitter_id_for_change = emitter.id.clone();
    let emitter_id_for_mc = emitter.id.clone();
    let emitter_id_for_sensor = emitter.id.clone();
    let on_done_for_remove = on_done.clone();

    // Live view of the children from state so per-child Remove buttons
    // immediately reflect in the form (the parent EmitterCard might still
    // hold the old snapshot until the next state.with() re-render).
    let fixed_children = Signal::derive(move || {
        let id = emitter_id_for_fixed.clone();
        state.with(|s| {
            s.product
                .general_definitions
                .emitters
                .as_ref()
                .and_then(|es| es.emitter.iter().find(|e| e.id == id))
                .map(|e| e.fixed_light_emitter.clone())
                .unwrap_or_default()
        })
    });
    let change_children = Signal::derive(move || {
        let id = emitter_id_for_change.clone();
        state.with(|s| {
            s.product
                .general_definitions
                .emitters
                .as_ref()
                .and_then(|es| es.emitter.iter().find(|e| e.id == id))
                .map(|e| e.changeable_light_emitter.clone())
                .unwrap_or_default()
        })
    });
    let mc_children = Signal::derive(move || {
        let id = emitter_id_for_mc.clone();
        state.with(|s| {
            s.product
                .general_definitions
                .emitters
                .as_ref()
                .and_then(|es| es.emitter.iter().find(|e| e.id == id))
                .map(|e| e.multi_channel_light_emitter.clone())
                .unwrap_or_default()
        })
    });
    let sensor_children = Signal::derive(move || {
        let id = emitter_id_for_sensor.clone();
        state.with(|s| {
            s.product
                .general_definitions
                .emitters
                .as_ref()
                .and_then(|es| es.emitter.iter().find(|e| e.id == id))
                .map(|e| e.sensor_emitter.clone())
                .unwrap_or_default()
        })
    });

    // One `Remove(idx)` dispatcher per LightEmitter kind. Arc so each
    // reactive section can `.clone()` it into its `move ||` closure
    // cheaply. Each dispatcher is `Fn(usize)` and ignores the click
    // event arg, so the call site wraps in a tiny `move |_| ...`.
    let make_remove: Arc<dyn Fn(LightEmitterKind, usize) + Send + Sync> = {
        let emitter_id = emitter_id.clone();
        Arc::new(move |kind: LightEmitterKind, index: usize| {
            state.update(|s| s.apply(GldfAction::RemoveLightEmitter {
                emitter_id: emitter_id.clone(),
                kind,
                index,
            }));
        })
    };
    let remove_fixed = make_remove.clone();
    let remove_change = make_remove.clone();
    let remove_mc = make_remove.clone();
    let remove_sensor = make_remove;
    let on_remove_emitter = move |_| {
        let id = emitter_id_for_remove.clone();
        let msg = format!("Remove emitter '{}'? Variants that mount it will dangle.", id);
        if web_sys::window()
            .and_then(|w| w.confirm_with_message(&msg).ok())
            .unwrap_or(false)
        {
            state.update(|s| s.apply(GldfAction::RemoveEmitter { id }));
            on_done_for_remove();
        }
    };
    let on_close = {
        let done = on_done.clone();
        move |_| done()
    };

    view! {
        <div class="emitter-edit-form" style="display: flex; flex-direction: column; gap: 12px;">
            // Fixed children
            {let h_fixed = h_fixed.clone(); move || {
                let kids = fixed_children.get();
                if kids.is_empty() {
                    return view! { <></> }.into_any();
                }
                let remove = remove_fixed.clone();
                let h_fixed = h_fixed.clone();
                view! {
                    <div>
                        <h4 style="margin: 0 0 6px; font-size: 12px; color: var(--text-secondary);">{ h_fixed }</h4>
                        {kids.into_iter().enumerate().map(|(idx, fle)| {
                            let photo = fle.photometry_reference.photometry_id.clone();
                            let ls = fle.light_source_reference.fixed_light_source_id.clone();
                            let remove = remove.clone();
                            let on_remove = move |_| remove(LightEmitterKind::Fixed, idx);
                            view! {
                                <div style="display: flex; gap: 8px; align-items: center; padding: 6px 8px; background: var(--bg-secondary); border-radius: 4px; margin-bottom: 4px; font-size: 11px;">
                                    <span style="color: var(--text-tertiary);">{ format!("[{}]", idx) }</span>
                                    <span><span style="color: var(--text-tertiary);">{ "Photometry: " }</span><span style="color: var(--accent-purple);">{ photo }</span></span>
                                    {ls.map(|id| view! {
                                        <span><span style="color: var(--text-tertiary);">{ "LS: " }</span><span style="color: var(--accent-blue);">{ id }</span></span>
                                    })}
                                    <button
                                        on:click=on_remove
                                        style="margin-left: auto; padding: 2px 6px; font-size: 11px; background: none; border: 1px solid var(--accent-orange); border-radius: 4px; cursor: pointer; color: var(--accent-orange);"
                                    >{ "✕ Remove" }</button>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}

            // Changeable children
            {let h_change = h_change.clone(); move || {
                let kids = change_children.get();
                if kids.is_empty() {
                    return view! { <></> }.into_any();
                }
                let remove = remove_change.clone();
                let h_change = h_change.clone();
                view! {
                    <div>
                        <h4 style="margin: 0 0 6px; font-size: 12px; color: var(--text-secondary);">{ h_change }</h4>
                        {kids.into_iter().enumerate().map(|(idx, cle)| {
                            let photo = cle.photometry_reference.photometry_id.clone();
                            let remove = remove.clone();
                            let on_remove = move |_| remove(LightEmitterKind::Changeable, idx);
                            view! {
                                <div style="display: flex; gap: 8px; align-items: center; padding: 6px 8px; background: var(--bg-secondary); border-radius: 4px; margin-bottom: 4px; font-size: 11px;">
                                    <span style="color: var(--text-tertiary);">{ format!("[{}]", idx) }</span>
                                    <span><span style="color: var(--text-tertiary);">{ "Photometry: " }</span><span style="color: var(--accent-purple);">{ photo }</span></span>
                                    <button
                                        on:click=on_remove
                                        style="margin-left: auto; padding: 2px 6px; font-size: 11px; background: none; border: 1px solid var(--accent-orange); border-radius: 4px; cursor: pointer; color: var(--accent-orange);"
                                    >{ "✕ Remove" }</button>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}

            // MultiChannel children
            {let h_mc = h_mc.clone(); move || {
                let kids = mc_children.get();
                if kids.is_empty() {
                    return view! { <></> }.into_any();
                }
                let remove = remove_mc.clone();
                let h_mc = h_mc.clone();
                view! {
                    <div>
                        <h4 style="margin: 0 0 6px; font-size: 12px; color: var(--text-secondary);">{ h_mc }</h4>
                        {kids.into_iter().enumerate().map(|(idx, mle)| {
                            let ls = mle.light_source_reference.multi_channel_light_source_id.clone();
                            let remove = remove.clone();
                            let on_remove = move |_| remove(LightEmitterKind::MultiChannel, idx);
                            view! {
                                <div style="display: flex; gap: 8px; align-items: center; padding: 6px 8px; background: var(--bg-secondary); border-radius: 4px; margin-bottom: 4px; font-size: 11px;">
                                    <span style="color: var(--text-tertiary);">{ format!("[{}]", idx) }</span>
                                    <span><span style="color: var(--text-tertiary);">{ "MC LightSource: " }</span><span style="color: var(--accent-purple);">{ ls }</span></span>
                                    <button
                                        on:click=on_remove
                                        style="margin-left: auto; padding: 2px 6px; font-size: 11px; background: none; border: 1px solid var(--accent-orange); border-radius: 4px; cursor: pointer; color: var(--accent-orange);"
                                    >{ "✕ Remove" }</button>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}

            // Sensor children
            {let h_sensor = h_sensor.clone(); move || {
                let kids = sensor_children.get();
                if kids.is_empty() {
                    return view! { <></> }.into_any();
                }
                let remove = remove_sensor.clone();
                let h_sensor = h_sensor.clone();
                view! {
                    <div>
                        <h4 style="margin: 0 0 6px; font-size: 12px; color: var(--text-secondary);">{ h_sensor }</h4>
                        {kids.into_iter().enumerate().map(|(idx, se)| {
                            let sid = se.sensor_reference.sensor_id.clone();
                            let remove = remove.clone();
                            let on_remove = move |_| remove(LightEmitterKind::Sensor, idx);
                            view! {
                                <div style="display: flex; gap: 8px; align-items: center; padding: 6px 8px; background: var(--bg-secondary); border-radius: 4px; margin-bottom: 4px; font-size: 11px;">
                                    <span style="color: var(--text-tertiary);">{ format!("[{}]", idx) }</span>
                                    <span><span style="color: var(--text-tertiary);">{ "Sensor: " }</span><span style="color: var(--accent-blue);">{ sid }</span></span>
                                    <button
                                        on:click=on_remove
                                        style="margin-left: auto; padding: 2px 6px; font-size: 11px; background: none; border: 1px solid var(--accent-orange); border-radius: 4px; cursor: pointer; color: var(--accent-orange);"
                                    >{ "✕ Remove" }</button>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}

            <div style="padding: 8px; background: var(--bg-secondary); border-radius: 4px;">
                <h4 style="margin: 0 0 6px; font-size: 12px; color: var(--text-secondary);">{ "Add LightEmitter child" }</h4>
                <AddChildForm
                    emitter_id=emitter_id_for_add
                    photometry_ids=photometry_ids
                    fixed_ls_ids=fixed_ls_ids
                    multi_channel_ls_ids=multi_channel_ls_ids
                    sensor_ids=sensor_ids
                />
            </div>

            <div style="display: flex; gap: 6px; align-items: center; padding-top: 8px; border-top: 1px solid var(--border-color);">
                <button
                    on:click=on_close
                    style="padding: 4px 10px; font-size: 12px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;"
                >{ "Done" }</button>
                <button
                    on:click=on_remove_emitter
                    style="margin-left: auto; padding: 4px 10px; font-size: 12px; background: none; border: 1px solid var(--accent-orange); border-radius: 4px; cursor: pointer; color: var(--accent-orange);"
                >{ "Remove this Emitter" }</button>
            </div>
        </div>
    }
}

/// Per-card "Add LightEmitter child" form. Renders an open-on-click toggle
/// that flips into the shared `EmitterTypeForm` (type-first picker).
#[component]
fn AddChildForm(
    emitter_id: String,
    photometry_ids: Vec<String>,
    fixed_ls_ids: Vec<String>,
    multi_channel_ls_ids: Vec<String>,
    sensor_ids: Vec<String>,
) -> impl IntoView {
    let state = use_gldf();
    let is_open = RwSignal::new(false);
    let emitter_id_for_submit = emitter_id.clone();

    let on_open = move |_| is_open.set(true);
    let on_cancel: Arc<dyn Fn() + Send + Sync> = Arc::new(move || is_open.set(false));

    let on_submit: Arc<dyn Fn(EmitterDraft) + Send + Sync> = {
        let emitter_id = emitter_id_for_submit.clone();
        Arc::new(move |draft: EmitterDraft| {
            let emitter_id = emitter_id.clone();
            match draft.kind {
                LightEmitterKind::Fixed => state.update(|s| s.apply(GldfAction::AddFixedLightEmitter {
                    emitter_id,
                    photometry_id: draft.photometry_id,
                    light_source_id: draft.fixed_light_source_id,
                })),
                LightEmitterKind::Changeable => state.update(|s| s.apply(GldfAction::AddChangeableLightEmitter {
                    emitter_id,
                    photometry_id: draft.photometry_id,
                })),
                LightEmitterKind::MultiChannel => state.update(|s| s.apply(GldfAction::AddMultiChannelLightEmitter {
                    emitter_id,
                    multi_channel_light_source_id: draft.multi_channel_light_source_id,
                })),
                LightEmitterKind::Sensor => state.update(|s| s.apply(GldfAction::AddSensorEmitter {
                    emitter_id,
                    sensor_id: draft.sensor_id,
                })),
            }
            is_open.set(false);
        })
    };

    view! {
        <div style="margin-top: 12px;">
            {move || if is_open.get() {
                let photos = photometry_ids.clone();
                let fixed = fixed_ls_ids.clone();
                let mc = multi_channel_ls_ids.clone();
                let sensors = sensor_ids.clone();
                let submit = on_submit.clone();
                let cancel = on_cancel.clone();
                view! {
                    <EmitterTypeForm
                        show_name_field=false
                        submit_label="+ Add child".to_string()
                        available_photometries=photos
                        available_fixed_ls=fixed
                        available_multi_channel_ls=mc
                        available_sensors=sensors
                        on_submit=submit
                        on_cancel=cancel
                    />
                }.into_any()
            } else {
                view! {
                    <button
                        on:click=on_open
                        style="padding: 4px 10px; font-size: 12px; background: none; border: 1px dashed var(--accent-blue); border-radius: 4px; cursor: pointer; color: var(--accent-blue);"
                    >{ "+ Add LightEmitter child" }</button>
                }.into_any()
            }}
        </div>
    }
}

// (The old hand-rolled AddLightEmitterForm was replaced by the shared
// `EmitterTypeForm` + `AddChildForm` wrapper above. The reusable form
// matches the XSD's xs:choice between Fixed / Changeable / MultiChannel /
// Sensor instead of mashing two paths into one row.)

/// Inline ControlGear picker for a single Fixed- or MultiChannel-
/// LightEmitter slot inside an Emitter. The picker reads declared
/// `<ControlGear>` ids live and dispatches `SetEmitterControlGearReference`
/// to bind/clear. When `editable=false` it renders a compact read-only
/// row (the bound driver id or a "no driver" placeholder).
#[component]
fn ControlGearPicker(
    emitter_id: String,
    kind: LightEmitterKind,
    light_emitter_index: usize,
    current: Option<String>,
    editable: bool,
) -> impl IntoView {
    let state = use_gldf();
    let cg_ids = Signal::derive(move || {
        state.with(|s| {
            s.product
                .general_definitions
                .control_gears
                .as_ref()
                .map(|c| c.control_gear.iter().map(|g| g.id.clone()).collect::<Vec<_>>())
                .unwrap_or_default()
        })
    });

    if !editable {
        let label = current.clone().unwrap_or_else(|| "—".to_string());
        return view! {
            <div style="display: flex; gap: 8px; align-items: center; margin-top: 6px; font-size: 11px; color: var(--text-secondary);">
                <span style="color: var(--text-tertiary);">{ "Control Gear: " }</span>
                <span style="color: var(--accent-yellow);">{ label }</span>
            </div>
        }.into_any();
    }

    let emitter_id_for_pick = emitter_id.clone();
    let on_pick = move |e: crate::components::edit_util::EditEvent| {
        use wasm_bindgen::JsCast;
        let Some(target) = e.target() else { return };
        let Ok(sel) = target.dyn_into::<web_sys::HtmlSelectElement>() else { return };
        let val = sel.value();
        let control_gear_id = if val.is_empty() { None } else { Some(val) };
        let eid = emitter_id_for_pick.clone();
        state.update(|s| {
            s.apply(GldfAction::SetEmitterControlGearReference {
                emitter_id: eid,
                kind,
                light_emitter_index,
                control_gear_id,
            });
        });
    };

    let current_for_show = current.clone();
    view! {
        <div style="display: flex; gap: 8px; align-items: center; margin-top: 6px; font-size: 11px;">
            <span style="color: var(--text-tertiary);">{ "Control Gear: " }</span>
            <select on:change=on_pick style="font-size: 11px; padding: 1px 4px;">
                <option value="" selected=current_for_show.is_none()>{ "— none —" }</option>
                {move || cg_ids.get().into_iter().map(|cid| {
                    let label = cid.clone();
                    let selected = current.as_deref() == Some(cid.as_str());
                    view! { <option value=cid selected=selected>{ label }</option> }
                }).collect::<Vec<_>>()}
            </select>
            {move || {
                if cg_ids.with(|v| v.is_empty()) {
                    view! {
                        <span style="font-size: 10px; color: var(--text-tertiary);">
                            { "No Control Gear declared — add one on " }
                            <strong style="color: var(--accent-blue);">{ "⚡ Control Gear" }</strong>
                            { "." }
                        </span>
                    }.into_any()
                } else { view! { <span /> }.into_any() }
            }}
        </div>
    }.into_any()
}
