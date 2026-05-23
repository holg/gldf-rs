//! Phase-1 smoke tests: API surface compiles + an empty export round-trips
//! without touching the filesystem.

use gldf_unreal::{ExportOptions, UnitSystem, VariantSelector};

#[test]
fn default_options_have_sensible_values() {
    let opts = ExportOptions::default();
    assert_eq!(opts.units, UnitSystem::Cm);
    assert!(!opts.embed_textures);
    assert!(!opts.apply_mounting);
    assert!(!opts.overwrite);
    assert!(matches!(opts.variants, VariantSelector::All));
}

#[test]
fn unit_system_scale_factors() {
    assert!((UnitSystem::Cm.scale_from_mm() - 0.1).abs() < 1e-9);
    assert!((UnitSystem::M.scale_from_mm() - 0.001).abs() < 1e-9);
}

#[test]
fn error_codes_are_stable() {
    use gldf_unreal::ExportError;
    // The discriminant numbers are part of the C ABI contract — pin them.
    assert_eq!(ExportError::NoGeometry.code(), 4);
    assert_eq!(
        ExportError::VariantNotFound("foo".into()).code(),
        3
    );
    assert_eq!(ExportError::Internal("x".into()).code(), 99);
}
