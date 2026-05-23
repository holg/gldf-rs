//! CLI: convert a `.gldf` to one or more `.udatasmith` bundles.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};

use gldf_unreal::{ExportOptions, Exporter, UnitSystem, VariantSelector};

/// GLDF → Unreal Engine Datasmith exporter.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the input `.gldf` file.
    #[arg(long, value_name = "FILE")]
    gldf: PathBuf,

    /// Output directory. One subdirectory per variant is created inside.
    #[arg(long, value_name = "DIR", default_value = ".")]
    out: PathBuf,

    /// Bundle name prefix. Defaults to the GLDF file stem.
    #[arg(long, value_name = "NAME")]
    name: Option<String>,

    /// Unit system for the emitted geometry. UE 5.4 default is cm.
    #[arg(long, value_enum, default_value_t = CliUnits::Cm)]
    units: CliUnits,

    /// Which variants to emit.
    ///
    /// * `all` (default) — every variant.
    /// * `first` — only the first variant.
    /// * `<id>[,<id>…]` — explicit variant ids (comma-separated).
    #[arg(long, value_name = "SPEC", default_value = "all")]
    variants: String,

    /// Apply GLDF MountingInfo offsets to the luminaire root Actor.
    /// Default leaves the actor at world origin so the level designer
    /// places it.
    #[arg(long)]
    apply_mounting: bool,

    /// Copy MTL-referenced textures into the bundle's Assets/Geometry/.
    #[arg(long)]
    embed_textures: bool,

    /// Replace an existing bundle directory with the same name.
    #[arg(long)]
    overwrite: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum CliUnits {
    Cm,
    M,
}

impl From<CliUnits> for UnitSystem {
    fn from(u: CliUnits) -> Self {
        match u {
            CliUnits::Cm => UnitSystem::Cm,
            CliUnits::M => UnitSystem::M,
        }
    }
}

fn parse_variant_selector(spec: &str) -> VariantSelector {
    let trimmed = spec.trim();
    match trimmed {
        "all" => VariantSelector::All,
        "first" => VariantSelector::First,
        _ => VariantSelector::Only(
            trimmed
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        ),
    }
}

fn run(cli: Cli) -> Result<(), gldf_unreal::ExportError> {
    let bundle_name = cli
        .name
        .clone()
        .or_else(|| {
            cli.gldf
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "Luminaire".into());

    let opts = ExportOptions {
        out_dir: cli.out.clone(),
        bundle_name,
        units: cli.units.into(),
        variants: parse_variant_selector(&cli.variants),
        embed_textures: cli.embed_textures,
        apply_mounting: cli.apply_mounting,
        overwrite: cli.overwrite,
    };

    let report = Exporter::from_path(&cli.gldf)?.export(&opts)?;
    println!("Wrote {} bundle(s):", report.bundles.len());
    for b in &report.bundles {
        println!(
            "  variant={:<20} → {}",
            b.variant_id,
            b.udatasmith_path.display()
        );
        for p in &b.asset_paths {
            println!("    asset: {}", p.display());
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error ({}): {}", e.code(), e);
            ExitCode::from(e.code().clamp(1, 255) as u8)
        }
    }
}
