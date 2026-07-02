//! Geometries page — author SimpleGeometry primitives (Cuboid/Cylinder)
//! that variants can bind via SimpleGeometryReference. ModelGeometry
//! entries (.l3d, .m3d, .r3d) are registered by the Files page on drop;
//! this page lists them read-only alongside the editable simple primitives.

use crate::components::edit_util::event_i32;
use crate::state::{use_gldf, GldfAction};
use leptos::prelude::*;

#[component]
pub fn GeometriesPage(#[prop(default = false)] editable: bool) -> impl IntoView {
    let state = use_gldf();

    // Pending form fields for the "+ Add Cuboid" / "+ Add Cylinder"
    // affordances. Reset to defaults after a successful dispatch.
    let cuboid_w = RwSignal::new(600);
    let cuboid_l = RwSignal::new(600);
    let cuboid_h = RwSignal::new(100);
    let cyl_d = RwSignal::new(200);
    let cyl_h = RwSignal::new(100);

    let on_add_cuboid = move |_| {
        let w = cuboid_w.get();
        let l = cuboid_l.get();
        let h = cuboid_h.get();
        state.update(|s| {
            s.apply(GldfAction::AddSimpleGeometryCuboid {
                width_mm: w,
                length_mm: l,
                height_mm: h,
            });
        });
    };
    let on_add_cylinder = move |_| {
        let d = cyl_d.get();
        let h = cyl_h.get();
        state.update(|s| {
            s.apply(GldfAction::AddSimpleGeometryCylinder {
                plane: None,
                diameter_mm: d,
                height_mm: h,
            });
        });
    };

    view! {
        <div class="editor-section geometries-page">
            <h2>{ "Geometries" }</h2>
            <p class="section-description">
                { "Reusable shape definitions. Variants bind these by id via SimpleGeometryReference. Cuboid + Cylinder primitives are authored here; ModelGeometry entries (.l3d) are registered automatically by the Files page when you drop a .l3d file." }
            </p>

            {move || state.with(|s| {
                let geoms = s.product.general_definitions.geometries.as_ref();
                let simple: Vec<_> = geoms
                    .map(|g| g.simple_geometry.clone())
                    .unwrap_or_default();
                let model: Vec<_> = geoms
                    .map(|g| g.model_geometry.clone())
                    .unwrap_or_default();

                view! {
                    <div style="display: flex; flex-direction: column; gap: 16px;">
                        <div>
                            <h3 style="margin: 0 0 8px;">{ format!("Simple ({})", simple.len()) }</h3>
                            {if simple.is_empty() {
                                view! { <p class="empty-message">{ "No simple geometries defined yet." }</p> }.into_any()
                            } else {
                                view! {
                                    <ul style="margin: 0; padding-left: 18px;">
                                        {simple.into_iter().map(|sg| {
                                            let id_for_summary = sg.id.clone();
                                            let id_for_remove = sg.id.clone();
                                            let mut parts = Vec::new();
                                            for c in &sg.cuboid {
                                                let w = c.width.first().copied().unwrap_or(0);
                                                let l = c.length.first().copied().unwrap_or(0);
                                                let h = c.height.first().copied().unwrap_or(0);
                                                parts.push(format!("Cuboid {}×{}×{}mm", w, l, h));
                                            }
                                            for c in &sg.cylinder {
                                                let d = c.diameter.first().copied().unwrap_or(0);
                                                let h = c.height.first().cloned().unwrap_or_default();
                                                parts.push(format!("Cylinder Ø{}mm × {}mm", d, h));
                                            }
                                            if parts.is_empty() {
                                                parts.push("(empty)".to_string());
                                            }
                                            let summary = parts.join(" · ");
                                            let on_remove = move |_| {
                                                state.update(|s| {
                                                    s.apply(GldfAction::RemoveSimpleGeometry {
                                                        id: id_for_remove.clone(),
                                                    });
                                                });
                                            };
                                            view! {
                                                <li style="display: flex; gap: 8px; align-items: center; margin-bottom: 4px;">
                                                    <span style="font-family: var(--font-mono); font-size: 12px; color: var(--text-secondary);">
                                                        { id_for_summary }
                                                    </span>
                                                    <span style="font-size: 12px;">{ summary }</span>
                                                    {if editable {
                                                        view! {
                                                            <button type="button" class="btn-icon" title="Remove" on:click=on_remove style="margin-left: auto;">{ "✕" }</button>
                                                        }.into_any()
                                                    } else { ().into_any() }}
                                                </li>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </ul>
                                }.into_any()
                            }}
                            {if editable {
                                view! {
                                    <div style="display: flex; flex-direction: column; gap: 12px; margin-top: 12px; padding: 10px 12px; border: 1px solid var(--border-color); border-radius: 6px; background: var(--bg-secondary);">
                                        <div>
                                            <strong style="font-size: 12px;">{ "Add Cuboid" }</strong>
                                            <div style="display: grid; grid-template-columns: repeat(3, auto) auto; gap: 8px; align-items: center; margin-top: 6px; font-size: 12px;">
                                                <label>{ "W (mm)" }</label>
                                                <input type="number" min="0" prop:value=move || cuboid_w.get().to_string()
                                                    on:input=move |e| if let Some(v) = event_i32(&e) { cuboid_w.set(v) }
                                                    style="width: 100px;" />
                                                <label>{ "L (mm)" }</label>
                                                <input type="number" min="0" prop:value=move || cuboid_l.get().to_string()
                                                    on:input=move |e| if let Some(v) = event_i32(&e) { cuboid_l.set(v) }
                                                    style="width: 100px;" />
                                                <label>{ "H (mm)" }</label>
                                                <input type="number" min="0" prop:value=move || cuboid_h.get().to_string()
                                                    on:input=move |e| if let Some(v) = event_i32(&e) { cuboid_h.set(v) }
                                                    style="width: 100px;" />
                                                <button type="button" class="btn btn-secondary" on:click=on_add_cuboid>{ "+ Add" }</button>
                                            </div>
                                        </div>
                                        <div>
                                            <strong style="font-size: 12px;">{ "Add Cylinder" }</strong>
                                            <div style="display: grid; grid-template-columns: repeat(2, auto) auto; gap: 8px; align-items: center; margin-top: 6px; font-size: 12px;">
                                                <label>{ "Ø (mm)" }</label>
                                                <input type="number" min="0" prop:value=move || cyl_d.get().to_string()
                                                    on:input=move |e| if let Some(v) = event_i32(&e) { cyl_d.set(v) }
                                                    style="width: 100px;" />
                                                <label>{ "H (mm)" }</label>
                                                <input type="number" min="0" prop:value=move || cyl_h.get().to_string()
                                                    on:input=move |e| if let Some(v) = event_i32(&e) { cyl_h.set(v) }
                                                    style="width: 100px;" />
                                                <button type="button" class="btn btn-secondary" on:click=on_add_cylinder>{ "+ Add" }</button>
                                            </div>
                                        </div>
                                    </div>
                                }.into_any()
                            } else { ().into_any() }}
                        </div>

                        <div>
                            <h3 style="margin: 0 0 8px;">{ format!("Model (.l3d / .m3d / .r3d) ({})", model.len()) }</h3>
                            {if model.is_empty() {
                                view! {
                                    <p style="font-size: 12px; color: var(--text-tertiary);">
                                        { "No model geometries. Drop a .l3d file on the Files page — it auto-registers as a ModelGeometry." }
                                    </p>
                                }.into_any()
                            } else {
                                view! {
                                    <ul style="margin: 0; padding-left: 18px;">
                                        {model.into_iter().map(|mg| {
                                            let file_id = mg.geometry_file_reference.first().map(|r| r.file_id.clone()).unwrap_or_default();
                                            view! {
                                                <li style="display: flex; gap: 8px; align-items: center; margin-bottom: 4px; font-size: 12px;">
                                                    <span style="font-family: var(--font-mono); color: var(--text-secondary);">{ mg.id.clone() }</span>
                                                    <span>{ format!("→ file: {}", file_id) }</span>
                                                </li>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </ul>
                                }.into_any()
                            }}
                        </div>
                    </div>
                }
            })}
        </div>
    }
}
