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
        mounting_type: String::new(),
        mounting_recessed_depth_mm: String::new(),
        mounting_pendant_length_mm: String::new(),
        mounting_height_mm: String::new(),
        mounting_pole_height_mm: String::new(),
    sensor_bytes: None,
    sensor_filename: String::new(),
        emergency_bytes: None,
        emergency_filename: String::new(),
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
        mounting_type: String::new(),
        mounting_recessed_depth_mm: String::new(),
        mounting_pendant_length_mm: String::new(),
        mounting_height_mm: String::new(),
        mounting_pole_height_mm: String::new(),
    sensor_bytes: None,
    sensor_filename: String::new(),
        emergency_bytes: None,
        emergency_filename: String::new(),
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
        mounting_type: String::new(),
        mounting_recessed_depth_mm: String::new(),
        mounting_pendant_length_mm: String::new(),
        mounting_height_mm: String::new(),
        mounting_pole_height_mm: String::new(),
    sensor_bytes: None,
    sensor_filename: String::new(),
        emergency_bytes: None,
        emergency_filename: String::new(),
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
        mounting_type: String::new(),
        mounting_recessed_depth_mm: String::new(),
        mounting_pendant_length_mm: String::new(),
        mounting_height_mm: String::new(),
        mounting_pole_height_mm: String::new(),
    sensor_bytes: None,
    sensor_filename: String::new(),
        emergency_bytes: None,
        emergency_filename: String::new(),
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

#[test]
fn wizard_writes_ceiling_recessed_mountings() {
    // Smoke test for the Mountings wizard step: a draft with
    // `mounting_type = "ceiling/recessed"` should produce a variant
    // with `<Mountings><Ceiling><Recessed recessedDepth=50/>` after
    // apply_wizard runs. This drives the Bevy 3D viewer's recessed
    // placement end-to-end (the visible payoff of the slice).
    let draft = WizardDraft {
        header_author: "Mounting test".to_string(),
        header_manufacturer: "Acme".to_string(),
        header_created_with: "test".to_string(),
        header_default_language: "en".to_string(),
        ls_kind: WizardLightSourceKind::Fixed,
        ls_name: "LED".to_string(),
        variant_name: "Recessed downlight".to_string(),
        mounting_type: "ceiling/recessed".to_string(),
        mounting_recessed_depth_mm: "50".to_string(),
        ..WizardDraft::default()
    };

    let mut state = GldfState::default();
    apply_wizard(&mut state, &draft);
    assert!(state.last_error.is_none(), "{:?}", state.last_error);

    let variants = state.product.product_definitions.variants.as_ref();
    let variants = variants.expect("wizard creates the required Variant");
    let v = &variants.variant[0];
    let m = v.mountings.as_ref().expect("Mountings block written");
    let ceiling = m.ceiling.as_ref().expect("Ceiling branch written");
    let recessed = ceiling.recessed.as_ref().expect("Recessed sub-block written");
    assert_eq!(recessed.recessed_depth, 50);
    assert!(ceiling.surface_mounted.is_none());
    assert!(ceiling.pendant.is_none());
    assert!(m.wall.is_none());
    assert!(m.ground.is_none());
    assert!(m.working_plane.is_none());
}

#[test]
fn wizard_skips_mountings_when_type_empty() {
    let draft = WizardDraft {
        header_author: "Skip test".to_string(),
        header_manufacturer: "Acme".to_string(),
        header_created_with: "test".to_string(),
        header_default_language: "en".to_string(),
        ls_kind: WizardLightSourceKind::Fixed,
        ls_name: "LED".to_string(),
        variant_name: "Standard".to_string(),
        // Empty mounting_type → no Mountings block on the variant.
        mounting_type: String::new(),
        ..WizardDraft::default()
    };

    let mut state = GldfState::default();
    apply_wizard(&mut state, &draft);
    let v = &state
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant[0];
    assert!(v.mountings.is_none(), "skipped step writes no Mountings");
}

#[test]
fn wizard_writes_ground_pole_top_mountings() {
    let draft = WizardDraft {
        header_author: "Pole test".to_string(),
        header_manufacturer: "Acme".to_string(),
        header_created_with: "test".to_string(),
        header_default_language: "en".to_string(),
        ls_kind: WizardLightSourceKind::Fixed,
        ls_name: "LED".to_string(),
        variant_name: "Streetlight".to_string(),
        mounting_type: "ground/pole_top".to_string(),
        mounting_pole_height_mm: "6500".to_string(),
        ..WizardDraft::default()
    };

    let mut state = GldfState::default();
    apply_wizard(&mut state, &draft);
    let v = &state
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant[0];
    let m = v.mountings.as_ref().expect("Mountings block written");
    let ground = m.ground.as_ref().expect("Ground branch written");
    let pole = ground.pole_top.as_ref().expect("PoleTop sub-block");
    // The wizard prefers the @poleHeight attribute form (XSD-current);
    // the legacy <poleHeight> element form stays None.
    assert_eq!(pole.pole_height, Some(6500));
    assert!(pole.pole_height_element.is_none());
}

#[test]
fn wizard_writes_sensor_file_with_correct_content_type() {
    // Smoke test for the Sensor wizard step: dropping sensor.ldt bytes
    // via the wizard should land them as a sensor/sensldt File AND
    // mint a fresh <Sensor> entry pointing at it — no follow-up
    // UpdateFile dispatch on the Files page should be needed.
    let bytes: Vec<u8> = b"vendor-sensor-data".to_vec();
    let draft = WizardDraft {
        header_author: "Sensor wiz".to_string(),
        header_manufacturer: "Acme".to_string(),
        header_created_with: "test".to_string(),
        header_default_language: "en".to_string(),
        ls_kind: WizardLightSourceKind::Fixed,
        ls_name: "LED".to_string(),
        variant_name: "Standard".to_string(),
        sensor_bytes: Some(bytes.clone()),
        sensor_filename: "sensor.ldt".to_string(),
        ..WizardDraft::default()
    };
    let mut state = GldfState::default();
    apply_wizard(&mut state, &draft);
    assert!(state.last_error.is_none(), "{:?}", state.last_error);
    let sensors = state.product.general_definitions.sensors.as_ref();
    let sensors = sensors.expect("Sensors block written");
    assert_eq!(sensors.sensor.len(), 1);
    let sensor = &sensors.sensor[0];
    let file_id = &sensor.sensor_file_reference.as_ref().unwrap().file_id;
    // The File should be sensor/sensldt out of the gate — NOT
    // ldc/eulumdat with a follow-up retype dispatch.
    let file = state
        .product
        .general_definitions
        .files
        .file
        .iter()
        .find(|f| &f.id == file_id)
        .expect("File written");
    assert_eq!(file.content_type, "sensor/sensldt");
    assert_eq!(file.file_name, "sensor.ldt");
    // The bytes should be in embedded_files for export.
    assert_eq!(state.embedded_files.get(file_id), Some(&bytes));
    // The wizard should NOT have minted a stray <Photometry> for the
    // sensor file — the Photometry slot stays empty when the
    // photometry step itself was skipped.
    let phot_count = state
        .product
        .general_definitions
        .photometries
        .as_ref()
        .map(|p| p.photometry.len())
        .unwrap_or(0);
    assert_eq!(phot_count, 0, "sensor file must not mint <Photometry>");
}

#[test]
fn wizard_writes_emergency_photometry_alongside_normal() {
    // The Emergency wizard step is essentially "drop a second photometry
    // file": apply_wizard dispatches AddPhotometryFile for the emergency
    // bytes so the GLDF declares TWO <Photometry> entries. The wizard
    // does NOT bind the emergency entry to any emitter — the user /
    // demo wires an EmergencyOnly emitter to the resulting id later.
    let normal_bytes: Vec<u8> = b"normal-photometry-data".to_vec();
    let emergency_bytes: Vec<u8> = b"emergency-photometry-data".to_vec();
    let draft = WizardDraft {
        header_author: "Emergency wiz".to_string(),
        header_manufacturer: "Acme".to_string(),
        header_created_with: "test".to_string(),
        header_default_language: "en".to_string(),
        ls_kind: WizardLightSourceKind::Fixed,
        ls_name: "LED".to_string(),
        variant_name: "Standard".to_string(),
        photometry_bytes: Some(normal_bytes.clone()),
        photometry_filename: "middle.ldt".to_string(),
        emergency_bytes: Some(emergency_bytes.clone()),
        emergency_filename: "emergency.ldt".to_string(),
        ..WizardDraft::default()
    };
    let mut state = GldfState::default();
    apply_wizard(&mut state, &draft);
    assert!(state.last_error.is_none(), "{:?}", state.last_error);

    // Two <Photometry> entries: photometryFile_1 (normal) +
    // photometryFile_2 (emergency).
    let photometries = state
        .product
        .general_definitions
        .photometries
        .as_ref()
        .expect("Photometries block written");
    assert_eq!(photometries.photometry.len(), 2);

    // Two <File> entries — both ldc/eulumdat (the normal sniffer's
    // verdict for .ldt). The Emergency step intentionally uses
    // AddPhotometryFile (not a separate sensor-flavoured dispatch) so
    // the resulting File is shape-identical to the normal one — the
    // user / demo renames the auto id to `ph_emergency` later.
    let files = &state.product.general_definitions.files.file;
    let phot_files: Vec<_> = files
        .iter()
        .filter(|f| f.content_type == "ldc/eulumdat")
        .collect();
    assert_eq!(phot_files.len(), 2);

    // Embedded bytes preserved for export.
    assert_eq!(
        state.embedded_files.get(&phot_files[0].id),
        Some(&normal_bytes)
    );
    assert_eq!(
        state.embedded_files.get(&phot_files[1].id),
        Some(&emergency_bytes)
    );
}
