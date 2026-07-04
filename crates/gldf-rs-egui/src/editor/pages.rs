//! Editor pages. Every page reads from `app.state` (cloning the small
//! bits it renders) and mutates exclusively via `app.dispatch(action)` —
//! the same reducer contract as the WASM and TUI frontends.

use super::{locale_from, locale_text, GldfEditorApp};
use eframe::egui;
use gldf_rs_editor::{GldfAction, LightEmitterKind};

/// Two-column label/value grid helper.
fn kv_grid(ui: &mut egui::Ui, id: &str, rows: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([12.0, 6.0])
        .min_col_width(140.0)
        .show(ui, rows);
}

/// Editable single-line text bound to a String value; returns the new
/// value when the user changed it this frame.
fn edit_text(ui: &mut egui::Ui, mut value: String) -> Option<String> {
    if ui
        .add(egui::TextEdit::singleline(&mut value).desired_width(280.0))
        .changed()
    {
        Some(value)
    } else {
        None
    }
}

// ---------------------------------------------------------------------
// Overview
// ---------------------------------------------------------------------

pub fn overview(app: &mut GldfEditorApp, ui: &mut egui::Ui) {
    ui.heading(app.t("gldf.nav.overview"));
    ui.add_space(8.0);

    let header = app.state.product.header.clone();
    let pmd = app
        .state
        .product
        .product_definitions
        .product_meta_data
        .clone();

    ui.strong("Product");
    kv_grid(ui, "ov-product", |ui| {
        ui.label(app.t("gldf.overview.product_name"));
        let name = locale_text(pmd.as_ref().and_then(|p| p.name.as_ref()));
        if let Some(v) = edit_text(ui, name) {
            app.dispatch(GldfAction::SetProductMetaName(locale_from(&v)));
        }
        ui.end_row();
        ui.label(app.t("gldf.overview.description"));
        let desc = locale_text(pmd.as_ref().and_then(|p| p.description.as_ref()));
        if let Some(v) = edit_text(ui, desc) {
            app.dispatch(GldfAction::SetProductMetaDescription(locale_from(&v)));
        }
        ui.end_row();
        ui.label(app.t("gldf.header.manufacturer"));
        ui.label(header.manufacturer.clone());
        ui.end_row();
        ui.label(app.t("gldf.header.author"));
        ui.label(header.author.clone());
        ui.end_row();
    });

    ui.add_space(12.0);
    ui.strong("Contents");
    let gd = &app.state.product.general_definitions;
    let counts = [
        ("Files", gd.files.file.len()),
        (
            "Photometries",
            gd.photometries
                .as_ref()
                .map(|p| p.photometry.len())
                .unwrap_or(0),
        ),
        (
            "Light sources",
            gd.light_sources
                .as_ref()
                .map(|l| {
                    l.fixed_light_source.len()
                        + l.changeable_light_source.len()
                        + l.multi_channel_light_source.len()
                })
                .unwrap_or(0),
        ),
        (
            "Emitters",
            gd.emitters.as_ref().map(|e| e.emitter.len()).unwrap_or(0),
        ),
        (
            "Geometries",
            gd.geometries
                .as_ref()
                .map(|g| g.model_geometry.len())
                .unwrap_or(0),
        ),
        (
            "Variants",
            app.state
                .product
                .product_definitions
                .variants
                .as_ref()
                .map(|v| v.variant.len())
                .unwrap_or(0),
        ),
    ];
    ui.horizontal_wrapped(|ui| {
        for (label, n) in counts {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.strong(n.to_string());
                    ui.weak(label);
                });
            });
        }
    });
}

// ---------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------

pub fn header(app: &mut GldfEditorApp, ui: &mut egui::Ui) {
    ui.heading(app.t("gldf.header.title"));
    ui.add_space(8.0);

    let h = app.state.product.header.clone();
    kv_grid(ui, "header-grid", |ui| {
        ui.label(app.t("gldf.header.author"));
        if let Some(v) = edit_text(ui, h.author.clone()) {
            app.dispatch(GldfAction::SetAuthor(v));
        }
        ui.end_row();

        ui.label(app.t("gldf.header.manufacturer"));
        if let Some(v) = edit_text(ui, h.manufacturer.clone()) {
            app.dispatch(GldfAction::SetManufacturer(v));
        }
        ui.end_row();

        ui.label(app.t("gldf.header.created_with"));
        if let Some(v) = edit_text(ui, h.created_with_application.clone()) {
            app.dispatch(GldfAction::SetCreatedWithApplication(v));
        }
        ui.end_row();

        ui.label(app.t("gldf.header.created_at"));
        if let Some(v) = edit_text(ui, h.creation_time_code.clone()) {
            app.dispatch(GldfAction::SetCreationTimeCode(v));
        }
        ui.end_row();

        ui.label(app.t("gldf.header.default_language"));
        let lang = h.default_language.clone().unwrap_or_default();
        if let Some(v) = edit_text(ui, lang) {
            app.dispatch(GldfAction::SetDefaultLanguage(if v.trim().is_empty() {
                None
            } else {
                Some(v)
            }));
        }
        ui.end_row();

        ui.label(app.t("gldf.header.format_version"));
        ui.label(format!(
            "{}.{}{}",
            h.format_version.major,
            h.format_version.minor,
            h.format_version
                .pre_release
                .map(|p| format!("-rc{}", p))
                .unwrap_or_default()
        ));
        ui.end_row();

        ui.label(app.t("gldf.header.unique_id"));
        ui.monospace(h.unique_gldf_id.clone().unwrap_or_default());
        ui.end_row();
    });
}

// ---------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------

pub fn files(app: &mut GldfEditorApp, ui: &mut egui::Ui) {
    ui.heading(app.t("gldf.nav.files"));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("+ Add files…").clicked() {
            if let Some(paths) = rfd::FileDialog::new().pick_files() {
                for path in paths {
                    match std::fs::read(&path) {
                        Ok(bytes) => app.add_file_bytes(&path, bytes),
                        Err(e) => app.status = Some((format!("read {}: {}", path.display(), e), true)),
                    }
                }
            }
        }
        ui.weak("or drop files anywhere — .ldt/.ies, .l3d, images, .spd and sensor sheets are linked automatically");
    });
    ui.add_space(6.0);

    #[derive(Clone)]
    struct Row {
        id: String,
        file_name: String,
        content_type: String,
        size: Option<usize>,
    }
    let rows: Vec<Row> = app
        .state
        .product
        .general_definitions
        .files
        .file
        .iter()
        .map(|f| Row {
            id: f.id.clone(),
            file_name: f.file_name.clone(),
            content_type: f.content_type.clone(),
            size: app.state.embedded_files.get(&f.id).map(|b| b.len()),
        })
        .collect();

    let selected = app.sel_file.clone();

    ui.columns(2, |cols| {
        // --- Left: table ---
        let ui = &mut cols[0];
        use egui_extras::{Column, TableBuilder};
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size + 8.0;
        TableBuilder::new(ui)
            .striped(true)
            .sense(egui::Sense::click())
            .column(Column::auto().at_least(110.0).resizable(true))
            .column(Column::remainder())
            .column(Column::auto().at_least(90.0))
            .column(Column::auto().at_least(60.0))
            .header(20.0, |mut h| {
                h.col(|ui| {
                    ui.strong("id");
                });
                h.col(|ui| {
                    ui.strong("file");
                });
                h.col(|ui| {
                    ui.strong("type");
                });
                h.col(|ui| {
                    ui.strong("size");
                });
            })
            .body(|body| {
                body.rows(text_height, rows.len(), |mut tr| {
                    let row = &rows[tr.index()];
                    tr.set_selected(selected.as_deref() == Some(row.id.as_str()));
                    tr.col(|ui| {
                        ui.monospace(&row.id);
                    });
                    tr.col(|ui| {
                        ui.label(&row.file_name);
                    });
                    tr.col(|ui| {
                        ui.weak(&row.content_type);
                    });
                    tr.col(|ui| {
                        ui.weak(
                            row.size
                                .map(|s| format!("{:.1} kB", s as f64 / 1024.0))
                                .unwrap_or_else(|| "url".into()),
                        );
                    });
                    if tr.response().clicked() {
                        app.sel_file = Some(row.id.clone());
                        app.rename_target = None;
                    }
                });
            });

        // --- Right: detail / preview for the selected file ---
        let ui = &mut cols[1];
        let Some(sel_id) = app.sel_file.clone() else {
            ui.centered_and_justified(|ui| {
                ui.weak("Select a file to preview");
            });
            return;
        };
        let Some(row) = rows.iter().find(|r| r.id == sel_id) else {
            app.sel_file = None;
            return;
        };
        ui.strong(&row.file_name);
        ui.weak(format!("{} · {}", row.id, row.content_type));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            // Rename id (staged: type, then apply — a rename cascades
            // through every reference, so it must not fire per keystroke).
            if app.rename_target.as_ref().map(|(k, id)| (*k, id.as_str()))
                == Some(("file", sel_id.as_str()))
            {
                ui.add(
                    egui::TextEdit::singleline(&mut app.rename_buf).desired_width(140.0),
                );
                if ui.button("Apply").clicked() && !app.rename_buf.trim().is_empty() {
                    let new_id = app.rename_buf.trim().to_string();
                    app.dispatch(GldfAction::RenameFile {
                        old_id: sel_id.clone(),
                        new_id: new_id.clone(),
                    });
                    app.sel_file = Some(new_id);
                    app.rename_target = None;
                }
                if ui.button("✕").clicked() {
                    app.rename_target = None;
                }
            } else if ui.button("Rename id").clicked() {
                app.rename_target = Some(("file", sel_id.clone()));
                app.rename_buf = sel_id.clone();
            }
            if ui.button("Export…").clicked() {
                if let Some(bytes) = app.state.embedded_files.get(&sel_id).cloned() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(&row.file_name)
                        .save_file()
                    {
                        match std::fs::write(&path, bytes) {
                            Ok(()) => app.info(format!("Wrote {}", path.display())),
                            Err(e) => app.status = Some((e.to_string(), true)),
                        }
                    }
                }
            }
            if ui.button("Remove").clicked() {
                app.dispatch(GldfAction::RemoveFile(sel_id.clone()));
                app.sel_file = None;
                return;
            }
        });
        ui.separator();
        if let Some(bytes) = app.state.embedded_files.get(&sel_id).cloned() {
            file_preview(ui, &row.file_name, &bytes);
        } else {
            // URL file — fetch in the background and preview once loaded
            // (same cache the retired viewer app used).
            crate::preview::render_url_file(
                ui,
                &row.file_name,
                &row.file_name.to_ascii_lowercase(),
            );
        }
    });
}

/// Dispatch a preview by extension, reusing the viewer crate's
/// renderers (polar diagram for photometry, image loader, hex fallback).
pub fn file_preview(ui: &mut egui::Ui, file_name: &str, bytes: &[u8]) {
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".ldt") || lower.ends_with(".ies") {
        crate::preview::render_photometry_preview(ui, bytes, file_name);
    } else if lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".webp")
    {
        crate::preview::render_image_preview(ui, bytes, file_name);
    } else if lower.ends_with(".xml") || lower.ends_with(".spd") || lower.ends_with(".txt") {
        crate::preview::render_text_preview(ui, bytes);
    } else if lower.ends_with(".l3d") {
        // Rich L3D preview; includes the "Open Interactive 3D Viewer"
        // button when built with the `viewer3d` feature.
        crate::preview::render_l3d_preview(ui, bytes, file_name);
        match gldf_rs::extract_leos(bytes) {
            Ok(leos) if !leos.is_empty() => {
                ui.label(format!(
                    "LEOs: {}",
                    leos.iter()
                        .map(|l| l.part_name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            _ => {}
        }
    } else if lower.ends_with(".pdf") {
        crate::preview::render_pdf_preview(ui, bytes, file_name);
    } else {
        crate::preview::render_hex_preview(ui, bytes);
    }
}

// ---------------------------------------------------------------------
// Photometries
// ---------------------------------------------------------------------

pub fn photometries(app: &mut GldfEditorApp, ui: &mut egui::Ui) {
    ui.heading(app.t("gldf.nav.photometry"));
    ui.add_space(4.0);

    #[derive(Clone)]
    struct Row {
        id: String,
        file_id: Option<String>,
        file_name: String,
    }
    let rows: Vec<Row> = {
        let gd = &app.state.product.general_definitions;
        gd.photometries
            .as_ref()
            .map(|ps| {
                ps.photometry
                    .iter()
                    .map(|p| {
                        let file_id = p
                            .photometry_file_reference
                            .as_ref()
                            .map(|r| r.file_id.clone());
                        let file_name = file_id
                            .as_deref()
                            .and_then(|fid| {
                                gd.files.file.iter().find(|f| f.id == fid)
                            })
                            .map(|f| f.file_name.clone())
                            .unwrap_or_default();
                        Row {
                            id: p.id.clone(),
                            file_id,
                            file_name,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    if rows.is_empty() {
        ui.weak("No photometries yet — drop an .ldt/.ies file anywhere or add one on the Files page.");
        return;
    }
    if app.sel_photometry >= rows.len() {
        app.sel_photometry = 0;
    }

    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        for (i, row) in rows.iter().enumerate() {
            let label = format!("{}  ·  {}", row.id, row.file_name);
            if ui
                .selectable_label(app.sel_photometry == i, label)
                .clicked()
            {
                app.sel_photometry = i;
                app.rename_target = None;
            }
        }

        let ui = &mut cols[1];
        let row = rows[app.sel_photometry].clone();
        ui.strong(&row.id);
        ui.horizontal(|ui| {
            if app.rename_target.as_ref().map(|(k, id)| (*k, id.as_str()))
                == Some(("photometry", row.id.as_str()))
            {
                ui.add(egui::TextEdit::singleline(&mut app.rename_buf).desired_width(140.0));
                if ui.button("Apply").clicked() && !app.rename_buf.trim().is_empty() {
                    app.dispatch(GldfAction::RenamePhotometry {
                        old_id: row.id.clone(),
                        new_id: app.rename_buf.trim().to_string(),
                    });
                    app.rename_target = None;
                }
                if ui.button("✕").clicked() {
                    app.rename_target = None;
                }
            } else if ui.button("Rename").clicked() {
                app.rename_target = Some(("photometry", row.id.clone()));
                app.rename_buf = row.id.clone();
            }
            if ui.button("Remove").clicked() {
                app.dispatch(GldfAction::RemovePhotometry { id: row.id.clone() });
                return;
            }
        });
        ui.separator();
        if let Some(bytes) = row
            .file_id
            .as_deref()
            .and_then(|fid| app.state.embedded_files.get(fid))
            .cloned()
        {
            crate::preview::render_photometry_preview(ui, &bytes, &row.file_name);
        } else {
            ui.weak("No embedded photometry bytes");
        }
    });
}

// ---------------------------------------------------------------------
// Light sources
// ---------------------------------------------------------------------

pub fn light_sources(app: &mut GldfEditorApp, ui: &mut egui::Ui) {
    ui.heading(app.t("gldf.nav.light_sources"));
    ui.add_space(4.0);

    #[derive(Clone)]
    struct Row {
        id: String,
        name: String,
        power: Option<f64>,
    }
    let fixed: Vec<Row> = app
        .state
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .map(|ls| {
            ls.fixed_light_source
                .iter()
                .map(|l| Row {
                    id: l.id.clone(),
                    name: locale_text(Some(&l.name)),
                    power: l.rated_input_power,
                })
                .collect()
        })
        .unwrap_or_default();

    ui.strong("Fixed");
    kv_grid(ui, "ls-fixed", |ui| {
        for (i, row) in fixed.iter().enumerate() {
            ui.monospace(&row.id);
            ui.horizontal(|ui| {
                if let Some(v) = edit_text(ui, row.name.clone()) {
                    app.dispatch(GldfAction::SetFixedLightSourceName(i, locale_from(&v)));
                }
                if let Some(w) = row.power {
                    ui.weak(format!("{} W", w));
                }
                if ui.small_button("✕").clicked() {
                    app.dispatch(GldfAction::RemoveLightSource { id: row.id.clone() });
                }
            });
            ui.end_row();
        }
    });

    ui.add_space(8.0);
    ui.collapsing("+ Add fixed light source", |ui| {
        let id = egui::Id::new("new-ls");
        let mut name: String = ui.memory_mut(|m| m.data.get_temp(id.with("n")).unwrap_or_default());
        let mut power: String = ui.memory_mut(|m| m.data.get_temp(id.with("p")).unwrap_or_default());
        kv_grid(ui, "ls-new", |ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut name);
            ui.end_row();
            ui.label("Rated power (W)");
            ui.text_edit_singleline(&mut power);
            ui.end_row();
        });
        ui.memory_mut(|m| {
            m.data.insert_temp(id.with("n"), name.clone());
            m.data.insert_temp(id.with("p"), power.clone());
        });
        if ui.button("Add").clicked() {
            use gldf_rs::gldf::general_definitions::lightsources::FixedLightSource;
            let mut ls = FixedLightSource::default();
            ls.name = locale_from(&name);
            ls.rated_input_power = power.trim().parse::<f64>().ok();
            app.dispatch(GldfAction::AddFixedLightSource {
                payload: Box::new(ls),
            });
            ui.memory_mut(|m| {
                m.data.remove::<String>(id.with("n"));
                m.data.remove::<String>(id.with("p"));
            });
        }
    });

    // Changeable / MultiChannel — listed read-only for now; the typed
    // add flows exist in the reducer and can be wired the same way.
    let (changeable, multi): (Vec<(String, String)>, Vec<(String, String)>) = app
        .state
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .map(|ls| {
            (
                ls.changeable_light_source
                    .iter()
                    .map(|l| (l.id.clone(), locale_text(Some(&l.name))))
                    .collect(),
                ls.multi_channel_light_source
                    .iter()
                    .map(|l| (l.id.clone(), locale_text(Some(&l.name))))
                    .collect(),
            )
        })
        .unwrap_or_default();
    if !changeable.is_empty() {
        ui.add_space(8.0);
        ui.strong("Changeable");
        for (id, name) in &changeable {
            ui.horizontal(|ui| {
                ui.monospace(id);
                ui.label(name);
            });
        }
    }
    if !multi.is_empty() {
        ui.add_space(8.0);
        ui.strong("MultiChannel");
        for (id, name) in &multi {
            ui.horizontal(|ui| {
                ui.monospace(id);
                ui.label(name);
            });
        }
    }
}

// ---------------------------------------------------------------------
// Emitters
// ---------------------------------------------------------------------

pub fn emitters(app: &mut GldfEditorApp, ui: &mut egui::Ui) {
    ui.heading(app.t("gldf.nav.emitters"));
    ui.add_space(4.0);

    #[derive(Clone)]
    struct Row {
        id: String,
        summary: String,
    }
    let rows: Vec<Row> = app
        .state
        .product
        .general_definitions
        .emitters
        .as_ref()
        .map(|es| {
            es.emitter
                .iter()
                .map(|e| {
                    let mut parts = Vec::new();
                    for fle in &e.fixed_light_emitter {
                        parts.push(format!(
                            "Fixed → {} / {}",
                            fle.photometry_reference.photometry_id,
                            fle.light_source_reference
                                .fixed_light_source_id
                                .clone()
                                .unwrap_or_default()
                        ));
                    }
                    for cle in &e.changeable_light_emitter {
                        parts.push(format!(
                            "Changeable → {}",
                            cle.photometry_reference.photometry_id
                        ));
                    }
                    if !e.multi_channel_light_emitter.is_empty() {
                        parts.push(format!(
                            "MultiChannel ×{}",
                            e.multi_channel_light_emitter.len()
                        ));
                    }
                    if !e.sensor_emitter.is_empty() {
                        parts.push("Sensor".to_string());
                    }
                    Row {
                        id: e.id.clone(),
                        summary: parts.join(" · "),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    for row in &rows {
        ui.horizontal(|ui| {
            ui.monospace(&row.id);
            ui.label(&row.summary);
            if ui.small_button("✕").clicked() {
                app.dispatch(GldfAction::RemoveEmitter { id: row.id.clone() });
            }
        });
    }
    if rows.is_empty() {
        ui.weak("No emitters yet.");
    }

    ui.add_space(8.0);
    ui.collapsing("+ Add emitter", |ui| {
        let photometries: Vec<String> = app
            .state
            .product
            .general_definitions
            .photometries
            .as_ref()
            .map(|p| p.photometry.iter().map(|x| x.id.clone()).collect())
            .unwrap_or_default();
        let light_sources: Vec<String> = app
            .state
            .product
            .general_definitions
            .light_sources
            .as_ref()
            .map(|l| l.fixed_light_source.iter().map(|x| x.id.clone()).collect())
            .unwrap_or_default();

        let id = egui::Id::new("new-emitter");
        let mut name: String =
            ui.memory_mut(|m| m.data.get_temp(id.with("name")).unwrap_or_default());
        let mut pid: String = ui.memory_mut(|m| {
            m.data
                .get_temp(id.with("pid"))
                .unwrap_or_else(|| photometries.first().cloned().unwrap_or_default())
        });
        let mut lid: String = ui.memory_mut(|m| {
            m.data
                .get_temp(id.with("lid"))
                .unwrap_or_else(|| light_sources.first().cloned().unwrap_or_default())
        });

        kv_grid(ui, "em-new", |ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut name);
            ui.end_row();
            ui.label("Photometry");
            egui::ComboBox::from_id_salt("em-new-p")
                .selected_text(if pid.is_empty() { "(none)" } else { &pid })
                .show_ui(ui, |ui| {
                    for p in &photometries {
                        ui.selectable_value(&mut pid, p.clone(), p);
                    }
                });
            ui.end_row();
            ui.label("Light source");
            egui::ComboBox::from_id_salt("em-new-l")
                .selected_text(if lid.is_empty() { "(none)" } else { &lid })
                .show_ui(ui, |ui| {
                    for l in &light_sources {
                        ui.selectable_value(&mut lid, l.clone(), l);
                    }
                });
            ui.end_row();
        });
        ui.memory_mut(|m| {
            m.data.insert_temp(id.with("name"), name.clone());
            m.data.insert_temp(id.with("pid"), pid.clone());
            m.data.insert_temp(id.with("lid"), lid.clone());
        });
        if ui.button("Add").clicked() {
            app.dispatch(GldfAction::AddEmitterWithChild {
                child_name: if name.trim().is_empty() {
                    None
                } else {
                    Some(name.clone())
                },
                kind: LightEmitterKind::Fixed,
                photometry_id: pid.clone(),
                fixed_light_source_id: lid.clone(),
                multi_channel_light_source_id: String::new(),
                sensor_id: String::new(),
            });
        }
    });
}

// ---------------------------------------------------------------------
// Variants (incl. LEO plan picker)
// ---------------------------------------------------------------------

pub fn variants(app: &mut GldfEditorApp, ui: &mut egui::Ui) {
    ui.heading(app.t("gldf.nav.variants"));
    ui.add_space(4.0);

    #[derive(Clone)]
    struct Row {
        id: String,
        name: String,
        product_number: String,
        geometry_id: Option<String>,
    }
    let rows: Vec<Row> = app
        .state
        .product
        .product_definitions
        .variants
        .as_ref()
        .map(|vs| {
            vs.variant
                .iter()
                .map(|v| Row {
                    id: v.id.clone(),
                    name: locale_text(v.name.as_ref()),
                    product_number: locale_text(v.product_number.as_ref()),
                    geometry_id: v
                        .geometry
                        .as_ref()
                        .and_then(|g| g.model_geometry_reference.as_ref())
                        .map(|m| m.geometry_id.clone()),
                })
                .collect()
        })
        .unwrap_or_default();

    ui.horizontal(|ui| {
        if ui.button("+ Add variant").clicked() {
            app.dispatch(GldfAction::AddVariantSimple {
                name: "Standard".to_string(),
            });
        }
        if !rows.is_empty() && ui.button("Duplicate selected").clicked() {
            app.dispatch(GldfAction::DuplicateVariant {
                src_idx: app.sel_variant,
            });
        }
    });
    if rows.is_empty() {
        return;
    }
    if app.sel_variant >= rows.len() {
        app.sel_variant = 0;
    }
    ui.add_space(6.0);

    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        for (i, row) in rows.iter().enumerate() {
            let label = format!(
                "{}  ·  {}",
                row.id,
                if row.name.is_empty() { "—" } else { &row.name }
            );
            if ui.selectable_label(app.sel_variant == i, label).clicked() {
                app.sel_variant = i;
                app.leo_sel = None;
            }
        }

        let ui = &mut cols[1];
        let idx = app.sel_variant;
        let row = rows[idx].clone();
        ui.strong(&row.id);
        kv_grid(ui, "variant-grid", |ui| {
            ui.label("Name");
            if let Some(v) = edit_text(ui, row.name.clone()) {
                app.dispatch(GldfAction::SetVariantName(idx, locale_from(&v)));
            }
            ui.end_row();
            ui.label("Product number");
            if let Some(v) = edit_text(ui, row.product_number.clone()) {
                app.dispatch(GldfAction::SetVariantProductNumber(idx, locale_from(&v)));
            }
            ui.end_row();
        });
        ui.horizontal(|ui| {
            if ui.button("Remove variant").clicked() {
                app.dispatch(GldfAction::RemoveVariant { id: row.id.clone() });
                return;
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.strong("Geometry");

        match row.geometry_id.clone() {
            Some(geometry_id) => {
                ui.weak(format!("Model geometry: {}", geometry_id));
                ui.add_space(4.0);
                super::leo_picker::leo_plan_picker(app, ui, idx, &geometry_id);
            }
            None => {
                let geometries: Vec<String> = app
                    .state
                    .product
                    .general_definitions
                    .geometries
                    .as_ref()
                    .map(|g| g.model_geometry.iter().map(|m| m.id.clone()).collect())
                    .unwrap_or_default();
                if geometries.is_empty() {
                    ui.weak("No model geometry in the catalog — drop an .l3d file first.");
                } else {
                    ui.horizontal(|ui| {
                        let id = egui::Id::new("variant-geo-pick");
                        let mut pick: String = ui.memory_mut(|m| {
                            m.data
                                .get_temp(id)
                                .unwrap_or_else(|| geometries[0].clone())
                        });
                        egui::ComboBox::from_id_salt("variant-geo")
                            .selected_text(pick.clone())
                            .show_ui(ui, |ui| {
                                for g in &geometries {
                                    ui.selectable_value(&mut pick, g.clone(), g);
                                }
                            });
                        ui.memory_mut(|m| m.data.insert_temp(id, pick.clone()));
                        if ui.button("Bind 3D model").clicked() {
                            app.dispatch(GldfAction::SetVariantGeometryModel {
                                variant_idx: idx,
                                geometry_id: pick,
                                bindings: Vec::new(),
                            });
                        }
                    });
                }
            }
        }
    });
}
