//! `gldf-cli` — command-line driver for the gldf-rs editor API.
//!
//! All four subcommands speak the same JSON action surface that the
//! Leptos, ratatui, and Python frontends drive. A `gldf-cli apply`
//! invocation IS the most direct demonstration that the API is a
//! product, not a frontend internal.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Input, Select};
use gldf_rs::EditableGldf;
use gldf_rs_editor::{
    apply_script, apply_wizard, expand_file_refs, EmitterDraft, FsResolver, GldfAction, GldfState,
    LightEmitterKind, WizardDraft, WizardLightSourceKind,
};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "gldf-cli",
    about = "Command-line driver for the gldf-rs editor API."
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Apply a JSON array of GldfAction to an input file. Writes the
    /// resulting GLDF to `-o`/`--output`.
    Apply {
        /// JSON file containing an array of `GldfAction` objects.
        #[arg(long, short = 's')]
        script: PathBuf,
        /// GLDF to start from. Omit to start with an empty state.
        #[arg(long, short = 'i')]
        input: Option<PathBuf>,
        /// Where to write the resulting `.gldf`.
        #[arg(long, short = 'o')]
        output: PathBuf,
    },
    /// Round-trip an input GLDF through the loader and writer. Useful
    /// for canonicalising or smoke-testing the writer.
    ///
    /// `--dialux-compat` strips `#` comment lines from every
    /// `spectrum/text` payload (DIALux silently rejects them; real
    /// GLDF readers tolerate them). Off by default — the output is
    /// canonical GLDF unless you opt in.
    ///
    /// `--inline-spectrum` embeds every external `.spd` as
    /// `<Intensity wavelength=…>` rows inside product.xml; the
    /// `.spd` file entries are dropped.
    ///
    /// `--tm33` merges paired (photometry, spectrum) into a single
    /// `.iesxml` file. Photometries without a spectrum stay as
    /// `.ldt`/`.ies`.
    Save {
        #[arg(long, short = 'i')]
        input: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
        /// Apply DIALux-import compatibility transforms (currently:
        /// strip `#` lines from `spectrum/text` payloads). Off by
        /// default; see [`gldf_rs::dialux_compat`].
        #[arg(long, default_value_t = false)]
        dialux_compat: bool,
        /// Inline every external `spectrum/text` SPD into
        /// `<Intensity wavelength=…>` rows; drop the .spd files.
        #[arg(long, default_value_t = false)]
        inline_spectrum: bool,
        /// Merge each (photometry, spectrum) pair into a single
        /// TM-33 (`ldc/iesxml`) file. Bare photometries unchanged.
        #[arg(long, default_value_t = false)]
        tm33: bool,
    },
    /// Print the loaded product as JSON to stdout.
    Dump {
        #[arg(long, short = 'i')]
        input: PathBuf,
    },
    /// Run the New GLDF wizard. With `--draft`, applies the given
    /// `WizardDraft` JSON non-interactively. Without it, prompts the
    /// user through 5 steps in the terminal and also writes the
    /// populated draft next to the output so the run can be replayed
    /// later via `--draft`.
    Wizard {
        /// Pre-populated `WizardDraft` JSON. When set, runs
        /// non-interactively (CI / scripted).
        #[arg(long, short = 'd')]
        draft: Option<PathBuf>,
        /// Where to write the resulting `.gldf`.
        #[arg(long, short = 'o')]
        output: PathBuf,
    },
    /// Batch-convert photometry files (.ldt / .ies) to GLDF (and
    /// optionally IFC) driven by a JSON template. The template is either
    /// a `WizardDraft` object (same shape as `wizard --draft`; fields
    /// left out are auto-filled from each input LDT) or a `GldfAction`
    /// array (same shape as `apply --script`). String values may use
    /// placeholders like `{input.stem}`, `{ldt.name}`,
    /// `{ldt.manufacturer}`, `{ldt.watts}`; `{"@file": "@input"}`
    /// resolves to the current input's bytes, other `@file` paths
    /// resolve relative to the template.
    Batch {
        /// Photometry files and/or directories (searched recursively
        /// for .ldt / .ies).
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// JSON template (`WizardDraft` object or `GldfAction` array).
        #[arg(long, short = 't')]
        template: PathBuf,
        /// Output directory; one `<stem>.gldf` (and `<stem>.ifc`) per
        /// input file. Created if missing.
        #[arg(long, short = 'o')]
        out_dir: PathBuf,
        /// Also write an IFC STEP file per input.
        #[arg(long, default_value_t = false)]
        ifc: bool,
        /// Skip the .gldf output (e.g. IFC-only runs).
        #[arg(long, default_value_t = false)]
        no_gldf: bool,
        /// XSD-validate every written .gldf and report violations.
        #[arg(long, default_value_t = false)]
        validate: bool,
    },
    /// Validate a `.gldf` against the rc-3 XSD. Prints findings to
    /// stderr. Exit code 0 means clean, 1 means violations, 2 means
    /// the input couldn't be read/parsed.
    Validate {
        /// `.gldf` archive to validate.
        #[arg(long, short = 'i')]
        input: PathBuf,
    },
    /// Dump the L3D structure of a GLDF (or raw .l3d): every joint,
    /// LightEmittingObject and SensorObject with its position +
    /// rotation. Used to debug gldf-bevy's emitter walker without
    /// spinning up the WASM viewer.
    DumpL3d {
        /// `.gldf` or `.l3d` archive.
        #[arg(long, short = 'i')]
        input: PathBuf,
    },
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    match cli.command {
        Cmd::Apply {
            script,
            input,
            output,
        } => cmd_apply(&script, input.as_deref(), &output),
        Cmd::Save {
            input,
            output,
            dialux_compat,
            inline_spectrum,
            tm33,
        } => cmd_save(&input, &output, dialux_compat, inline_spectrum, tm33),
        Cmd::Dump { input } => cmd_dump(&input),
        Cmd::Batch {
            inputs,
            template,
            out_dir,
            ifc,
            no_gldf,
            validate,
        } => cmd_batch(&inputs, &template, &out_dir, ifc, no_gldf, validate),
        Cmd::Wizard { draft, output } => cmd_wizard(draft.as_deref(), &output),
        Cmd::Validate { input } => cmd_validate(&input),
        Cmd::DumpL3d { input } => cmd_dump_l3d(&input),
    }
}

fn cmd_dump_l3d(input: &Path) -> Result<()> {
    let bytes = std::fs::read(input).with_context(|| format!("read {}", input.display()))?;

    // Accept either .gldf (find embedded .l3d) or raw .l3d.
    let l3d_bytes: Vec<u8> = match input.extension().and_then(|s| s.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("l3d") => bytes,
        _ => {
            // GLDF: load via the editor's parser and pluck the
            // embedded l3d from the loaded archive.
            let state = load_state(Some(input))?;
            // GLDF archives nest the .l3d under various directories
            // (`geo/`, `geometry/`, root, etc.); rather than relying on
            // the product.xml `<File>` reference matching the archive
            // path verbatim, just scan the zip for any `.l3d` member
            // and read the first one.
            let _ = state; // loaded above purely to validate the GLDF.
            let archive_bytes = std::fs::read(input)?;
            let cursor = std::io::Cursor::new(&archive_bytes);
            let mut zip = zip::ZipArchive::new(cursor).context("open gldf archive")?;
            let mut found: Option<Vec<u8>> = None;
            for i in 0..zip.len() {
                let mut entry = zip.by_index(i)?;
                if entry.name().to_ascii_lowercase().ends_with(".l3d") {
                    use std::io::Read as _;
                    let mut buf = Vec::new();
                    entry.read_to_end(&mut buf)?;
                    println!("# embedded l3d found at: {}", entry.name());
                    found = Some(buf);
                    break;
                }
            }
            found.context("no .l3d member found inside gldf archive")?
        }
    };

    let l3d = l3d_rs::from_buffer(&l3d_bytes);
    let lum = l3d_rs::Luminaire::from_xml(&l3d.file.structure)
        .context("parse structure.xml")?;
    println!("# L3D dump for {}", input.display());
    println!("# parts in <Model>: {}", l3d.model.parts.len());
    for p in &l3d.model.parts {
        println!("  part: {}", p.path);
    }
    println!("# structure tree:");
    dump_geom(&lum.structure.geometry, 0);
    Ok(())
}

fn dump_geom(g: &l3d_rs::Geometry, depth: usize) {
    let pad = "  ".repeat(depth);
    println!("{}geometry partName={:?}", pad, g.part_name);
    println!(
        "{}  pos=({}, {}, {})  rot=({}, {}, {})",
        pad, g.position.x, g.position.y, g.position.z, g.rotation.x, g.rotation.y, g.rotation.z
    );
    if let Some(leos) = &g.light_emitting_objects {
        for o in leos.objects() {
            let (px, py, pz) = o.position();
            let (rx, ry, rz) = o.rotation();
            let size = if let Some(r) = o.rectangle() {
                let (w, h) = r.size();
                format!("rect {}x{}", w, h)
            } else if let Some(c) = o.circle() {
                format!("circle d={}", c.diameter())
            } else {
                "?".to_string()
            };
            println!(
                "{}  LEO partName={:?} pos=({}, {}, {}) rot=({}, {}, {}) [{}]",
                pad,
                o.part_name(),
                px,
                py,
                pz,
                rx,
                ry,
                rz,
                size
            );
        }
    }
    if let Some(sens) = &g.sensor_objects {
        for o in sens.objects() {
            let (px, py, pz) = o.position();
            let (rx, ry, rz) = o.rotation();
            println!(
                "{}  SENSOR partName={:?} pos=({}, {}, {}) rot=({}, {}, {})",
                pad,
                o.part_name(),
                px,
                py,
                pz,
                rx,
                ry,
                rz
            );
        }
    }
    if let Some(joints) = &g.joints {
        for j in &joints.joint {
            println!(
                "{}  joint partName={:?} pos=({}, {}, {}) rot=({}, {}, {})",
                pad,
                j.part_name,
                j.position.x,
                j.position.y,
                j.position.z,
                j.rotation.x,
                j.rotation.y,
                j.rotation.z
            );
            for child in &j.geometries.geometry {
                dump_geom(child, depth + 2);
            }
        }
    }
}

fn cmd_validate(input: &Path) -> Result<()> {
    let bytes = std::fs::read(input)
        .with_context(|| format!("read {}", input.display()))?;
    let findings = gldf_rs::validation::xsd_validate_gldf_archive(&bytes)
        .with_context(|| format!("validate {}", input.display()))?;
    if findings.is_empty() {
        eprintln!("✓ {} — XSD-clean", input.display());
        return Ok(());
    }
    eprintln!("✗ {} — {} violation(s):", input.display(), findings.len());
    for (i, f) in findings.iter().enumerate() {
        eprintln!("  [{:>3}] {}", i + 1, f);
    }
    std::process::exit(1);
}

fn cmd_apply(
    script: &std::path::Path,
    input: Option<&std::path::Path>,
    output: &std::path::Path,
) -> Result<()> {
    let mut state = load_state(input)?;
    let raw = std::fs::read_to_string(script)
        .with_context(|| format!("read script {}", script.display()))?;
    let root = script
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();
    apply_script(&mut state, &raw, &FsResolver { root })
        .with_context(|| format!("apply script {}", script.display()))?;
    write_state(&state, output)
}

fn cmd_save(
    input: &std::path::Path,
    output: &std::path::Path,
    dialux_compat: bool,
    inline_spectrum: bool,
    tm33: bool,
) -> Result<()> {
    let state = load_state(Some(input))?;
    let any_opt = dialux_compat || inline_spectrum || tm33;
    if any_opt {
        let opts = gldf_rs::editable::ExportOptions {
            dialux_compat,
            inline_spectrum,
            merge_into_tm33: tm33,
        };
        let bytes = state.save_to_buf_with_options(&opts)?;
        std::fs::write(output, bytes)
            .with_context(|| format!("write {}", output.display()))?;
        let mut tags = Vec::new();
        if dialux_compat { tags.push("dialux-compat"); }
        if inline_spectrum { tags.push("inline-spectrum"); }
        if tm33 { tags.push("tm33"); }
        eprintln!("wrote {} ({})", output.display(), tags.join(", "));
        Ok(())
    } else {
        write_state(&state, output)
    }
}

fn cmd_dump(input: &std::path::Path) -> Result<()> {
    let state = load_state(Some(input))?;
    let json = serde_json::to_string_pretty(&state.product)?;
    println!("{}", json);
    Ok(())
}

fn cmd_batch(
    inputs: &[PathBuf],
    template_path: &Path,
    out_dir: &Path,
    ifc: bool,
    no_gldf: bool,
    validate: bool,
) -> Result<()> {
    use gldf_rs_editor::batch::{
        collect_inputs, run_batch, BatchEvent, BatchJob, BatchOptions, BatchTemplate,
    };

    let raw = std::fs::read_to_string(template_path)
        .with_context(|| format!("read template {}", template_path.display()))?;
    let template = BatchTemplate::parse(&raw)
        .with_context(|| format!("parse template {}", template_path.display()))?;
    let template_dir = template_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let files = collect_inputs(inputs);
    if files.is_empty() {
        anyhow::bail!("no .ldt / .ies files found in the given inputs");
    }
    eprintln!("batch: {} photometry file(s) → {}", files.len(), out_dir.display());

    let job = BatchJob {
        template,
        template_dir,
        inputs: files,
        out_dir: out_dir.to_path_buf(),
        options: BatchOptions {
            write_gldf: !no_gldf,
            write_ifc: ifc,
        },
    };

    let reports = run_batch(&job, &mut |ev| {
        if let BatchEvent::Finished { index, total, report } = ev {
            let name = report
                .input
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?");
            match &report.error {
                Some(err) => eprintln!("[{}/{}] ✗ {} — {}", index + 1, total, name, err),
                None => {
                    let mut wrote = Vec::new();
                    if let Some(p) = &report.gldf_path {
                        wrote.push(p.display().to_string());
                    }
                    if let Some(p) = &report.ifc_path {
                        wrote.push(p.display().to_string());
                    }
                    eprintln!("[{}/{}] ✓ {} → {}", index + 1, total, name, wrote.join(", "));
                }
            }
        }
        true
    })?;

    let mut violations_total = 0usize;
    if validate {
        for report in reports.iter().filter(|r| r.error.is_none()) {
            if let Some(path) = &report.gldf_path {
                let bytes = std::fs::read(path)?;
                let findings = gldf_rs::validation::xsd_validate_gldf_archive(&bytes)
                    .with_context(|| format!("validate {}", path.display()))?;
                if !findings.is_empty() {
                    violations_total += findings.len();
                    eprintln!("✗ {} — {} XSD violation(s):", path.display(), findings.len());
                    for f in &findings {
                        eprintln!("    {}", f);
                    }
                }
            }
        }
    }

    let failed = reports.iter().filter(|r| r.error.is_some()).count();
    eprintln!(
        "batch done: {} ok, {} failed{}",
        reports.len() - failed,
        failed,
        if validate {
            format!(", {} XSD violation(s)", violations_total)
        } else {
            String::new()
        }
    );
    if failed > 0 || violations_total > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_wizard(draft_path: Option<&Path>, output: &Path) -> Result<()> {
    let (draft, draft_dir): (WizardDraft, PathBuf) = if let Some(path) = draft_path {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("read draft {}", path.display()))?;
        let mut value: serde_json::Value = serde_json::from_str(&raw)
            .with_context(|| format!("parse JSON in {}", path.display()))?;
        let root = path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        let resolver = FsResolver {
            root: root.clone(),
        };
        expand_file_refs(&mut value, &resolver)
            .with_context(|| format!("expand @file refs in {}", path.display()))?;
        let d: WizardDraft = serde_json::from_value(value)
            .with_context(|| format!("parse draft in {}", path.display()))?;
        (d, root)
    } else {
        let d = prompt_wizard_draft().context("interactive wizard failed")?;
        let dir = output
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        // Persist the populated draft next to the output so the
        // session is replayable via `gldf-cli wizard --draft …`.
        let draft_out = output.with_extension("draft.json");
        let pretty = serde_json::to_string_pretty(&d)?;
        std::fs::write(&draft_out, pretty)
            .with_context(|| format!("write draft {}", draft_out.display()))?;
        eprintln!("wrote {}", draft_out.display());
        (d, dir)
    };

    // The draft's photometry bytes may have been left empty in
    // interactive mode (user didn't pick a file). When non-interactive
    // and the draft uses @file refs, expand_file_refs above resolved
    // them already. Either way, apply_wizard works against the draft as
    // built.
    let _ = draft_dir; // currently only meaningful in --draft mode
    let mut state = GldfState::default();
    apply_wizard(&mut state, &draft);
    if let Some(err) = &state.last_error {
        anyhow::bail!("wizard failed: {}", err);
    }
    write_state(&state, output)
}

/// Walk the user through the 5 wizard steps via stdin prompts, return
/// a populated `WizardDraft`. The prompts mirror the Leptos / TUI
/// wizard UIs — same `WizardDraft` struct lands at the same
/// `apply_wizard` reducer.
///
/// When stdin is a TTY, uses `dialoguer` for nicer prompts (default
/// values, themed colors). When stdin is piped (CI tests, scripted
/// runs), falls back to a simple `read_line` loop so the wizard stays
/// drivable without a real terminal.
fn prompt_wizard_draft() -> Result<WizardDraft> {
    use std::io::IsTerminal;
    let tty = std::io::stdin().is_terminal();
    let mut p = Prompter::new(tty);
    eprintln!("\n=== New GLDF wizard ===\n");

    // --- Step 1: Header ---
    eprintln!("Step 1/7 — Header");
    let header_author = p.text("Author", "")?;
    let header_manufacturer = p.text("Manufacturer", "")?;
    let header_created_with = p.text("Created with application", "gldf-cli wizard")?;
    let header_default_language = p.text("Default language (e.g. en, de)", "en")?;

    // --- Step 2: Photometry (optional) ---
    eprintln!("\nStep 2/7 — Photometry");
    let photometry_path = p.text("Path to .ldt or .ies file (blank to skip)", "")?;
    let (photometry_bytes, photometry_filename) = if photometry_path.trim().is_empty() {
        (None, String::new())
    } else {
        let path = Path::new(photometry_path.trim());
        let bytes = std::fs::read(path)
            .with_context(|| format!("read photometry {}", path.display()))?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("photometry")
            .to_string();
        (Some(bytes), filename)
    };

    // --- Step 3: LightSource ---
    eprintln!("\nStep 3/7 — LightSource");
    let ls_kind = match p.select("LightSource kind", &["Fixed", "Changeable", "MultiChannel"], 0)?
    {
        0 => WizardLightSourceKind::Fixed,
        1 => WizardLightSourceKind::Changeable,
        _ => WizardLightSourceKind::MultiChannel,
    };
    let ls_name = p.text("LightSource name", "")?;
    let ls_manufacturer = p.text("LightSource manufacturer", "")?;
    let ls_rated_power = p.text("Rated input power (W, blank to skip)", "")?;

    // --- Step 4: Emitter ---
    eprintln!("\nStep 4/7 — Emitter");
    let want_emitter =
        p.select("Create an Emitter linked to the LightSource?", &["Yes", "No"], 0)? == 0;
    let emitter_draft = if want_emitter {
        let emitter_kind = match ls_kind {
            WizardLightSourceKind::Fixed => LightEmitterKind::Fixed,
            WizardLightSourceKind::Changeable => LightEmitterKind::Changeable,
            WizardLightSourceKind::MultiChannel => LightEmitterKind::MultiChannel,
        };
        let child_name = p.text("Emitter display name", "Primary")?;
        Some(EmitterDraft {
            child_name,
            kind: emitter_kind,
            photometry_id: String::new(),
            fixed_light_source_id: String::new(),
            multi_channel_light_source_id: String::new(),
            sensor_id: String::new(),
        })
    } else {
        None
    };

    // --- Step 5: Geometry ---
    eprintln!("\nStep 5/7 — Geometry");
    let geometry_path = p.text("Path to .l3d file (blank to skip)", "")?;
    let (geometry_bytes, geometry_filename) = if geometry_path.trim().is_empty() {
        (None, String::new())
    } else {
        let p2 = std::path::Path::new(geometry_path.trim());
        let bytes = std::fs::read(p2)
            .with_context(|| format!("read geometry {}", p2.display()))?;
        let filename = p2
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("luminaire.l3d")
            .to_string();
        (Some(bytes), filename)
    };

    // --- Step 6: Variant ---
    eprintln!("\nStep 6/7 — Variant");
    let variant_name = p.text("Variant name (blank → 'Standard')", "")?;

    // --- Step 7: Review ---
    eprintln!("\nStep 7/7 — Review");
    eprintln!("  Author:        {}", header_author);
    eprintln!("  Manufacturer:  {}", header_manufacturer);
    eprintln!("  Language:      {}", header_default_language);
    eprintln!(
        "  Photometry:    {}",
        if photometry_bytes.is_some() {
            photometry_filename.as_str()
        } else {
            "(none)"
        }
    );
    eprintln!("  LightSource:   {:?} \"{}\"", ls_kind, ls_name);
    eprintln!(
        "  Emitter:       {}",
        emitter_draft
            .as_ref()
            .map(|e| format!("{:?} \"{}\"", e.kind, e.child_name))
            .unwrap_or_else(|| "(none)".to_string())
    );
    eprintln!(
        "  Variant:       \"{}\"",
        if variant_name.trim().is_empty() {
            "Standard"
        } else {
            variant_name.as_str()
        }
    );
    if p.select("Build the GLDF?", &["Yes, build", "Cancel"], 0)? != 0 {
        anyhow::bail!("wizard cancelled");
    }

    Ok(WizardDraft {
        header_author,
        header_manufacturer,
        header_created_with,
        header_default_language,
        photometry_bytes,
        photometry_filename,
        ls_kind,
        ls_name,
        ls_manufacturer,
        ls_rated_power,
        emitter_draft,
        variant_name,
        geometry_bytes,
        geometry_filename,
        // CLI hasn't grown a Driver prompt yet (the wizard's CLI flow is
        // for headless / batch use; per-emitter ControlGear binding via
        // the editor surface area is enough for the human flows). Wire
        // when there's a real CLI consumer that asks for it.
        driver_enabled: false,
        driver_name: String::new(),
        driver_dimmable: false,
        driver_standby_power: String::new(),
        // Mounting step — same story as Driver. CLI flow leaves the
        // variant's <Mountings> empty; the WASM editor's
        // VariantMountingsEditor lets the user add one after Finish.
        mounting_type: String::new(),
        mounting_recessed_depth_mm: String::new(),
        mounting_pendant_length_mm: String::new(),
        mounting_height_mm: String::new(),
        mounting_pole_height_mm: String::new(),
    sensor_bytes: None,
    sensor_filename: String::new(),
        emergency_bytes: None,
        emergency_filename: String::new(),
    })
}

/// Tiny abstraction that dispatches each prompt to `dialoguer` when
/// stdin is a TTY, and to plain stdin reads otherwise. Letting both
/// paths run through the same helper keeps the wizard body short.
struct Prompter {
    tty: bool,
    theme: ColorfulTheme,
}

impl Prompter {
    fn new(tty: bool) -> Self {
        Self {
            tty,
            theme: ColorfulTheme::default(),
        }
    }

    fn text(&mut self, prompt: &str, default: &str) -> Result<String> {
        if self.tty {
            let mut input = Input::<String>::with_theme(&self.theme)
                .with_prompt(prompt)
                .allow_empty(true);
            if !default.is_empty() {
                input = input.default(default.to_string());
            }
            Ok(input.interact_text()?)
        } else {
            self.read_line_with_default(prompt, default)
        }
    }

    fn select(&mut self, prompt: &str, items: &[&str], default: usize) -> Result<usize> {
        if self.tty {
            Ok(Select::with_theme(&self.theme)
                .with_prompt(prompt)
                .items(items)
                .default(default)
                .interact()?)
        } else {
            let menu = items
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{}) {}", i + 1, s))
                .collect::<Vec<_>>()
                .join("  ");
            let raw = self.read_line_with_default(
                &format!("{} [{}]", prompt, menu),
                &(default + 1).to_string(),
            )?;
            let idx: usize = raw.trim().parse().with_context(|| {
                format!("expected a number 1..={}; got {:?}", items.len(), raw)
            })?;
            if idx == 0 || idx > items.len() {
                anyhow::bail!("choice {} out of range 1..={}", idx, items.len());
            }
            Ok(idx - 1)
        }
    }

    fn read_line_with_default(&self, prompt: &str, default: &str) -> Result<String> {
        use std::io::{BufRead, Write};
        if default.is_empty() {
            eprint!("{}: ", prompt);
        } else {
            eprint!("{} [{}]: ", prompt, default);
        }
        std::io::stderr().flush().ok();
        let mut line = String::new();
        let n = std::io::stdin()
            .lock()
            .read_line(&mut line)
            .context("read stdin")?;
        if n == 0 {
            // EOF — accept the default.
            return Ok(default.to_string());
        }
        let trimmed = line.trim_end_matches(['\n', '\r']).to_string();
        if trimmed.is_empty() && !default.is_empty() {
            Ok(default.to_string())
        } else {
            Ok(trimmed)
        }
    }
}

fn load_state(input: Option<&std::path::Path>) -> Result<GldfState> {
    let mut state = GldfState::default();
    if let Some(path) = input {
        let editable = EditableGldf::from_gldf(
            path.to_str().context("non-UTF-8 path")?,
        )
        .with_context(|| format!("read {}", path.display()))?;
        state.apply(GldfAction::LoadWithFiles {
            product: editable.product,
            embedded_files: editable.embedded_files,
        });
    }
    Ok(state)
}

fn write_state(state: &GldfState, output: &std::path::Path) -> Result<()> {
    let bytes = state.save_to_buf()?;
    std::fs::write(output, bytes).with_context(|| format!("write {}", output.display()))?;
    eprintln!("wrote {}", output.display());
    Ok(())
}
