//! Equipment editor — `GeneralDefinitions/Equipments/Equipment`.
//!
//! Each Equipment is a LightSource × ControlGear × rated power
//! assembly, optionally with emergency-ballast parameters. It sits
//! parallel to the Emitter graph and is referenced by integrators via
//! `EquipmentReference` (where applicable).

use crate::components::edit_util::event_value;
use crate::components::EditModal;
use crate::i18n::ui::t as ui_t;
use crate::state::{use_gldf, GldfAction, LightEmitterKind};
use gldf_rs::gldf::general_definitions::electrical::Equipment;
use leptos::prelude::*;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use web_sys::HtmlSelectElement;

#[component]
pub fn EquipmentPage(editable: bool) -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));

    let on_add = move |_| {
        state.update(|s| {
            s.apply(GldfAction::AddEquipment { prefix: None });
        });
    };

    view! {
        <div class="editor-section equipment-page">
            <h2>{ move || tr("gldf.equipment.title") }</h2>
            <p class="section-description">
                { move || tr("gldf.equipment.description") }
            </p>
            {if editable {
                view! {
                    <div style="margin-bottom: 12px;">
                        <button type="button" on:click=on_add>{ move || tr("gldf.equipment.add") }</button>
                    </div>
                }.into_any()
            } else { ().into_any() }}

            {move || state.with(|s| {
                let equipments: Vec<Equipment> = s.product
                    .general_definitions
                    .equipments
                    .as_ref()
                    .map(|c| c.equipment.clone())
                    .unwrap_or_default();
                if equipments.is_empty() {
                    let empty = tr("gldf.equipment.empty");
                    view! { <p class="empty-message">{ empty }</p> }.into_any()
                } else {
                    view! {
                        <div style="display: flex; flex-direction: column; gap: 10px;">
                            {equipments.into_iter().map(|e| view! {
                                <EquipmentCard equipment=e editable=editable />
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            })}
        </div>
    }
}

#[component]
fn EquipmentCard(equipment: Equipment, editable: bool) -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));
    let is_editing = RwSignal::new(false);
    let id_for_remove = equipment.id.clone();
    let id_for_modal = equipment.id.clone();

    let on_edit = move |_| is_editing.set(true);
    let on_remove = move |_| {
        let id = id_for_remove.clone();
        let msg = format!("Remove equipment '{}'?", id);
        if web_sys::window()
            .and_then(|w| w.confirm_with_message(&msg).ok())
            .unwrap_or(false)
        {
            state.update(|s| s.apply(GldfAction::RemoveEquipment { id }));
        }
    };

    let ls_label = equipment
        .light_source_reference
        .as_ref()
        .map(|lsr| {
            if lsr.changeable_light_source_id.is_empty() {
                "(unbound)".to_string()
            } else {
                format!("LightSource: {}", lsr.changeable_light_source_id)
            }
        })
        .unwrap_or_else(|| "(no LightSource)".to_string());
    let cg_label = equipment
        .control_gear_reference
        .as_ref()
        .map(|cgr| cgr.control_gear_id.clone())
        .unwrap_or_else(|| "(no ControlGear)".to_string());

    view! {
        <div class="card" style="border: 1px solid var(--border-color); border-radius: 6px; padding: 10px;">
            <div style="display: flex; gap: 8px; align-items: center;">
                <span style="font-family: var(--font-mono); font-size: 11px; color: var(--text-secondary);">{ equipment.id.clone() }</span>
                <span style="flex: 1; font-size: 12px;">{ format!("💡 {} · ⚡ {}", ls_label, cg_label) }</span>
                <span style="font-size: 11px; color: var(--text-secondary);">{ format!("{} W", equipment.rated_input_power) }</span>
                {if editable {
                    let id_for_edit = equipment.id.clone();
                    let edit_label = tr("gldf.common.edit");
                    let remove_label = tr("gldf.common.remove");
                    view! {
                        <span style="display: inline-flex; gap: 6px;">
                            <button
                                on:click=on_edit
                                data-testid=format!("eq-edit-{}", id_for_edit)
                                style="padding: 2px 8px; font-size: 11px;"
                            >{ edit_label }</button>
                            <button on:click=on_remove style="padding: 2px 8px; font-size: 11px; border: 1px solid var(--accent-orange); color: var(--accent-orange); background: none;">{ remove_label }</button>
                        </span>
                    }.into_any()
                } else { ().into_any() }}
            </div>
            {if editable {
                let eq_for_form = equipment.clone();
                let on_close: Arc<dyn Fn() + Send + Sync> = Arc::new(move || is_editing.set(false));
                view! {
                    <EditModal
                        open=is_editing
                        title=format!("Edit Equipment: {}", id_for_modal)
                        on_close=on_close.clone()
                    >
                        <EquipmentEditForm
                            equipment=eq_for_form.clone()
                            on_done=move || is_editing.set(false)
                        />
                    </EditModal>
                }.into_any()
            } else { ().into_any() }}
        </div>
    }
}

#[component]
fn EquipmentEditForm(equipment: Equipment, on_done: impl Fn() + Send + Sync + 'static + Clone) -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));
    let id = equipment.id.clone();

    // Equipment LightSourceReference only carries one attribute
    // (@changeableLightSourceId) per the rc-3 XSD; the kind selector
    // stays for action-signature compatibility but no longer affects
    // serialization.
    let (init_ls_id, init_kind) = match equipment.light_source_reference.as_ref() {
        Some(lsr) if !lsr.changeable_light_source_id.is_empty() => (
            lsr.changeable_light_source_id.clone(),
            "changeable".to_string(),
        ),
        _ => (String::new(), "changeable".to_string()),
    };
    let init_cg = equipment
        .control_gear_reference
        .as_ref()
        .map(|cgr| cgr.control_gear_id.clone())
        .unwrap_or_default();

    let s_kind = RwSignal::new(init_kind);
    let s_ls = RwSignal::new(init_ls_id);
    let s_cg = RwSignal::new(init_cg);
    let s_power = RwSignal::new(equipment.rated_input_power.to_string());
    let s_eblf = RwSignal::new(
        equipment.emergency_ballast_lumen_factor
            .map(|v| v.to_string())
            .unwrap_or_default(),
    );
    let s_erlf = RwSignal::new(
        equipment.emergency_rated_luminous_flux
            .map(|v| v.to_string())
            .unwrap_or_default(),
    );

    let fixed_ids = Signal::derive(move || {
        state.with(|s| {
            s.product
                .general_definitions
                .light_sources
                .as_ref()
                .map(|ls| ls.fixed_light_source.iter().map(|f| f.id.clone()).collect::<Vec<_>>())
                .unwrap_or_default()
        })
    });
    let changeable_ids = Signal::derive(move || {
        state.with(|s| {
            s.product
                .general_definitions
                .light_sources
                .as_ref()
                .map(|ls| ls.changeable_light_source.iter().map(|c| c.id.clone()).collect::<Vec<_>>())
                .unwrap_or_default()
        })
    });
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

    let on_kind = move |e: crate::components::edit_util::EditEvent| {
        if let Some(target) = e.target() {
            if let Ok(sel) = target.dyn_into::<HtmlSelectElement>() {
                s_kind.set(sel.value());
                s_ls.set(String::new());
            }
        }
    };
    let on_ls = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = event_value(&e) {
            s_ls.set(v);
        }
    };
    let on_cg = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = event_value(&e) {
            s_cg.set(v);
        }
    };

    let on_cancel = {
        let done = on_done.clone();
        move |_| done()
    };
    let on_save = move |_| {
        let kind_str = s_kind.get();
        let kind = match kind_str.as_str() {
            "changeable" => LightEmitterKind::Changeable,
            _ => LightEmitterKind::Fixed,
        };
        let light_source_id = {
            let id = s_ls.get();
            if id.is_empty() { None } else { Some(id) }
        };
        let control_gear_id = {
            let id = s_cg.get();
            if id.is_empty() { None } else { Some(id) }
        };
        let rated_input_power = s_power.with(|t| t.trim().parse::<f64>().unwrap_or(0.0));
        let emergency_ballast_lumen_factor = s_eblf.with(|t| t.trim().parse::<f64>().ok());
        let emergency_rated_luminous_flux = s_erlf.with(|t| t.trim().parse::<i32>().ok());

        state.update(|s| {
            s.apply(GldfAction::UpdateEquipment {
                id: id.clone(),
                light_source_id,
                light_source_kind: light_source_id_kind(&kind),
                control_gear_id,
                rated_input_power,
                emergency_ballast_lumen_factor,
                emergency_rated_luminous_flux,
            });
        });
        on_done();
    };

    // Same `Box::leak` pattern as ControlGear's edit form — TextField
    // takes `&'static str`. Leaking a few short strings per modal open
    // is bounded. (Larger refactor punted: switch TextField to
    // `Into<String>`.)
    let l_power = Box::leak("Rated input power (W)".to_string().into_boxed_str());
    let l_eblf = Box::leak("Emergency ballast lumen factor".to_string().into_boxed_str());
    let l_erlf = Box::leak("Emergency rated luminous flux (lm)".to_string().into_boxed_str());
    // The above three keys aren't translated yet (they didn't make this
    // cut's key budget — emergency-ballast parameters are a smaller XSD
    // niche). The save/cancel buttons + the three select labels above
    // are the high-value DE/EN wins.

    view! {
        <div style="display: flex; flex-direction: column; gap: 8px;">
            <label style="display: flex; gap: 8px; align-items: center; font-size: 12px;">
                <span style="flex: 0 0 180px; color: var(--text-secondary);">{ tr("gldf.equipment.edit.ls_kind") }</span>
                <select on:change=on_kind prop:value=s_kind.get() data-testid="eq-edit-kind" style="flex: 1;">
                    <option value="fixed" selected=s_kind.with(|v| v == "fixed")>{ tr("gldf.equipment.edit.ls_kind.fixed") }</option>
                    <option value="changeable" selected=s_kind.with(|v| v == "changeable")>{ tr("gldf.equipment.edit.ls_kind.changeable") }</option>
                </select>
            </label>
            <label style="display: flex; gap: 8px; align-items: center; font-size: 12px;">
                <span style="flex: 0 0 180px; color: var(--text-secondary);">{ tr("gldf.equipment.edit.ls_id") }</span>
                <select on:change=on_ls prop:value=s_ls.get() data-testid="eq-edit-lightsource" style="flex: 1;">
                    <option value="" selected=s_ls.with(|v| v.is_empty())>{ tr("gldf.common.none") }</option>
                    {move || {
                        let kind = s_kind.get();
                        let ids = if kind == "changeable" {
                            changeable_ids.get()
                        } else {
                            fixed_ids.get()
                        };
                        ids.into_iter().map(|id| {
                            let label = id.clone();
                            let selected = s_ls.with(|v| v == &id);
                            view! { <option value=id selected=selected>{ label }</option> }
                        }).collect::<Vec<_>>()
                    }}
                </select>
            </label>
            <label style="display: flex; gap: 8px; align-items: center; font-size: 12px;">
                <span style="flex: 0 0 180px; color: var(--text-secondary);">{ tr("gldf.equipment.edit.cg_id") }</span>
                <select on:change=on_cg prop:value=s_cg.get() data-testid="eq-edit-controlgear" style="flex: 1;">
                    <option value="" selected=s_cg.with(|v| v.is_empty())>{ tr("gldf.common.none") }</option>
                    {move || cg_ids.get().into_iter().map(|id| {
                        let label = id.clone();
                        let selected = s_cg.with(|v| v == &id);
                        view! { <option value=id selected=selected>{ label }</option> }
                    }).collect::<Vec<_>>()}
                </select>
            </label>
            <TextField label=l_power value=s_power />
            <TextField label=l_eblf value=s_eblf />
            <TextField label=l_erlf value=s_erlf />
            <div style="display: flex; gap: 6px; margin-top: 6px;">
                <button on:click=on_save style="padding: 4px 10px; background: var(--accent-blue); color: white; border: none; border-radius: 4px;">{ tr("gldf.common.save") }</button>
                <button on:click=on_cancel style="padding: 4px 10px; background: none; border: 1px solid var(--border-color); border-radius: 4px;">{ tr("gldf.common.cancel") }</button>
            </div>
        </div>
    }
}

fn light_source_id_kind(kind: &LightEmitterKind) -> Option<LightEmitterKind> {
    Some(*kind)
}

#[component]
fn TextField(label: &'static str, value: RwSignal<String>) -> impl IntoView {
    let on_input = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = event_value(&e) {
            value.set(v);
        }
    };
    view! {
        <label style="display: flex; gap: 8px; align-items: center; font-size: 12px;">
            <span style="flex: 0 0 180px; color: var(--text-secondary);">{ label }</span>
            <input type="text" prop:value=value.get() on:input=on_input style="flex: 1;" />
        </label>
    }
}
