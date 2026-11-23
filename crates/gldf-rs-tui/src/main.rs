//! `gldf-edit` — terminal frontend for the gldf-rs editor.
//!
//! Cut 1: viewer-only. Open a `.gldf`, navigate a section sidebar
//! mirroring the WASM left-nav, see read-only detail. Edit parity with
//! the WASM editor lands in follow-up cuts on top of this scaffold.

mod app;
mod edit_modal;
mod event;
mod file_picker;
mod fs_lang_store;
mod l3d_view;
mod polar_view;
mod ui;
mod wizard;

use anyhow::{Context, Result};
use app::App;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "gldf-edit", about = "Terminal editor for GLDF files")]
struct Cli {
    /// Path to a .gldf file to open. If omitted, the welcome screen
    /// suggests the wizard or opening a file from within the TUI.
    #[arg(long, short = 'f')]
    file: Option<PathBuf>,

    /// Dump the loaded product as JSON to stdout and exit. No TUI.
    /// Useful as a smoke test: `gldf-edit --file foo.gldf --dump | head`.
    #[arg(long)]
    dump: bool,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let mut app = App::new();
    if let Some(path) = cli.file.as_ref() {
        app.load_file(path)
            .with_context(|| format!("loading {}", path.display()))?;
    }
    if cli.dump {
        // No TUI: print the loaded product as JSON and exit.
        let json = serde_json::to_string_pretty(&app.state.product)?;
        println!("{}", json);
        return Ok(());
    }
    app.run()
}
