//! Slice 3 end-to-end: drive `apply_wizard` from a `WizardDraft`, save
//! to GLDF bytes, reload via `EditableGldf::from_buf`, and verify every
//! field the wizard set survives the round-trip. This is the test that
//! gates the New GLDF wizard slice — the same draft → bytes → reload
//! contract the CLI / Python / TUI / WASM frontends rely on.

use gldf_rs::EditableGldf;
use gldf_rs_editor::{
    apply_wizard, EmitterDraft, GldfAction, GldfState, LightEmitterKind, WizardDraft,
    WizardLightSourceKind,
};

#[test]
fn full_wizard_run_roundtrips_through_gldf_bytes() {
    let draft = WizardDraft {
        header_author: "Slice 3 Test Engineer".to_string(),
        header_manufacturer: "Acme Lighting".to_string(),
        header_created_with: "wizard-roundtrip test".to_string(),
        header_default_language: "en".to_string(),
        photometry_bytes: None,
        photometry_filename: String::new(),
        ls_kind: WizardLightSourceKind::Fixed,
        ls_name: "Main LED".to_string(),
        ls_manufacturer: "Acme Optics".to_string(),
        ls_rated_power: "42.5".to_string(),
        emitter_draft: Some(EmitterDraft {
            child_name: "Primary emitter".to_string(),
            kind: LightEmitterKind::Fixed,
            ..EmitterDraft::default()
        }),
        variant_name: "Standard 2700K".to_string(),
        geometry_bytes: None,
        geometry_filename: String::new(),
        driver_enabled: false,
        driver_name: String::new(),
        driver_dimmable: false,
        driver_standby_power: String::new(),
    };

    // 1. Run the wizard against a fresh state — same code the CLI /
    //    Python / TUI / WASM frontends drive.
    let mut state = GldfState::default();
    apply_wizard(&mut state, &draft);
    assert!(state.last_error.is_none(), "{:?}", state.last_error);

    // 2. Serialize to GLDF bytes (the same call every Save button hits).
    let bytes = state.save_to_buf().expect("wizard output saves");
    assert!(!bytes.is_empty(), "non-empty GLDF bytes");

    // 3. Reload via `EditableGldf::from_buf` — the entry point every
    //    consumer uses to import a GLDF.
    let editable = EditableGldf::from_buf(bytes).expect("reload from bytes");

    // 4. Pump it back through the reducer the same way `gldf-cli dump`
    //    and the WASM file-open path do, to prove the loaded shape
    //    survives the action layer too.
    let mut reloaded = GldfState::default();
    reloaded.apply(GldfAction::LoadWithFiles {
        product: editable.product,
        embedded_files: editable.embedded_files,
    });

    // 5. Every field the wizard set is present after round-trip.
    let p = &reloaded.product;
    assert_eq!(p.header.author, "Slice 3 Test Engineer");
    assert_eq!(p.header.manufacturer, "Acme Lighting");
    assert_eq!(p.header.created_with_application, "wizard-roundtrip test");
    assert_eq!(p.header.default_language.as_deref(), Some("en"));

    let ls = p
        .general_definitions
        .light_sources
        .as_ref()
        .expect("light_sources present after reload");
    assert_eq!(ls.fixed_light_source.len(), 1);
    let fls = &ls.fixed_light_source[0];
    assert!(fls.id.starts_with("lightsource_"));
    assert_eq!(fls.manufacturer.as_deref(), Some("Acme Optics"));
    assert_eq!(fls.rated_input_power, Some(42.5));

    let emitters = p
        .general_definitions
        .emitters
        .as_ref()
        .expect("emitters present after reload");
    assert_eq!(emitters.emitter.len(), 1);
    assert_eq!(emitters.emitter[0].fixed_light_emitter.len(), 1);
}

/// When the wizard runs with both photometry_bytes AND geometry_bytes
/// set, BOTH files must land in `embedded_files`. This is the contract
/// the Bevy 3D viewer relies on: it looks up the L3D bytes by file_id
/// in `embedded_files` to render the variant's housing.
///
/// Regression guard for the bug where the wizard finished with only the
/// LDT in embedded_files (Bevy canvas blank); the L3D either silently
/// got dropped before ImportL3dFile, or got a clashing file_id and the
/// `add_file` error skipped the embedded-bytes insert.
#[test]
fn wizard_with_geometry_lands_l3d_in_embedded_files() {
    // Minimal but valid LDT — eulumdat-rs only reads up to RatedInputPower,
    // so a 60-line stub is enough for the LDT-meta auto-fill not to panic.
    // The bytes themselves are opaque to the test; what we care about is
    // that `embedded_files` gets a key whose value matches what we passed.
    let ldt_bytes: Vec<u8> = b"stub-ldt-bytes\n".to_vec();
    let l3d_bytes: Vec<u8> = b"stub-l3d-bytes\n".to_vec();

    let draft = WizardDraft {
        header_author: "L3D Test".to_string(),
        header_manufacturer: "Acme".to_string(),
        header_created_with: "wizard L3D test".to_string(),
        header_default_language: "en".to_string(),
        photometry_bytes: Some(ldt_bytes.clone()),
        photometry_filename: "middle.ldt".to_string(),
        ls_kind: WizardLightSourceKind::Fixed,
        ls_name: "LED".to_string(),
        ls_manufacturer: "Acme Optics".to_string(),
        ls_rated_power: "18".to_string(),
        emitter_draft: Some(EmitterDraft {
            child_name: "LEO0".to_string(),
            kind: LightEmitterKind::Fixed,
            ..EmitterDraft::default()
        }),
        variant_name: "Standard".to_string(),
        geometry_bytes: Some(l3d_bytes.clone()),
        geometry_filename: "example_007.l3d".to_string(),
        driver_enabled: false,
        driver_name: String::new(),
        driver_dimmable: false,
        driver_standby_power: String::new(),
    };

    let mut state = GldfState::default();
    apply_wizard(&mut state, &draft);
    assert!(state.last_error.is_none(), "{:?}", state.last_error);

    // The headline assertion — both files in the embedded map.
    assert_eq!(
        state.embedded_files.len(),
        2,
        "expected LDT + L3D in embedded_files, got keys: {:?}",
        state.embedded_files.keys().collect::<Vec<_>>()
    );

    // The wizard's File entries should reference the same keys.
    let file_ids: Vec<&String> = state
        .product
        .general_definitions
        .files
        .file
        .iter()
        .map(|f| &f.id)
        .collect();
    assert_eq!(file_ids.len(), 2, "<File> count mismatch: {:?}", file_ids);
    for fid in &file_ids {
        assert!(
            state.embedded_files.contains_key(fid.as_str()),
            "<File id={}> not in embedded_files (keys: {:?})",
            fid,
            state.embedded_files.keys().collect::<Vec<_>>()
        );
    }

    // Content types: exactly one LDT and one L3D, regardless of order.
    let content_types: std::collections::HashSet<&str> = state
        .product
        .general_definitions
        .files
        .file
        .iter()
        .map(|f| f.content_type.as_str())
        .collect();
    assert!(
        content_types.contains("ldc/eulumdat"),
        "missing ldc/eulumdat in {:?}",
        content_types
    );
    assert!(
        content_types.contains("geo/l3d"),
        "missing geo/l3d in {:?}",
        content_types
    );

    // The ModelGeometry must exist and point at the L3D's file_id.
    let geoms = state
        .product
        .general_definitions
        .geometries
        .as_ref()
        .expect("geometries section present");
    assert_eq!(geoms.model_geometry.len(), 1);
    let model_geom = &geoms.model_geometry[0];
    assert_eq!(
        model_geom.geometry_file_reference.len(),
        1,
        "ModelGeometry should have exactly one GeometryFileReference"
    );
    let geom_file_id = &model_geom.geometry_file_reference[0].file_id;
    let l3d_file = state
        .product
        .general_definitions
        .files
        .file
        .iter()
        .find(|f| f.content_type == "geo/l3d")
        .expect("geo/l3d <File> entry");
    assert_eq!(
        &l3d_file.id, geom_file_id,
        "ModelGeometry references {} but the L3D <File> has id {}",
        geom_file_id, l3d_file.id
    );

    // And the bytes really are the bytes we passed in.
    assert_eq!(
        state.embedded_files.get(&l3d_file.id),
        Some(&l3d_bytes),
        "embedded L3D bytes don't match what was passed to the wizard"
    );
}

/// Regression: a Locale with empty `value` must NOT break the
/// save-then-reload round-trip. The auto-derived serialize impl would
/// emit `<Locale language=""/>` (self-closing tag), and quick-xml's
/// `#[serde(rename = "$text")]` deserializer then fails with
/// "missing field `$text`" on the way back in — every wizard-built
/// GLDF that ever added an empty-name LightSource became corrupt.
#[test]
fn empty_value_locale_roundtrips_via_bytes() {
    use gldf_rs::gldf::header::{Locale, LocaleFoo};
    use gldf_rs::gldf::general_definitions::FixedLightSource;
    use gldf_rs::EditableGldf;

    // Direct fixture: a single Fixed light source whose name has an
    // empty Locale. Skip the wizard so this test stays focused on the
    // serializer surface.
    let mut product = gldf_rs::GldfProduct::default();
    let ls = FixedLightSource {
        id: "lightsource_1".to_string(),
        name: LocaleFoo {
            locale: vec![Locale {
                language: String::new(),
                value: String::new(),
            }],
        },
        ..Default::default()
    };
    let _ = product
        .general_definitions
        .light_sources
        .get_or_insert_with(Default::default);
    product
        .general_definitions
        .light_sources
        .as_mut()
        .unwrap()
        .fixed_light_source
        .push(ls);

    let editable = EditableGldf::from_product(product);
    let bytes = editable
        .save_to_buf()
        .expect("save empty-Locale GLDF to bytes");

    // The headline assertion — reload doesn't error.
    let reloaded = EditableGldf::from_buf(bytes).expect(
        "Reload of GLDF with an empty-value Locale must succeed. Regression for \
         'missing field $text' (self-closing Locale tags).",
    );
    let ls = &reloaded
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .unwrap()
        .fixed_light_source[0];
    assert_eq!(ls.id, "lightsource_1");
}

/// Regression: dropping an L3D on the Geometry step MUST also wire the
/// variant's `Geometry` element to point at the new ModelGeometry, even
/// when no emitter was created (empty bindings). Without this, the
/// Bevy 3D viewer can't resolve the variant → ModelGeometry → L3D
/// chain — the canvas renders blank because the variant has no
/// `<Geometry>` element at all, even though a perfectly good
/// `<ModelGeometry>` exists at the document level.
#[test]
fn wizard_with_geometry_binds_variant_geometry_even_without_emitter() {
    let draft = WizardDraft {
        header_author: "Auto-bind test".to_string(),
        header_manufacturer: "Acme".to_string(),
        header_created_with: "wizard test".to_string(),
        header_default_language: "en".to_string(),
        photometry_bytes: Some(b"stub-ldt\n".to_vec()),
        photometry_filename: "middle.ldt".to_string(),
        ls_kind: WizardLightSourceKind::Fixed,
        ls_name: "LED".to_string(),
        ls_manufacturer: "Acme Optics".to_string(),
        ls_rated_power: "18".to_string(),
        emitter_draft: None,
        variant_name: "Standard".to_string(),
        geometry_bytes: Some(b"stub-l3d\n".to_vec()),
        geometry_filename: "example_007.l3d".to_string(),
        driver_enabled: false,
        driver_name: String::new(),
        driver_dimmable: false,
        driver_standby_power: String::new(),
    };

    let mut state = GldfState::default();
    apply_wizard(&mut state, &draft);
    assert!(state.last_error.is_none(), "{:?}", state.last_error);

    // Pull the variant and verify its Geometry points at the L3D.
    let variants = state
        .product
        .product_definitions
        .variants
        .as_ref()
        .expect("variants section present");
    assert_eq!(variants.variant.len(), 1);
    let geom = variants.variant[0]
        .geometry
        .as_ref()
        .expect("variant.Geometry must be set when an L3D was provided");
    let mgr = geom
        .model_geometry_reference
        .as_ref()
        .expect("variant.Geometry must be a ModelGeometryReference (not bare/Simple)");

    // The referenced geometry_id must exist in <Geometries>.
    let geoms = state
        .product
        .general_definitions
        .geometries
        .as_ref()
        .expect("Geometries section present");
    let matching = geoms
        .model_geometry
        .iter()
        .find(|g| g.id == mgr.geometry_id);
    assert!(
        matching.is_some(),
        "variant references geometry_id={} but no such ModelGeometry exists (have: {:?})",
        mgr.geometry_id,
        geoms
            .model_geometry
            .iter()
            .map(|g| &g.id)
            .collect::<Vec<_>>()
    );

    // And the L3D file the geometry points at is in embedded_files.
    let file_id = &matching.unwrap().geometry_file_reference[0].file_id;
    assert!(
        state.embedded_files.contains_key(file_id),
        "ModelGeometry's file_id={} not in embedded_files (keys: {:?})",
        file_id,
        state.embedded_files.keys().collect::<Vec<_>>()
    );
}

/// Regression: even if the user clicks through the Emitter step without
/// configuring a draft (empty `emitter_draft`), dropping an L3D on the
/// Geometry step MUST still land the L3D in `embedded_files`. The bug
/// this guards against: the wizard's Geometry path was gated on having
/// an emitter to bind, so a no-emitter draft silently dropped the L3D
/// bytes — the WASM Overview showed "Files: 1" (only the LDT) and the
/// Bevy 3D viewer rendered a blank canvas because the L3D was nowhere.
#[test]
fn wizard_with_geometry_but_no_emitter_still_lands_l3d() {
    let draft = WizardDraft {
        header_author: "L3D no-emitter Test".to_string(),
        header_manufacturer: "Acme".to_string(),
        header_created_with: "wizard L3D test".to_string(),
        header_default_language: "en".to_string(),
        photometry_bytes: Some(b"stub-ldt\n".to_vec()),
        photometry_filename: "middle.ldt".to_string(),
        ls_kind: WizardLightSourceKind::Fixed,
        ls_name: "LED".to_string(),
        ls_manufacturer: "Acme Optics".to_string(),
        ls_rated_power: "18".to_string(),
        // Headline difference: user skipped the Emitter step → no draft.
        emitter_draft: None,
        variant_name: "Standard".to_string(),
        geometry_bytes: Some(b"stub-l3d\n".to_vec()),
        geometry_filename: "example_007.l3d".to_string(),
        driver_enabled: false,
        driver_name: String::new(),
        driver_dimmable: false,
        driver_standby_power: String::new(),
    };

    let mut state = GldfState::default();
    apply_wizard(&mut state, &draft);
    assert!(state.last_error.is_none(), "{:?}", state.last_error);

    // The L3D bytes must be in embedded_files even though no Emitter
    // was created. (Variant.Geometry binding is allowed to fall back to
    // None — the user can wire it manually later from the Variants page.)
    let l3d_present = state.product.general_definitions.files.file.iter().any(|f| {
        f.content_type == "geo/l3d"
            && state.embedded_files.contains_key(&f.id)
            && state.embedded_files.get(&f.id).map(|b| b.as_slice())
                == Some(b"stub-l3d\n".as_slice())
    });
    assert!(
        l3d_present,
        "L3D bytes were lost when emitter_draft was None. \
         <File> entries: {:?}, embedded_files keys: {:?}",
        state
            .product
            .general_definitions
            .files
            .file
            .iter()
            .map(|f| (&f.id, &f.content_type))
            .collect::<Vec<_>>(),
        state.embedded_files.keys().collect::<Vec<_>>()
    );
}
