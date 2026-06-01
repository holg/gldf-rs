//! `gldf-cli` — command-line driver for the gldf-rs editor API.
//!
//! All four subcommands speak the same JSON action surface that the
//! Leptos, ratatui, and Python frontends drive. A `gldf-cli apply`
//! invocation IS the most direct demonstration that the API is a
//! product, not a frontend internal.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use gldf_rs::EditableGldf;
use gldf_rs_editor::{apply_wizard, GldfAction, GldfState, WizardDraft};
use std::path::PathBuf;

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
    Save {
        #[arg(long, short = 'i')]
        input: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
    },
    /// Print the loaded product as JSON to stdout.
    Dump {
        #[arg(long, short = 'i')]
        input: PathBuf,
    },
    /// Run `apply_wizard` non-interactively from a `WizardDraft` JSON
    /// document. Demonstrates the wizard logic is portable.
    Wizard {
        #[arg(long, short = 'd')]
        draft: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
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
        Cmd::Save { input, output } => cmd_save(&input, &output),
        Cmd::Dump { input } => cmd_dump(&input),
        Cmd::Wizard { draft, output } => cmd_wizard(&draft, &output),
    }
}

fn cmd_apply(
    script: &std::path::Path,
    input: Option<&std::path::Path>,
    output: &std::path::Path,
) -> Result<()> {
    let mut state = load_state(input)?;
    let raw = std::fs::read_to_string(script)
        .with_context(|| format!("read script {}", script.display()))?;
    let mut value: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("parse JSON in {}", script.display()))?;
    let script_dir = script.parent().unwrap_or_else(|| std::path::Path::new("."));
    expand_file_refs(&mut value, script_dir)
        .with_context(|| format!("expand @file refs in {}", script.display()))?;
    let actions: Vec<GldfAction> = serde_json::from_value(value)
        .with_context(|| format!("parse actions in {}", script.display()))?;
    for action in actions {
        state.apply(action);
        if let Some(err) = &state.last_error {
            anyhow::bail!("apply failed: {}", err);
        }
    }
    write_state(&state, output)
}

/// Walk the parsed action JSON and replace `{"bytes": {"@file": "path"}}`
/// with the file's contents (read as `Vec<u8>` and serialized as a JSON
/// byte array, which is what serde expects for `Vec<u8>` fields).
/// Paths are resolved relative to the script directory.
fn expand_file_refs(value: &mut serde_json::Value, script_dir: &std::path::Path) -> Result<()> {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            // Special-case any field whose value is `{"@file": "path"}`.
            // Replacing the entry is simplest — collect keys first to
            // avoid borrowing map twice.
            let file_ref_keys: Vec<String> = map
                .iter()
                .filter_map(|(k, v)| {
                    v.as_object()
                        .and_then(|o| o.get("@file"))
                        .and_then(|p| p.as_str())
                        .map(|_| k.clone())
                })
                .collect();
            for k in file_ref_keys {
                let path_str = map[&k]["@file"].as_str().unwrap().to_string();
                let path = script_dir.join(&path_str);
                let bytes = std::fs::read(&path)
                    .with_context(|| format!("read referenced file {}", path.display()))?;
                map[&k] = Value::Array(
                    bytes
                        .into_iter()
                        .map(|b| Value::Number(b.into()))
                        .collect(),
                );
            }
            for v in map.values_mut() {
                expand_file_refs(v, script_dir)?;
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                expand_file_refs(v, script_dir)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn cmd_save(input: &std::path::Path, output: &std::path::Path) -> Result<()> {
    let state = load_state(Some(input))?;
    write_state(&state, output)
}

fn cmd_dump(input: &std::path::Path) -> Result<()> {
    let state = load_state(Some(input))?;
    let json = serde_json::to_string_pretty(&state.product)?;
    println!("{}", json);
    Ok(())
}

fn cmd_wizard(draft_path: &std::path::Path, output: &std::path::Path) -> Result<()> {
    let raw = std::fs::read_to_string(draft_path)
        .with_context(|| format!("read draft {}", draft_path.display()))?;
    let mut value: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("parse JSON in {}", draft_path.display()))?;
    let draft_dir = draft_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    expand_file_refs(&mut value, draft_dir)
        .with_context(|| format!("expand @file refs in {}", draft_path.display()))?;
    let draft: WizardDraft = serde_json::from_value(value)
        .with_context(|| format!("parse draft in {}", draft_path.display()))?;
    let mut state = GldfState::default();
    apply_wizard(&mut state, &draft);
    if let Some(err) = &state.last_error {
        anyhow::bail!("wizard failed: {}", err);
    }
    write_state(&state, output)
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
