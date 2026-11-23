//! egui port of the WASM `LeoPlanPicker`: a clickable plan-view grid of
//! the variant L3D's Light Emitting Objects, with an assignment panel
//! that binds a LEO to an Emitter (or creates a Fixed emitter for it in
//! one step). Dispatches the exact same actions as the WASM picker —
//! `SetVariantGeometryModel` with the LEO's `part_name` as the binding's
//! `external_name`, and `AddFixedEmitter` for the create-and-bind flow.

use super::GldfEditorApp;
use eframe::egui;
use gldf_rs::{LeoFootprint, LeoShape};
use gldf_rs_editor::action::ModelEmitterBinding;
use gldf_rs_editor::GldfAction;

/// Same resolution the WASM Bevy page uses: geometry id → ModelGeometry
/// → first GeometryFileReference → embedded bytes.
fn resolve_l3d_bytes(
    product: &gldf_rs::GldfProduct,
    embedded: &std::collections::HashMap<String, Vec<u8>>,
    geometry_id: &str,
) -> Option<Vec<u8>> {
    let model_geom = product
        .general_definitions
        .geometries
        .as_ref()?
        .model_geometry
        .iter()
        .find(|g| g.id == geometry_id)?;
    let file_id = &model_geom.geometry_file_reference.first()?.file_id;
    embedded.get(file_id).cloned()
}

/// Merge one LEO's binding into the list, preserving every other LEO.
/// `emitter_id == None` clears this LEO's binding.
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

fn ellipsize(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

pub fn leo_plan_picker(
    app: &mut GldfEditorApp,
    ui: &mut egui::Ui,
    variant_idx: usize,
    geometry_id: &str,
) {
    // --- Data snapshot (cloned; dispatches happen after rendering) ---
    let leos: Vec<LeoFootprint> =
        resolve_l3d_bytes(&app.state.product, &app.state.embedded_files, geometry_id)
            .map(|bytes| gldf_rs::extract_leos(&bytes).unwrap_or_default())
            .unwrap_or_default();

    if leos.is_empty() {
        ui.weak("No Light Emitting Objects found in this L3D.");
        return;
    }

    // Current bindings (external_name → emitter_id).
    let bindings: Vec<(String, String)> = app
        .state
        .product
        .product_definitions
        .variants
        .as_ref()
        .and_then(|vs| vs.variant.get(variant_idx))
        .and_then(|v| v.geometry.as_ref())
        .and_then(|g| g.model_geometry_reference.as_ref())
        .map(|m| {
            m.emitter_reference
                .iter()
                .map(|e| (e.emitter_object_external_name.clone(), e.emitter_id.clone()))
                .collect()
        })
        .unwrap_or_default();

    // Per-LEO assigned photometry file name for the card label.
    let leo_photometry: std::collections::HashMap<String, String> = {
        let variant_id = app
            .state
            .product
            .product_definitions
            .variants
            .as_ref()
            .and_then(|vs| vs.variant.get(variant_idx))
            .map(|v| v.id.clone())
            .unwrap_or_default();
        let data = gldf_rs::mapping::get_variant_emitter_data(&app.state.product, &variant_id);
        data.emitters
            .into_iter()
            .filter_map(|e| e.ldt_file_name.map(|f| (e.leo_name, f)))
            .collect()
    };

    // Emitter / photometry / light source catalogs for the panel.
    let gd = &app.state.product.general_definitions;
    let emitters: Vec<(String, Option<String>)> = gd
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
                .collect()
        })
        .unwrap_or_default();
    let photometries: Vec<(String, String)> = gd
        .photometries
        .as_ref()
        .map(|ps| {
            ps.photometry
                .iter()
                .map(|p| {
                    let fname = p
                        .photometry_file_reference
                        .as_ref()
                        .and_then(|r| gd.files.file.iter().find(|f| f.id == r.file_id))
                        .map(|f| f.file_name.clone())
                        .unwrap_or_else(|| p.id.clone());
                    (p.id.clone(), fname)
                })
                .collect()
        })
        .unwrap_or_default();
    let light_sources: Vec<(String, Option<String>)> = gd
        .light_sources
        .as_ref()
        .map(|ls| {
            ls.fixed_light_source
                .iter()
                .map(|l| {
                    let hint = l.name.locale.first().map(|loc| loc.value.clone());
                    (l.id.clone(), hint)
                })
                .collect()
        })
        .unwrap_or_default();

    ui.strong("Light emitting objects — click a card to assign");

    // --- Plan-view grid of LEO cards (painter-drawn, click-to-select).
    //     v1 lays LEOs out on a uniform grid (raw L3D positions are
    //     joint-local and can collapse onto one point), mirroring the
    //     WASM picker. ---
    const CELL: f32 = 104.0;
    let cols = leos.len().min(3).max(1);
    let rows = leos.len().div_ceil(cols);
    let desired = egui::vec2(cols as f32 * CELL, rows as f32 * CELL);
    let (grid_rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let painter = ui.painter_at(grid_rect);

    let accent_blue = egui::Color32::from_rgb(0x3b, 0x82, 0xf6);
    let accent_orange = egui::Color32::from_rgb(0xf5, 0x9e, 0x0b);
    let gray = ui.visuals().weak_text_color();

    let mut clicked_leo: Option<String> = None;
    for (i, leo) in leos.iter().enumerate() {
        let col = (i % cols) as f32;
        let row = (i / cols) as f32;
        let cell = egui::Rect::from_min_size(
            grid_rect.min + egui::vec2(col * CELL, row * CELL),
            egui::vec2(CELL, CELL),
        )
        .shrink(4.0);

        let name = leo.part_name.clone();
        let is_bound = bindings.iter().any(|(ext, _)| ext == &name);
        let is_sel = app.leo_sel.as_deref() == Some(name.as_str());

        let resp = ui.interact(
            cell,
            ui.id().with(("leo-card", variant_idx, i)),
            egui::Sense::click(),
        );
        if resp.clicked() {
            clicked_leo = Some(name.clone());
        }

        // Card frame.
        let border = if is_sel {
            egui::Stroke::new(2.0, accent_orange)
        } else if resp.hovered() {
            egui::Stroke::new(1.0, ui.visuals().text_color())
        } else {
            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
        };
        painter.rect(
            cell,
            6.0,
            ui.visuals().extreme_bg_color,
            border,
            egui::StrokeKind::Inside,
        );

        // Shape glyph: circle for Circle/Point, rounded square for
        // Rectangle. Filled when bound, outlined when not.
        let glyph_c = egui::pos2(cell.center().x, cell.min.y + cell.height() * 0.34);
        let r = CELL * 0.20;
        let (fill, stroke) = if is_bound {
            (accent_blue, egui::Stroke::new(2.0, accent_blue))
        } else {
            (egui::Color32::TRANSPARENT, egui::Stroke::new(2.0, gray))
        };
        match leo.shape {
            LeoShape::Rectangle { .. } => {
                let s = r * 1.6;
                let rect = egui::Rect::from_center_size(glyph_c, egui::vec2(s, s));
                painter.rect(rect, 4.0, fill, stroke, egui::StrokeKind::Inside);
            }
            _ => {
                painter.circle(glyph_c, r, fill, stroke);
            }
        }

        // Labels: part name + assigned photometry (or "unassigned").
        painter.text(
            egui::pos2(cell.center().x, cell.min.y + cell.height() * 0.66),
            egui::Align2::CENTER_CENTER,
            &name,
            egui::TextStyle::Monospace.resolve(ui.style()),
            ui.visuals().text_color(),
        );
        let label2 = if is_bound {
            leo_photometry
                .get(&name)
                .cloned()
                .unwrap_or_else(|| "—".to_string())
        } else {
            "unassigned".to_string()
        };
        painter.text(
            egui::pos2(cell.center().x, cell.min.y + cell.height() * 0.84),
            egui::Align2::CENTER_CENTER,
            ellipsize(&label2, 16),
            egui::TextStyle::Small.resolve(ui.style()),
            if is_bound { accent_blue } else { gray },
        );
    }
    if let Some(name) = clicked_leo {
        // Seed the create-form defaults when selecting a new LEO.
        if app.leo_sel.as_deref() != Some(name.as_str()) {
            app.leo_form.photometry_id = photometries
                .first()
                .map(|(id, _)| id.clone())
                .unwrap_or_default();
            app.leo_form.light_source_id = light_sources
                .first()
                .map(|(id, _)| id.clone())
                .unwrap_or_default();
        }
        app.leo_sel = Some(name);
    }

    // --- Assignment panel for the selected LEO ---
    let Some(leo) = app.leo_sel.clone() else {
        return;
    };
    if !leos.iter().any(|l| l.part_name == leo) {
        app.leo_sel = None;
        return;
    }

    let current_emitter = bindings
        .iter()
        .find(|(ext, _)| ext == &leo)
        .map(|(_, eid)| eid.clone())
        .unwrap_or_default();

    let to_bindings = |pairs: &[(String, String)]| -> Vec<ModelEmitterBinding> {
        pairs
            .iter()
            .map(|(ext, eid)| ModelEmitterBinding {
                emitter_id: eid.clone(),
                external_name: ext.clone(),
                target_model_type: Some("l3d".to_string()),
            })
            .collect()
    };

    ui.add_space(6.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.strong(format!("Assign to {}", leo));

        // Canonical path: pick an existing emitter.
        ui.horizontal(|ui| {
            ui.label("Emitter");
            let mut picked: Option<String> = None;
            egui::ComboBox::from_id_salt(("leo-emitter", variant_idx))
                .selected_text(if current_emitter.is_empty() {
                    "(pick emitter…)".to_string()
                } else {
                    current_emitter.clone()
                })
                .show_ui(ui, |ui| {
                    for (eid, hint) in &emitters {
                        let label = match hint {
                            Some(n) if !n.is_empty() => format!("{} ({})", eid, n),
                            _ => eid.clone(),
                        };
                        if ui
                            .selectable_label(*eid == current_emitter, label)
                            .clicked()
                        {
                            picked = Some(eid.clone());
                        }
                    }
                });
            if let Some(eid) = picked {
                let next = upsert_binding(to_bindings(&bindings), &leo, Some(eid));
                app.dispatch(GldfAction::SetVariantGeometryModel {
                    variant_idx,
                    geometry_id: geometry_id.to_string(),
                    bindings: next,
                });
            }
            if !current_emitter.is_empty() && ui.button("Clear").clicked() {
                let next = upsert_binding(to_bindings(&bindings), &leo, None);
                app.dispatch(GldfAction::SetVariantGeometryModel {
                    variant_idx,
                    geometry_id: geometry_id.to_string(),
                    bindings: next,
                });
            }
        });

        // Create-and-bind: mint a Fixed emitter *for this LEO* in one
        // step (photometry + light source + flux, child_name = LEO).
        let default_open = current_emitter.is_empty() && !photometries.is_empty();
        egui::CollapsingHeader::new(format!("+ New emitter for {}…", leo))
            .default_open(default_open)
            .show(ui, |ui| {
                if photometries.is_empty() {
                    ui.weak(
                        "No photometries yet — add an LDT/IES on the Photometries or \
                         Files page first, then come back.",
                    );
                    return;
                }
                new_emitter_form(
                    app,
                    ui,
                    variant_idx,
                    &leo,
                    geometry_id,
                    &photometries,
                    &light_sources,
                    &bindings,
                );
            });
    });
}

/// The inline "＋ New emitter for this LEO" form: photometry + light
/// source + flux → `AddFixedEmitter` (child_name = the LEO) followed by
/// `SetVariantGeometryModel` binding the fresh emitter id (read back
/// from state, never predicted — same pattern as the WASM picker).
#[allow(clippy::too_many_arguments)]
fn new_emitter_form(
    app: &mut GldfEditorApp,
    ui: &mut egui::Ui,
    variant_idx: usize,
    leo: &str,
    geometry_id: &str,
    photometries: &[(String, String)],
    light_sources: &[(String, Option<String>)],
    bindings: &[(String, String)],
) {
    egui::Grid::new(("leo-new-emitter", variant_idx))
        .num_columns(2)
        .spacing([10.0, 5.0])
        .show(ui, |ui| {
            ui.label("Photometry");
            let sel_name = photometries
                .iter()
                .find(|(id, _)| *id == app.leo_form.photometry_id)
                .map(|(_, f)| f.clone())
                .unwrap_or_else(|| "(pick…)".to_string());
            egui::ComboBox::from_id_salt(("leo-new-p", variant_idx))
                .selected_text(sel_name)
                .show_ui(ui, |ui| {
                    for (pid, fname) in photometries {
                        ui.selectable_value(&mut app.leo_form.photometry_id, pid.clone(), fname);
                    }
                });
            ui.end_row();

            ui.label("Light source");
            if light_sources.is_empty() {
                ui.colored_label(
                    egui::Color32::from_rgb(0xf5, 0x9e, 0x0b),
                    "none — add a Light Source first",
                );
            } else {
                egui::ComboBox::from_id_salt(("leo-new-l", variant_idx))
                    .selected_text(if app.leo_form.light_source_id.is_empty() {
                        "(pick…)".to_string()
                    } else {
                        app.leo_form.light_source_id.clone()
                    })
                    .show_ui(ui, |ui| {
                        for (lid, hint) in light_sources {
                            let label = match hint {
                                Some(n) if !n.is_empty() => format!("{} ({})", lid, n),
                                _ => lid.clone(),
                            };
                            ui.selectable_value(
                                &mut app.leo_form.light_source_id,
                                lid.clone(),
                                label,
                            );
                        }
                    });
            }
            ui.end_row();

            ui.label("Flux (lm)");
            ui.add(
                egui::DragValue::new(&mut app.leo_form.flux)
                    .range(0..=200_000)
                    .speed(50),
            );
            ui.end_row();
        });

    let can_create =
        !app.leo_form.photometry_id.is_empty() && !app.leo_form.light_source_id.is_empty();
    if ui
        .add_enabled(
            can_create,
            egui::Button::new(format!("Create & bind to {}", leo)),
        )
        .clicked()
    {
        let cur: Vec<ModelEmitterBinding> = bindings
            .iter()
            .map(|(ext, eid)| ModelEmitterBinding {
                emitter_id: eid.clone(),
                external_name: ext.clone(),
                target_model_type: Some("l3d".to_string()),
            })
            .collect();
        app.dispatch(GldfAction::AddFixedEmitter {
            emitter_id: String::new(),
            child_name: Some(leo.to_string()),
            photometry_id: app.leo_form.photometry_id.clone(),
            light_source_id: app.leo_form.light_source_id.clone(),
            rated_luminous_flux: app.leo_form.flux,
        });
        let emitter_id = app
            .state
            .product
            .general_definitions
            .emitters
            .as_ref()
            .and_then(|es| es.emitter.last())
            .map(|e| e.id.clone())
            .unwrap_or_default();
        let next = upsert_binding(cur, leo, Some(emitter_id));
        app.dispatch(GldfAction::SetVariantGeometryModel {
            variant_idx,
            geometry_id: geometry_id.to_string(),
            bindings: next,
        });
    }
}
