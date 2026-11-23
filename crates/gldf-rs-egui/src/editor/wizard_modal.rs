//! New-GLDF wizard as a floating egui window. All the actual logic —
//! step list, helper copy, the draft shape, and the apply pass — comes
//! from `gldf_rs_editor::wizard`; this file only renders widgets. The
//! same `WizardDraft` a Leptos form or `gldf-cli wizard --draft` builds
//! lands at the same `apply_wizard` reducer.

use super::GldfEditorApp;
use eframe::egui;
use gldf_rs_editor::{
    apply_wizard, EmitterDraft, LightEmitterKind, WizardDraft, WizardLightSourceKind, WizardStep,
};

pub struct WizardModal {
    step: usize,
    draft: WizardDraft,
    include_emitter: bool,
    emitter_name: String,
}

impl Default for WizardModal {
    fn default() -> Self {
        Self {
            step: 0,
            draft: WizardDraft {
                header_created_with: "gldf-editor-egui".to_string(),
                header_default_language: "en".to_string(),
                ..Default::default()
            },
            include_emitter: true,
            emitter_name: "Primary".to_string(),
        }
    }
}

impl WizardModal {
    /// Render; returns false when the modal should close.
    pub fn show(&mut self, ctx: &egui::Context, app: &mut GldfEditorApp) -> bool {
        let mut open = true;
        let mut finished = false;

        egui::Window::new("New GLDF wizard")
            .id(egui::Id::new("gldf-wizard"))
            .open(&mut open)
            .default_size([520.0, 480.0])
            .resizable(true)
            .show(ctx, |ui| {
                let step = WizardStep::ALL[self.step];

                // Step breadcrumb.
                ui.horizontal_wrapped(|ui| {
                    for (i, s) in WizardStep::ALL.iter().enumerate() {
                        let sel = i == self.step;
                        if ui
                            .selectable_label(sel, format!("{}", s.number()))
                            .clicked()
                        {
                            self.step = i;
                        }
                    }
                });
                ui.heading(step.headline());
                ui.weak(step.blurb());
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        self.step_body(ui, step);
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    if self.step > 0 && ui.button("Back").clicked() {
                        self.step -= 1;
                    }
                    if self.step + 1 < WizardStep::ALL.len() && ui.button("Next").clicked() {
                        self.step += 1;
                    }
                    if matches!(step, WizardStep::Review)
                        && ui
                            .add(
                                egui::Button::new("Create GLDF")
                                    .fill(egui::Color32::from_rgb(0x2e, 0x7d, 0x32)),
                            )
                            .clicked()
                    {
                        self.sync_emitter();
                        apply_wizard(&mut app.state, &self.draft);
                        match app.state.last_error.clone() {
                            Some(err) => app.status = Some((err, true)),
                            None => {
                                app.info("Wizard applied — product created");
                                finished = true;
                            }
                        }
                    }
                });
            });

        open && !finished
    }

    fn sync_emitter(&mut self) {
        if self.include_emitter {
            let kind = match self.draft.ls_kind {
                WizardLightSourceKind::Fixed => LightEmitterKind::Fixed,
                WizardLightSourceKind::Changeable => LightEmitterKind::Changeable,
                WizardLightSourceKind::MultiChannel => LightEmitterKind::MultiChannel,
            };
            self.draft.emitter_draft = Some(EmitterDraft {
                child_name: self.emitter_name.clone(),
                kind,
                ..EmitterDraft::default()
            });
        } else {
            self.draft.emitter_draft = None;
        }
    }

    fn grid(ui: &mut egui::Ui, id: &str, rows: impl FnOnce(&mut egui::Ui)) {
        egui::Grid::new(id)
            .num_columns(2)
            .spacing([10.0, 6.0])
            .min_col_width(150.0)
            .show(ui, rows);
    }

    /// A "pick file → staged bytes" row shared by the four byte-slots.
    fn file_slot(
        ui: &mut egui::Ui,
        label: &str,
        filters: &[&str],
        bytes: &mut Option<Vec<u8>>,
        filename: &mut String,
    ) {
        ui.horizontal(|ui| {
            if ui.button(format!("Pick {}…", label)).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(label, filters)
                    .pick_file()
                {
                    if let Ok(data) = std::fs::read(&path) {
                        *filename = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("file")
                            .to_string();
                        *bytes = Some(data);
                    }
                }
            }
            match bytes {
                Some(b) => {
                    ui.label(gldf_rs_editor::wizard_text::photometry_loaded(
                        filename,
                        b.len(),
                    ));
                    if ui.small_button("✕").clicked() {
                        *bytes = None;
                        filename.clear();
                    }
                }
                None => {
                    ui.weak("(skipped)");
                }
            }
        });
    }

    fn step_body(&mut self, ui: &mut egui::Ui, step: WizardStep) {
        use gldf_rs_editor::wizard_text as text;
        let d = &mut self.draft;
        match step {
            WizardStep::Header => Self::grid(ui, "wz-header", |ui| {
                ui.label(text::HEADER_AUTHOR_LABEL);
                ui.text_edit_singleline(&mut d.header_author);
                ui.end_row();
                ui.label(text::HEADER_MANUFACTURER_LABEL);
                ui.text_edit_singleline(&mut d.header_manufacturer);
                ui.end_row();
                ui.label(text::HEADER_CREATED_WITH_LABEL);
                ui.text_edit_singleline(&mut d.header_created_with);
                ui.end_row();
                ui.label(text::HEADER_DEFAULT_LANGUAGE_LABEL);
                ui.text_edit_singleline(&mut d.header_default_language);
                ui.end_row();
            }),
            WizardStep::Photometry => {
                Self::file_slot(
                    ui,
                    "photometry (.ldt/.ies)",
                    &["ldt", "ies", "LDT", "IES"],
                    &mut d.photometry_bytes,
                    &mut d.photometry_filename,
                );
            }
            WizardStep::Sensor => {
                Self::file_slot(
                    ui,
                    "sensor sheet",
                    &["sensxml", "sensldt", "xml", "ldt"],
                    &mut d.sensor_bytes,
                    &mut d.sensor_filename,
                );
            }
            WizardStep::Emergency => {
                Self::file_slot(
                    ui,
                    "emergency photometry",
                    &["ldt", "ies", "LDT", "IES"],
                    &mut d.emergency_bytes,
                    &mut d.emergency_filename,
                );
            }
            WizardStep::LightSource => Self::grid(ui, "wz-ls", |ui| {
                ui.label(text::LS_KIND_LABEL);
                egui::ComboBox::from_id_salt("wz-ls-kind")
                    .selected_text(d.ls_kind.long_label())
                    .show_ui(ui, |ui| {
                        for kind in [
                            WizardLightSourceKind::Fixed,
                            WizardLightSourceKind::Changeable,
                            WizardLightSourceKind::MultiChannel,
                        ] {
                            ui.selectable_value(&mut d.ls_kind, kind, kind.long_label());
                        }
                    });
                ui.end_row();
                ui.label(text::LS_NAME_LABEL);
                ui.text_edit_singleline(&mut d.ls_name);
                ui.end_row();
                ui.label(text::LS_MANUFACTURER_LABEL);
                ui.text_edit_singleline(&mut d.ls_manufacturer);
                ui.end_row();
                ui.label(text::LS_POWER_LABEL);
                ui.text_edit_singleline(&mut d.ls_rated_power);
                ui.end_row();
            }),
            WizardStep::Emitter => {
                ui.checkbox(&mut self.include_emitter, text::EMITTER_INCLUDE_LABEL);
                if self.include_emitter {
                    Self::grid(ui, "wz-em", |ui| {
                        ui.label(text::EMITTER_NAME_LABEL);
                        ui.text_edit_singleline(&mut self.emitter_name);
                        ui.end_row();
                    });
                }
            }
            WizardStep::Geometry => {
                Self::file_slot(
                    ui,
                    "geometry (.l3d)",
                    &["l3d", "L3D"],
                    &mut d.geometry_bytes,
                    &mut d.geometry_filename,
                );
            }
            WizardStep::Mounting => Self::grid(ui, "wz-mount", |ui| {
                ui.label("Type");
                let choices = [
                    ("", "(none)"),
                    ("ceiling/recessed", "Ceiling — recessed"),
                    ("ceiling/surface", "Ceiling — surface"),
                    ("ceiling/pendant", "Ceiling — pendant"),
                    ("wall", "Wall"),
                    ("ground/pole_top", "Ground — pole top"),
                    ("ground/free_standing", "Ground — free standing"),
                    ("working_plane", "Working plane"),
                ];
                let sel = choices
                    .iter()
                    .find(|(v, _)| *v == d.mounting_type)
                    .map(|(_, l)| *l)
                    .unwrap_or("(none)");
                egui::ComboBox::from_id_salt("wz-mount-type")
                    .selected_text(sel)
                    .show_ui(ui, |ui| {
                        for (value, label) in choices {
                            ui.selectable_value(&mut d.mounting_type, value.to_string(), label);
                        }
                    });
                ui.end_row();
                match d.mounting_type.as_str() {
                    "ceiling/recessed" => {
                        ui.label("Recessed depth (mm)");
                        ui.text_edit_singleline(&mut d.mounting_recessed_depth_mm);
                        ui.end_row();
                    }
                    "ceiling/pendant" => {
                        ui.label("Pendant length (mm)");
                        ui.text_edit_singleline(&mut d.mounting_pendant_length_mm);
                        ui.end_row();
                    }
                    "wall" => {
                        ui.label("Mounting height (mm)");
                        ui.text_edit_singleline(&mut d.mounting_height_mm);
                        ui.end_row();
                    }
                    "ground/pole_top" => {
                        ui.label("Pole height (mm)");
                        ui.text_edit_singleline(&mut d.mounting_pole_height_mm);
                        ui.end_row();
                    }
                    _ => {}
                }
            }),
            WizardStep::Driver => {
                ui.checkbox(&mut d.driver_enabled, "Include driver / ballast");
                if d.driver_enabled {
                    Self::grid(ui, "wz-driver", |ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut d.driver_name);
                        ui.end_row();
                        ui.label("Dimmable");
                        ui.checkbox(&mut d.driver_dimmable, "");
                        ui.end_row();
                        ui.label("Standby power (W)");
                        ui.text_edit_singleline(&mut d.driver_standby_power);
                        ui.end_row();
                    });
                }
            }
            WizardStep::Variant => Self::grid(ui, "wz-variant", |ui| {
                ui.label(text::VARIANT_NAME_LABEL);
                ui.add(
                    egui::TextEdit::singleline(&mut d.variant_name)
                        .hint_text(text::VARIANT_NAME_PLACEHOLDER),
                );
                ui.end_row();
            }),
            WizardStep::Review => {
                Self::grid(ui, "wz-review", |ui| {
                    ui.label("Author");
                    ui.label(&d.header_author);
                    ui.end_row();
                    ui.label("Manufacturer");
                    ui.label(&d.header_manufacturer);
                    ui.end_row();
                    ui.label("Photometry");
                    ui.label(if d.photometry_bytes.is_some() {
                        d.photometry_filename.as_str()
                    } else {
                        text::REVIEW_PHOTOMETRY_SKIPPED
                    });
                    ui.end_row();
                    ui.label("Light source");
                    ui.label(format!("{} \"{}\"", d.ls_kind.short_label(), d.ls_name));
                    ui.end_row();
                    ui.label("Emitter");
                    ui.label(if self.include_emitter {
                        self.emitter_name.as_str()
                    } else {
                        text::REVIEW_EMITTER_SKIPPED
                    });
                    ui.end_row();
                    ui.label("Geometry");
                    ui.label(if d.geometry_bytes.is_some() {
                        d.geometry_filename.as_str()
                    } else {
                        "(none)"
                    });
                    ui.end_row();
                    ui.label("Variant");
                    ui.label(if d.variant_name.trim().is_empty() {
                        "Standard"
                    } else {
                        d.variant_name.as_str()
                    });
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.weak(text::REVIEW_FOOTNOTE);
            }
        }
    }
}
