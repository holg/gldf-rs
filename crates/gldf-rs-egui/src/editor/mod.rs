//! Native GLDF **editor** (egui) — the serious "can egui do the
//! sophisticated UI?" attempt.
//!
//! Architecture mirrors the WASM/TUI editors: every mutation goes
//! through the shared `gldf_rs_editor::GldfState` reducer as a
//! [`GldfAction`], so this frontend is only widgets + dispatch. The
//! sidebar/page layout follows the WASM editor; labels come from
//! `gldf-rs-i18n` with the same `gldf.nav.*` / `gldf.header.*` keys.
//!
//! Deliberately NOT dependent on three-d (`viewer3d` feature), so this
//! binary cross-builds like `gldf-batch-egui`.

mod leo_picker;
mod pages;
mod wizard_modal;

use eframe::egui;
use gldf_rs_editor::{GldfAction, GldfState};
use gldf_rs_i18n::GldfStaticTranslator;
use gldf_rs::i18n::Translator;
use std::path::PathBuf;

pub use wizard_modal::WizardModal;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Overview,
    Header,
    Files,
    Photometries,
    LightSources,
    Emitters,
    Variants,
    Batch,
}

impl Page {
    pub const ALL: &'static [Page] = &[
        Page::Overview,
        Page::Header,
        Page::Files,
        Page::Photometries,
        Page::LightSources,
        Page::Emitters,
        Page::Variants,
        Page::Batch,
    ];

    /// i18n key for the sidebar label (same keys the WASM editor uses).
    fn nav_key(self) -> &'static str {
        match self {
            Page::Overview => "gldf.nav.overview",
            Page::Header => "gldf.nav.header",
            Page::Files => "gldf.nav.files",
            Page::Photometries => "gldf.nav.photometry",
            Page::LightSources => "gldf.nav.light_sources",
            Page::Emitters => "gldf.nav.emitters",
            Page::Variants => "gldf.nav.variants",
            Page::Batch => "gldf.nav.batch",
        }
    }

    fn fallback_label(self) -> &'static str {
        match self {
            Page::Overview => "Overview",
            Page::Header => "Header",
            Page::Files => "Files",
            Page::Photometries => "Photometries",
            Page::LightSources => "Light sources",
            Page::Emitters => "Emitters",
            Page::Variants => "Variants",
            Page::Batch => "Batch convert",
        }
    }
}

/// Scratch state for the LEO picker's "new emitter" form.
#[derive(Default, Clone)]
pub struct LeoNewEmitterForm {
    pub photometry_id: String,
    pub light_source_id: String,
    pub flux: i32,
}

pub struct GldfEditorApp {
    pub state: GldfState,
    pub page: Page,
    pub file_path: Option<PathBuf>,
    tr: GldfStaticTranslator,
    /// Status-bar message; `true` = error styling.
    pub status: Option<(String, bool)>,
    pub wizard: Option<WizardModal>,

    // --- per-page scratch (immediate-mode UI state that isn't model) ---
    /// Selected `<File>` id on the Files page (drives the preview pane).
    pub sel_file: Option<String>,
    /// Selected photometry index on the Photometries page.
    pub sel_photometry: usize,
    /// Selected variant index on the Variants page.
    pub sel_variant: usize,
    /// Selected LEO part_name in the plan picker.
    pub leo_sel: Option<String>,
    pub leo_form: LeoNewEmitterForm,
    /// Rename staging: (kind, old_id) → buffer. Kind keeps buffers from
    /// leaking across entity types with colliding ids.
    pub rename_target: Option<(&'static str, String)>,
    pub rename_buf: String,

    /// Batch-conversion page state (worker thread, rows, template).
    pub batch: crate::batch_app::BatchPanel,

    /// Self-test harness: when `GLDF_EDITOR_SHOT_DIR` is set, the app
    /// walks every page, saves a screenshot of each into that dir, and
    /// exits. Lets CI (or an agent) verify the UI renders without a
    /// human driving it.
    autoshot: Option<AutoShot>,
}

struct AutoShot {
    dir: PathBuf,
    page_idx: usize,
    frames_on_page: usize,
}

impl GldfEditorApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            state: GldfState::default(),
            page: Page::Overview,
            file_path: None,
            tr: GldfStaticTranslator::new(),
            status: None,
            wizard: None,
            sel_file: None,
            sel_photometry: 0,
            sel_variant: 0,
            leo_sel: None,
            leo_form: LeoNewEmitterForm {
                flux: 1600,
                ..Default::default()
            },
            rename_target: None,
            rename_buf: String::new(),
            batch: crate::batch_app::BatchPanel::default(),
            autoshot: std::env::var("GLDF_EDITOR_SHOT_DIR").ok().map(|dir| AutoShot {
                dir: PathBuf::from(dir),
                page_idx: 0,
                frames_on_page: 0,
            }),
        }
    }

    /// Translate an i18n key for the current UI language, falling back
    /// to a humanized key tail so a missing key degrades gracefully.
    pub fn t(&self, key: &str) -> String {
        self.tr
            .t(key, &self.state.ui_language)
            .unwrap_or_else(|| {
                key.rsplit('.')
                    .next()
                    .unwrap_or(key)
                    .replace('_', " ")
            })
    }

    /// Dispatch through the shared reducer; surface reducer errors in
    /// the status bar (same contract as the WASM banner).
    pub fn dispatch(&mut self, action: GldfAction) {
        self.state.apply(action);
        if let Some(err) = self.state.last_error.clone() {
            self.status = Some((err, true));
        }
    }

    pub fn info(&mut self, msg: impl Into<String>) {
        self.status = Some((msg.into(), false));
    }

    fn open_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("GLDF", &["gldf"])
            .pick_file()
        else {
            return;
        };
        self.open_path(path);
    }

    pub fn open_path(&mut self, path: PathBuf) {
        match std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                gldf_rs::GldfProduct::load_gldf_from_buf_all(bytes).map_err(|e| e.to_string())
            }) {
            Ok(loaded) => {
                let mut embedded = std::collections::HashMap::new();
                for f in loaded.files {
                    if let (Some(id), Some(content)) = (f.file_id, f.content) {
                        embedded.insert(id, content);
                    }
                }
                self.dispatch(GldfAction::LoadWithFiles {
                    product: loaded.gldf,
                    embedded_files: embedded,
                });
                // Kick off background fetches for URL-type files so the
                // Files page preview is warm by the time it's opened.
                let urls: Vec<String> = self
                    .state
                    .product
                    .general_definitions
                    .files
                    .file
                    .iter()
                    .filter(|f| !self.state.embedded_files.contains_key(&f.id))
                    .map(|f| f.file_name.clone())
                    .collect();
                if !urls.is_empty() {
                    crate::preview::fetch_all_urls(urls);
                }
                self.file_path = Some(path.clone());
                self.sel_file = None;
                self.sel_photometry = 0;
                self.sel_variant = 0;
                self.leo_sel = None;
                self.info(format!("Loaded {}", path.display()));
            }
            Err(e) => self.status = Some((format!("open {}: {}", path.display(), e), true)),
        }
    }

    fn save(&mut self, ask_path: bool) {
        let path = if ask_path || self.file_path.is_none() {
            let mut dialog = rfd::FileDialog::new().add_filter("GLDF", &["gldf"]);
            if let Some(parent) = self.file_path.as_ref().and_then(|p| p.parent()) {
                dialog = dialog.set_directory(parent);
            }
            match dialog.set_file_name("product.gldf").save_file() {
                Some(p) => p,
                None => return,
            }
        } else {
            self.file_path.clone().unwrap()
        };
        match self.state.save_to_buf().and_then(|bytes| {
            std::fs::write(&path, bytes).map_err(anyhow::Error::from)
        }) {
            Ok(()) => {
                self.file_path = Some(path.clone());
                self.state.is_modified = false;
                self.info(format!("Saved {}", path.display()));
            }
            Err(e) => self.status = Some((format!("save: {:#}", e), true)),
        }
    }

    fn export_ifc(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("IFC", &["ifc"])
            .set_file_name("product.ifc")
            .save_file()
        else {
            return;
        };
        match self
            .state
            .export_ifc()
            .and_then(|ifc| std::fs::write(&path, ifc).map_err(anyhow::Error::from))
        {
            Ok(()) => self.info(format!("Exported {}", path.display())),
            Err(e) => self.status = Some((format!("ifc export: {:#}", e), true)),
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button(self.t("gldf.toolbar.open")).clicked() {
                self.open_dialog();
            }
            if ui.button(self.t("gldf.toolbar.save")).clicked() {
                self.save(false);
            }
            if ui.button(self.t("gldf.toolbar.save_as")).clicked() {
                self.save(true);
            }
            if ui.button(self.t("gldf.toolbar.wizard")).clicked() {
                self.wizard = Some(WizardModal::default());
            }
            if ui.button("Export IFC").clicked() {
                self.export_ifc();
            }

            ui.separator();

            // UI language switcher — persists via the reducer's
            // SetUiLanguage arm, same as every other frontend.
            let current = self.state.ui_language.clone();
            let mut picked: Option<String> = None;
            let native = |code: &str| -> String {
                gldf_rs_editor::ui_language_native_name(code)
                    .map(str::to_string)
                    .unwrap_or_else(|| code.to_string())
            };
            egui::ComboBox::from_id_salt("ui-lang")
                .selected_text(native(&current))
                .width(110.0)
                .show_ui(ui, |ui| {
                    for lang in gldf_rs_editor::SUPPORTED_UI_LANGUAGES {
                        if ui
                            .selectable_label(*lang == current, native(lang))
                            .clicked()
                        {
                            picked = Some((*lang).to_string());
                        }
                    }
                });
            if let Some(lang) = picked {
                self.dispatch(GldfAction::SetUiLanguage(lang));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let name = self
                    .file_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("(unsaved)");
                let dirty = if self.state.is_modified { " ●" } else { "" };
                ui.label(format!("{}{}", name, dirty));
            });
        });
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        for page in Page::ALL {
            let label = self
                .tr
                .t(page.nav_key(), &self.state.ui_language)
                .unwrap_or_else(|| page.fallback_label().to_string());
            if ui
                .selectable_label(self.page == *page, label)
                .clicked()
            {
                self.page = *page;
            }
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            match &self.status {
                Some((msg, true)) => {
                    ui.colored_label(ui.visuals().error_fg_color, format!("⚠ {}", msg));
                }
                Some((msg, false)) => {
                    ui.weak(msg.clone());
                }
                None => {
                    ui.weak("");
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let gd = &self.state.product.general_definitions;
                let n_files = gd.files.file.len();
                let n_var = self
                    .state
                    .product
                    .product_definitions
                    .variants
                    .as_ref()
                    .map(|v| v.variant.len())
                    .unwrap_or(0);
                ui.weak(format!("{} files · {} variants", n_files, n_var));
            });
        });
    }

    /// Route dropped files. On the Batch page, drops feed the batch
    /// input list (paths, read at run time); everywhere else they land
    /// in the document via the same extension sniffer the WASM editor
    /// uses.
    fn handle_dropped(&mut self, ctx: &egui::Context) {
        let paths: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if paths.is_empty() {
            return;
        }
        if self.page == Page::Batch {
            self.batch.add_dropped(paths);
            return;
        }
        for path in paths {
            match std::fs::read(&path) {
                Ok(bytes) => self.add_file_bytes(&path, bytes),
                Err(e) => {
                    self.status = Some((format!("read {}: {}", path.display(), e), true))
                }
            }
        }
    }

    pub fn add_file_bytes(&mut self, path: &std::path::Path, bytes: Vec<u8>) {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let action = match ext.as_str() {
            "gldf" => {
                // A dropped GLDF replaces the document, not a file entry.
                self.open_path(path.to_path_buf());
                return;
            }
            "ldt" | "ies" => GldfAction::AddPhotometryFile { bytes, filename },
            "l3d" => GldfAction::ImportL3dFile { bytes, filename },
            "sensxml" | "sensldt" => GldfAction::AddSensorFile { bytes, filename },
            "jpg" | "jpeg" | "png" | "webp" => GldfAction::AddProductPictureFile {
                bytes,
                filename,
                image_type: "Product Picture".to_string(),
            },
            "spd" => {
                // Spectrum needs a light source to hang off; fall back to
                // a bare file when none exists yet.
                let ls_id = self
                    .state
                    .product
                    .general_definitions
                    .light_sources
                    .as_ref()
                    .and_then(|ls| ls.fixed_light_source.first())
                    .map(|l| l.id.clone());
                match ls_id {
                    Some(light_source_id) => GldfAction::AddSpectrumFile {
                        bytes,
                        filename,
                        light_source_id,
                    },
                    None => GldfAction::AddFileWithBytes { bytes, filename },
                }
            }
            _ => GldfAction::AddFileWithBytes { bytes, filename },
        };
        self.dispatch(action);
        if self.state.last_error.is_none() {
            self.info(format!("Added {}", path.display()));
        }
    }
}

impl GldfEditorApp {
    /// Autoshot pump: capture-per-page, advance, exit after the last.
    fn autoshot_tick(&mut self, ctx: &egui::Context) {
        if self.autoshot.is_none() {
            return;
        }

        // Harvest a screenshot delivered for the PREVIOUS request.
        let shot: Option<std::sync::Arc<egui::ColorImage>> = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = shot {
            let a = self.autoshot.as_mut().unwrap();
            let label = Page::ALL
                .get(a.page_idx)
                .map(|p| p.fallback_label().to_lowercase().replace(' ', "-"))
                .unwrap_or_else(|| "wizard".to_string());
            let _ = std::fs::create_dir_all(&a.dir);
            let path = a.dir.join(format!("{:02}-{}.png", a.page_idx, label));
            let px: Vec<u8> = image
                .pixels
                .iter()
                .flat_map(|c| c.to_array())
                .collect();
            if let Some(img) = image::RgbaImage::from_raw(
                image.width() as u32,
                image.height() as u32,
                px,
            ) {
                let _ = img.save(&path);
                eprintln!("autoshot: {}", path.display());
            }
            // Past the last page, one extra frame captures the wizard.
            if a.page_idx >= Page::ALL.len() {
                std::process::exit(0);
            }
            a.page_idx += 1;
            a.frames_on_page = 0;
            if a.page_idx >= Page::ALL.len() {
                self.wizard = Some(WizardModal::default());
            } else {
                self.page = Page::ALL[a.page_idx];
            }
            // Pre-select first entities so detail panes render content.
            self.sel_file = self
                .state
                .product
                .general_definitions
                .files
                .file
                .first()
                .map(|f| f.id.clone());
        }

        let a = self.autoshot.as_mut().unwrap();
        a.frames_on_page += 1;
        if a.frames_on_page == 4 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        ctx.request_repaint();
    }
}

impl eframe::App for GldfEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_dropped(ctx);
        self.autoshot_tick(ctx);

        // Wizard modal floats above everything; it owns its own window.
        if let Some(mut wizard) = self.wizard.take() {
            let keep = wizard.show(ctx, self);
            if keep {
                self.wizard = Some(wizard);
            }
        }

        egui::TopBottomPanel::top("editor-top").show(ctx, |ui| {
            ui.add_space(4.0);
            self.toolbar(ui);
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("editor-status").show(ctx, |ui| {
            ui.add_space(2.0);
            self.status_bar(ui);
            ui.add_space(2.0);
        });

        egui::SidePanel::left("editor-nav")
            .resizable(false)
            .default_width(150.0)
            .show(ctx, |ui| {
                self.sidebar(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| match self.page {
                Page::Overview => pages::overview(self, ui),
                Page::Header => pages::header(self, ui),
                Page::Files => pages::files(self, ui),
                Page::Photometries => pages::photometries(self, ui),
                Page::LightSources => pages::light_sources(self, ui),
                Page::Emitters => pages::emitters(self, ui),
                Page::Variants => pages::variants(self, ui),
                Page::Batch => {
                    let heading = self
                        .tr
                        .t("gldf.nav.batch", &self.state.ui_language)
                        .unwrap_or_else(|| Page::Batch.fallback_label().to_string());
                    ui.heading(heading);
                    ui.add_space(4.0);
                    self.batch.ui(ui);
                }
            });
        });
    }
}

// --- Small shared helpers for LocaleFoo round-tripping in text fields ---

use gldf_rs::gldf::header::{Locale, LocaleFoo};

/// First-locale text of an optional LocaleFoo (the editing convention all
/// frontends use for the "default language" value).
pub fn locale_text(l: Option<&LocaleFoo>) -> String {
    l.and_then(|n| n.locale.first())
        .map(|x| x.value.clone())
        .unwrap_or_default()
}

/// Build a LocaleFoo from a plain string (empty → empty LocaleFoo).
pub fn locale_from(s: &str) -> LocaleFoo {
    if s.trim().is_empty() {
        LocaleFoo::default()
    } else {
        LocaleFoo {
            locale: vec![Locale {
                language: String::new(),
                value: s.to_string(),
            }],
        }
    }
}
