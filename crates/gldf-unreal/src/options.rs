//! Export options.

use std::path::PathBuf;

/// Configuration for one export run.
#[derive(Clone, Debug)]
pub struct ExportOptions {
    /// Directory where per-variant bundles are written. Each bundle is a
    /// subdirectory `<bundle_name>__<variant_id>/`.
    pub out_dir: PathBuf,

    /// Bundle name prefix. Defaults to a sanitised product id when the
    /// caller doesn't override.
    pub bundle_name: String,

    /// World-unit system for the emitted geometry. Unreal's default is
    /// centimetres; the M variant exists for projects that have explicitly
    /// switched their UE project to metres.
    pub units: UnitSystem,

    /// Which variants to emit.
    pub variants: VariantSelector,

    /// If true, copy textures referenced by L3D MTL files into the bundle's
    /// `Assets/Geometry/textures/` directory.
    pub embed_textures: bool,

    /// If true, apply the GLDF `MountingInfo` offsets to the luminaire
    /// root Actor's transform. Default false (origin at world zero) so the
    /// UE level designer places the actor.
    pub apply_mounting: bool,

    /// Replace any existing bundle directory with the same name.
    pub overwrite: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            out_dir: PathBuf::from("."),
            bundle_name: "Luminaire".into(),
            units: UnitSystem::Cm,
            variants: VariantSelector::All,
            embed_textures: false,
            apply_mounting: false,
            overwrite: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitSystem {
    /// Unreal Engine default — centimetres.
    Cm,
    /// Metres. Use only when the UE project's World Unit is set to metres.
    M,
}

impl UnitSystem {
    /// Scale factor applied to convert L3D millimetres to this unit.
    pub fn scale_from_mm(self) -> f64 {
        match self {
            UnitSystem::Cm => 0.1,
            UnitSystem::M => 0.001,
        }
    }
}

#[derive(Clone, Debug)]
pub enum VariantSelector {
    /// Every variant in the GLDF.
    All,
    /// First variant only (useful for quick smoke tests / single-variant
    /// luminaires).
    First,
    /// Specific variant ids to emit.
    Only(Vec<String>),
}
