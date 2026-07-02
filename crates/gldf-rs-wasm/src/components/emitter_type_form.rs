//! Reusable "pick an Emitter LightEmitter type and fill its refs" form.
//!
//! The GLDF schema has FOUR LightEmitter kinds (`xs:choice` inside
//! `<Emitter>` at XSD L2915), each binding refs differently:
//!
//! - **FixedLightEmitter** — required PhotometryReference + required
//!   LightSourceReference @fixedLightSourceId. The lamp is baked in.
//! - **ChangeableLightEmitter** — required PhotometryReference; the lamp
//!   is *not* a direct ref (user picks at install via an optional
//!   EquipmentReference that names ChangeableLightSource(s)).
//! - **MultiChannelLightEmitter** — required LightSourceReference
//!   @multiChannelLightSourceId. Photometry lives per-channel inside the
//!   MultiChannelLightSource itself.
//! - **SensorEmitter** — required SensorReference @sensorId.
//!
//! Earlier the editor mashed "Photometry + FixedLS" together for both
//! Fixed and Changeable Add buttons, which was misleading. This component
//! makes the type the first decision so the rest of the form matches
//! exactly what the schema requires.
//!
//! Reuse: this is the same form the upcoming New GLDF Wizard will drop
//! into its Emitter step.

use crate::state::LightEmitterKind;
use leptos::prelude::*;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};

// `EmitterDraft` lives in `gldf-rs-editor::emitter_draft` so the
// wizard, the TUI, the CLI, and Python all see the same plain-data
// struct. Re-export it here to preserve the existing
// `use crate::components::EmitterDraft;` import sites.
pub use gldf_rs_editor::EmitterDraft;

/// Reusable type-first emitter creation form.
///
/// `show_name_field`: render the emitter Name input at the top (true for
/// "+ New Emitter slot", false for "+ Add child to existing emitter").
/// `submit_label`: button text (e.g. "Create Emitter" or "+ Add").
#[component]
pub fn EmitterTypeForm(
    show_name_field: bool,
    submit_label: String,
    available_photometries: Vec<String>,
    available_fixed_ls: Vec<String>,
    available_multi_channel_ls: Vec<String>,
    available_sensors: Vec<String>,
    on_submit: Arc<dyn Fn(EmitterDraft) + Send + Sync>,
    on_cancel: Arc<dyn Fn() + Send + Sync>,
) -> impl IntoView {
    let s_name = RwSignal::new(String::new());
    let s_kind = RwSignal::new(LightEmitterKind::Fixed);
    let s_photo = RwSignal::new(
        available_photometries.first().cloned().unwrap_or_default(),
    );
    let s_fixed_ls = RwSignal::new(
        available_fixed_ls.first().cloned().unwrap_or_default(),
    );
    let s_mc_ls = RwSignal::new(
        available_multi_channel_ls.first().cloned().unwrap_or_default(),
    );
    let s_sensor = RwSignal::new(available_sensors.first().cloned().unwrap_or_default());

    let photos_for_select = available_photometries.clone();
    let fixed_for_select = available_fixed_ls.clone();
    let mc_for_select = available_multi_channel_ls.clone();
    let sensors_for_select = available_sensors.clone();

    let on_kind_change = move |e: web_sys::Event| {
        let Some(target) = e.target() else { return };
        let select: HtmlSelectElement = match target.dyn_into() {
            Ok(s) => s,
            Err(_) => return,
        };
        let new_kind = match select.value().as_str() {
            "Fixed" => LightEmitterKind::Fixed,
            "Changeable" => LightEmitterKind::Changeable,
            "MultiChannel" => LightEmitterKind::MultiChannel,
            "Sensor" => LightEmitterKind::Sensor,
            _ => return,
        };
        s_kind.set(new_kind);
    };

    let on_name_input = move |e: web_sys::Event| {
        let Some(t) = e.target() else { return };
        let input: HtmlInputElement = match t.dyn_into() {
            Ok(i) => i,
            Err(_) => return,
        };
        s_name.set(input.value());
    };

    let make_select_handler = |sig: RwSignal<String>| {
        move |e: web_sys::Event| {
            let Some(target) = e.target() else { return };
            let select: HtmlSelectElement = match target.dyn_into() {
                Ok(s) => s,
                Err(_) => return,
            };
            sig.set(select.value());
        }
    };
    let on_photo_change = make_select_handler(s_photo);
    let on_fixed_ls_change = make_select_handler(s_fixed_ls);
    let on_mc_change = make_select_handler(s_mc_ls);
    let on_sensor_change = make_select_handler(s_sensor);

    let on_submit_click = move |_| {
        let draft = EmitterDraft {
            child_name: s_name.get(),
            kind: s_kind.get(),
            photometry_id: s_photo.get(),
            fixed_light_source_id: s_fixed_ls.get(),
            multi_channel_light_source_id: s_mc_ls.get(),
            sensor_id: s_sensor.get(),
        };
        // Validate required ids per kind. If the user picked a kind whose
        // catalog is empty, the form has the disabled-state below; this
        // guard is the belt-and-braces backup.
        let ok = match draft.kind {
            LightEmitterKind::Fixed => !draft.photometry_id.is_empty() && !draft.fixed_light_source_id.is_empty(),
            LightEmitterKind::Changeable => !draft.photometry_id.is_empty(),
            LightEmitterKind::MultiChannel => !draft.multi_channel_light_source_id.is_empty(),
            LightEmitterKind::Sensor => !draft.sensor_id.is_empty(),
        };
        if ok {
            on_submit(draft);
        }
    };
    let on_cancel_click = {
        let cancel = on_cancel.clone();
        move |_| cancel()
    };

    let no_photo = available_photometries.is_empty();
    let no_fixed_ls = available_fixed_ls.is_empty();
    let no_mc_ls = available_multi_channel_ls.is_empty();
    let no_sensor = available_sensors.is_empty();

    view! {
        <div class="emitter-type-form" style="display: flex; flex-direction: column; gap: 10px; padding: 12px; background: var(--bg-primary); border: 1px solid var(--accent-blue); border-radius: 6px;">
            {if show_name_field {
                view! {
                    <label style="display: grid; grid-template-columns: 160px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                        <span style="color: var(--text-secondary);">{ "Emitter name (optional)" }</span>
                        <input
                            type="text"
                            placeholder="e.g. Main slot"
                            on:input=on_name_input
                            style="padding: 3px 6px; font-size: 12px; background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: 4px; color: var(--text-primary);"
                        />
                    </label>
                }.into_any()
            } else { ().into_any() }}

            <label style="display: grid; grid-template-columns: 160px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                <span style="color: var(--text-secondary);">{ "Type" }</span>
                <select on:change=on_kind_change style="font-size: 12px; padding: 3px 6px;">
                    <option value="Fixed" selected=true>{ "Fixed — beam shape + baked-in lamp" }</option>
                    <option value="Changeable">{ "Changeable — beam shape, user-picked lamp" }</option>
                    <option value="MultiChannel">{ "MultiChannel — tunable / RGB lamp (photometry per-channel)" }</option>
                    <option value="Sensor">{ "Sensor — detection element (e.g. PIR)" }</option>
                </select>
            </label>

            {move || match s_kind.get() {
                LightEmitterKind::Fixed => {
                    let photos = photos_for_select.clone();
                    let fixed_ls = fixed_for_select.clone();
                    let on_photo = on_photo_change.clone();
                    let on_fixed = on_fixed_ls_change.clone();
                    view! {
                        <>
                            <p style="margin: 0; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                                { "Fixed: the lamp is part of the fixture. Bind one Photometry (beam shape) and one FixedLightSource (the actual lamp)." }
                            </p>
                            <label style="display: grid; grid-template-columns: 160px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                                <span style="color: var(--text-secondary);">{ "Photometry" }</span>
                                <select on:change=on_photo data-testid="emitter-form-photometry" style="font-size: 12px; padding: 3px 6px;">
                                    {if photos.is_empty() {
                                        view! { <option value="">{ "(no Photometry declared)" }</option> }.into_any()
                                    } else {
                                        view! {
                                            {photos.into_iter().map(|p| {
                                                let label = p.clone();
                                                view! { <option value=p>{ label }</option> }
                                            }).collect::<Vec<_>>()}
                                        }.into_any()
                                    }}
                                </select>
                            </label>
                            <label style="display: grid; grid-template-columns: 160px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                                <span style="color: var(--text-secondary);">{ "Fixed LightSource" }</span>
                                <select on:change=on_fixed data-testid="emitter-form-lightsource" style="font-size: 12px; padding: 3px 6px;">
                                    {if fixed_ls.is_empty() {
                                        view! { <option value="">{ "(no FixedLightSource)" }</option> }.into_any()
                                    } else {
                                        view! {
                                            {fixed_ls.into_iter().map(|l| {
                                                let label = l.clone();
                                                view! { <option value=l>{ label }</option> }
                                            }).collect::<Vec<_>>()}
                                        }.into_any()
                                    }}
                                </select>
                            </label>
                        </>
                    }.into_any()
                }
                LightEmitterKind::Changeable => {
                    let photos = photos_for_select.clone();
                    let on_photo = on_photo_change.clone();
                    view! {
                        <>
                            <p style="margin: 0; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                                { "Changeable: the user installs a lamp at runtime. Bind only a Photometry (beam shape). The lamp is named indirectly via an Equipment block (not edited here yet) or chosen by the user at install time." }
                            </p>
                            <label style="display: grid; grid-template-columns: 160px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                                <span style="color: var(--text-secondary);">{ "Photometry" }</span>
                                <select on:change=on_photo style="font-size: 12px; padding: 3px 6px;">
                                    {if photos.is_empty() {
                                        view! { <option value="">{ "(no Photometry declared)" }</option> }.into_any()
                                    } else {
                                        view! {
                                            {photos.into_iter().map(|p| {
                                                let label = p.clone();
                                                view! { <option value=p>{ label }</option> }
                                            }).collect::<Vec<_>>()}
                                        }.into_any()
                                    }}
                                </select>
                            </label>
                        </>
                    }.into_any()
                }
                LightEmitterKind::MultiChannel => {
                    let mcs = mc_for_select.clone();
                    let on_mc = on_mc_change.clone();
                    view! {
                        <>
                            <p style="margin: 0; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                                { "MultiChannel: tunable-white / RGB lamp. Bind the MultiChannelLightSource; its channels carry their own per-channel photometry and spectrum." }
                            </p>
                            <label style="display: grid; grid-template-columns: 160px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                                <span style="color: var(--text-secondary);">{ "MultiChannel LightSource" }</span>
                                <select on:change=on_mc style="font-size: 12px; padding: 3px 6px;">
                                    {if mcs.is_empty() {
                                        view! { <option value="">{ "(no MultiChannelLightSource declared)" }</option> }.into_any()
                                    } else {
                                        view! {
                                            {mcs.into_iter().map(|m| {
                                                let label = m.clone();
                                                view! { <option value=m>{ label }</option> }
                                            }).collect::<Vec<_>>()}
                                        }.into_any()
                                    }}
                                </select>
                            </label>
                        </>
                    }.into_any()
                }
                LightEmitterKind::Sensor => {
                    let sensors = sensors_for_select.clone();
                    let on_sensor = on_sensor_change.clone();
                    view! {
                        <>
                            <p style="margin: 0; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                                { "Sensor: a detection element on the luminaire (e.g. PIR motion sensor). Bind a declared <Sensor>." }
                            </p>
                            <label style="display: grid; grid-template-columns: 160px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                                <span style="color: var(--text-secondary);">{ "Sensor" }</span>
                                <select on:change=on_sensor style="font-size: 12px; padding: 3px 6px;">
                                    {if sensors.is_empty() {
                                        view! { <option value="">{ "(no Sensor declared)" }</option> }.into_any()
                                    } else {
                                        view! {
                                            {sensors.into_iter().map(|s| {
                                                let label = s.clone();
                                                view! { <option value=s>{ label }</option> }
                                            }).collect::<Vec<_>>()}
                                        }.into_any()
                                    }}
                                </select>
                            </label>
                        </>
                    }.into_any()
                }
            }}

            {move || {
                let kind = s_kind.get();
                let disabled = match kind {
                    LightEmitterKind::Fixed => no_photo || no_fixed_ls,
                    LightEmitterKind::Changeable => no_photo,
                    LightEmitterKind::MultiChannel => no_mc_ls,
                    LightEmitterKind::Sensor => no_sensor,
                };
                let label = submit_label.clone();
                let on_click = on_submit_click.clone();
                let on_cancel = on_cancel_click.clone();
                view! {
                    <div style="display: flex; gap: 6px; margin-top: 4px;">
                        <button
                            on:click=on_click
                            disabled=disabled
                            data-testid="emitter-form-submit"
                            style="padding: 4px 10px; font-size: 12px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;"
                        >{ label }</button>
                        <button
                            on:click=on_cancel
                            style="padding: 4px 10px; font-size: 12px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                        >{ "Cancel" }</button>
                        {if disabled {
                            view! {
                                <span style="font-size: 11px; color: var(--accent-orange); font-style: italic; align-self: center;">
                                    { "Catalog empty for this type — add a Photometry / LightSource / Sensor first." }
                                </span>
                            }.into_any()
                        } else { ().into_any() }}
                    </div>
                }
            }}
        </div>
    }
}
