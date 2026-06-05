//! Marketing editor — HousingColors (RAL + locale name), Markets (Region
//! locales), ApprovalMarks (strings), DesignAwards (strings).
//!
//! Scope-aware via the shared `ScopePicker`: edits either the
//! product-wide `Marketing` block or a specific variant's override.
//!
//! All four lists are typed structs on `Marketing` in the lib; this page
//! is pure editor surface that dispatches the matching `GldfAction`.

use crate::components::edit_util::event_value;
use crate::components::ScopePicker;
use crate::state::{use_active_scope, use_gldf, GldfAction, Scope};
use gldf_rs::gldf::product_definitions::{
    ApprovalMarks, DesignAwards, HousingColors, Markets,
};
use leptos::prelude::*;

#[component]
pub fn MarketingEditorPage() -> impl IntoView {
    let scope = use_active_scope();

    view! {
        <div class="editor-section marketing-editor">
            <h2>{ "Marketing" }</h2>
            <p class="section-description">
                { "Housing colors (RAL + name), target markets, and design awards. All scope-aware — variants may override the product-wide defaults. Approval marks have moved to their own sidebar entry (✅ Approval Marks) so the Labels/ApprovalMarks XSD overlap is explained in one place." }
            </p>

            <ScopePicker scope=scope />

            <HousingColorsSection scope=scope />
            <MarketsSection scope=scope />
            <DesignAwardsSection scope=scope />

            <p style="font-size: 11px; color: var(--text-tertiary); margin-top: 16px;">
                { "Other Marketing fields (designer, list prices, hyperlinks) are edited on their own pages — Hyperlinks has its own sidebar entry, and List Prices/Designer are part of the Marketing schema but not yet exposed." }
            </p>
        </div>
    }
}

fn read_housing_colors(s: &crate::state::GldfState, scope: Scope) -> Vec<(Option<i32>, String, String)> {
    let marketing = match scope {
        Scope::Product => s
            .product
            .product_definitions
            .product_meta_data
            .as_ref()
            .and_then(|m| m.descriptive_attributes.as_ref())
            .and_then(|a| a.marketing.as_ref()),
        Scope::Variant(idx) => s
            .product
            .product_definitions
            .variants
            .as_ref()
            .and_then(|vs| vs.variant.get(idx))
            .and_then(|v| v.descriptive_attributes.as_ref())
            .and_then(|a| a.marketing.as_ref()),
    };
    marketing
        .and_then(|m| m.housing_colors.as_ref())
        .map(|hc| {
            hc.housing_color
                .iter()
                .map(|h| (h.ral, h.locale.language.clone(), h.locale.value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

fn read_markets(s: &crate::state::GldfState, scope: Scope) -> Vec<(String, String)> {
    let marketing = match scope {
        Scope::Product => s
            .product
            .product_definitions
            .product_meta_data
            .as_ref()
            .and_then(|m| m.descriptive_attributes.as_ref())
            .and_then(|a| a.marketing.as_ref()),
        Scope::Variant(idx) => s
            .product
            .product_definitions
            .variants
            .as_ref()
            .and_then(|vs| vs.variant.get(idx))
            .and_then(|v| v.descriptive_attributes.as_ref())
            .and_then(|a| a.marketing.as_ref()),
    };
    marketing
        .and_then(|m| m.markets.as_ref())
        .map(|mk| {
            mk.region
                .iter()
                .map(|r| (r.language.clone(), r.value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn read_approval_marks(s: &crate::state::GldfState, scope: Scope) -> Vec<String> {
    let marketing = match scope {
        Scope::Product => s
            .product
            .product_definitions
            .product_meta_data
            .as_ref()
            .and_then(|m| m.descriptive_attributes.as_ref())
            .and_then(|a| a.marketing.as_ref()),
        Scope::Variant(idx) => s
            .product
            .product_definitions
            .variants
            .as_ref()
            .and_then(|vs| vs.variant.get(idx))
            .and_then(|v| v.descriptive_attributes.as_ref())
            .and_then(|a| a.marketing.as_ref()),
    };
    marketing
        .and_then(|m| m.approval_marks.as_ref())
        .map(|am| am.approval_mark.clone())
        .unwrap_or_default()
}

fn read_design_awards(s: &crate::state::GldfState, scope: Scope) -> Vec<String> {
    let marketing = match scope {
        Scope::Product => s
            .product
            .product_definitions
            .product_meta_data
            .as_ref()
            .and_then(|m| m.descriptive_attributes.as_ref())
            .and_then(|a| a.marketing.as_ref()),
        Scope::Variant(idx) => s
            .product
            .product_definitions
            .variants
            .as_ref()
            .and_then(|vs| vs.variant.get(idx))
            .and_then(|v| v.descriptive_attributes.as_ref())
            .and_then(|a| a.marketing.as_ref()),
    };
    marketing
        .and_then(|m| m.design_awards.as_ref())
        .map(|da| da.design_award.clone())
        .unwrap_or_default()
}

#[component]
fn HousingColorsSection(scope: RwSignal<Scope>) -> impl IntoView {
    let state = use_gldf();
    let on_add = move |_| {
        let scp = scope.get();
        state.update(|s| s.apply(GldfAction::AddHousingColor(scp)));
    };

    view! {
        <div style="margin-top: 16px;">
            <h3 style="margin-bottom: 8px;">{ "🎨 Housing colors" }</h3>
            <p style="font-size: 11px; color: var(--text-secondary); margin-bottom: 8px;">
                { "Each entry is a RAL number plus an optional localized name (e.g. \"white\" in en, \"weiß\" in de). RAL is a 4-digit integer — 9016 for traffic white, 9005 for jet black. Type a RAL name in either language and the number snaps to match." }
            </p>
            <RalNamesDatalist />
            {move || state.with(|s| {
                let entries = read_housing_colors(s, scope.get());
                if entries.is_empty() {
                    view! { <p class="empty-message">{ "No housing colors." }</p> }.into_any()
                } else {
                    view! {
                        <div style="display: flex; flex-direction: column; gap: 6px;">
                            {entries.into_iter().enumerate().map(|(idx, (ral, lang, value))| view! {
                                <HousingColorRow idx=idx ral=ral language=lang value=value scope=scope />
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            })}
            <button type="button" on:click=on_add data-testid="marketing-add-housing-color" style="margin-top: 8px;">{ "+ Add housing color" }</button>
        </div>
    }
}

/// One housing-color row. Centerpiece is a RAL Classic dropdown where
/// every `<option>` is painted with the actual sRGB of its RAL entry,
/// so the user picks a colour by sight, not by memorising a number.
///
/// Selecting a swatch fills both the RAL number AND the localized
/// name in one shot — that's what dispatches `UpdateHousingColor`,
/// which in turn rewrites the L3D's housing material, which in turn
/// lights up the 3D viewer with the new colour (hot-reloaded thanks
/// to `L3dPanel`'s new effect).
///
/// The free-text "name" field is kept as an override for users who
/// want a custom locale string (e.g. marketing copy: "korallenrot
/// (Sonderfarbe)" instead of the official "Korallenrot"). Same goes
/// for the language code — pre-filled to the current UI language but
/// editable so the user can attach a name in any locale they want.
#[component]
fn HousingColorRow(
    idx: usize,
    ral: Option<i32>,
    language: String,
    value: String,
    scope: RwSignal<Scope>,
) -> impl IntoView {
    let state = use_gldf();
    let ui_language = state.with(|s| s.ui_language.clone());
    let s_ral = RwSignal::new(ral.unwrap_or(0));
    let s_lang = RwSignal::new(if language.is_empty() {
        ui_language.clone()
    } else {
        language
    });
    let s_value = RwSignal::new(value.clone());

    let dispatch = move || {
        let scp = scope.get();
        let ral = if s_ral.get() == 0 { None } else { Some(s_ral.get()) };
        state.update(|s| {
            s.apply(GldfAction::UpdateHousingColor {
                scope: scp,
                index: idx,
                ral,
                language: s_lang.get(),
                value: s_value.get(),
            });
        });
    };

    // When the user picks a RAL from the dropdown, fill BOTH the RAL
    // number AND the locale name from the table entry. The user can
    // still override the name afterwards via the free-text field.
    let on_pick_ral = {
        let dispatch = dispatch.clone();
        let ui_language = ui_language.clone();
        move |e: web_sys::Event| {
            let raw = event_value(&e).unwrap_or_default();
            let n: i32 = raw.parse().unwrap_or(0);
            s_ral.set(n);
            if let Some(entry) = gldf_rs::ral::by_number(n) {
                // Auto-fill the localized name only when the field is
                // empty OR matches the previous RAL's name (so user
                // edits aren't clobbered).
                let new_name = entry.fallback_name(&ui_language).to_string();
                let cur = s_value.get();
                let was_default = cur.is_empty()
                    || cur == entry.name_en
                    || cur == entry.name_de
                    || gldf_rs::ral::by_name(&cur).is_some();
                if was_default {
                    s_value.set(new_name);
                }
            }
            dispatch();
        }
    };
    let on_lang = {
        let dispatch = dispatch.clone();
        move |e: web_sys::Event| {
            s_lang.set(event_value(&e).unwrap_or_default());
            dispatch();
        }
    };
    // Reverse direction of the bidirectional binding: when the user
    // types (or picks from the datalist below) a RAL name, see if it
    // resolves via the lib's fuzzy `by_name`. If it does, snap the RAL
    // number to match so the swatch + dropdown follow the name. The
    // user can still type a custom marketing name (e.g. "korallenrot
    // (Sonderfarbe)") — `by_name` simply returns None and the RAL
    // number is left alone.
    let on_value = {
        let dispatch = dispatch.clone();
        move |e: web_sys::Event| {
            let typed = event_value(&e).unwrap_or_default();
            s_value.set(typed.clone());
            if let Some(entry) = gldf_rs::ral::by_name(&typed) {
                if s_ral.get() != entry.number {
                    s_ral.set(entry.number);
                }
            }
            dispatch();
        }
    };
    let on_remove = move |_| {
        let scp = scope.get();
        state.update(|s| s.apply(GldfAction::RemoveHousingColor { scope: scp, index: idx }));
    };

    // sRGB swatch chip showing the *currently selected* RAL colour
    // (or a neutral border if nothing is picked). Re-renders whenever
    // s_ral changes so the user has immediate visual confirmation.
    let swatch_style = move || {
        let n = s_ral.get();
        if let Some(e) = gldf_rs::ral::by_number(n) {
            format!(
                "width: 24px; height: 24px; border-radius: 4px; border: 1px solid var(--border-color); background: {}; flex-shrink: 0;",
                e.hex()
            )
        } else {
            "width: 24px; height: 24px; border-radius: 4px; border: 1px dashed var(--border-color); background: transparent; flex-shrink: 0;".to_string()
        }
    };

    view! {
        <div style="display: flex; gap: 8px; align-items: center; border: 1px solid var(--border-color); border-radius: 6px; padding: 8px;">
            <div style=swatch_style></div>
            <select
                on:change=on_pick_ral
                prop:value=move || s_ral.get().to_string()
                style="font-family: var(--font-mono); font-size: 12px; padding: 4px 6px; min-width: 240px;"
                title="Pick a RAL Classic colour"
                data-testid=format!("marketing-ral-select-{}", idx)
            >
                <option value="0" selected=move || s_ral.get() == 0>{ "— select RAL —" }</option>
                {gldf_rs::ral::RAL_CLASSIC.iter().map(|e| {
                    let n = e.number;
                    let selected = s_ral.get() == n;
                    let name = e.fallback_name(&ui_language);
                    let label = format!("RAL {}  ·  {}", n, name);
                    let bg = e.hex();
                    // Pick black or white text for contrast based on luma.
                    let luma = 0.299 * e.rgb[0] as f32 + 0.587 * e.rgb[1] as f32 + 0.114 * e.rgb[2] as f32;
                    let fg = if luma > 140.0 { "#000" } else { "#fff" };
                    let opt_style = format!("background: {}; color: {};", bg, fg);
                    view! {
                        <option value=n.to_string() selected=selected style=opt_style>
                            { label }
                        </option>
                    }
                }).collect::<Vec<_>>()}
            </select>
            <label style="font-size: 11px; color: var(--text-secondary);">{ "lang:" }</label>
            <input type="text" maxlength="5" placeholder="en" prop:value=s_lang.get() on:change=on_lang style="width: 60px;" />
            <label style="font-size: 11px; color: var(--text-secondary);">{ "name:" }</label>
            <input
                type="text"
                placeholder="(auto from RAL — or type a RAL name)"
                prop:value=s_value.get()
                on:change=on_value
                style="flex: 1;"
                list="gldf-ral-names"
                title="Type a RAL Classic colour name (DE or EN) to autofill the RAL number."
            />
            <button type="button" title="Remove" on:click=on_remove>{ "✕" }</button>
        </div>
    }
}

/// Hidden `<datalist>` populating an autocomplete dropdown for every
/// HousingColor row's name field. Lives at the top of the section so
/// the same id (`gldf-ral-names`) is shared by every row's `<input
/// list="…">`. Each option carries the canonical English name; the
/// localized German name lands in the `data-de` attribute as a label
/// hint, but `by_name`'s fuzzy lookup means either language resolves.
#[component]
fn RalNamesDatalist() -> impl IntoView {
    view! {
        <datalist id="gldf-ral-names">
            {gldf_rs::ral::RAL_CLASSIC.iter().flat_map(|e| {
                let n = e.number;
                // One option per language so the dropdown surfaces both
                // names. `by_name` normalises whitespace + case so typing
                // either resolves to the same entry.
                vec![
                    view! {
                        <option value=e.name_en>{ format!("RAL {} · DE: {}", n, e.name_de) }</option>
                    },
                    view! {
                        <option value=e.name_de>{ format!("RAL {} · EN: {}", n, e.name_en) }</option>
                    },
                ]
            }).collect::<Vec<_>>()}
        </datalist>
    }
}

#[component]
fn MarketsSection(scope: RwSignal<Scope>) -> impl IntoView {
    let state = use_gldf();
    let on_add = move |_| {
        let scp = scope.get();
        state.update(|s| s.apply(GldfAction::AddMarketRegion(scp)));
    };

    view! {
        <div style="margin-top: 16px;">
            <h3 style="margin-bottom: 8px;">{ "🌍 Markets" }</h3>
            <p style="font-size: 11px; color: var(--text-secondary); margin-bottom: 8px;">
                { "Target market regions per locale (e.g. \"EU\" in en, \"Europe\" in de)." }
            </p>
            {move || state.with(|s| {
                let entries = read_markets(s, scope.get());
                if entries.is_empty() {
                    view! { <p class="empty-message">{ "No markets." }</p> }.into_any()
                } else {
                    view! {
                        <div style="display: flex; flex-direction: column; gap: 6px;">
                            {entries.into_iter().enumerate().map(|(idx, (lang, value))| view! {
                                <MarketRow idx=idx language=lang value=value scope=scope />
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            })}
            <button type="button" on:click=on_add style="margin-top: 8px;">{ "+ Add market" }</button>
        </div>
    }
}

#[component]
fn MarketRow(
    idx: usize,
    language: String,
    value: String,
    scope: RwSignal<Scope>,
) -> impl IntoView {
    let state = use_gldf();
    let s_lang = RwSignal::new(language.clone());
    let s_value = RwSignal::new(value.clone());

    let on_change = move || {
        let scp = scope.get();
        state.update(|s| {
            s.apply(GldfAction::UpdateMarketRegion {
                scope: scp,
                index: idx,
                language: s_lang.get(),
                value: s_value.get(),
            });
        });
    };
    let on_lang = {
        let on_change = on_change.clone();
        move |e: web_sys::Event| {
            let v = event_value(&e).unwrap_or_default();
            s_lang.set(v);
            on_change();
        }
    };
    let on_value = {
        let on_change = on_change.clone();
        move |e: web_sys::Event| {
            let v = event_value(&e).unwrap_or_default();
            s_value.set(v);
            on_change();
        }
    };
    let on_remove = move |_| {
        let scp = scope.get();
        state.update(|s| s.apply(GldfAction::RemoveMarketRegion { scope: scp, index: idx }));
    };

    view! {
        <div style="display: flex; gap: 6px; align-items: center; border: 1px solid var(--border-color); border-radius: 6px; padding: 6px;">
            <label style="font-size: 11px; color: var(--text-secondary);">{ "lang:" }</label>
            <input type="text" maxlength="5" placeholder="en" prop:value=s_lang.get() on:change=on_lang style="width: 60px;" />
            <label style="font-size: 11px; color: var(--text-secondary);">{ "region:" }</label>
            <input type="text" placeholder="EU" prop:value=s_value.get() on:change=on_value style="flex: 1;" />
            <button type="button" title="Remove" on:click=on_remove>{ "✕" }</button>
        </div>
    }
}

// ApprovalMarksSection moved to LabelsEditorPage (the merged
// "Approval Marks" sidebar entry). See labels_editor.rs.

#[component]
fn DesignAwardsSection(scope: RwSignal<Scope>) -> impl IntoView {
    let state = use_gldf();
    let on_add = move |_| {
        let scp = scope.get();
        state.update(|s| s.apply(GldfAction::AddDesignAward(scp)));
    };

    view! {
        <div style="margin-top: 16px;">
            <h3 style="margin-bottom: 8px;">{ "🏆 Design awards" }</h3>
            <p style="font-size: 11px; color: var(--text-secondary); margin-bottom: 8px;">
                { "Awards the luminaire has received (e.g. Red Dot 2024, iF Design Award)." }
            </p>
            {move || state.with(|s| {
                let entries = read_design_awards(s, scope.get());
                if entries.is_empty() {
                    view! { <p class="empty-message">{ "No design awards." }</p> }.into_any()
                } else {
                    view! {
                        <div style="display: flex; flex-direction: column; gap: 6px;">
                            {entries.into_iter().enumerate().map(|(idx, value)| view! {
                                <StringListRow
                                    idx=idx
                                    value=value
                                    scope=scope
                                    on_update=Box::new(|scope, idx, value| GldfAction::UpdateDesignAward { scope, index: idx, value })
                                    on_remove=Box::new(|scope, idx| GldfAction::RemoveDesignAward { scope, index: idx })
                                    placeholder="Red Dot 2024"
                                />
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            })}
            <button type="button" on:click=on_add style="margin-top: 8px;">{ "+ Add design award" }</button>
        </div>
    }
}

#[component]
fn StringListRow(
    idx: usize,
    value: String,
    scope: RwSignal<Scope>,
    on_update: Box<dyn Fn(Scope, usize, String) -> GldfAction + Send + Sync>,
    on_remove: Box<dyn Fn(Scope, usize) -> GldfAction + Send + Sync>,
    placeholder: &'static str,
) -> impl IntoView {
    let state = use_gldf();
    let s_value = RwSignal::new(value.clone());
    let update_arc = std::sync::Arc::new(on_update);
    let remove_arc = std::sync::Arc::new(on_remove);

    let on_input = {
        let update_arc = update_arc.clone();
        move |e: web_sys::Event| {
            let v = event_value(&e).unwrap_or_default();
            s_value.set(v.clone());
            let scp = scope.get();
            let action = update_arc(scp, idx, v);
            state.update(|s| s.apply(action));
        }
    };
    let on_click_remove = {
        let remove_arc = remove_arc.clone();
        move |_| {
            let scp = scope.get();
            let action = remove_arc(scp, idx);
            state.update(|s| s.apply(action));
        }
    };

    view! {
        <div style="display: flex; gap: 6px; align-items: center; border: 1px solid var(--border-color); border-radius: 6px; padding: 6px;">
            <input type="text" placeholder=placeholder prop:value=s_value.get() on:change=on_input style="flex: 1;" />
            <button type="button" title="Remove" on:click=on_click_remove>{ "✕" }</button>
        </div>
    }
}

// Unused imports the compiler might warn about — silence by re-exporting
// the typed structs so feature consumers can import them from one place.
#[allow(dead_code)]
fn _types_marker(_: HousingColors, _: Markets, _: ApprovalMarks, _: DesignAwards) {}
