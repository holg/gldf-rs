//! New GLDF Wizard — multi-step modal that builds a GLDF from zero.
//!
//! Architecture: each step lives in its own component and holds its own
//! RwSignal-backed draft state. The wizard's outer `WizardState` owns the
//! drafts (so Back/Next don't lose data) and a `current_step` index. The
//! wizard never mutates the loaded GLDF until the user hits **Finish** —
//! at that moment all `apply()` dispatches run in sequence, building the
//! product. This means Back never has to undo an action.
//!
//! Cut 1 ships 4 core steps (Header, Photometry, LightSource, Emitter)
//! and the shell. Follow-up cuts wire Spectrum, Pictures, Variant
//! metadata, Geometry binding, etc., by adding entries to `STEPS` and
//! their corresponding form components.

use crate::components::{EditModal, EmitterDraft, EmitterTypeForm};
use crate::state::use_gldf;
use gldf_rs_editor::{apply_wizard, WizardDraft, WizardLightSourceKind};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};

/// Wizard-wide draft state. Each step writes into its own slice; the
/// final `apply_all` reads everything in order.
#[derive(Clone)]
pub struct WizardState {
    // Header step
    pub header_author: RwSignal<String>,
    pub header_manufacturer: RwSignal<String>,
    pub header_created_with: RwSignal<String>,
    pub header_default_language: RwSignal<String>,

    // Photometry step — bytes + filename of the dropped file (None until
    // the user actually drops one; Finish skips the step if None).
    pub photometry_bytes: RwSignal<Option<Vec<u8>>>,
    pub photometry_filename: RwSignal<String>,

    // LightSource step
    pub ls_kind: RwSignal<WizardLightSourceKind>,
    pub ls_name: RwSignal<String>,
    pub ls_manufacturer: RwSignal<String>,
    pub ls_rated_power: RwSignal<String>,

    // Emitter step — Photometry id will exist after Photometry step
    // dispatches; the user picks LightEmitter kind + ref ids via the
    // existing EmitterTypeForm.
    pub emitter_draft: RwSignal<Option<EmitterDraft>>,
}

impl WizardState {
    fn new() -> Self {
        Self {
            header_author: RwSignal::new(String::new()),
            header_manufacturer: RwSignal::new(String::new()),
            header_created_with: RwSignal::new("gldf.icu wizard".to_string()),
            header_default_language: RwSignal::new("en".to_string()),
            photometry_bytes: RwSignal::new(None),
            photometry_filename: RwSignal::new(String::new()),
            ls_kind: RwSignal::new(WizardLightSourceKind::Fixed),
            ls_name: RwSignal::new(String::new()),
            ls_manufacturer: RwSignal::new(String::new()),
            ls_rated_power: RwSignal::new(String::new()),
            emitter_draft: RwSignal::new(None),
        }
    }
}

/// Static list of step titles, indexed by step number.
const STEPS: &[&str] = &[
    "Header",
    "Photometry",
    "Light source",
    "Emitter",
    "Review & Finish",
];

/// Top-level wizard modal. `open` toggles visibility; `on_close` fires
/// after Finish (or Cancel) so the welcome screen can hide.
#[component]
pub fn NewGldfWizard(
    open: RwSignal<bool>,
    on_close: Arc<dyn Fn() + Send + Sync>,
) -> impl IntoView {
    let state = use_gldf();
    let wizard = WizardState::new();
    let current = RwSignal::new(0_usize);

    let on_back = move |_| {
        let c = current.get();
        if c > 0 {
            current.set(c - 1);
        }
    };
    let on_next = move |_| {
        let c = current.get();
        if c + 1 < STEPS.len() {
            current.set(c + 1);
        }
    };

    // Wrap on_finish in Arc<dyn Fn> so the reactive `move ||` closure
    // that switches between the Next/Create buttons can clone it on
    // each evaluation. Leptos children must be `Fn`, not `FnOnce`.
    let on_finish: Arc<dyn Fn() + Send + Sync> = {
        let wizard = wizard.clone();
        let on_close = on_close.clone();
        Arc::new(move || {
            apply_all(state, &wizard);
            on_close();
        })
    };

    let wizard_for_step = wizard.clone();
    let on_close_for_modal = on_close.clone();

    view! {
        <EditModal
            open=open
            title="Create a new GLDF".to_string()
            on_close=on_close_for_modal.clone()
        >
            <div style="display: grid; grid-template-columns: 180px 1fr; gap: 16px; min-height: 360px;">
                // Left pane: step list
                <div style="border-right: 1px solid var(--border-color); padding-right: 12px;">
                    <ol style="margin: 0; padding: 0; list-style: none; display: flex; flex-direction: column; gap: 4px;">
                        {STEPS.iter().enumerate().map(|(idx, label)| {
                            let label = label.to_string();
                            view! {
                                <li>
                                    <StepChip idx=idx label=label current=current />
                                </li>
                            }
                        }).collect::<Vec<_>>()}
                    </ol>
                </div>

                // Right pane: active step
                <div>
                    {
                        let wiz = wizard_for_step.clone();
                        move || {
                            let step = current.get();
                            let w = wiz.clone();
                            match step {
                                0 => view! { <HeaderStep w=w /> }.into_any(),
                                1 => view! { <PhotometryStep w=w /> }.into_any(),
                                2 => view! { <LightSourceStep w=w /> }.into_any(),
                                3 => view! { <EmitterStep w=w /> }.into_any(),
                                _ => view! { <ReviewStep w=w /> }.into_any(),
                            }
                        }
                    }

                    <div style="display: flex; gap: 8px; margin-top: 16px; padding-top: 12px; border-top: 1px solid var(--border-color);">
                        <button
                            on:click=on_back
                            disabled=move || current.get() == 0
                            style="padding: 4px 10px; font-size: 12px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                        >{ "← Back" }</button>
                        {
                            // Hoist the Arc clone out of the reactive
                            // closure so the closure stays `Fn` (not
                            // FnOnce).
                            let on_finish_for_button = on_finish.clone();
                            move || {
                            let c = current.get();
                            let is_last = c + 1 == STEPS.len();
                            if is_last {
                                let on_finish = on_finish_for_button.clone();
                                let on_click = move |_| on_finish();
                                view! {
                                    <button
                                        on:click=on_click
                                        style="margin-left: auto; padding: 4px 12px; font-size: 12px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 500;"
                                    >{ "Create GLDF" }</button>
                                }.into_any()
                            } else {
                                view! {
                                    <button
                                        on:click=on_next
                                        style="margin-left: auto; padding: 4px 12px; font-size: 12px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;"
                                    >{ "Next →" }</button>
                                }.into_any()
                            }
                            }
                        }
                    </div>
                </div>
            </div>
        </EditModal>
    }
}

/// One entry in the left-pane step list. Clickable to jump to any step.
#[component]
fn StepChip(idx: usize, label: String, current: RwSignal<usize>) -> impl IntoView {
    let on_click = move |_| current.set(idx);
    view! {
        {move || {
            let is_active = current.get() == idx;
            let is_done = current.get() > idx;
            let bg = if is_active { "var(--accent-blue)" }
                else if is_done { "var(--bg-secondary)" }
                else { "transparent" };
            let color = if is_active { "white" }
                else if is_done { "var(--text-primary)" }
                else { "var(--text-secondary)" };
            let style = format!(
                "display: flex; align-items: center; gap: 8px; width: 100%; padding: 6px 10px; font-size: 12px; background: {}; color: {}; border: none; border-radius: 4px; cursor: pointer; text-align: left;",
                bg, color,
            );
            let marker = if is_done { "✓".to_string() } else { format!("{}", idx + 1) };
            view! {
                <button on:click=on_click.clone() style=style>
                    <span style="font-weight: 500; min-width: 16px;">{ marker }</span>
                    <span>{ label.clone() }</span>
                </button>
            }
        }}
    }
}

// ============================================================
// Steps
// ============================================================

#[component]
fn HeaderStep(w: WizardState) -> impl IntoView {
    let on_author = move |e: web_sys::Event| {
        if let Some(t) = e.target() {
            if let Ok(i) = t.dyn_into::<HtmlInputElement>() {
                w.header_author.set(i.value());
            }
        }
    };
    let on_manuf = move |e: web_sys::Event| {
        if let Some(t) = e.target() {
            if let Ok(i) = t.dyn_into::<HtmlInputElement>() {
                w.header_manufacturer.set(i.value());
            }
        }
    };
    let on_created_with = move |e: web_sys::Event| {
        if let Some(t) = e.target() {
            if let Ok(i) = t.dyn_into::<HtmlInputElement>() {
                w.header_created_with.set(i.value());
            }
        }
    };
    let on_lang = move |e: web_sys::Event| {
        if let Some(t) = e.target() {
            if let Ok(i) = t.dyn_into::<HtmlInputElement>() {
                w.header_default_language.set(i.value());
            }
        }
    };

    view! {
        <div>
            <h3 style="margin: 0 0 4px; font-size: 14px;">{ "1 · Header" }</h3>
            <p style="margin: 0 0 12px; font-size: 12px; color: var(--text-secondary);">
                { "Document-level metadata. Author and manufacturer show up on the Overview card." }
            </p>
            <div style="display: flex; flex-direction: column; gap: 8px;">
                <label style="display: grid; grid-template-columns: 140px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                    <span style="color: var(--text-secondary);">{ "Author" }</span>
                    <input type="text" prop:value=move || w.header_author.get() on:input=on_author placeholder="Your name or organization"
                        style="padding: 4px 6px; font-size: 12px; background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: 4px; color: var(--text-primary);" />
                </label>
                <label style="display: grid; grid-template-columns: 140px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                    <span style="color: var(--text-secondary);">{ "Manufacturer" }</span>
                    <input type="text" prop:value=move || w.header_manufacturer.get() on:input=on_manuf placeholder="The luminaire brand"
                        style="padding: 4px 6px; font-size: 12px; background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: 4px; color: var(--text-primary);" />
                </label>
                <label style="display: grid; grid-template-columns: 140px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                    <span style="color: var(--text-secondary);">{ "Created with" }</span>
                    <input type="text" prop:value=move || w.header_created_with.get() on:input=on_created_with
                        style="padding: 4px 6px; font-size: 12px; background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: 4px; color: var(--text-primary);" />
                </label>
                <label style="display: grid; grid-template-columns: 140px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                    <span style="color: var(--text-secondary);">{ "Default language" }</span>
                    <input type="text" prop:value=move || w.header_default_language.get() on:input=on_lang placeholder="en / de / fr / …"
                        style="padding: 4px 6px; font-size: 12px; background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: 4px; color: var(--text-primary);" />
                </label>
            </div>
        </div>
    }
}

#[component]
fn PhotometryStep(w: WizardState) -> impl IntoView {
    let on_change = move |e: web_sys::Event| {
        let Some(t) = e.target() else { return };
        let input: HtmlInputElement = match t.dyn_into() {
            Ok(i) => i,
            Err(_) => return,
        };
        let Some(files) = input.files() else { return };
        let Some(file) = files.get(0) else { return };
        let filename = file.name();
        let gloo_file = gloo::file::File::from(file);
        spawn_local(async move {
            match gloo::file::futures::read_as_bytes(&gloo_file).await {
                Ok(bytes) => {
                    w.photometry_filename.set(filename);
                    w.photometry_bytes.set(Some(bytes));
                }
                Err(err) => log::error!("read photometry: {}", err),
            }
        });
        input.set_value("");
    };

    view! {
        <div>
            <h3 style="margin: 0 0 4px; font-size: 14px;">{ "2 · Photometry" }</h3>
            <p style="margin: 0 0 12px; font-size: 12px; color: var(--text-secondary);">
                { "Drop a .ldt (EULUMDAT) or .ies (IESNA LM-63) file. The wizard will create the matching <File> and <Photometry> entries when you click Create at the end." }
            </p>
            <label style="display: inline-flex; gap: 12px; align-items: center; padding: 10px 12px; border: 1px dashed var(--accent-blue); border-radius: 6px; background: var(--bg-secondary); cursor: pointer;">
                <span style="color: var(--accent-blue); font-weight: 500; font-size: 13px;">{ "Drop or pick a photometry file" }</span>
                <input type="file" accept=".ldt,.ies" on:change=on_change style="font-size: 12px;" />
            </label>
            {move || {
                if let Some(bytes) = w.photometry_bytes.get() {
                    view! {
                        <p style="margin: 8px 0 0; font-size: 12px; color: var(--accent-green);">
                            { format!("✓ {} ({} bytes) staged", w.photometry_filename.get(), bytes.len()) }
                        </p>
                    }.into_any()
                } else {
                    view! {
                        <p style="margin: 8px 0 0; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                            { "Optional — Skip with Next if you'll add photometry later." }
                        </p>
                    }.into_any()
                }
            }}
        </div>
    }
}

#[component]
fn LightSourceStep(w: WizardState) -> impl IntoView {
    let on_kind = move |e: web_sys::Event| {
        let Some(t) = e.target() else { return };
        let sel: HtmlSelectElement = match t.dyn_into() {
            Ok(s) => s,
            Err(_) => return,
        };
        let kind = match sel.value().as_str() {
            "Fixed" => WizardLightSourceKind::Fixed,
            "Changeable" => WizardLightSourceKind::Changeable,
            "MultiChannel" => WizardLightSourceKind::MultiChannel,
            _ => return,
        };
        w.ls_kind.set(kind);
    };
    let on_name = move |e: web_sys::Event| {
        if let Some(t) = e.target() {
            if let Ok(i) = t.dyn_into::<HtmlInputElement>() {
                w.ls_name.set(i.value());
            }
        }
    };
    let on_manuf = move |e: web_sys::Event| {
        if let Some(t) = e.target() {
            if let Ok(i) = t.dyn_into::<HtmlInputElement>() {
                w.ls_manufacturer.set(i.value());
            }
        }
    };
    let on_power = move |e: web_sys::Event| {
        if let Some(t) = e.target() {
            if let Ok(i) = t.dyn_into::<HtmlInputElement>() {
                w.ls_rated_power.set(i.value());
            }
        }
    };

    view! {
        <div>
            <h3 style="margin: 0 0 4px; font-size: 14px;">{ "3 · Light source" }</h3>
            <p style="margin: 0 0 12px; font-size: 12px; color: var(--text-secondary);">
                { "The lamp itself. Fixed = baked into the fixture. Changeable = user-replaceable bulb. MultiChannel = tunable-white / RGB." }
            </p>
            <div style="display: flex; flex-direction: column; gap: 8px;">
                <label style="display: grid; grid-template-columns: 140px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                    <span style="color: var(--text-secondary);">{ "Kind" }</span>
                    <select on:change=on_kind style="padding: 4px 6px; font-size: 12px;">
                        <option value="Fixed" selected=true>{ "Fixed (baked-in lamp)" }</option>
                        <option value="Changeable">{ "Changeable (user-replaceable)" }</option>
                        <option value="MultiChannel">{ "MultiChannel (tunable / RGB)" }</option>
                    </select>
                </label>
                <label style="display: grid; grid-template-columns: 140px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                    <span style="color: var(--text-secondary);">{ "Display name" }</span>
                    <input type="text" prop:value=move || w.ls_name.get() on:input=on_name placeholder="e.g. LED 3000K chip"
                        style="padding: 4px 6px; font-size: 12px; background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: 4px; color: var(--text-primary);" />
                </label>
                <label style="display: grid; grid-template-columns: 140px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                    <span style="color: var(--text-secondary);">{ "Manufacturer" }</span>
                    <input type="text" prop:value=move || w.ls_manufacturer.get() on:input=on_manuf placeholder="Lamp brand (optional)"
                        style="padding: 4px 6px; font-size: 12px; background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: 4px; color: var(--text-primary);" />
                </label>
                <label style="display: grid; grid-template-columns: 140px 1fr; gap: 8px; align-items: center; font-size: 12px;">
                    <span style="color: var(--text-secondary);">{ "Rated power (W)" }</span>
                    <input type="number" min="0" step="0.1" prop:value=move || w.ls_rated_power.get() on:input=on_power placeholder="e.g. 24"
                        style="padding: 4px 6px; font-size: 12px; background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: 4px; color: var(--text-primary); width: 120px;" />
                </label>
            </div>
        </div>
    }
}

#[component]
fn EmitterStep(w: WizardState) -> impl IntoView {
    // The Emitter step doesn't know the auto-generated Photometry / LS ids
    // yet (they happen at Finish), but the EmitterTypeForm expects existing
    // catalog ids. So this step shows placeholder ids ("photometry_1",
    // "lightsource_1") that match what generate_unique_id will produce
    // when Finish runs in the documented order. If the user wants more
    // control they can add Emitters via the editor after Finish.
    let placeholder_photo = vec!["photometry_1".to_string()];
    let placeholder_fixed = vec!["lightsource_1".to_string()];
    let placeholder_mc = vec!["lightsource_1".to_string()];
    let placeholder_sensor = vec![];

    let on_submit: Arc<dyn Fn(EmitterDraft) + Send + Sync> = Arc::new(move |draft: EmitterDraft| {
        w.emitter_draft.set(Some(draft));
    });
    let on_cancel: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        w.emitter_draft.set(None);
    });

    view! {
        <div>
            <h3 style="margin: 0 0 4px; font-size: 14px;">{ "4 · Emitter" }</h3>
            <p style="margin: 0 0 12px; font-size: 12px; color: var(--text-secondary);">
                { "An Emitter is a slot on the luminaire that binds the photometry (beam shape) to the light source (lamp). The form below uses the same XSD-conformant type picker as the editor." }
            </p>
            {move || {
                if let Some(draft) = w.emitter_draft.get() {
                    view! {
                        <div style="padding: 10px 12px; background: var(--bg-secondary); border-left: 3px solid var(--accent-green); border-radius: 4px;">
                            <div style="font-size: 12px; color: var(--accent-green); margin-bottom: 4px;">{ "✓ Emitter staged" }</div>
                            <div style="font-size: 11px; color: var(--text-secondary);">
                                { format!("Type: {:?} · Photometry: {} · Fixed LS: {} · MC LS: {} · Sensor: {}",
                                    draft.kind,
                                    draft.photometry_id,
                                    draft.fixed_light_source_id,
                                    draft.multi_channel_light_source_id,
                                    draft.sensor_id,
                                ) }
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <EmitterTypeForm
                            show_name_field=true
                            submit_label="Stage Emitter".to_string()
                            available_photometries=placeholder_photo.clone()
                            available_fixed_ls=placeholder_fixed.clone()
                            available_multi_channel_ls=placeholder_mc.clone()
                            available_sensors=placeholder_sensor.clone()
                            on_submit=on_submit.clone()
                            on_cancel=on_cancel.clone()
                        />
                    }.into_any()
                }
            }}
        </div>
    }
}

#[component]
fn ReviewStep(w: WizardState) -> impl IntoView {
    view! {
        <div>
            <h3 style="margin: 0 0 4px; font-size: 14px;">{ "5 · Review & Finish" }</h3>
            <p style="margin: 0 0 12px; font-size: 12px; color: var(--text-secondary);">
                { "Confirm what the wizard will create. Clicking Create GLDF dispatches all changes in order and drops you on the Overview page." }
            </p>
            <ul style="margin: 0; padding-left: 18px; font-size: 12px; display: flex; flex-direction: column; gap: 4px;">
                <li>
                    <strong>{ "Header" }</strong>
                    { format!(" — author: \"{}\" · manufacturer: \"{}\" · created with: \"{}\" · default language: \"{}\"",
                        w.header_author.get(), w.header_manufacturer.get(),
                        w.header_created_with.get(), w.header_default_language.get(),
                    ) }
                </li>
                <li>
                    <strong>{ "Photometry" }</strong>
                    { move || if let Some(bytes) = w.photometry_bytes.get() {
                        format!(" — {} ({} bytes)", w.photometry_filename.get(), bytes.len())
                    } else {
                        " — (skipped; add later via the editor)".to_string()
                    } }
                </li>
                <li>
                    <strong>{ "Light source" }</strong>
                    { format!(" — kind: {:?} · name: \"{}\" · manufacturer: \"{}\" · power: {}W",
                        w.ls_kind.get(), w.ls_name.get(),
                        w.ls_manufacturer.get(), w.ls_rated_power.get(),
                    ) }
                </li>
                <li>
                    <strong>{ "Emitter" }</strong>
                    { move || if let Some(d) = w.emitter_draft.get() {
                        format!(" — type: {:?}", d.kind)
                    } else {
                        " — (skipped)".to_string()
                    } }
                </li>
            </ul>
            <p style="margin: 12px 0 0; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                { "After Create, use the toolbar's Save button to download the .gldf file." }
            </p>
        </div>
    }
}

// ============================================================
// Finish — build a WizardDraft from the RwSignal-backed wizard fields,
// then dispatch to the shared `apply_wizard` reducer in gldf-rs-editor.
// The reducer is identical across WASM / TUI / CLI / Python — every
// frontend converges to one Rust function here.
// ============================================================

fn apply_all(state: crate::state::GldfContext, w: &WizardState) {
    let draft = WizardDraft {
        header_author: w.header_author.get(),
        header_manufacturer: w.header_manufacturer.get(),
        header_created_with: w.header_created_with.get(),
        header_default_language: w.header_default_language.get(),
        photometry_bytes: w.photometry_bytes.get(),
        photometry_filename: w.photometry_filename.get(),
        ls_kind: w.ls_kind.get(),
        ls_name: w.ls_name.get(),
        ls_manufacturer: w.ls_manufacturer.get(),
        ls_rated_power: w.ls_rated_power.get(),
        emitter_draft: w.emitter_draft.get(),
    };
    state.update(|s| apply_wizard(s, &draft));
}
