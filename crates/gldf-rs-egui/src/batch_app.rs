//! Batch-conversion panel: a folder of `.ldt` / `.ies` (and `.uld`)
//! photometries plus one JSON template → one GLDF (and optionally IFC)
//! per input.
//!
//! Thin egui shell over `gldf_rs_editor::batch` — template parsing,
//! placeholder substitution, conversion, and output writing all live in
//! the lib (shared with `gldf-cli batch`); this file only owns the
//! widgets and the worker-thread plumbing. Hosted as the editor app's
//! "Batch convert" page (it used to be a standalone `gldf-batch-egui`
//! binary; one app is enough).

use crate::template_editor::{TemplateEditor, TemplateEditorEvent};
use eframe::egui;
use gldf_rs_editor::batch::{
    collect_inputs, run_batch, BatchEvent, BatchJob, BatchOptions, BatchTemplate, FileReport,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

/// Per-input row in the results table.
#[derive(Clone)]
enum RowStatus {
    Pending,
    Running,
    Done(FileReport),
    Failed(String),
}

struct Row {
    input: PathBuf,
    status: RowStatus,
}

enum WorkerMsg {
    Started { index: usize },
    Finished { index: usize, report: FileReport },
    Fatal(String),
    AllDone,
}

struct RunHandle {
    rx: Receiver<WorkerMsg>,
    cancel: Arc<AtomicBool>,
}

pub struct BatchPanel {
    template_path: Option<PathBuf>,
    /// Parse feedback shown next to the template picker: Ok(summary) /
    /// Err(message). `None` until a template is picked.
    template_status: Option<Result<String, String>>,
    rows: Vec<Row>,
    out_dir: Option<PathBuf>,
    write_gldf: bool,
    write_ifc: bool,
    run: Option<RunHandle>,
    summary: Option<String>,
    template_editor: Option<TemplateEditor>,
}

impl Default for BatchPanel {
    fn default() -> Self {
        Self {
            template_path: None,
            template_status: None,
            rows: Vec::new(),
            out_dir: None,
            write_gldf: true,
            write_ifc: true,
            run: None,
            summary: None,
            template_editor: None,
        }
    }
}

impl BatchPanel {
    fn set_template(&mut self, path: PathBuf) {
        self.template_status = Some(match load_template(&path) {
            Ok(BatchTemplate::Wizard(_)) => Ok("wizard template".to_string()),
            Ok(BatchTemplate::Script(v)) => Ok(format!(
                "action script ({} actions)",
                v.as_array().map(|a| a.len()).unwrap_or(0)
            )),
            Err(e) => Err(format!("{:#}", e)),
        });
        self.template_path = Some(path);
    }

    fn add_inputs(&mut self, paths: &[PathBuf]) {
        let found = collect_inputs(paths);
        for input in found {
            if !self.rows.iter().any(|r| r.input == input) {
                self.rows.push(Row {
                    input,
                    status: RowStatus::Pending,
                });
            }
        }
        // Suggest an output directory next to the inputs on first add.
        if self.out_dir.is_none() {
            if let Some(first) = self.rows.first() {
                if let Some(parent) = first.input.parent() {
                    self.out_dir = Some(parent.join("gldf-batch-out"));
                }
            }
        }
    }

    /// Entry point for files dropped while the batch page is active:
    /// a `.json` becomes the template, everything else feeds the input
    /// list (directories are walked for photometries).
    pub fn add_dropped(&mut self, paths: Vec<PathBuf>) {
        if self.run.is_some() {
            return;
        }
        let mut inputs = Vec::new();
        for path in paths {
            if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("json"))
                .unwrap_or(false)
            {
                self.set_template(path);
            } else {
                inputs.push(path);
            }
        }
        if !inputs.is_empty() {
            self.add_inputs(&inputs);
        }
    }

    fn can_run(&self) -> Option<String> {
        if self.run.is_some() {
            return Some("already running".to_string());
        }
        if self.rows.is_empty() {
            return Some("add .ldt / .ies / .uld files first".to_string());
        }
        if self.template_path.is_none() {
            return Some("pick a template JSON first".to_string());
        }
        if matches!(self.template_status, Some(Err(_))) {
            return Some("template does not parse".to_string());
        }
        if self.out_dir.is_none() {
            return Some("pick an output directory".to_string());
        }
        if !self.write_gldf && !self.write_ifc {
            return Some("enable GLDF and/or IFC output".to_string());
        }
        None
    }

    fn start_run(&mut self, ctx: &egui::Context) {
        let template_path = match &self.template_path {
            Some(p) => p.clone(),
            None => return,
        };
        let template = match load_template(&template_path) {
            Ok(t) => t,
            Err(e) => {
                self.summary = Some(format!("template error: {:#}", e));
                return;
            }
        };
        let out_dir = match &self.out_dir {
            Some(d) => d.clone(),
            None => return,
        };

        for row in &mut self.rows {
            row.status = RowStatus::Pending;
        }
        self.summary = None;

        let job = BatchJob {
            template,
            template_dir: template_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
            inputs: self.rows.iter().map(|r| r.input.clone()).collect(),
            out_dir,
            options: BatchOptions {
                write_gldf: self.write_gldf,
                write_ifc: self.write_ifc,
            },
        };

        let (tx, rx): (Sender<WorkerMsg>, Receiver<WorkerMsg>) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_worker = cancel.clone();
        let repaint = ctx.clone();
        std::thread::spawn(move || {
            let result = run_batch(&job, &mut |ev| {
                match ev {
                    BatchEvent::Started { index, .. } => {
                        let _ = tx.send(WorkerMsg::Started { index });
                    }
                    BatchEvent::Finished { index, report, .. } => {
                        let _ = tx.send(WorkerMsg::Finished {
                            index,
                            report: report.clone(),
                        });
                    }
                }
                repaint.request_repaint();
                !cancel_worker.load(Ordering::Relaxed)
            });
            if let Err(e) = result {
                let _ = tx.send(WorkerMsg::Fatal(format!("{:#}", e)));
            }
            let _ = tx.send(WorkerMsg::AllDone);
            repaint.request_repaint();
        });
        self.run = Some(RunHandle { rx, cancel });
    }

    fn pump_worker(&mut self) {
        let Some(run) = &self.run else { return };
        let mut finished = false;
        while let Ok(msg) = run.rx.try_recv() {
            match msg {
                WorkerMsg::Started { index } => {
                    if let Some(row) = self.rows.get_mut(index) {
                        row.status = RowStatus::Running;
                    }
                }
                WorkerMsg::Finished { index, report } => {
                    if let Some(row) = self.rows.get_mut(index) {
                        row.status = match &report.error {
                            Some(err) => RowStatus::Failed(err.clone()),
                            None => RowStatus::Done(report),
                        };
                    }
                }
                WorkerMsg::Fatal(err) => {
                    self.summary = Some(format!("batch failed: {}", err));
                }
                WorkerMsg::AllDone => finished = true,
            }
        }
        if finished {
            self.run = None;
            let ok = self
                .rows
                .iter()
                .filter(|r| matches!(r.status, RowStatus::Done(_)))
                .count();
            let failed = self
                .rows
                .iter()
                .filter(|r| matches!(r.status, RowStatus::Failed(_)))
                .count();
            let skipped = self.rows.len() - ok - failed;
            let mut text = format!("done: {} ok, {} failed", ok, failed);
            if skipped > 0 {
                text.push_str(&format!(", {} skipped (cancelled)", skipped));
            }
            if self.summary.is_none() {
                self.summary = Some(text);
            }
        }
    }

    /// Render the whole panel into the host page. The host owns window
    /// chrome; this draws config, the results table, and the run bar.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.pump_worker();

        // Template editor floats as its own window above the app.
        if let Some(editor) = &mut self.template_editor {
            let (keep_open, event) = editor.show(ui.ctx());
            if let TemplateEditorEvent::Saved(path) = event {
                self.set_template(path);
            }
            if !keep_open {
                self.template_editor = None;
            }
        }

        ui.weak(
            "Photometry files (.ldt / .ies / .uld) + JSON template in, GLDF / IFC out. \
             Drops land here while this page is open.",
        );
        ui.add_space(6.0);
        self.config_panel(ui);
        ui.add_space(6.0);
        self.run_bar(ui);
        ui.add_space(6.0);
        ui.separator();

        if self.rows.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.weak("Drop .ldt / .ies / .uld files or folders here");
            });
        } else {
            self.results_table(ui);
        }
    }

    fn config_panel(&mut self, ui: &mut egui::Ui) {
        let running = self.run.is_some();
        ui.add_enabled_ui(!running, |ui| {
            egui::Grid::new("batch-config")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Template");
                    ui.horizontal(|ui| {
                        if ui.button("New…").clicked() {
                            self.template_editor = Some(TemplateEditor::new_blank());
                        }
                        if ui.button("Pick JSON…").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("JSON template", &["json"])
                                .pick_file()
                            {
                                self.set_template(path);
                            }
                        }
                        if let Some(path) = self.template_path.clone() {
                            if ui.button("Edit…").clicked() {
                                match TemplateEditor::from_file(&path) {
                                    Ok(editor) => self.template_editor = Some(editor),
                                    Err(e) => {
                                        self.summary = Some(format!("open template: {:#}", e));
                                    }
                                }
                            }
                        }
                        match (&self.template_path, &self.template_status) {
                            (Some(p), Some(Ok(kind))) => {
                                ui.label(format!("{} ({})", p.display(), kind));
                            }
                            (Some(p), Some(Err(e))) => {
                                ui.colored_label(
                                    ui.visuals().error_fg_color,
                                    format!("{} — {}", p.display(), e),
                                );
                            }
                            _ => {
                                ui.weak("drop a template .json here or pick one");
                            }
                        }
                    });
                    ui.end_row();

                    ui.label("Inputs");
                    ui.horizontal(|ui| {
                        if ui.button("Add files…").clicked() {
                            if let Some(paths) = rfd::FileDialog::new()
                                .add_filter(
                                    "Photometry",
                                    &["ldt", "ies", "uld", "LDT", "IES", "ULD"],
                                )
                                .pick_files()
                            {
                                self.add_inputs(&paths);
                            }
                        }
                        if ui.button("Add folder…").clicked() {
                            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                self.add_inputs(&[dir]);
                            }
                        }
                        if ui.button("Clear").clicked() {
                            self.rows.clear();
                            self.summary = None;
                        }
                        ui.weak(format!("{} file(s)", self.rows.len()));
                    });
                    ui.end_row();

                    ui.label("Output dir");
                    ui.horizontal(|ui| {
                        if ui.button("Pick…").clicked() {
                            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                self.out_dir = Some(dir);
                            }
                        }
                        match &self.out_dir {
                            Some(d) => ui.label(d.display().to_string()),
                            None => ui.weak("not set"),
                        };
                    });
                    ui.end_row();

                    ui.label("Formats");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.write_gldf, "GLDF");
                        ui.checkbox(&mut self.write_ifc, "IFC");
                    });
                    ui.end_row();
                });
        });
    }

    fn run_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.run.is_some() {
                if ui.button("Cancel").clicked() {
                    if let Some(run) = &self.run {
                        run.cancel.store(true, Ordering::Relaxed);
                    }
                }
                let done = self
                    .rows
                    .iter()
                    .filter(|r| matches!(r.status, RowStatus::Done(_) | RowStatus::Failed(_)))
                    .count();
                let total = self.rows.len().max(1);
                ui.add(
                    egui::ProgressBar::new(done as f32 / total as f32)
                        .text(format!("{}/{}", done, self.rows.len()))
                        .desired_width(280.0),
                );
            } else {
                let blocker = self.can_run();
                let button = ui.add_enabled(blocker.is_none(), egui::Button::new("Run batch"));
                if let Some(reason) = &blocker {
                    button.on_disabled_hover_text(reason.clone());
                } else if button.clicked() {
                    let ctx = ui.ctx().clone();
                    self.start_run(&ctx);
                }
                if let Some(out_dir) = self.out_dir.clone() {
                    let any_done = self
                        .rows
                        .iter()
                        .any(|r| matches!(r.status, RowStatus::Done(_)));
                    if any_done && ui.button("Open output folder").clicked() {
                        let _ = open_in_file_manager(&out_dir);
                    }
                }
            }
            if let Some(summary) = &self.summary {
                ui.separator();
                ui.label(summary.clone());
            }
        });
    }

    fn results_table(&mut self, ui: &mut egui::Ui) {
        use egui_extras::{Column, TableBuilder};
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size + 6.0;
        TableBuilder::new(ui)
            .striped(true)
            .column(Column::auto().at_least(220.0).resizable(true))
            .column(Column::auto().at_least(80.0))
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Input");
                });
                header.col(|ui| {
                    ui.strong("Status");
                });
                header.col(|ui| {
                    ui.strong("Result");
                });
            })
            .body(|body| {
                body.rows(text_height, self.rows.len(), |mut table_row| {
                    let row = &self.rows[table_row.index()];
                    table_row.col(|ui| {
                        let name = row
                            .input
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("?");
                        ui.label(name)
                            .on_hover_text(row.input.display().to_string());
                    });
                    table_row.col(|ui| match &row.status {
                        RowStatus::Pending => {
                            ui.weak("pending");
                        }
                        RowStatus::Running => {
                            ui.spinner();
                        }
                        RowStatus::Done(_) => {
                            ui.colored_label(egui::Color32::from_rgb(0x2e, 0xa0, 0x43), "✓ ok");
                        }
                        RowStatus::Failed(_) => {
                            ui.colored_label(ui.visuals().error_fg_color, "✗ failed");
                        }
                    });
                    table_row.col(|ui| match &row.status {
                        RowStatus::Done(report) => {
                            let mut parts = Vec::new();
                            if let Some(name) = &report.product_name {
                                parts.push(name.clone());
                            }
                            let mut wrote = Vec::new();
                            if let Some(p) = &report.gldf_path {
                                wrote.push(
                                    p.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("gldf")
                                        .to_string(),
                                );
                            }
                            if let Some(p) = &report.ifc_path {
                                wrote.push(
                                    p.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("ifc")
                                        .to_string(),
                                );
                            }
                            if !wrote.is_empty() {
                                parts.push(format!("→ {}", wrote.join(", ")));
                            }
                            ui.label(parts.join("  "));
                        }
                        RowStatus::Failed(err) => {
                            ui.label(err.as_str()).on_hover_text(err);
                        }
                        _ => {}
                    });
                });
            });
    }
}

fn load_template(path: &Path) -> anyhow::Result<BatchTemplate> {
    let raw = std::fs::read_to_string(path)?;
    BatchTemplate::parse(&raw)
}

fn open_in_file_manager(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "windows")]
    let program = "explorer";
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let program = "xdg-open";
    std::process::Command::new(program).arg(path).spawn()?;
    Ok(())
}
