//! Graphical 2D plan-view picker for assigning L3D Light Emitting
//! Objects (LEOs / "Lichtaustritte") to photometries (LVKs) on a
//! variant.
//!
//! Renders every LEO of the variant's L3D as a clickable SVG footprint
//! (circle or rectangle) at its plan-view position. Click a LEO →
//! highlight → pick an Emitter (canonical) or a Photometry (convenience,
//! resolves-or-creates an emitter). Each assignment merges into the
//! variant's `ModelGeometryReference` bindings via
//! `SetVariantGeometryModel` — the LEO's `part_name` becomes the
//! binding's `external_name` (the gap the old text-only editor left
//! hard-coded to "Primary").
//!
//! Everything is read reactively from `use_gldf()` given `idx` (variant
//! index) + `geometry_id`, so the host (`ModelGeometryEditor`) just
//! mounts `<LeoPlanPicker idx geometry_id .../>` and the SVG re-colours
//! itself after every dispatch.

use leptos::prelude::*;

use crate::components::pages::bevy_viewer::resolve_l3d_bytes_for_geometry;
use crate::state::use_gldf;
use gldf_rs::LeoShape;
use gldf_rs_editor::action::ModelEmitterBinding;
use gldf_rs_editor::GldfAction;

/// One choosable photometry: its id, display file name, and the emitter
/// (if any) that already references it. `emitter_id == None` means
/// "no emitter uses this photometry yet" → picking it auto-creates one.
#[derive(Clone, Debug)]
struct PhotoChoice {
    photometry_id: String,
    file_name: String,
    emitter_id: Option<String>,
}

/// Merge a single LEO's binding into the existing list, preserving every
/// other LEO. `emitter_id == None` clears this LEO's binding.
fn upsert_binding(
    mut bindings: Vec<ModelEmitterBinding>,
    leo_name: &str,
    emitter_id: Option<String>,
) -> Vec<ModelEmitterBinding> {
    bindings.retain(|b| b.external_name != leo_name);
    if let Some(eid) = emitter_id {
        bindings.push(ModelEmitterBinding {
            emitter_id: eid,
            external_name: leo_name.to_string(),
            target_model_type: Some("l3d".to_string()),
        });
    }
    bindings
}

/// Truncate `s` to at most `max` characters, appending `…` when cut.
/// Counts by `char` (not bytes) so multibyte file names stay intact.
fn ellipsize(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let keep = max.saturating_sub(1);
    let mut out: String = s.chars().take(keep).collect();
    out.push('…');
    out
}

#[component]
pub fn LeoPlanPicker(idx: usize, geometry_id: String) -> impl IntoView {
    let state = use_gldf();
    // Currently-selected LEO part_name (the clicked footprint).
    let selected = RwSignal::new(None::<String>);

    // --- Reactive data, derived from state + the geometry id ---
    let geom_for_leos = geometry_id.clone();
    // LEO footprints from the variant's L3D (recomputed if the embedded
    // bytes change). Stable per (product, geometry).
    let leos = Signal::derive(move || {
        state.with(|s| {
            resolve_l3d_bytes_for_geometry(&s.product, &s.embedded_files, &geom_for_leos)
                .map(|bytes| gldf_rs::extract_leos(&bytes).unwrap_or_default())
                .unwrap_or_default()
        })
    });

    // Current bindings for this variant's model geometry (leo -> emitter).
    let bindings = Signal::derive(move || {
        state.with(|s| {
            s.product
                .product_definitions
                .variants
                .as_ref()
                .and_then(|vs| vs.variant.get(idx))
                .and_then(|v| v.geometry.as_ref())
                .and_then(|g| g.model_geometry_reference.as_ref())
                .map(|m| {
                    m.emitter_reference
                        .iter()
                        .map(|e| {
                            (
                                e.emitter_object_external_name.clone(),
                                e.emitter_id.clone(),
                            )
                        })
                        .collect::<Vec<(String, String)>>()
                })
                .unwrap_or_default()
        })
    });

    // Per-LEO assigned photometry FILE name (for the footprint label),
    // via the same resolver the Bevy page uses.
    let leo_photometry = Signal::derive(move || {
        state.with(|s| {
            let variant_id = s
                .product
                .product_definitions
                .variants
                .as_ref()
                .and_then(|vs| vs.variant.get(idx))
                .map(|v| v.id.clone())
                .unwrap_or_default();
            let data = gldf_rs::mapping::get_variant_emitter_data(&s.product, &variant_id);
            data.emitters
                .into_iter()
                .filter_map(|e| e.ldt_file_name.map(|f| (e.leo_name, f)))
                .collect::<std::collections::HashMap<String, String>>()
        })
    });

    // Emitter options: (id, name-hint).
    let emitters = Signal::derive(move || {
        state.with(|s| {
            s.product
                .general_definitions
                .emitters
                .as_ref()
                .map(|es| {
                    es.emitter
                        .iter()
                        .map(|e| {
                            let hint = e
                                .fixed_light_emitter
                                .first()
                                .and_then(|fle| fle.name.as_ref())
                                .and_then(|n| n.locale.first())
                                .map(|l| l.value.clone());
                            (e.id.clone(), hint)
                        })
                        .collect::<Vec<(String, Option<String>)>>()
                })
                .unwrap_or_default()
        })
    });

    // Fixed light source options: (id, name-hint) — fed to the inline
    // "＋ New emitter for this LEO" form.
    let light_sources = Signal::derive(move || {
        state.with(|s| {
            s.product
                .general_definitions
                .light_sources
                .as_ref()
                .map(|ls| {
                    ls.fixed_light_source
                        .iter()
                        .map(|l| {
                            let hint = l.name.locale.first().map(|loc| loc.value.clone());
                            (l.id.clone(), hint)
                        })
                        .collect::<Vec<(String, Option<String>)>>()
                })
                .unwrap_or_default()
        })
    });

    // Photometry choices: (id, file_name, emitter-that-references-it?).
    let photometries = Signal::derive(move || {
        state.with(|s| {
            let gd = &s.product.general_definitions;
            let file_name = |fid: &str| -> String {
                gd.files
                    .file
                    .iter()
                    .find(|f| f.id == fid)
                    .map(|f| f.file_name.clone())
                    .unwrap_or_else(|| fid.to_string())
            };
            gd.photometries
                .as_ref()
                .map(|ps| {
                    ps.photometry
                        .iter()
                        .map(|p| {
                            let fname = p
                                .photometry_file_reference
                                .as_ref()
                                .map(|r| file_name(&r.file_id))
                                .unwrap_or_else(|| p.id.clone());
                            // First emitter whose FixedLightEmitter references this photometry.
                            let eid = gd.emitters.as_ref().and_then(|es| {
                                es.emitter
                                    .iter()
                                    .find(|e| {
                                        e.fixed_light_emitter.iter().any(|fle| {
                                            fle.photometry_reference.photometry_id == p.id
                                        })
                                    })
                                    .map(|e| e.id.clone())
                            });
                            PhotoChoice {
                                photometry_id: p.id.clone(),
                                file_name: fname,
                                emitter_id: eid,
                            }
                        })
                        .collect::<Vec<PhotoChoice>>()
                })
                .unwrap_or_default()
        })
    });

    // --- Dispatch helpers (move closures capture state + ids) ---
    let geom_for_pick = geometry_id.clone();
    // Bind a LEO to an explicit emitter id.
    let bind_emitter = move |leo: String, emitter_id: String| {
        let geometry_id = geom_for_pick.clone();
        let cur: Vec<ModelEmitterBinding> = bindings
            .get_untracked()
            .into_iter()
            .map(|(ext, eid)| ModelEmitterBinding {
                emitter_id: eid,
                external_name: ext,
                target_model_type: Some("l3d".to_string()),
            })
            .collect();
        let next = upsert_binding(cur, &leo, Some(emitter_id));
        state.update(|s| {
            s.apply(GldfAction::SetVariantGeometryModel {
                variant_idx: idx,
                geometry_id,
                bindings: next,
            });
        });
    };

    let geom_for_clear = geometry_id.clone();
    let clear_binding = move |leo: String| {
        let geometry_id = geom_for_clear.clone();
        let cur: Vec<ModelEmitterBinding> = bindings
            .get_untracked()
            .into_iter()
            .map(|(ext, eid)| ModelEmitterBinding {
                emitter_id: eid,
                external_name: ext,
                target_model_type: Some("l3d".to_string()),
            })
            .collect();
        let next = upsert_binding(cur, &leo, None);
        state.update(|s| {
            s.apply(GldfAction::SetVariantGeometryModel {
                variant_idx: idx,
                geometry_id,
                bindings: next,
            });
        });
    };

    let geom_for_create = geometry_id.clone();
    // Create a brand-new Fixed emitter *for this LEO* (photometry + light
    // source + flux) and bind it — the inline "＋ New emitter" path. Same
    // single-`state.update` create-then-read-id pattern as `bind_photometry`
    // so the new emitter id is read back from state, never predicted.
    // `child_name = leo` makes the emitter "born for" this LEO.
    let create_and_bind_emitter =
        move |leo: String, photometry_id: String, light_source_id: String, flux: i32| {
            let geometry_id = geom_for_create.clone();
            let cur: Vec<ModelEmitterBinding> = bindings
                .get_untracked()
                .into_iter()
                .map(|(ext, eid)| ModelEmitterBinding {
                    emitter_id: eid,
                    external_name: ext,
                    target_model_type: Some("l3d".to_string()),
                })
                .collect();
            state.update(|s| {
                s.apply(GldfAction::AddFixedEmitter {
                    emitter_id: String::new(),
                    child_name: Some(leo.clone()),
                    photometry_id,
                    light_source_id,
                    rated_luminous_flux: flux,
                });
                let emitter_id = s
                    .product
                    .general_definitions
                    .emitters
                    .as_ref()
                    .and_then(|es| es.emitter.last())
                    .map(|e| e.id.clone())
                    .unwrap_or_default();
                let next = upsert_binding(cur, &leo, Some(emitter_id));
                s.apply(GldfAction::SetVariantGeometryModel {
                    variant_idx: idx,
                    geometry_id,
                    bindings: next,
                });
            });
        };

    // --- SVG viewBox from the LEO bounding box (Y-flipped to read
    //     grid layout) ---
    //
    // v1 lays the LEOs out on a clean uniform grid rather than at their
    // raw L3D positions: in an articulated luminaire the LEO `position`
    // is local to its joint subtree, and we don't compose joint
    // transforms yet — so raw positions can collapse multiple LEOs onto
    // the same point. A grid of labelled cards is always readable and
    // clickable regardless of geometry complexity. (Faithful
    // positioning via composed transforms is a later phase.)
    const CELL: f64 = 100.0; // grid cell size in SVG units
    let cols = Signal::derive(move || {
        let n = leos.get().len().max(1);
        // up to 3 per row, fewer when there are fewer LEOs
        n.min(3).max(1)
    });
    let view_box = Signal::derive(move || {
        let n = leos.get().len().max(1);
        let c = cols.get();
        let rows = n.div_ceil(c);
        format!("0 0 {} {}", c as f64 * CELL, rows as f64 * CELL)
    });

    let render_leos = move || {
        let ls = leos.get();
        let bnd = bindings.get();
        let photo = leo_photometry.get();
        let sel = selected.get();
        let c = cols.get();
        ls.into_iter()
            .enumerate()
            .map(|(i, l)| {
                let name = l.part_name.clone();
                let is_bound = bnd.iter().any(|(ext, _)| ext == &name);
                let is_sel = sel.as_deref() == Some(name.as_str());
                // Cell top-left.
                let col = (i % c) as f64;
                let row = (i / c) as f64;
                let x0 = col * CELL;
                let y0 = row * CELL;
                let cx = x0 + CELL / 2.0;
                let glyph_cy = y0 + CELL * 0.40; // shape centred upper-middle
                let r = CELL * 0.22;
                let fill = if is_bound {
                    "var(--accent-blue, #3b82f6)"
                } else {
                    "transparent"
                };
                let stroke = if is_sel {
                    "var(--accent-orange, #f59e0b)"
                } else if is_bound {
                    "var(--accent-blue, #3b82f6)"
                } else {
                    "var(--text-tertiary, #888)"
                };
                let stroke_w = if is_sel { 4.0 } else { 2.0 };
                let dash = if is_bound { "0" } else { "6" };
                let label2 = if is_bound {
                    photo.get(&name).cloned().unwrap_or_else(|| "—".to_string())
                } else {
                    "unassigned".to_string()
                };
                let name_for_click = name.clone();
                // Draw a circle for Circle/Point, a square for Rectangle —
                // proportional, NOT to scale (sizes are tiny in metres).
                let shape = match l.shape {
                    LeoShape::Rectangle { .. } => {
                        let s = r * 1.6;
                        view! {
                            <rect x=cx - s / 2.0 y=glyph_cy - s / 2.0
                                width=s height=s rx=4.0
                                fill=fill stroke=stroke stroke-width=stroke_w
                                stroke-dasharray=dash style="cursor: pointer;" />
                        }
                        .into_any()
                    }
                    _ => view! {
                        <circle cx=cx cy=glyph_cy r=r
                            fill=fill stroke=stroke stroke-width=stroke_w
                            stroke-dasharray=dash style="cursor: pointer;" />
                    }
                    .into_any(),
                };
                let name_y = y0 + CELL * 0.74;
                let label_y = y0 + CELL * 0.88;
                view! {
                    <g data-testid=format!("leo-{}-{}", idx, name)
                        on:click=move |_| selected.set(Some(name_for_click.clone()))
                        style="cursor: pointer;">
                        // transparent cell hit-area so the whole card is clickable
                        <rect x=x0 + 4.0 y=y0 + 4.0 width=CELL - 8.0 height=CELL - 8.0
                            rx=6.0 fill="transparent"
                            stroke=if is_sel { "var(--accent-orange, #f59e0b)" } else { "var(--border-color, #333)" }
                            stroke-width=1.0 />
                        {shape}
                        <text x=cx y=name_y text-anchor="middle" font-size="12"
                            fill="var(--text-primary, #ddd)"
                            style="pointer-events: none; user-select: none; font-family: var(--font-mono); font-weight: 600;">
                            { name.clone() }
                        </text>
                        <text x=cx y=label_y text-anchor="middle" font-size="9"
                            fill=if is_bound { "var(--accent-blue, #3b82f6)" } else { "var(--text-tertiary, #888)" }
                            style="pointer-events: none; user-select: none; font-family: var(--font-mono);">
                            { ellipsize(&label2, 18) }
                        </text>
                    </g>
                }
                .into_any()
            })
            .collect_view()
    };

    // --- Picker panel for the selected LEO ---
    let picker_panel = move || {
        let Some(leo) = selected.get() else {
            return ().into_any();
        };
        let leo_for_em = leo.clone();
        let leo_for_create = leo.clone();
        let leo_for_clear = leo.clone();
        let current_emitter = bindings
            .get()
            .into_iter()
            .find(|(ext, _)| ext == &leo)
            .map(|(_, eid)| eid)
            .unwrap_or_default();
        // Separate copy for the "auto-open the New-emitter panel when this
        // LEO is still unbound" check (current_emitter is moved into the
        // emitter <option> closure below).
        let has_emitter = !current_emitter.is_empty();
        let bind_emitter = bind_emitter.clone();
        let create_and_bind_emitter = create_and_bind_emitter.clone();
        let clear_binding = clear_binding.clone();
        let photos = photometries.get();
        let lss = light_sources.get();

        // Local form state for the "＋ New emitter for this LEO" panel,
        // seeded from the first photometry / light source. Recreated each
        // time a different LEO is selected (the panel re-renders), so the
        // defaults always reflect the current catalog.
        let new_photometry = RwSignal::new(
            photos.first().map(|c| c.photometry_id.clone()).unwrap_or_default(),
        );
        let new_light_source = RwSignal::new(
            lss.first().map(|(id, _)| id.clone()).unwrap_or_default(),
        );
        let new_flux = RwSignal::new(1600_i32);
        let can_create = Signal::derive(move || {
            !new_photometry.get().is_empty() && !new_light_source.get().is_empty()
        });

        let on_emitter = {
            let leo = leo_for_em.clone();
            move |e: crate::components::edit_util::EditEvent| {
                let v = event_target_value(&e);
                if !v.is_empty() {
                    bind_emitter(leo.clone(), v);
                }
            }
        };
        let on_create = {
            let leo = leo_for_create.clone();
            move |_| {
                let pid = new_photometry.get();
                let lid = new_light_source.get();
                if pid.is_empty() || lid.is_empty() {
                    return;
                }
                create_and_bind_emitter(leo.clone(), pid, lid, new_flux.get());
            }
        };
        let on_clear = move |_| clear_binding(leo_for_clear.clone());

        view! {
            <div style="margin-top: 8px; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; font-size: 12px;">
                <div style="font-weight: 600; margin-bottom: 6px;">
                    { format!("Assign to {leo}") }
                </div>
                <div style="display: grid; grid-template-columns: 90px 1fr; gap: 6px; align-items: center;">
                    <label>{ "Emitter" }</label>
                    <select on:change=on_emitter
                        data-testid=format!("leo-emitter-select-{}", idx)
                        style="font-size: 11px; font-family: var(--font-mono);">
                        <option value="">{ "(pick emitter…)" }</option>
                        {move || emitters.get().into_iter().map(|(eid, hint)| {
                            let sel = eid == current_emitter;
                            let label = match hint {
                                Some(n) if !n.is_empty() => format!("{eid} ({n})"),
                                _ => eid.clone(),
                            };
                            view! { <option value=eid.clone() selected=sel>{ label }</option> }
                        }).collect::<Vec<_>>()}
                    </select>
                </div>
                // The canonical binding is "pick an existing Emitter" above.
                // When no emitter fits yet, this inline form CREATES a Fixed
                // emitter *for this LEO* (photometry + light source + flux,
                // child_name = the LEO) and binds it in one step — closing the
                // old two-page "go make an emitter, come back, bind it" dance.
                <details style="margin-top: 6px;" open=!has_emitter && photos.iter().any(|c| c.emitter_id.is_none())>
                    <summary style="cursor: pointer; color: var(--text-secondary, #aaa);">
                        { format!("＋ New emitter for {leo}…") }
                    </summary>
                    {if photos.is_empty() {
                        view! {
                            <div style="margin-top: 6px; font-size: 10px; color: var(--text-tertiary, #888);">
                                { "No photometries yet — add an LDT/IES on the Photometry \
                                   or Files page first, then come back to create an emitter." }
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div style="margin-top: 6px; display: grid; grid-template-columns: 90px 1fr; gap: 6px; align-items: center;">
                                <label>{ "Photometry" }</label>
                                <select
                                    data-testid=format!("leo-new-photometry-{}", idx)
                                    on:change=move |e| new_photometry.set(event_target_value(&e))
                                    style="font-size: 11px; font-family: var(--font-mono);">
                                    {photos.iter().map(|c| {
                                        let sel = c.photometry_id == new_photometry.get_untracked();
                                        view! { <option value=c.photometry_id.clone() selected=sel>{ c.file_name.clone() }</option> }
                                    }).collect::<Vec<_>>()}
                                </select>
                                <label>{ "Light source" }</label>
                                {if lss.is_empty() {
                                    view! {
                                        <span style="font-size: 10px; color: var(--accent-orange, #f59e0b);">
                                            { "none — add a Light Source first" }
                                        </span>
                                    }.into_any()
                                } else {
                                    view! {
                                        <select
                                            data-testid=format!("leo-new-lightsource-{}", idx)
                                            on:change=move |e| new_light_source.set(event_target_value(&e))
                                            style="font-size: 11px; font-family: var(--font-mono);">
                                            {lss.iter().map(|(id, hint)| {
                                                let sel = *id == new_light_source.get_untracked();
                                                let label = match hint {
                                                    Some(n) if !n.is_empty() => format!("{id} ({n})"),
                                                    _ => id.clone(),
                                                };
                                                view! { <option value=id.clone() selected=sel>{ label }</option> }
                                            }).collect::<Vec<_>>()}
                                        </select>
                                    }.into_any()
                                }}
                                <label>{ "Flux (lm)" }</label>
                                <input type="number" min="0" step="50"
                                    data-testid=format!("leo-new-flux-{}", idx)
                                    prop:value=move || new_flux.get().to_string()
                                    on:input=move |e| {
                                        if let Ok(v) = event_target_value(&e).parse::<i32>() {
                                            new_flux.set(v);
                                        }
                                    }
                                    style="font-size: 11px; font-family: var(--font-mono); width: 80px;" />
                            </div>
                            <button type="button" class="btn btn-primary"
                                data-testid=format!("leo-create-emitter-{}", idx)
                                style="margin-top: 6px; font-size: 11px;"
                                disabled=move || !can_create.get()
                                on:click=on_create>
                                { format!("Create & bind to {leo}") }
                            </button>
                        }.into_any()
                    }}
                </details>
                <button type="button" class="btn btn-secondary"
                    style="margin-top: 6px; font-size: 11px;"
                    on:click=on_clear>{ "Clear this LEO" }</button>
            </div>
        }
        .into_any()
    };

    view! {
        <div data-testid=format!("leo-plan-picker-{}", idx)>
            <strong>{ "Light emitting objects (click to assign an emitter)" }</strong>
            <div style="margin-top: 6px; background: var(--bg-primary, #111); border: 1px solid var(--border-color); border-radius: 4px; padding: 8px;">
                <svg viewBox=move || view_box.get()
                    preserveAspectRatio="xMidYMid meet"
                    style="width: 100%; max-width: 420px; height: auto; aspect-ratio: 1; display: block; margin: 0 auto;">
                    {render_leos}
                </svg>
            </div>
            {picker_panel}
        </div>
    }
}
