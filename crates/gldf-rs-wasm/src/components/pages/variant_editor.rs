//! Variant editor — per-SKU overrides of ProductMetaData fields.
//!
//! UX: compact summary cards in a grid. Each card shows id/name/GTIN
//! and an action cluster (↑ ↓ ⎘ ✕ Edit). Clicking **Edit** opens a
//! centered [`EditModal`] containing the full form (LocaleFoo fields,
//! GTIN, sortOrder, Symbol, plus read-only Mountings/Geometry
//! summaries). Same modal pattern as the LightSource and Emitter
//! editors so the layout stays consistent.
//!
//! Mountings/Geometry full editors are a follow-on cut; today they're
//! summarised so users at least see what's there.

use crate::components::edit_util::{event_i32, event_value, optional_string};
use crate::components::{EditModal, LocaleFooEditor};
use crate::i18n::ui::t as ui_t;
use crate::state::{use_gldf, GldfAction};
use gldf_rs::gldf::product_definitions::{Ceiling, Ground, Mountings, Variant, Wall};
use gldf_rs::gldf::LocaleFoo;
use gldf_rs_editor::ModelEmitterBinding;
use leptos::prelude::*;
use std::sync::Arc;

/// Geometry sub-mode picker for a single variant. Mirrors the XSD
/// `xs:choice` (bare / simple / model). The user picks one mode and
/// the form below it edits exactly that branch.
#[derive(Clone, Copy, Debug, PartialEq)]
enum GeometryMode {
    None,
    Bare,
    Simple,
    Model,
}

impl GeometryMode {
    fn from_geometry(g: &gldf_rs::gldf::product_definitions::Geometry) -> Self {
        if g.simple_geometry_reference.is_some() {
            Self::Simple
        } else if g.model_geometry_reference.is_some() {
            Self::Model
        } else if !g.emitter_reference.is_empty() {
            Self::Bare
        } else {
            Self::None
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::None => "None (no geometry)",
            Self::Bare => "Bare EmitterReference",
            Self::Simple => "Simple Geometry",
            Self::Model => "Model Geometry (.l3d)",
        }
    }

    fn testid_slug(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Bare => "bare",
            Self::Simple => "simple",
            Self::Model => "model",
        }
    }
}

#[component]
pub fn VariantEditorPage() -> impl IntoView {
    let state = use_gldf();
    // `Some(idx)` while the edit modal for that variant is open.
    let editing = RwSignal::new(None::<usize>);

    let on_add = move |_| {
        state.update(|s| {
            let n = s
                .product
                .product_definitions
                .variants
                .as_ref()
                .map(|v| v.variant.len())
                .unwrap_or(0)
                + 1;
            s.apply(GldfAction::AddVariantSimple {
                name: format!("Variant {}", n),
            });
        });
    };

    view! {
        <div class="editor-section variant-editor">
            <h2>{ "Variants" }</h2>
            <p class="section-description">
                { "Per-SKU variants. Each entry overrides ProductMetaData for one article number / GTIN. Document-level metadata lives under Product Metadata → Identity. Per-category DescriptiveAttribute overrides (Electrical, Mechanical, Pictures, …) live on those pages — use the Scope picker at the top of each to target a variant." }
            </p>

            {move || state.with(|s| {
                let variants: Vec<Variant> = s
                    .product
                    .product_definitions
                    .variants
                    .as_ref()
                    .map(|v| v.variant.clone())
                    .unwrap_or_default();
                let count = variants.len();
                let active_lang = s.active_language.clone();
                view! {
                    <div class="variants-section">
                        <div style="display: flex; gap: 8px; align-items: center; margin-bottom: 8px;">
                            <h3 style="margin: 0;">{ format!("Variants ({})", count) }</h3>
                            <button type="button" class="btn btn-secondary" on:click=on_add style="margin-left: auto;">
                                { "+ New variant" }
                            </button>
                        </div>
                        {if variants.is_empty() {
                            view! { <p class="empty-message">{ "No variants defined." }</p> }.into_any()
                        } else {
                            view! {
                                <div class="variant-cards">
                                    {variants.into_iter().enumerate().map(|(idx, variant)| {
                                        let active_lang = active_lang.clone();
                                        let last_idx = count - 1;
                                        view! {
                                            <VariantSummaryCard idx=idx variant=variant active_lang=active_lang editing=editing last_idx=last_idx />
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        }}
                    </div>
                }
            })}

            // Centered edit modal — re-binds to whatever variant is in `editing`
            // so opening / closing doesn't leak stale Cards.
            //
            // `open` is a Memo, not a Signal::derive: a Memo caches the
            // value and only notifies subscribers when it *flips*, so the
            // host's open-mirror Effect won't re-fire on every in-modal
            // state.apply. Without that guarantee the mirror Effect would
            // race against the body's re-renders and tear down the modal
            // mid-edit (issue #136).
            {
                let open = Memo::new(move |_| editing.get().is_some());
                let on_close: Arc<dyn Fn() + Send + Sync> = Arc::new(move || editing.set(None));
                view! {
                    <VariantModalHost editing=editing open=open on_close=on_close />
                }
            }
        </div>
    }
}

/// Wraps `EditModal` and renders the body for whichever variant index
/// is in `editing`. Splitting this out so the closures don't fight over
/// captures with the outer page.
///
/// Visible to siblings so the read-mode `VariantsPage` can mount the
/// same modal — opening Edit on a read-mode card flips the same signal
/// the editor page would. Lets us avoid duplicating the Mountings /
/// ProductSeries / Geometry editors.
#[component]
pub(crate) fn VariantModalHost(
    editing: RwSignal<Option<usize>>,
    open: Memo<bool>,
    on_close: Arc<dyn Fn() + Send + Sync>,
) -> impl IntoView {
    let state = use_gldf();
    // EditModal needs `RwSignal<bool>`; mirror `open` (a Memo, so it
    // only ticks on actual flips) into a local signal. Single Effect
    // direction — host → modal. Backdrop / Esc / Save all go through
    // `on_close` which writes `editing`, which `open` derives from. No
    // second Effect, no round-trip → no race (issue #136).
    let open_signal = RwSignal::new(open.get_untracked());
    Effect::new(move |_| {
        open_signal.set(open.get());
    });

    view! {
        <EditModal open=open_signal title="Edit variant".to_string() on_close=on_close.clone()>
            {move || {
                let Some(idx) = editing.get() else { return ().into_any(); };
                let snap = state.with(|s| {
                    s.product
                        .product_definitions
                        .variants
                        .as_ref()
                        .and_then(|vs| vs.variant.get(idx).cloned())
                });
                let Some(variant) = snap else { return ().into_any(); };
                view! { <VariantEditForm idx=idx variant=variant /> }.into_any()
            }}
        </EditModal>
    }
}

/// Compact summary card (read-only mini-view + action cluster). Clicking
/// Edit opens the modal; the other buttons mutate state directly.
#[component]
fn VariantSummaryCard(
    idx: usize,
    variant: Variant,
    active_lang: String,
    editing: RwSignal<Option<usize>>,
    last_idx: usize,
) -> impl IntoView {
    let state = use_gldf();

    let on_edit = move |_| editing.set(Some(idx));
    let on_duplicate = move |e: web_sys::MouseEvent| {
        e.stop_propagation();
        state.update(|s| s.apply(GldfAction::DuplicateVariant { src_idx: idx }));
    };
    let on_delete = {
        let id_for_remove = variant.id.clone();
        move |e: web_sys::MouseEvent| {
            e.stop_propagation();
            let win = web_sys::window();
            let confirmed = win
                .and_then(|w| {
                    w.confirm_with_message(&format!(
                        "Delete variant '{}'? This cannot be undone.",
                        id_for_remove
                    ))
                    .ok()
                })
                .unwrap_or(true);
            if confirmed {
                state.update(|s| {
                    s.apply(GldfAction::RemoveVariant {
                        id: id_for_remove.clone(),
                    })
                });
            }
        }
    };
    let on_move_up = move |e: web_sys::MouseEvent| {
        e.stop_propagation();
        state.update(|s| s.apply(GldfAction::MoveVariant { index: idx, delta: -1 }));
    };
    let on_move_down = move |e: web_sys::MouseEvent| {
        e.stop_propagation();
        state.update(|s| s.apply(GldfAction::MoveVariant { index: idx, delta: 1 }));
    };

    let id_str = variant.id.clone();
    let name = variant
        .name
        .as_ref()
        .and_then(|n| {
            n.locale
                .iter()
                .find(|l| l.language == active_lang)
                .or_else(|| n.locale.first())
        })
        .map(|l| l.value.clone())
        .unwrap_or_else(|| "(No name)".to_string());
    let gtin = variant.gtin.clone();
    let sort = variant.sort_order;
    let can_move_up = idx > 0;
    let can_move_down = idx < last_idx;

    view! {
        <div class="variant-card" style="background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 6px; padding: 10px 12px; display: flex; flex-direction: column; gap: 6px;">
            <div style="display: flex; gap: 8px; align-items: center;">
                <span class="card-id" style="font-family: var(--font-mono); color: var(--text-secondary); font-size: 11px;">{ id_str.clone() }</span>
                {sort.map(|n| view! { <span class="sort-order" style="font-size: 10px; color: var(--text-tertiary);">{ format!("#{}", n) }</span> })}
                <span style="margin-left: auto; display: flex; gap: 2px; flex-shrink: 0;">
                    <button type="button" class="btn-icon" title="Move up" on:click=on_move_up prop:disabled=!can_move_up>{ "↑" }</button>
                    <button type="button" class="btn-icon" title="Move down" on:click=on_move_down prop:disabled=!can_move_down>{ "↓" }</button>
                    <button type="button" class="btn-icon" title="Duplicate" data-testid=format!("variant-duplicate-{}", idx) on:click=on_duplicate>{ "⎘" }</button>
                    <button type="button" class="btn-icon" title="Delete" on:click=on_delete>{ "✕" }</button>
                </span>
            </div>
            <div style="display: flex; gap: 8px; align-items: baseline;">
                <span style="font-size: 14px; font-weight: 500;">{ name }</span>
            </div>
            {gtin.map(|g| view! {
                <div style="font-size: 11px; color: var(--text-tertiary); font-family: var(--font-mono);">
                    { format!("GTIN: {}", g) }
                </div>
            })}
            <button type="button" class="btn btn-secondary" data-testid=format!("variant-edit-{}", idx) on:click=on_edit style="margin-top: 4px; align-self: flex-start;">
                { "✎ Edit" }
            </button>
        </div>
    }
}

/// The full edit form rendered inside the modal. Reads fresh values
/// from state on each open (the parent passes a cloned `Variant`
/// snapshot, but the modal body re-renders on every state change).
#[component]
fn VariantEditForm(idx: usize, variant: Variant) -> impl IntoView {
    let state = use_gldf();

    let on_name = Callback::new(move |v: LocaleFoo| {
        state.update(|s| s.apply(GldfAction::SetVariantName(idx, v)));
    });
    let on_pn = Callback::new(move |v: LocaleFoo| {
        state.update(|s| s.apply(GldfAction::SetVariantProductNumber(idx, v)));
    });
    let on_desc = Callback::new(move |v: LocaleFoo| {
        state.update(|s| s.apply(GldfAction::SetVariantDescription(idx, v)));
    });
    let on_tender = Callback::new(move |v: LocaleFoo| {
        state.update(|s| s.apply(GldfAction::SetVariantTenderText(idx, v)));
    });
    let on_gtin = move |e: crate::components::edit_util::EditEvent| {
        let v = event_value(&e).unwrap_or_default();
        state.update(|s| {
            s.apply(GldfAction::SetVariantGTIN {
                index: idx,
                gtin: optional_string(v),
            })
        });
    };
    let on_sort = move |e: crate::components::edit_util::EditEvent| {
        let v = event_i32(&e);
        state.update(|s| {
            s.apply(GldfAction::SetVariantSortOrder {
                index: idx,
                sort_order: v,
            })
        });
    };
    let on_symbol = move |e: crate::components::edit_util::EditEvent| {
        let v = event_value(&e).unwrap_or_default();
        state.update(|s| {
            s.apply(GldfAction::SetVariantSymbol {
                index: idx,
                file_id: optional_string(v),
            })
        });
    };

    let gtin_str = variant.gtin.clone().unwrap_or_default();
    let sort_order_str = variant
        .sort_order
        .map(|n| n.to_string())
        .unwrap_or_default();
    let symbol_file_id = variant
        .symbol
        .as_ref()
        .map(|s| s.file_id.clone())
        .unwrap_or_default();
    let name_val = variant.name.clone().unwrap_or_default();
    let pn_val = variant.product_number.clone().unwrap_or_default();
    let desc_val = variant.description.clone().unwrap_or_default();
    let tender_val = variant.tender_text.clone().unwrap_or_default();
    let mountings = variant.mountings.clone();
    // The geometry display is now handled by `<VariantGeometryEditor>`
    // which reads fresh from state; no need to clone the snapshot here.

    view! {
        <div class="variant-info" style="display: flex; flex-direction: column; gap: 12px;">
            <LocaleFooEditor label="Variant Name" value=name_val on_change=on_name />
            <LocaleFooEditor label="Product Number" value=pn_val on_change=on_pn />
            <LocaleFooEditor label="Description" value=desc_val on_change=on_desc multiline=true />
            <LocaleFooEditor label="Tender Text" value=tender_val on_change=on_tender multiline=true />
            <div class="form-row" style="display: grid; grid-template-columns: 140px 1fr; gap: 6px; align-items: center; font-size: 12px;">
                <label for=format!("gtin-{}", idx)>{ "GTIN" }</label>
                <input id=format!("gtin-{}", idx) type="text" prop:value=gtin_str on:change=on_gtin placeholder="13 digits" />
            </div>
            <div class="form-row" style="display: grid; grid-template-columns: 140px 1fr; gap: 6px; align-items: center; font-size: 12px;">
                <label for=format!("sort-{}", idx)>{ "Sort order" }</label>
                <input id=format!("sort-{}", idx) type="number" prop:value=sort_order_str on:change=on_sort placeholder="auto (document order)" style="max-width: 200px;" />
            </div>
            <div class="form-row" style="display: grid; grid-template-columns: 140px 1fr; gap: 6px; align-items: center; font-size: 12px;">
                <label for=format!("symbol-{}", idx)>{ "Symbol (CAD)" }</label>
                {
                    let st = state;
                    let selected = symbol_file_id.clone();
                    move || st.with(|s| {
                        let files: Vec<(String, String)> = s
                            .product
                            .general_definitions
                            .files
                            .file
                            .iter()
                            .map(|f| (f.id.clone(), f.file_name.clone()))
                            .collect();
                        let sel = selected.clone();
                        view! {
                            <select id=format!("symbol-{}", idx) on:change=on_symbol>
                                <option value="" selected=sel.is_empty()>{ "(none)" }</option>
                                {files.into_iter().map(|(id, name)| {
                                    let is_sel = id == sel;
                                    view! {
                                        <option value=id.clone() selected=is_sel>
                                            { format!("{} · {}", id, name) }
                                        </option>
                                    }
                                }).collect::<Vec<_>>()}
                            </select>
                        }
                    })
                }
            </div>
        </div>
        <VariantMountingsEditor idx=idx mountings=mountings.clone() />
        <VariantProductSeriesEditor idx=idx series=variant.product_series.clone() />
        <VariantGeometryEditor idx=idx />
        <p style="margin-top: 12px; font-size: 11px; color: var(--text-tertiary); font-style: italic;">
            { "Variant overrides for Electrical / Mechanical / Pictures / Hyperlinks / Emergency live on their respective pages — use the Scope picker at the top to target this variant." }
        </p>
    }
}


/// Geometry editor for a single variant. Mode picker (None / Bare /
/// Simple / Model) + per-mode form. Dispatches the appropriate
/// `SetVariantGeometry*` / `ClearVariantGeometry` action.
#[component]
fn VariantGeometryEditor(idx: usize) -> impl IntoView {
    let state = use_gldf();

    let on_set_mode = move |mode: GeometryMode| {
        state.update(|s| {
            // Read current emitter/geometry ids to seed the new mode
            // with sensible defaults — the user can re-pick afterwards.
            let first_emitter = s
                .product
                .general_definitions
                .emitters
                .as_ref()
                .and_then(|es| es.emitter.first())
                .map(|e| e.id.clone());
            let first_simple_geom = s
                .product
                .general_definitions
                .geometries
                .as_ref()
                .and_then(|gs| gs.simple_geometry.first())
                .map(|g| g.id.clone());
            let first_model_geom = s
                .product
                .general_definitions
                .geometries
                .as_ref()
                .and_then(|gs| gs.model_geometry.first())
                .map(|g| g.id.clone());

            match mode {
                GeometryMode::None => {
                    s.apply(GldfAction::ClearVariantGeometry { variant_idx: idx });
                }
                GeometryMode::Bare => {
                    let emitter_ids = first_emitter.into_iter().collect();
                    s.apply(GldfAction::SetVariantGeometryEmitterRefs {
                        variant_idx: idx,
                        emitter_ids,
                    });
                }
                GeometryMode::Simple => {
                    // Always switch into Simple mode even when no
                    // SimpleGeometry definitions exist yet — the panel
                    // below will surface the warning + a link to where
                    // to add one. Without this the mode click silently
                    // no-ops and looks like a bug.
                    s.apply(GldfAction::SetVariantGeometrySimple {
                        variant_idx: idx,
                        geometry_id: first_simple_geom.unwrap_or_default(),
                        emitter_id: first_emitter.clone().unwrap_or_default(),
                    });
                }
                GeometryMode::Model => {
                    let emitter_id = first_emitter.unwrap_or_default();
                    s.apply(GldfAction::SetVariantGeometryModel {
                        variant_idx: idx,
                        geometry_id: first_model_geom.unwrap_or_default(),
                        bindings: if emitter_id.is_empty() {
                            Vec::new()
                        } else {
                            vec![ModelEmitterBinding {
                                emitter_id,
                                external_name: "Primary".to_string(),
                                target_model_type: Some("l3d".to_string()),
                            }]
                        },
                    });
                }
            }
        });
    };

    view! {
        <div class="variant-geometry-editor" style="margin-top: 12px;">
            <h5 style="margin: 0 0 6px;">{ "Geometry" }</h5>
            {move || state.with(|s| {
                let geom = s
                    .product
                    .product_definitions
                    .variants
                    .as_ref()
                    .and_then(|vs| vs.variant.get(idx))
                    .and_then(|v| v.geometry.as_ref())
                    .cloned();
                let cur_mode = geom.as_ref().map(GeometryMode::from_geometry).unwrap_or(GeometryMode::None);
                // Snapshot the available pickers so closures can read them
                // without re-borrowing state.
                let available_emitters: Vec<(String, Option<String>)> = s
                    .product
                    .general_definitions
                    .emitters
                    .as_ref()
                    .map(|es| {
                        es.emitter
                            .iter()
                            .map(|e| {
                                // First LightEmitter child's name as a hint;
                                // None if neither field is set.
                                let hint = e
                                    .fixed_light_emitter
                                    .first()
                                    .and_then(|le| le.name.as_ref())
                                    .and_then(|n| n.locale.first())
                                    .map(|l| l.value.clone());
                                (e.id.clone(), hint)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let model_geometries: Vec<String> = s
                    .product
                    .general_definitions
                    .geometries
                    .as_ref()
                    .map(|gs| gs.model_geometry.iter().map(|g| g.id.clone()).collect())
                    .unwrap_or_default();
                let simple_geometries: Vec<String> = s
                    .product
                    .general_definitions
                    .geometries
                    .as_ref()
                    .map(|gs| gs.simple_geometry.iter().map(|g| g.id.clone()).collect())
                    .unwrap_or_default();

                view! {
                    <div style="display: flex; gap: 4px; flex-wrap: wrap; margin-bottom: 8px;">
                        {[GeometryMode::None, GeometryMode::Bare, GeometryMode::Simple, GeometryMode::Model].iter().map(|m| {
                            let mode = *m;
                            let active = mode == cur_mode;
                            let style = if active {
                                "padding: 4px 10px; font-size: 12px; background: var(--accent-blue); color: white; border: 1px solid var(--accent-blue); border-radius: 4px; cursor: pointer;"
                            } else {
                                "padding: 4px 10px; font-size: 12px; background: var(--bg-secondary); color: var(--text-primary); border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer;"
                            };
                            view! {
                                <button type="button" style=style data-testid=format!("variant-geometry-mode-{}-{}", idx, mode.testid_slug()) on:click=move |_| on_set_mode(mode)>
                                    { mode.label() }
                                </button>
                            }
                        }).collect::<Vec<_>>()}
                    </div>

                    {match cur_mode {
                        GeometryMode::None => view! {
                            <p style="font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                                { "Variant has no Geometry. The GLDF will be incomplete (XSD requires an Emitter binding). Pick a mode above." }
                            </p>
                        }.into_any(),
                        GeometryMode::Bare => {
                            let bound: Vec<String> = geom
                                .as_ref()
                                .map(|g| g.emitter_reference.iter().map(|e| e.emitter_id.clone()).collect())
                                .unwrap_or_default();
                            view! {
                                <BareEmitterList idx=idx bound=bound available=available_emitters />
                            }.into_any()
                        }
                        GeometryMode::Simple => {
                            let sg = geom.as_ref().and_then(|g| g.simple_geometry_reference.clone());
                            view! {
                                <SimpleGeometryPicker idx=idx current=sg simple_ids=simple_geometries available_emitters=available_emitters />
                            }.into_any()
                        }
                        GeometryMode::Model => {
                            let mg = geom.as_ref().and_then(|g| g.model_geometry_reference.clone());
                            view! {
                                <ModelGeometryEditor idx=idx current=mg model_ids=model_geometries available_emitters=available_emitters />
                            }.into_any()
                        }
                    }}
                }
            })}
        </div>
    }
}

/// Render the bare-emitter list and a "+ Add" dropdown.
#[component]
fn BareEmitterList(
    idx: usize,
    bound: Vec<String>,
    available: Vec<(String, Option<String>)>,
) -> impl IntoView {
    let state = use_gldf();
    // Filter out already-bound from the picker.
    let bound_for_filter = bound.clone();
    let pickable: Vec<(String, Option<String>)> = available
        .into_iter()
        .filter(|(id, _)| !bound_for_filter.contains(id))
        .collect();

    let bound_for_add = bound.clone();
    let on_add = move |e: crate::components::edit_util::EditEvent| {
        let Some(v) = event_value(&e) else { return };
        if v.is_empty() {
            return;
        }
        let mut next = bound_for_add.clone();
        next.push(v);
        state.update(|s| {
            s.apply(GldfAction::SetVariantGeometryEmitterRefs {
                variant_idx: idx,
                emitter_ids: next,
            });
        });
    };

    view! {
        <div style="display: flex; flex-direction: column; gap: 4px; font-size: 12px;">
            {bound.into_iter().map(|eid| {
                let label = eid.clone();
                let eid_for_remove = eid.clone();
                let cur_for_remove = label.clone();
                let on_remove = move |_| {
                    let cur = cur_for_remove.clone();
                    state.update(|s| {
                        let mut keep: Vec<String> = s
                            .product
                            .product_definitions
                            .variants
                            .as_ref()
                            .and_then(|vs| vs.variant.get(idx))
                            .and_then(|v| v.geometry.as_ref())
                            .map(|g| g.emitter_reference.iter().map(|e| e.emitter_id.clone()).collect())
                            .unwrap_or_default();
                        keep.retain(|id| id != &cur);
                        s.apply(GldfAction::SetVariantGeometryEmitterRefs {
                            variant_idx: idx,
                            emitter_ids: keep,
                        });
                    });
                    let _ = &eid_for_remove;
                };
                view! {
                    <div style="display: flex; gap: 6px; align-items: center;">
                        <span class="code" style="font-family: var(--font-mono); flex: 1;">{ label }</span>
                        <button type="button" class="btn-icon" title="Remove" on:click=on_remove>{ "✕" }</button>
                    </div>
                }
            }).collect::<Vec<_>>()}
            {if pickable.is_empty() {
                view! { <p class="hint" style="font-size: 11px; color: var(--text-tertiary);">{ "All emitters bound." }</p> }.into_any()
            } else {
                view! {
                    <select on:change=on_add style="margin-top: 4px;">
                        <option value="" selected=true>{ "+ Add emitter…" }</option>
                        {pickable.into_iter().map(|(id, name)| {
                            let display = match name {
                                Some(n) if !n.is_empty() => format!("{} ({})", id, n),
                                _ => id.clone(),
                            };
                            view! { <option value=id>{ display }</option> }
                        }).collect::<Vec<_>>()}
                    </select>
                }.into_any()
            }}
        </div>
    }
}

/// Pick a SimpleGeometry id + Emitter id.
#[component]
fn SimpleGeometryPicker(
    idx: usize,
    current: Option<gldf_rs::gldf::product_definitions::SimpleGeometryReference>,
    simple_ids: Vec<String>,
    available_emitters: Vec<(String, Option<String>)>,
) -> impl IntoView {
    let state = use_gldf();
    let cur_geom = current.as_ref().map(|s| s.geometry_id.clone()).unwrap_or_default();
    let cur_emitter = current.as_ref().map(|s| s.emitter_id.clone()).unwrap_or_default();

    let cur_emitter_for_geom = cur_emitter.clone();
    let on_geom = move |e: crate::components::edit_util::EditEvent| {
        let Some(v) = event_value(&e) else { return };
        let emitter_id = cur_emitter_for_geom.clone();
        state.update(|s| {
            s.apply(GldfAction::SetVariantGeometrySimple {
                variant_idx: idx,
                geometry_id: v,
                emitter_id,
            });
        });
    };
    let cur_geom_for_emitter = cur_geom.clone();
    let on_emitter = move |e: crate::components::edit_util::EditEvent| {
        let Some(v) = event_value(&e) else { return };
        let geometry_id = cur_geom_for_emitter.clone();
        state.update(|s| {
            s.apply(GldfAction::SetVariantGeometrySimple {
                variant_idx: idx,
                geometry_id,
                emitter_id: v,
            });
        });
    };

    let cur_geom_for_options = cur_geom.clone();
    let cur_emitter_for_options = cur_emitter.clone();
    let empty_geom_warning = simple_ids.is_empty();

    view! {
        <div style="display: flex; flex-direction: column; gap: 6px;">
            {if empty_geom_warning {
                view! {
                    <p style="margin: 0 0 4px; padding: 6px 8px; font-size: 11px; color: var(--accent-orange); background: rgba(255, 165, 0, 0.08); border-left: 3px solid var(--accent-orange); border-radius: 3px;">
                        { "No SimpleGeometry definitions yet. Add a Cuboid or Cylinder on the Geometries page (sidebar) before binding here." }
                    </p>
                }.into_any()
            } else { ().into_any() }}
            <div style="display: grid; grid-template-columns: 140px 1fr; gap: 6px; align-items: center; font-size: 12px;">
                <label>{ "SimpleGeometry id" }</label>
                <select on:change=on_geom>
                    <option value="" selected=cur_geom_for_options.is_empty()>{ "(none)" }</option>
                    {simple_ids.into_iter().map(|id| {
                        let selected = id == cur_geom_for_options;
                        let label = id.clone();
                        view! { <option value=id selected=selected>{ label }</option> }
                    }).collect::<Vec<_>>()}
                </select>
                <label>{ "Emitter id" }</label>
                <select on:change=on_emitter>
                    <option value="" selected=cur_emitter_for_options.is_empty()>{ "(none)" }</option>
                    {available_emitters.into_iter().map(|(id, _)| {
                        let selected = id == cur_emitter_for_options;
                        let label = id.clone();
                        view! { <option value=id selected=selected>{ label }</option> }
                    }).collect::<Vec<_>>()}
                </select>
            </div>
        </div>
    }
}

/// Edit a ModelGeometryReference with multi-emitter bindings.
#[component]
fn ModelGeometryEditor(
    idx: usize,
    current: Option<gldf_rs::gldf::product_definitions::ModelGeometryReference>,
    model_ids: Vec<String>,
    available_emitters: Vec<(String, Option<String>)>,
) -> impl IntoView {
    let state = use_gldf();
    let geometry_id = current.as_ref().map(|m| m.geometry_id.clone()).unwrap_or_default();
    let bindings_init: Vec<ModelEmitterBinding> = current
        .as_ref()
        .map(|m| {
            m.emitter_reference
                .iter()
                .map(|e| ModelEmitterBinding {
                    emitter_id: e.emitter_id.clone(),
                    external_name: e.emitter_object_external_name.clone(),
                    target_model_type: e.target_model_type.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    let bindings_for_geom = bindings_init.clone();
    let on_geom = move |e: crate::components::edit_util::EditEvent| {
        let Some(v) = event_value(&e) else { return };
        let bindings = bindings_for_geom.clone();
        state.update(|s| {
            s.apply(GldfAction::SetVariantGeometryModel {
                variant_idx: idx,
                geometry_id: v,
                bindings,
            });
        });
    };

    let geom_id_for_add = geometry_id.clone();
    let bindings_for_add = bindings_init.clone();
    let available_for_add = available_emitters.clone();
    let on_add_binding = move |_| {
        // Pick the first emitter id not already bound, or just the first.
        let Some((fresh_id, _)) = available_for_add
            .iter()
            .find(|(id, _)| !bindings_for_add.iter().any(|b| &b.emitter_id == id))
            .or_else(|| available_for_add.first())
        else {
            return;
        };
        let mut next = bindings_for_add.clone();
        next.push(ModelEmitterBinding {
            emitter_id: fresh_id.clone(),
            external_name: "Primary".to_string(),
            target_model_type: Some("l3d".to_string()),
        });
        let geometry_id = geom_id_for_add.clone();
        state.update(|s| {
            s.apply(GldfAction::SetVariantGeometryModel {
                variant_idx: idx,
                geometry_id,
                bindings: next,
            });
        });
    };

    let geometry_id_for_opts = geometry_id.clone();
    let geometry_id_for_picker = geometry_id.clone();
    let empty_model_warning = model_ids.is_empty();

    view! {
        <div style="display: flex; flex-direction: column; gap: 8px; font-size: 12px;">
            {if empty_model_warning {
                view! {
                    <p style="margin: 0; padding: 6px 8px; font-size: 11px; color: var(--accent-orange); background: rgba(255, 165, 0, 0.08); border-left: 3px solid var(--accent-orange); border-radius: 3px;">
                        { "No ModelGeometry definitions yet. Drop a .l3d file on the Files page — it auto-registers as a ModelGeometry that shows up here." }
                    </p>
                }.into_any()
            } else { ().into_any() }}
            <div style="display: grid; grid-template-columns: 140px 1fr; gap: 6px; align-items: center;">
                <label>{ "ModelGeometry id" }</label>
                <select on:change=on_geom data-testid=format!("variant-model-geometry-select-{}", idx)>
                    <option value="" selected=geometry_id_for_opts.is_empty()>{ "(none)" }</option>
                    {model_ids.into_iter().map(|id| {
                        let selected = id == geometry_id_for_opts;
                        let label = id.clone();
                        view! { <option value=id selected=selected>{ label }</option> }
                    }).collect::<Vec<_>>()}
                </select>
            </div>
            // Graphical LEO→photometry picker: a 2D plan view of the
            // L3D's Light Emitting Objects. Shown only when a model
            // geometry is selected (it reads the L3D bytes by id). The
            // manual binding list below stays as an "Advanced" fallback.
            {(!geometry_id_for_picker.is_empty()).then(|| view! {
                <crate::components::pages::leo_plan_picker::LeoPlanPicker
                    idx=idx geometry_id=geometry_id_for_picker.clone() />
            })}
            <details style="margin-top: 6px;">
                <summary style="cursor: pointer; font-size: 12px; color: var(--text-secondary);">
                    { "Advanced: raw emitter ↔ object bindings" }
                </summary>
                <strong>{ "Per-object emitter bindings" }</strong>
                <ul style="margin: 4px 0 0; padding-left: 18px;">
                    {bindings_init.iter().enumerate().map(|(i, b)| {
                        let binding_idx = i;
                        let current_emitter = b.emitter_id.clone();
                        let external_name = b.external_name.clone();
                        let target = b.target_model_type.clone().unwrap_or_else(|| "—".to_string());
                        let geom_id_for_select = geometry_id.clone();
                        let bindings_for_select = bindings_init.clone();
                        let on_pick = move |e: crate::components::edit_util::EditEvent| {
                            let Some(v) = event_value(&e) else { return };
                            let mut next = bindings_for_select.clone();
                            if let Some(slot) = next.get_mut(binding_idx) {
                                slot.emitter_id = v;
                            }
                            let geometry_id = geom_id_for_select.clone();
                            state.update(|s| {
                                s.apply(GldfAction::SetVariantGeometryModel {
                                    variant_idx: idx,
                                    geometry_id,
                                    bindings: next,
                                });
                            });
                        };
                        let geom_id_for_remove = geometry_id.clone();
                        let bindings_for_remove = bindings_init.clone();
                        let on_remove = move |_| {
                            let mut next = bindings_for_remove.clone();
                            if binding_idx < next.len() {
                                next.remove(binding_idx);
                            }
                            let geometry_id = geom_id_for_remove.clone();
                            state.update(|s| {
                                s.apply(GldfAction::SetVariantGeometryModel {
                                    variant_idx: idx,
                                    geometry_id,
                                    bindings: next,
                                });
                            });
                        };
                        let emitter_options = available_emitters.clone();
                        view! {
                            <li style="display: flex; gap: 6px; align-items: center;">
                                <select
                                    on:change=on_pick
                                    data-testid=format!("variant-binding-emitter-{}-{}", idx, binding_idx)
                                    style="font-size: 11px; font-family: var(--font-mono); padding: 2px 4px;"
                                >
                                    {emitter_options.into_iter().map(|(em_id, em_name)| {
                                        let selected = em_id == current_emitter;
                                        let label = match em_name {
                                            Some(n) if !n.is_empty() => format!("{} ({})", em_id, n),
                                            _ => em_id.clone(),
                                        };
                                        view! { <option value=em_id selected=selected>{ label }</option> }
                                    }).collect::<Vec<_>>()}
                                </select>
                                <span style="font-family: var(--font-mono); font-size: 11px; color: var(--text-secondary);">
                                    { format!("↔ {} [{}]", external_name, target) }
                                </span>
                                <button type="button" class="btn-icon" title="Remove binding" on:click=on_remove>{ "✕" }</button>
                            </li>
                        }
                    }).collect::<Vec<_>>()}
                </ul>
                <button type="button" class="btn btn-secondary" style="margin-top: 6px; font-size: 11px;" data-testid=format!("variant-add-binding-{}", idx) on:click=on_add_binding>
                    { "+ Add binding" }
                </button>
                <p class="hint" style="font-size: 11px; color: var(--text-tertiary); margin-top: 4px;">
                    { "Each binding pairs an Emitter id with the named object inside the 3D model. The plan-view picker above sets these graphically; this list is the raw fallback." }
                </p>
            </details>
        </div>
    }
}

// ===========================================================================
// Mountings editor
// ===========================================================================
//
// The XSD's `Variant > Mountings` is a record with four optional top-level
// children — Ceiling / Wall / Ground / WorkingPlane — each of which is itself
// a `choice` of sub-mount types (Recessed / SurfaceMounted / Pendant for
// Ceiling, etc.). A full editor for every leaf would explode the variant
// card; this cut surfaces the most common primitive per branch:
//
// * **Ceiling**: pick one of Recessed (with depth) / SurfaceMounted / Pendant
//   (with length). The XSD permits multiple branches simultaneously but
//   that's rare in practice — we let the user enable one at a time.
// * **Wall**: a mounting height (mm) plus optional Recessed depth.
// * **Ground**: a pole-mount choice (PoleTop / PoleIntegrated, with
//   poleHeight) or FreeStanding / SurfaceMounted / Recessed.
// * **WorkingPlane**: just a boolean toggle for FreeStanding (no fields).
//
// All four are independent — the user can leave any subset disabled. When
// all four are disabled the whole `<Mountings>` element disappears
// (`SetVariantMountings { payload: None }`).
//
// What this cut leaves on the floor: cutout dimensions
// (RectangularCutout / CircularCutout), depth, length, and the legacy
// `poleHeight` element vs `@poleHeight` attribute migration path. The
// model preserves them on round-trip; they just aren't exposed in the
// UI yet.

use gldf_rs::gldf::product_definitions::{
    FreeStanding, Pendant, PoleIntegrated, PoleTop, Recessed, SurfaceMounted, WorkingPlane,
};

#[component]
fn VariantMountingsEditor(idx: usize, mountings: Option<Mountings>) -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));
    let init = mountings.unwrap_or_default();

    // Per-branch state. Each branch holds its enable flag + the few fields
    // we expose. We rebuild a full Mountings struct on every dispatch so the
    // reducer doesn't need to know about the granular shape.
    let ceiling_enabled = RwSignal::new(init.ceiling.is_some());
    // Ceiling sub-mode: 0 = Recessed, 1 = SurfaceMounted, 2 = Pendant.
    let ceiling_mode = RwSignal::new(match init.ceiling.as_ref() {
        Some(c) if c.recessed.is_some() => 0u8,
        Some(c) if c.surface_mounted.is_some() => 1u8,
        Some(c) if c.pendant.is_some() => 2u8,
        _ => 0u8,
    });
    let ceiling_recessed_depth = RwSignal::new(
        init.ceiling
            .as_ref()
            .and_then(|c| c.recessed.as_ref())
            .map(|r| r.recessed_depth)
            .unwrap_or(0),
    );
    let ceiling_pendant_length = RwSignal::new(
        init.ceiling
            .as_ref()
            .and_then(|c| c.pendant.as_ref())
            .map(|p| p.pendant_length)
            .unwrap_or(0),
    );

    let wall_enabled = RwSignal::new(init.wall.is_some());
    let wall_mounting_height = RwSignal::new(
        init.wall.as_ref().map(|w| w.mounting_height).unwrap_or(0),
    );
    let wall_recessed_depth = RwSignal::new(
        init.wall
            .as_ref()
            .and_then(|w| w.recessed.as_ref())
            .map(|r| r.recessed_depth)
            .unwrap_or(0),
    );
    let wall_use_recessed = RwSignal::new(
        init.wall.as_ref().map(|w| w.recessed.is_some()).unwrap_or(false),
    );

    let ground_enabled = RwSignal::new(init.ground.is_some());
    // Ground sub-mode: 0 = PoleTop, 1 = PoleIntegrated, 2 = FreeStanding,
    // 3 = SurfaceMounted. (Recessed exists on Ground too but is rare.)
    let ground_mode = RwSignal::new(match init.ground.as_ref() {
        Some(g) if g.pole_top.is_some() => 0u8,
        Some(g) if g.pole_integrated.is_some() => 1u8,
        Some(g) if g.free_standing.is_some() => 2u8,
        Some(g) if g.surface_mounted.is_some() => 3u8,
        _ => 0u8,
    });
    let ground_pole_height = RwSignal::new(
        init.ground
            .as_ref()
            .and_then(|g| {
                g.pole_top
                    .as_ref()
                    .and_then(PoleTop::get_pole_height)
                    .or_else(|| {
                        g.pole_integrated.as_ref().and_then(PoleIntegrated::get_pole_height)
                    })
            })
            .unwrap_or(0),
    );

    let working_plane_enabled = RwSignal::new(init.working_plane.is_some());

    // Build a fresh Mountings from the signals and dispatch. Called on every
    // user mutation — cheap because the action just stores the value, no
    // schema validation.
    let dispatch = move || {
        let any_enabled = ceiling_enabled.get()
            || wall_enabled.get()
            || ground_enabled.get()
            || working_plane_enabled.get();
        if !any_enabled {
            state.update(|s| {
                s.apply(GldfAction::SetVariantMountings {
                    index: idx,
                    payload: None,
                });
            });
            return;
        }

        let ceiling = if ceiling_enabled.get() {
            let recessed = if ceiling_mode.get() == 0 {
                Some(Recessed {
                    recessed_depth: ceiling_recessed_depth.get(),
                    ..Default::default()
                })
            } else {
                None
            };
            let surface_mounted = if ceiling_mode.get() == 1 {
                Some(SurfaceMounted {})
            } else {
                None
            };
            let pendant = if ceiling_mode.get() == 2 {
                Some(Pendant {
                    pendant_length: ceiling_pendant_length.get(),
                })
            } else {
                None
            };
            Some(Ceiling {
                recessed,
                surface_mounted,
                pendant,
            })
        } else {
            None
        };

        let wall = if wall_enabled.get() {
            let recessed = if wall_use_recessed.get() {
                Some(Recessed {
                    recessed_depth: wall_recessed_depth.get(),
                    ..Default::default()
                })
            } else {
                None
            };
            Some(Wall {
                mounting_height: wall_mounting_height.get(),
                recessed,
                surface_mounted: None,
            })
        } else {
            None
        };

        let ground = if ground_enabled.get() {
            let pole_top = if ground_mode.get() == 0 {
                Some(PoleTop {
                    pole_height: if ground_pole_height.get() > 0 {
                        Some(ground_pole_height.get())
                    } else {
                        None
                    },
                    pole_height_element: None,
                })
            } else {
                None
            };
            let pole_integrated = if ground_mode.get() == 1 {
                Some(PoleIntegrated {
                    pole_height: if ground_pole_height.get() > 0 {
                        Some(ground_pole_height.get())
                    } else {
                        None
                    },
                    pole_height_element: None,
                })
            } else {
                None
            };
            let free_standing = if ground_mode.get() == 2 {
                Some(FreeStanding {})
            } else {
                None
            };
            let surface_mounted = if ground_mode.get() == 3 {
                Some(SurfaceMounted {})
            } else {
                None
            };
            Some(Ground {
                pole_top,
                pole_integrated,
                free_standing,
                surface_mounted,
                recessed: None,
            })
        } else {
            None
        };

        let working_plane = if working_plane_enabled.get() {
            Some(WorkingPlane {
                free_standing: Some(FreeStanding {}),
            })
        } else {
            None
        };

        let payload = Mountings {
            ceiling,
            wall,
            working_plane,
            ground,
        };

        state.update(|s| {
            s.apply(GldfAction::SetVariantMountings {
                index: idx,
                payload: Some(Box::new(payload)),
            });
        });
    };

    // One closure per signal change. They're stamped with `dispatch` so the
    // mutation lands as a single action; the reducer never sees an
    // intermediate half-applied state.
    let toggle_ceiling = {
        let dispatch = dispatch.clone();
        move |_| {
            ceiling_enabled.update(|v| *v = !*v);
            dispatch();
        }
    };
    let toggle_wall = {
        let dispatch = dispatch.clone();
        move |_| {
            wall_enabled.update(|v| *v = !*v);
            dispatch();
        }
    };
    let toggle_ground = {
        let dispatch = dispatch.clone();
        move |_| {
            ground_enabled.update(|v| *v = !*v);
            dispatch();
        }
    };
    let toggle_working_plane = {
        let dispatch = dispatch.clone();
        move |_| {
            working_plane_enabled.update(|v| *v = !*v);
            dispatch();
        }
    };
    let toggle_wall_recessed = {
        let dispatch = dispatch.clone();
        move |_| {
            wall_use_recessed.update(|v| *v = !*v);
            dispatch();
        }
    };
    let set_ceiling_mode = {
        let dispatch = dispatch.clone();
        move |e: crate::components::edit_util::EditEvent| {
            let v: u8 = event_value(&e).and_then(|s| s.parse().ok()).unwrap_or(0);
            ceiling_mode.set(v);
            dispatch();
        }
    };
    let set_ground_mode = {
        let dispatch = dispatch.clone();
        move |e: crate::components::edit_util::EditEvent| {
            let v: u8 = event_value(&e).and_then(|s| s.parse().ok()).unwrap_or(0);
            ground_mode.set(v);
            dispatch();
        }
    };
    let set_ceiling_depth = {
        let dispatch = dispatch.clone();
        move |e: crate::components::edit_util::EditEvent| {
            ceiling_recessed_depth.set(
                event_value(&e).and_then(|s| s.parse().ok()).unwrap_or(0),
            );
            dispatch();
        }
    };
    let set_ceiling_pendant = {
        let dispatch = dispatch.clone();
        move |e: crate::components::edit_util::EditEvent| {
            ceiling_pendant_length.set(
                event_value(&e).and_then(|s| s.parse().ok()).unwrap_or(0),
            );
            dispatch();
        }
    };
    let set_wall_height = {
        let dispatch = dispatch.clone();
        move |e: crate::components::edit_util::EditEvent| {
            wall_mounting_height.set(
                event_value(&e).and_then(|s| s.parse().ok()).unwrap_or(0),
            );
            dispatch();
        }
    };
    let set_wall_depth = {
        let dispatch = dispatch.clone();
        move |e: crate::components::edit_util::EditEvent| {
            wall_recessed_depth.set(
                event_value(&e).and_then(|s| s.parse().ok()).unwrap_or(0),
            );
            dispatch();
        }
    };
    let set_ground_height = {
        let dispatch = dispatch.clone();
        move |e: crate::components::edit_util::EditEvent| {
            ground_pole_height.set(
                event_value(&e).and_then(|s| s.parse().ok()).unwrap_or(0),
            );
            dispatch();
        }
    };

    // Each top-level branch row reuses `.application-checkbox-row` — the
    // same class Applications and Labels use. That style caps the
    // checkbox at `width: auto` (the global `input { width: 100% }`
    // reset would otherwise stretch every native checkbox to fill the
    // card, hiding the label) AND pins `color: var(--text-primary)` so
    // the row label is legible on the dark card background. Without
    // this, Mountings rendered as a column of fat checkboxes with a
    // grey-on-dark label that almost disappeared.
    let sub_style = "margin-left: 22px; display: flex; flex-wrap: wrap; gap: 8px; align-items: center; font-size: 12px;";

    view! {
        <div class="variant-mountings-editor" style="margin-top: 12px; border: 1px solid var(--border-color); border-radius: 6px; padding: 10px;">
            <h4 style="margin: 0 0 6px; font-size: 13px;">{ tr("gldf.mountings.title") }</h4>
            <p style="font-size: 11px; color: var(--text-tertiary); margin: 0 0 8px;">
                { tr("gldf.mountings.description") }
            </p>

            // Ceiling
            <label class="application-checkbox-row">
                <input type="checkbox" prop:checked=move || ceiling_enabled.get() on:change=toggle_ceiling data-testid=format!("variant-mountings-ceiling-{}", idx) />
                <span><strong>{ tr("gldf.mountings.ceiling") }</strong></span>
            </label>
            {move || ceiling_enabled.get().then(|| view! {
                <div style=sub_style>
                    <select on:change=set_ceiling_mode.clone() prop:value=move || ceiling_mode.get().to_string()>
                        <option value="0">{ tr("gldf.mountings.recessed") }</option>
                        <option value="1">{ tr("gldf.mountings.surface_mounted") }</option>
                        <option value="2">{ tr("gldf.mountings.pendant") }</option>
                    </select>
                    {move || (ceiling_mode.get() == 0).then(|| view! {
                        <>
                            <label>{ tr("gldf.mountings.depth_mm") }</label>
                            <input type="number" min="0" prop:value=move || ceiling_recessed_depth.get().to_string() on:change=set_ceiling_depth.clone() style="width: 80px;" />
                        </>
                    })}
                    {move || (ceiling_mode.get() == 2).then(|| view! {
                        <>
                            <label>{ tr("gldf.mountings.length_mm") }</label>
                            <input type="number" min="0" step="any" prop:value=move || ceiling_pendant_length.get().to_string() on:change=set_ceiling_pendant.clone() style="width: 80px;" />
                        </>
                    })}
                </div>
            })}

            // Wall
            <label class="application-checkbox-row">
                <input type="checkbox" prop:checked=move || wall_enabled.get() on:change=toggle_wall data-testid=format!("variant-mountings-wall-{}", idx) />
                <span><strong>{ tr("gldf.mountings.wall") }</strong></span>
            </label>
            {move || wall_enabled.get().then(|| view! {
                <div style=sub_style>
                    <label>{ tr("gldf.mountings.mounting_height_mm") }</label>
                    <input type="number" min="0" prop:value=move || wall_mounting_height.get().to_string() on:change=set_wall_height.clone() style="width: 100px;" />
                    <label style="margin-left: 8px;">
                        <input type="checkbox" prop:checked=move || wall_use_recessed.get() on:change=toggle_wall_recessed.clone() />
                        { format!(" {}", tr("gldf.mountings.recessed_label")) }
                    </label>
                    {move || wall_use_recessed.get().then(|| view! {
                        <>
                            <label>{ tr("gldf.mountings.depth_mm") }</label>
                            <input type="number" min="0" prop:value=move || wall_recessed_depth.get().to_string() on:change=set_wall_depth.clone() style="width: 80px;" />
                        </>
                    })}
                </div>
            })}

            // Ground
            <label class="application-checkbox-row">
                <input type="checkbox" prop:checked=move || ground_enabled.get() on:change=toggle_ground data-testid=format!("variant-mountings-ground-{}", idx) />
                <span><strong>{ tr("gldf.mountings.ground") }</strong></span>
            </label>
            {move || ground_enabled.get().then(|| view! {
                <div style=sub_style>
                    <select on:change=set_ground_mode.clone() prop:value=move || ground_mode.get().to_string()>
                        <option value="0">{ tr("gldf.mountings.pole_top") }</option>
                        <option value="1">{ tr("gldf.mountings.pole_integrated") }</option>
                        <option value="2">{ tr("gldf.mountings.free_standing") }</option>
                        <option value="3">{ tr("gldf.mountings.surface_mounted") }</option>
                    </select>
                    {move || matches!(ground_mode.get(), 0 | 1).then(|| view! {
                        <>
                            <label>{ tr("gldf.mountings.pole_height_mm") }</label>
                            <input type="number" min="0" prop:value=move || ground_pole_height.get().to_string() on:change=set_ground_height.clone() style="width: 100px;" />
                        </>
                    })}
                </div>
            })}

            // Working plane
            <label class="application-checkbox-row">
                <input type="checkbox" prop:checked=move || working_plane_enabled.get() on:change=toggle_working_plane data-testid=format!("variant-mountings-working-plane-{}", idx) />
                <span><strong>{ tr("gldf.mountings.working_plane") }</strong>{ " " }<span style="color: var(--text-tertiary); font-size: 11px;">{ tr("gldf.mountings.working_plane_hint") }</span></span>
            </label>
        </div>
    }
}

// ===========================================================================
// ProductSeries editor
// ===========================================================================
//
// XSD shape: `<ProductSeries>` is a wrapper around 1..N `<ProductSerie>`
// children, each carrying `@id`, optional `<Name>` Locale, optional
// `<Description>` Locale, plus `<Pictures>` and `<Hyperlinks>` blocks.
// This cut surfaces id + Name only — the most common case is a single
// series ("Tria 2 Round") referenced by every SKU. Pictures and
// Hyperlinks live on their own pages; adding them inline here would
// duplicate that work.
//
// `id` is required by the XSD; empty ids would fail validation, so the
// editor refuses to dispatch a partial row.

#[component]
fn VariantProductSeriesEditor(
    idx: usize,
    series: Option<gldf_rs::gldf::general_definitions::ProductSeries>,
) -> impl IntoView {
    use gldf_rs::gldf::general_definitions::ProductSerie;
    use gldf_rs::gldf::LocaleFoo;

    let state = use_gldf();
    let init: Vec<ProductSerie> = series
        .map(|ps| ps.product_serie)
        .unwrap_or_default();
    let rows = RwSignal::new(init);

    let dispatch = move || {
        let payload = rows.with(|rs| rs.clone());
        state.update(|s| {
            s.apply(GldfAction::SetVariantProductSeries {
                index: idx,
                payload,
            });
        });
    };

    let on_add = {
        let dispatch = dispatch.clone();
        move |_| {
            rows.update(|rs| {
                rs.push(ProductSerie {
                    id: format!("series_{}", rs.len() + 1),
                    name: None,
                    description: None,
                    pictures: None,
                    hyperlinks: None,
                });
            });
            dispatch();
        }
    };

    let on_remove = move |row_idx: usize| {
        rows.update(|rs| {
            if row_idx < rs.len() {
                rs.remove(row_idx);
            }
        });
        dispatch();
    };

    let on_id = move |row_idx: usize, new_id: String| {
        rows.update(|rs| {
            if let Some(r) = rs.get_mut(row_idx) {
                r.id = new_id;
            }
        });
        dispatch();
    };

    let on_name = move |row_idx: usize, locale: LocaleFoo| {
        rows.update(|rs| {
            if let Some(r) = rs.get_mut(row_idx) {
                // Drop empty names back to `None` so the XML doesn't carry
                // `<Name><Locale language=""/></Name>` placeholders.
                r.name = if locale.locale.is_empty() {
                    None
                } else {
                    Some(locale)
                };
            }
        });
        dispatch();
    };

    view! {
        <div class="variant-product-series-editor" style="margin-top: 12px; border: 1px solid var(--border-color); border-radius: 6px; padding: 10px;">
            <h4 style="margin: 0 0 6px; font-size: 13px;">{ "Product series" }</h4>
            <p style="font-size: 11px; color: var(--text-tertiary); margin: 0 0 8px;">
                { "Multi-SKU family this variant belongs to (e.g. \"Tria 2 Round\"). The id is required and stays stable; the localized name shows up in product catalogues." }
            </p>
            {move || rows.with(|rs| {
                if rs.is_empty() {
                    view! { <p class="empty-message">{ "No series." }</p> }.into_any()
                } else {
                    let entries: Vec<_> = rs.iter().enumerate().map(|(row_idx, r)| {
                        let id_value = r.id.clone();
                        let name_value = r.name.clone().unwrap_or_default();
                        let on_id = on_id.clone();
                        let on_name = on_name.clone();
                        let on_remove = on_remove.clone();
                        view! {
                            <div style="display: flex; flex-direction: column; gap: 4px; padding: 6px; border: 1px solid var(--border-color); border-radius: 4px;">
                                <div style="display: flex; align-items: center; gap: 6px;">
                                    <label style="font-size: 11px; color: var(--text-secondary); min-width: 30px;">{ "id:" }</label>
                                    <input
                                        type="text"
                                        prop:value=id_value
                                        on:change=move |e| on_id(row_idx, event_value(&e).unwrap_or_default())
                                        style="flex: 1; font-family: var(--font-mono); font-size: 12px;"
                                        data-testid=format!("variant-series-id-{}-{}", idx, row_idx)
                                    />
                                    <button type="button" title="Remove series" on:click=move |_| on_remove(row_idx)>{ "✕" }</button>
                                </div>
                                <LocaleFooEditor label="Name" value=name_value on_change=move |loc| on_name(row_idx, loc) />
                            </div>
                        }
                    }).collect();
                    view! { <div style="display: flex; flex-direction: column; gap: 6px;">{entries}</div> }.into_any()
                }
            })}
            <button type="button" on:click=on_add data-testid=format!("variant-series-add-{}", idx) style="margin-top: 8px;">{ "+ Add series" }</button>
        </div>
    }
}
