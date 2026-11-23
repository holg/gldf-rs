//! Template editor window for the batch converter: a form over
//! `gldf_rs_editor::batch::TemplateDraft` (the nice way) plus a raw
//! JSON tab (the escape hatch, and the only view for `GldfAction`
//! script templates). Saving hands the path back to the batch app,
//! which makes it the active template.

use eframe::egui;
use gldf_rs_editor::batch::TemplateDraft;
use gldf_rs_editor::WizardLightSourceKind;
use std::path::{Path, PathBuf};

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Form,
    Json,
}

pub enum TemplateEditorEvent {
    None,
    /// The template was written to this path — make it active.
    Saved(PathBuf),
}

pub struct TemplateEditor {
    tab: Tab,
    form: TemplateDraft,
    /// Emitter display name mirror (the draft stores it inside an
    /// `Option<EmitterDraft>`; empty → autofill creates "Primary").
    emitter_name: String,
    /// Whether the current JSON maps onto the form (false for action
    /// scripts / templates with inline byte arrays → JSON tab only).
    form_available: bool,
    json_text: String,
    json_error: Option<String>,
    path: Option<PathBuf>,
}

const MOUNTING_CHOICES: [(&str, &str); 8] = [
    ("", "(none)"),
    ("ceiling/recessed", "Ceiling — recessed"),
    ("ceiling/surface", "Ceiling — surface"),
    ("ceiling/pendant", "Ceiling — pendant"),
    ("wall", "Wall"),
    ("ground/pole_top", "Ground — pole top"),
    ("ground/free_standing", "Ground — free standing"),
    ("working_plane", "Working plane"),
];

const PLACEHOLDERS: [(&str, &str); 10] = [
    ("{input.filename}", "input file name, e.g. spot_930.ldt"),
    ("{input.stem}", "input file name without extension"),
    ("{ldt.name}", "luminaire name from the photometry"),
    ("{ldt.manufacturer}", "manufacturer (EULUMDAT line 1)"),
    ("{ldt.luminaire_number}", "luminaire number / article"),
    ("{ldt.lamp_type}", "lamp type description"),
    ("{ldt.lamp_count}", "number of lamps"),
    ("{ldt.watts}", "total wattage"),
    ("{ldt.lumens}", "total luminous flux"),
    ("{ldt.measurement_report}", "measurement report number"),
];

impl TemplateEditor {
    pub fn new_blank() -> Self {
        let mut form = TemplateDraft::default();
        form.draft.header_created_with = "gldf-batch-egui".to_string();
        let json_text = form.to_template_json();
        Self {
            tab: Tab::Form,
            form,
            emitter_name: String::new(),
            form_available: true,
            json_text,
            json_error: None,
            path: None,
        }
    }

    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let value: serde_json::Value = serde_json::from_str(&raw)?;
        let mut editor = match TemplateDraft::from_template_value(&value) {
            Ok(form) => {
                let emitter_name = form
                    .draft
                    .emitter_draft
                    .as_ref()
                    .map(|e| e.child_name.clone())
                    .unwrap_or_default();
                Self {
                    tab: Tab::Form,
                    form,
                    emitter_name,
                    form_available: true,
                    json_text: raw,
                    json_error: None,
                    path: Some(path.to_path_buf()),
                }
            }
            Err(_) => Self {
                tab: Tab::Json,
                form: TemplateDraft::default(),
                emitter_name: String::new(),
                form_available: false,
                json_text: raw,
                json_error: None,
                path: Some(path.to_path_buf()),
            },
        };
        if editor.form_available {
            // Normalise the display to what the form will re-emit.
            editor.json_text = editor.form.to_template_json();
        }
        Ok(editor)
    }

    /// Render the window. Returns `(keep_open, event)`.
    pub fn show(&mut self, ctx: &egui::Context) -> (bool, TemplateEditorEvent) {
        let mut open = true;
        let mut event = TemplateEditorEvent::None;
        let title = match &self.path {
            Some(p) => format!(
                "Template — {}",
                p.file_name().and_then(|n| n.to_str()).unwrap_or("?")
            ),
            None => "Template — new".to_string(),
        };
        egui::Window::new(title)
            .id(egui::Id::new("template-editor"))
            .open(&mut open)
            .default_size([560.0, 560.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(self.form_available, |ui| {
                        if ui.selectable_label(self.tab == Tab::Form, "Form").clicked() {
                            self.tab = Tab::Form;
                        }
                    });
                    if ui.selectable_label(self.tab == Tab::Json, "JSON").clicked() {
                        // Leaving the form: its JSON is already synced below.
                        self.tab = Tab::Json;
                    }
                    if !self.form_available {
                        ui.weak("action-script template — JSON only");
                    }
                });
                ui.separator();

                match self.tab {
                    Tab::Form => {
                        egui::ScrollArea::vertical()
                            .max_height(420.0)
                            .show(ui, |ui| {
                                self.form_ui(ui);
                            });
                        // Keep the JSON view in lockstep with the form.
                        self.sync_emitter();
                        self.json_text = self.form.to_template_json();
                        self.json_error = None;
                    }
                    Tab::Json => {
                        egui::ScrollArea::vertical()
                            .max_height(420.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.json_text)
                                        .code_editor()
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(24),
                                );
                            });
                        ui.horizontal(|ui| {
                            if ui.button("Apply to form").clicked() {
                                self.apply_json_to_form();
                            }
                            if let Some(err) = &self.json_error {
                                ui.colored_label(ui.visuals().error_fg_color, err.clone());
                            }
                        });
                    }
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Save…").clicked() {
                        if let Some(saved) = self.save_as() {
                            event = TemplateEditorEvent::Saved(saved);
                        }
                    }
                    if let Some(path) = self.path.clone() {
                        if ui.button("Save").clicked() {
                            if let Some(saved) = self.save_to(&path) {
                                event = TemplateEditorEvent::Saved(saved);
                            }
                        }
                    }
                    if let Some(err) = &self.json_error {
                        if self.tab == Tab::Form {
                            ui.colored_label(ui.visuals().error_fg_color, err.clone());
                        }
                    }
                });
            });
        (open, event)
    }

    fn sync_emitter(&mut self) {
        use gldf_rs_editor::{EmitterDraft, LightEmitterKind};
        let kind = match self.form.draft.ls_kind {
            WizardLightSourceKind::Fixed => LightEmitterKind::Fixed,
            WizardLightSourceKind::Changeable => LightEmitterKind::Changeable,
            WizardLightSourceKind::MultiChannel => LightEmitterKind::MultiChannel,
        };
        if self.emitter_name.trim().is_empty() {
            self.form.draft.emitter_draft = None;
        } else {
            self.form.draft.emitter_draft = Some(EmitterDraft {
                child_name: self.emitter_name.clone(),
                kind,
                photometry_id: String::new(),
                fixed_light_source_id: String::new(),
                multi_channel_light_source_id: String::new(),
                sensor_id: String::new(),
            });
        }
    }

    fn apply_json_to_form(&mut self) {
        match serde_json::from_str::<serde_json::Value>(&self.json_text)
            .map_err(|e| e.to_string())
            .and_then(|v| TemplateDraft::from_template_value(&v).map_err(|e| format!("{:#}", e)))
        {
            Ok(form) => {
                self.emitter_name = form
                    .draft
                    .emitter_draft
                    .as_ref()
                    .map(|e| e.child_name.clone())
                    .unwrap_or_default();
                self.form = form;
                self.form_available = true;
                self.json_error = None;
                self.tab = Tab::Form;
            }
            Err(e) => self.json_error = Some(e),
        }
    }

    fn save_as(&mut self) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new().add_filter("JSON template", &["json"]);
        if let Some(parent) = self.path.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(parent);
        }
        let target = dialog
            .set_file_name(
                self.path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("batch_template.json"),
            )
            .save_file()?;
        self.save_to(&target)
    }

    fn save_to(&mut self, path: &Path) -> Option<PathBuf> {
        match std::fs::write(path, self.json_text.as_bytes()) {
            Ok(()) => {
                self.path = Some(path.to_path_buf());
                self.json_error = None;
                Some(path.to_path_buf())
            }
            Err(e) => {
                self.json_error = Some(format!("save {}: {}", path.display(), e));
                None
            }
        }
    }

    fn form_ui(&mut self, ui: &mut egui::Ui) {
        let draft = &mut self.form.draft;

        ui.label(
            egui::RichText::new(
                "Every field is optional — blanks are auto-filled from each input \
                 photometry. Text fields accept placeholders (see below).",
            )
            .weak(),
        );
        ui.add_space(4.0);

        ui.strong("Header");
        egui::Grid::new("tpl-header").num_columns(2).show(ui, |ui| {
            ui.label("Author");
            ui.text_edit_singleline(&mut draft.header_author);
            ui.end_row();
            ui.label("Manufacturer");
            ui.text_edit_singleline(&mut draft.header_manufacturer);
            ui.end_row();
            ui.label("Created with");
            ui.text_edit_singleline(&mut draft.header_created_with);
            ui.end_row();
            ui.label("Default language");
            ui.text_edit_singleline(&mut draft.header_default_language);
            ui.end_row();
        });

        ui.add_space(6.0);
        ui.strong("Light source");
        egui::Grid::new("tpl-ls").num_columns(2).show(ui, |ui| {
            ui.label("Kind");
            egui::ComboBox::from_id_salt("tpl-ls-kind")
                .selected_text(match draft.ls_kind {
                    WizardLightSourceKind::Fixed => "Fixed",
                    WizardLightSourceKind::Changeable => "Changeable",
                    WizardLightSourceKind::MultiChannel => "MultiChannel",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut draft.ls_kind, WizardLightSourceKind::Fixed, "Fixed");
                    ui.selectable_value(
                        &mut draft.ls_kind,
                        WizardLightSourceKind::Changeable,
                        "Changeable",
                    );
                    ui.selectable_value(
                        &mut draft.ls_kind,
                        WizardLightSourceKind::MultiChannel,
                        "MultiChannel",
                    );
                });
            ui.end_row();
            ui.label("Name");
            ui.text_edit_singleline(&mut draft.ls_name);
            ui.end_row();
            ui.label("Manufacturer");
            ui.text_edit_singleline(&mut draft.ls_manufacturer);
            ui.end_row();
            ui.label("Rated power (W)");
            ui.text_edit_singleline(&mut draft.ls_rated_power);
            ui.end_row();
        });

        ui.add_space(6.0);
        ui.strong("Emitter & variant");
        egui::Grid::new("tpl-var").num_columns(2).show(ui, |ui| {
            ui.label("Emitter name");
            ui.add(egui::TextEdit::singleline(&mut self.emitter_name).hint_text("Primary (auto)"));
            ui.end_row();
            ui.label("Variant name");
            ui.add(
                egui::TextEdit::singleline(&mut draft.variant_name).hint_text("Standard (auto)"),
            );
            ui.end_row();
        });

        ui.add_space(6.0);
        ui.strong("Mounting");
        egui::Grid::new("tpl-mount").num_columns(2).show(ui, |ui| {
            ui.label("Type");
            let selected_label = MOUNTING_CHOICES
                .iter()
                .find(|(v, _)| *v == draft.mounting_type)
                .map(|(_, l)| *l)
                .unwrap_or("(none)");
            egui::ComboBox::from_id_salt("tpl-mount-type")
                .selected_text(selected_label)
                .show_ui(ui, |ui| {
                    for (value, label) in MOUNTING_CHOICES {
                        ui.selectable_value(&mut draft.mounting_type, value.to_string(), label);
                    }
                });
            ui.end_row();
            match draft.mounting_type.as_str() {
                "ceiling/recessed" => {
                    ui.label("Recessed depth (mm)");
                    ui.text_edit_singleline(&mut draft.mounting_recessed_depth_mm);
                    ui.end_row();
                }
                "ceiling/pendant" => {
                    ui.label("Pendant length (mm)");
                    ui.text_edit_singleline(&mut draft.mounting_pendant_length_mm);
                    ui.end_row();
                }
                "wall" => {
                    ui.label("Mounting height (mm)");
                    ui.text_edit_singleline(&mut draft.mounting_height_mm);
                    ui.end_row();
                    ui.label("Recessed depth (mm, optional)");
                    ui.text_edit_singleline(&mut draft.mounting_recessed_depth_mm);
                    ui.end_row();
                }
                "ground/pole_top" => {
                    ui.label("Pole height (mm)");
                    ui.text_edit_singleline(&mut draft.mounting_pole_height_mm);
                    ui.end_row();
                }
                _ => {}
            }
        });

        ui.add_space(6.0);
        ui.strong("Driver");
        egui::Grid::new("tpl-driver").num_columns(2).show(ui, |ui| {
            ui.label("Include driver");
            ui.checkbox(&mut draft.driver_enabled, "");
            ui.end_row();
            if draft.driver_enabled {
                ui.label("Name");
                ui.text_edit_singleline(&mut draft.driver_name);
                ui.end_row();
                ui.label("Dimmable");
                ui.checkbox(&mut draft.driver_dimmable, "");
                ui.end_row();
                ui.label("Standby power (W)");
                ui.text_edit_singleline(&mut draft.driver_standby_power);
                ui.end_row();
            }
        });

        ui.add_space(6.0);
        ui.strong("Shared files (applied to every output)");
        egui::Grid::new("tpl-files").num_columns(2).show(ui, |ui| {
            for (label, field, filters) in [
                (
                    "Geometry (.l3d)",
                    &mut self.form.geometry_file,
                    &["l3d"][..],
                ),
                (
                    "Sensor sheet",
                    &mut self.form.sensor_file,
                    &["sensxml", "sensldt", "xml", "ldt"][..],
                ),
                (
                    "Emergency photometry",
                    &mut self.form.emergency_file,
                    &["ldt", "ies"][..],
                ),
            ] {
                ui.label(label);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(field)
                            .hint_text("path relative to the template (or absolute)"),
                    );
                    if ui.button("Browse…").clicked() {
                        if let Some(picked) = rfd::FileDialog::new()
                            .add_filter(label, filters)
                            .pick_file()
                        {
                            *field = picked.display().to_string();
                        }
                    }
                    if !field.is_empty() && ui.button("✕").clicked() {
                        field.clear();
                    }
                });
                ui.end_row();
            }
        });

        ui.add_space(6.0);
        ui.collapsing("Placeholders", |ui| {
            egui::Grid::new("tpl-ph").num_columns(2).show(ui, |ui| {
                for (ph, desc) in PLACEHOLDERS {
                    if ui
                        .button(egui::RichText::new(ph).monospace())
                        .on_hover_text("click to copy")
                        .clicked()
                    {
                        ui.ctx().copy_text(ph.to_string());
                    }
                    ui.weak(desc);
                    ui.end_row();
                }
            });
        });
    }
}
