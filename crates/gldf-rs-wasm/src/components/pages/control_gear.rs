//! Control Gear editor — `GeneralDefinitions/ControlGears/ControlGear`.
//!
//! Each entry is a driver (LED driver / ballast) with electrical and
//! capability metadata: nominal voltage, standby power, dimmable, color-
//! controllable, CLO range. Used by `Equipment` to assemble the
//! LightSource × ControlGear bundle, and referenced from individual
//! Emitters via `ControlGearReference`.

use crate::components::edit_util::event_value;
use crate::components::EditModal;
use crate::i18n::ui::t as ui_t;
use crate::state::{use_gldf, GldfAction};
use gldf_rs::gldf::general_definitions::electrical::ControlGear;
use leptos::prelude::*;
use std::sync::Arc;

#[component]
pub fn ControlGearPage(editable: bool) -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));

    let on_add = move |_| {
        state.update(|s| {
            s.apply(GldfAction::AddControlGear { prefix: None });
        });
    };

    view! {
        <div class="editor-section control-gear-page">
            <h2>{ move || tr("gldf.controlgear.title") }</h2>
            <p class="section-description">
                { move || tr("gldf.controlgear.description") }
            </p>
            {if editable {
                view! {
                    <div style="margin-bottom: 12px;">
                        <button type="button" on:click=on_add>{ move || tr("gldf.controlgear.add") }</button>
                    </div>
                }.into_any()
            } else { ().into_any() }}

            {move || state.with(|s| {
                let gears: Vec<ControlGear> = s.product
                    .general_definitions
                    .control_gears
                    .as_ref()
                    .map(|c| c.control_gear.clone())
                    .unwrap_or_default();
                if gears.is_empty() {
                    let empty = tr("gldf.controlgear.empty");
                    view! { <p class="empty-message">{ empty }</p> }.into_any()
                } else {
                    view! {
                        <div style="display: flex; flex-direction: column; gap: 10px;">
                            {gears.into_iter().map(|g| view! {
                                <ControlGearCard gear=g editable=editable />
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            })}
        </div>
    }
}

#[component]
fn ControlGearCard(gear: ControlGear, editable: bool) -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));
    let is_editing = RwSignal::new(false);
    let id_for_remove = gear.id.clone();
    let id_for_modal = gear.id.clone();

    let on_edit = move |_| is_editing.set(true);
    let on_remove = move |_| {
        let id = id_for_remove.clone();
        let msg = format!("Remove control gear '{}'? Equipment and per-emitter references to it will dangle.", id);
        if web_sys::window()
            .and_then(|w| w.confirm_with_message(&msg).ok())
            .unwrap_or(false)
        {
            state.update(|s| s.apply(GldfAction::RemoveControlGear { id }));
        }
    };

    let name = gear.name.locale.first().map(|l| l.value.clone()).unwrap_or_default();
    let display_name = if name.is_empty() { gear.id.clone() } else { name };

    view! {
        <div class="card" style="border: 1px solid var(--border-color); border-radius: 6px; padding: 10px;">
            <div style="display: flex; gap: 8px; align-items: center;">
                <span style="font-family: var(--font-mono); font-size: 11px; color: var(--text-secondary);">{ gear.id.clone() }</span>
                <span style="flex: 1; font-weight: 600;">{ display_name }</span>
                {if let Some(true) = gear.dimmable {
                    let label = tr("gldf.controlgear.tag.dimmable");
                    view! { <span style="font-size: 10px; padding: 1px 6px; border: 1px solid var(--accent-yellow); border-radius: 8px; color: var(--accent-yellow);">{ label }</span> }.into_any()
                } else { ().into_any() }}
                {if let Some(true) = gear.color_controllable {
                    let label = tr("gldf.controlgear.tag.color");
                    view! { <span style="font-size: 10px; padding: 1px 6px; border: 1px solid var(--accent-purple); border-radius: 8px; color: var(--accent-purple);">{ label }</span> }.into_any()
                } else { ().into_any() }}
                {if editable {
                    let id_for_edit_testid = gear.id.clone();
                    let edit_label = tr("gldf.common.edit");
                    let remove_label = tr("gldf.common.remove");
                    view! {
                        <span style="display: inline-flex; gap: 6px;">
                            <button
                                on:click=on_edit
                                data-testid=format!("cg-edit-{}", id_for_edit_testid)
                                style="padding: 2px 8px; font-size: 11px;"
                            >{ edit_label }</button>
                            <button on:click=on_remove style="padding: 2px 8px; font-size: 11px; border: 1px solid var(--accent-orange); color: var(--accent-orange); background: none;">{ remove_label }</button>
                        </span>
                    }.into_any()
                } else { ().into_any() }}
            </div>
            <div style="display: flex; flex-wrap: wrap; gap: 12px; margin-top: 6px; font-size: 11px; color: var(--text-secondary);">
                {gear.nominal_voltage.as_ref().and_then(|v| v.fixed_voltage).map(|v| view! {
                    <span>{ format!("Voltage: {} V", v) }</span>
                })}
                {gear.standby_power.map(|p| view! {
                    <span>{ format!("Standby: {} W", p) }</span>
                })}
                {gear.constant_light_output_start_power.zip(gear.constant_light_output_end_power).map(|(s, e)| view! {
                    <span>{ format!("CLO: {}–{} W", s, e) }</span>
                })}
            </div>
            {if editable {
                let gear_for_form = gear.clone();
                let on_close: Arc<dyn Fn() + Send + Sync> = Arc::new(move || is_editing.set(false));
                view! {
                    <EditModal
                        open=is_editing
                        title=format!("Edit Control Gear: {}", id_for_modal)
                        on_close=on_close.clone()
                    >
                        <ControlGearEditForm
                            gear=gear_for_form.clone()
                            on_done=move || is_editing.set(false)
                        />
                    </EditModal>
                }.into_any()
            } else { ().into_any() }}
        </div>
    }
}

#[component]
fn ControlGearEditForm(gear: ControlGear, on_done: impl Fn() + Send + Sync + 'static + Clone) -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));
    let id = gear.id.clone();

    let name_first = gear.name.locale.first().map(|l| l.value.clone()).unwrap_or_default();
    let voltage = gear.nominal_voltage.as_ref().and_then(|v| v.fixed_voltage).map(|v| v.to_string()).unwrap_or_default();
    let standby = gear.standby_power.map(|p| p.to_string()).unwrap_or_default();
    let clo_start = gear.constant_light_output_start_power.map(|p| p.to_string()).unwrap_or_default();
    let clo_end = gear.constant_light_output_end_power.map(|p| p.to_string()).unwrap_or_default();
    let dimmable = gear.dimmable.unwrap_or(false);
    let color = gear.color_controllable.unwrap_or(false);

    let s_name = RwSignal::new(name_first);
    let s_volt = RwSignal::new(voltage);
    let s_standby = RwSignal::new(standby);
    let s_clo_start = RwSignal::new(clo_start);
    let s_clo_end = RwSignal::new(clo_end);
    let s_dim = RwSignal::new(dimmable);
    let s_color = RwSignal::new(color);

    let on_cancel = {
        let done = on_done.clone();
        move |_| done()
    };
    let on_save = move |_| {
        state.update(|s| {
            s.apply(GldfAction::UpdateControlGear {
                id: id.clone(),
                name: s_name.get(),
                nominal_voltage_value: s_volt.with(|t| t.trim().parse::<f64>().ok()),
                standby_power: s_standby.with(|t| t.trim().parse::<f64>().ok()),
                dimmable: Some(s_dim.get()),
                color_controllable: Some(s_color.get()),
                constant_light_output_start_power: s_clo_start
                    .with(|t| t.trim().parse::<f64>().ok()),
                constant_light_output_end_power: s_clo_end
                    .with(|t| t.trim().parse::<f64>().ok()),
            });
        });
        on_done();
    };

    // TextField + CheckboxField take `&'static str` labels. Translation
    // values come back as `String`, so we leak them once per form
    // construction — the form is short-lived but the leak is bounded
    // (~7 small strings per edit-modal open). A future cleanup is to
    // change TextField/CheckboxField to take `Into<String>`.
    let l_name = Box::leak(tr("gldf.controlgear.edit.name").into_boxed_str());
    let l_volt = Box::leak(tr("gldf.controlgear.edit.voltage").into_boxed_str());
    let l_standby = Box::leak(tr("gldf.controlgear.edit.standby").into_boxed_str());
    let l_dim = Box::leak(tr("gldf.controlgear.edit.dimmable").into_boxed_str());
    let l_color = Box::leak(tr("gldf.controlgear.edit.color_controllable").into_boxed_str());
    let l_clo_start = Box::leak(tr("gldf.controlgear.edit.clo_start").into_boxed_str());
    let l_clo_end = Box::leak(tr("gldf.controlgear.edit.clo_end").into_boxed_str());

    view! {
        <div style="display: flex; flex-direction: column; gap: 6px;">
            <TextField label=l_name value=s_name testid="cg-edit-name" />
            <TextField label=l_volt value=s_volt testid="cg-edit-voltage" />
            <TextField label=l_standby value=s_standby testid="cg-edit-standby" />
            <CheckboxField label=l_dim value=s_dim testid="cg-edit-dimmable" />
            <CheckboxField label=l_color value=s_color />
            <TextField label=l_clo_start value=s_clo_start />
            <TextField label=l_clo_end value=s_clo_end />
            <p style="font-size: 11px; color: var(--text-tertiary); font-style: italic; margin: 4px 0 0;">
                { tr("gldf.controlgear.edit.other_note") }
            </p>
            <div style="display: flex; gap: 6px; margin-top: 6px;">
                <button on:click=on_save data-testid="cg-edit-save" style="padding: 4px 10px; background: var(--accent-blue); color: white; border: none; border-radius: 4px;">{ tr("gldf.common.save") }</button>
                <button on:click=on_cancel style="padding: 4px 10px; background: none; border: 1px solid var(--border-color); border-radius: 4px;">{ tr("gldf.common.cancel") }</button>
            </div>
        </div>
    }
}

#[component]
fn TextField(
    label: &'static str,
    value: RwSignal<String>,
    #[prop(default = "")] testid: &'static str,
) -> impl IntoView {
    let on_input = move |e: web_sys::Event| {
        if let Some(v) = event_value(&e) {
            value.set(v);
        }
    };
    view! {
        <label style="display: flex; gap: 8px; align-items: center; font-size: 12px;">
            <span style="flex: 0 0 180px; color: var(--text-secondary);">{ label }</span>
            <input type="text" prop:value=value.get() on:input=on_input data-testid=testid style="flex: 1;" />
        </label>
    }
}

#[component]
fn CheckboxField(
    label: &'static str,
    value: RwSignal<bool>,
    #[prop(default = "")] testid: &'static str,
) -> impl IntoView {
    let on_change = move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            use wasm_bindgen::JsCast;
            if let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() {
                value.set(input.checked());
            }
        }
    };
    view! {
        <label style="display: flex; gap: 8px; align-items: center; font-size: 12px;">
            <span style="flex: 0 0 180px; color: var(--text-secondary);">{ label }</span>
            <input type="checkbox" prop:checked=value.get() on:change=on_change data-testid=testid />
        </label>
    }
}
