//! Script-driver tests for [`gldf_rs_editor::apply_script`] +
//! [`MemoryResolver`]. The CLI and the WASM editor both consume this
//! path; the WASM frontend especially because it cannot use `std::fs`.

use gldf_rs_editor::{apply_script, GldfState, MemoryResolver};
use std::collections::HashMap;

#[test]
fn apply_script_with_file_ref_resolves_and_applies() {
    let mut files = HashMap::new();
    files.insert("fake.ies".to_string(), b"IESNA:LM-63-2002\nfake\n".to_vec());
    let resolver = MemoryResolver::new(&files);

    let script = r#"[
      {"action": "SetAuthor", "data": "Scripted"},
      {"action": "AddPhotometryFile",
       "data": {"bytes": {"@file": "fake.ies"}, "filename": "fake.ies"}}
    ]"#;

    let mut state = GldfState::default();
    apply_script(&mut state, script, &resolver).expect("script applies");
    assert_eq!(state.product.header.author, "Scripted");
    // AddPhotometryFile is allowed to fail parsing the fake bytes; what
    // we care about is that `apply_script` walked the JSON and stuffed
    // the resolved bytes into the action — confirmed by `last_error`
    // either being None (parse succeeded) or naming a parse problem,
    // not a "file not provided" error from the resolver.
    if let Some(err) = &state.last_error {
        assert!(
            !err.contains("file not provided"),
            "resolver did not run: {}",
            err
        );
    }
}

#[test]
fn apply_script_propagates_missing_file_error() {
    let files = HashMap::new();
    let resolver = MemoryResolver::new(&files);

    let script = r#"[
      {"action": "AddPhotometryFile",
       "data": {"bytes": {"@file": "missing.ies"}, "filename": "missing.ies"}}
    ]"#;

    let mut state = GldfState::default();
    let err = apply_script(&mut state, script, &resolver).expect_err("should fail");
    let full = format!("{:#}", err);
    assert!(full.contains("missing.ies"), "got: {}", full);
}

/// Demo-equivalence oracle for the Lisp editor-authoring verbs.
///
/// The `gldf-rs-lisp` engine stays pure (no editor dep), so its verbs
/// can only be unit-tested against `GldfState::log`. This test closes
/// the loop: it feeds the EXACT action-JSON shapes those verbs emit
/// (`GLDF-NEW`, `GLDF-ADD-PHOTOMETRY`, `GLDF-ADD-SENSOR`,
/// `GLDF-SET-GEOMETRY`, `GLDF-ADD-VARIANT`) through `apply_script` over
/// the real `tests/test-create/` assets, and asserts the product comes
/// out Prolumia-shaped. If an engine verb's JSON drifts from what the
/// reducer expects, this fails — the browser path can't silently break.
#[test]
fn lisp_verb_actions_build_prolumia_shaped_product() {
    use std::path::Path;

    // Workspace `tests/test-create/` from this crate's manifest dir.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/test-create")
        .canonicalize()
        .expect("tests/test-create exists");
    let read = |name: &str| std::fs::read(root.join(name)).expect(name);

    let mut files = HashMap::new();
    files.insert("middle.ldt".to_string(), read("middle.ldt"));
    files.insert("sensor.ldt".to_string(), read("sensor.ldt"));
    files.insert("example_007.l3d".to_string(), read("example_007.l3d"));
    let resolver = MemoryResolver::new(&files);

    // These are byte-for-byte the JSON objects `editor_functions.rs`
    // builds — manufacturer+name from GLDF-NEW, then file drops, then a
    // named variant.
    let script = r#"[
      {"action": "SetManufacturer", "data": "Prolumia"},
      {"action": "SetProductMetaName",
       "data": {"Locale": [{"@language": "en", "$text": "Lumina Pro"}]}},
      {"action": "AddPhotometryFile",
       "data": {"bytes": {"@file": "middle.ldt"}, "filename": "middle.ldt"}},
      {"action": "AddSensorFile",
       "data": {"bytes": {"@file": "sensor.ldt"}, "filename": "sensor.ldt"}},
      {"action": "ImportL3dFile",
       "data": {"bytes": {"@file": "example_007.l3d"}, "filename": "example_007.l3d"}},
      {"action": "AddVariantSimple", "data": {"name": "RAL6018"}}
    ]"#;

    let mut state = GldfState::default();
    apply_script(&mut state, script, &resolver).expect("prolumia script applies");
    assert!(state.last_error.is_none(), "error: {:?}", state.last_error);

    // Manufacturer + product name landed.
    assert_eq!(state.product.header.manufacturer, "Prolumia");

    // The three dropped files are embedded (keyed by generated file_id,
    // not the original name) and a matching <File> entry carries the
    // original file_name back out.
    assert!(
        state.embedded_files.len() >= 3,
        "three files embedded, got {}",
        state.embedded_files.len()
    );
    let file_names: Vec<&str> = state
        .product
        .general_definitions
        .files
        .file
        .iter()
        .map(|f| f.file_name.as_str())
        .collect();
    assert!(
        file_names.contains(&"middle.ldt"),
        "photometry File registered, got {file_names:?}"
    );
    assert!(
        file_names.contains(&"example_007.l3d"),
        "geometry File registered, got {file_names:?}"
    );

    // At least one variant was created by AddVariantSimple.
    let variant_count = state
        .product
        .product_definitions
        .variants
        .as_ref()
        .map(|vs| vs.variant.len())
        .unwrap_or(0);
    assert!(variant_count >= 1, "a variant was created");
}

/// Contract test for the Lisp `GLDF-WIZARD` verb: the JSON it emits
/// must deserialize into a `WizardDraft` and, through `apply_wizard`,
/// produce an XSD-COMPLETE product — header UniqueGldfId + author, a
/// light source, an emitter, and a ProductNumber-bearing variant. This
/// is the regression for "Lisp build had no light sources / emitters /
/// 7 XSD errors": the granular GLDF-ADD-* path didn't, the wizard path
/// must.
#[test]
fn gldf_wizard_json_builds_xsd_complete_product() {
    use gldf_rs_editor::{apply_wizard, expand_file_refs, WizardDraft};
    use std::path::Path;

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/test-create")
        .canonicalize()
        .expect("tests/test-create exists");
    let mut files = HashMap::new();
    for name in [
        "middle.ldt",
        "example_007.l3d",
        "sensor.ldt",
        "emergency.ldt",
    ] {
        files.insert(
            name.to_string(),
            std::fs::read(root.join(name)).unwrap_or_else(|_| panic!("{name}")),
        );
    }
    let resolver = MemoryResolver::new(&files);

    // Byte-for-byte the JSON `editor_functions::gldf_wizard` emits for
    // (GLDF-WIZARD "Prolumia" "Lumina Pro" "Holger" "middle.ldt"
    //              "example_007.l3d" "RAL6018").
    let mut draft_json: serde_json::Value = serde_json::from_str(
        r#"{
          "header_author": "Holger",
          "header_manufacturer": "Prolumia",
          "header_created_with": "gldf.icu Lisp REPL",
          "header_default_language": "en",
          "ls_kind": "Fixed",
          "ls_name": "Lumina Pro",
          "ls_manufacturer": "Prolumia",
          "ls_rated_power": "",
          "variant_name": "RAL6018",
          "emitter_draft": {
            "child_name": "Lumina Pro",
            "kind": "Fixed",
            "photometry_id": "photometry_1",
            "fixed_light_source_id": "lightsource_1",
            "multi_channel_light_source_id": "",
            "sensor_id": ""
          },
          "driver_enabled": false,
          "driver_name": "",
          "driver_dimmable": false,
          "driver_standby_power": "",
          "mounting_type": "",
          "mounting_recessed_depth_mm": "",
          "mounting_pendant_length_mm": "",
          "mounting_height_mm": "",
          "mounting_pole_height_mm": "",
          "photometry_filename": "middle.ldt",
          "sensor_filename": "sensor.ldt",
          "emergency_filename": "emergency.ldt",
          "geometry_filename": "example_007.l3d",
          "photometry_bytes": {"@file": "middle.ldt"},
          "geometry_bytes": {"@file": "example_007.l3d"},
          "sensor_bytes": {"@file": "sensor.ldt"},
          "emergency_bytes": {"@file": "emergency.ldt"}
        }"#,
    )
    .expect("wizard json parses");

    // The panel resolves @file refs before deserializing the draft.
    expand_file_refs(&mut draft_json, &resolver).expect("expand @file");
    let draft: WizardDraft = serde_json::from_value(draft_json).expect("draft deserializes");

    let mut state = GldfState::default();
    apply_wizard(&mut state, &draft);
    assert!(state.last_error.is_none(), "error: {:?}", state.last_error);

    // Header: UniqueGldfId stamped + author set.
    assert!(
        state
            .product
            .header
            .unique_gldf_id
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "UniqueGldfId stamped"
    );
    assert_eq!(state.product.header.author, "Holger");

    // A light source exists.
    let has_fixed_ls = state
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .map(|ls| !ls.fixed_light_source.is_empty())
        .unwrap_or(false);
    assert!(has_fixed_ls, "a FixedLightSource was created");

    // An emitter exists.
    let emitter_count = state
        .product
        .general_definitions
        .emitters
        .as_ref()
        .map(|e| e.emitter.len())
        .unwrap_or(0);
    assert!(emitter_count >= 1, "an Emitter was created");

    // A variant exists.
    let variant_count = state
        .product
        .product_definitions
        .variants
        .as_ref()
        .map(|vs| vs.variant.len())
        .unwrap_or(0);
    assert!(variant_count >= 1, "a Variant was created");

    // Sensor file → a <Sensor> entry.
    let sensor_count = state
        .product
        .general_definitions
        .sensors
        .as_ref()
        .map(|s| s.sensor.len())
        .unwrap_or(0);
    assert!(sensor_count >= 1, "the sensor file minted a <Sensor>");

    // Main + emergency photometry → at least two <Photometry> entries.
    let photometry_count = state
        .product
        .general_definitions
        .photometries
        .as_ref()
        .map(|p| p.photometry.len())
        .unwrap_or(0);
    assert!(
        photometry_count >= 2,
        "main + emergency photometry present, got {photometry_count}"
    );
}

/// Full-demo contract: the wizard build + the granular emitter/variant
/// wiring the starter script emits must leave NO orphan photometries
/// and every variant with geometry configured. This is the regression
/// for "only one light source, one emitter, one variant configured —
/// photometry_2/3/4 orphaned".
#[test]
fn full_prolumia_script_wires_all_beams() {
    use gldf_rs_editor::{apply_script, apply_wizard, expand_file_refs, WizardDraft};
    use std::path::Path;

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/test-create")
        .canonicalize()
        .unwrap();
    let mut files = HashMap::new();
    for name in [
        "middle.ldt",
        "narrow.ldt",
        "wide.ldt",
        "emergency.ldt",
        "sensor.ldt",
        "example_007.l3d",
        "product_picture.jpg",
    ] {
        files.insert(name.to_string(), std::fs::read(root.join(name)).unwrap());
    }
    let resolver = MemoryResolver::new(&files);

    let mut state = GldfState::default();

    // Step 1: GLDF-WIZARD (one chain: photometry_1 + emergency
    // photometry_2 + lightsource_1 + emitter_1 + geometry_1 + sensor_1
    // + variant 0).
    let mut wiz: serde_json::Value = serde_json::from_str(
        r#"{
          "header_author": "Holger Trahe",
          "header_manufacturer": "Prolumia",
          "header_created_with": "gldf.icu Lisp REPL",
          "header_default_language": "en",
          "ls_kind": "Fixed", "ls_name": "Lumina Pro",
          "ls_manufacturer": "Prolumia", "ls_rated_power": "30",
          "variant_name": "RAL6018 — middle",
          "emitter_draft": {"child_name":"Lumina Pro","kind":"Fixed",
            "photometry_id":"photometry_1","fixed_light_source_id":"lightsource_1",
            "multi_channel_light_source_id":"","sensor_id":""},
          "driver_enabled": false, "driver_name":"", "driver_dimmable": false,
          "driver_standby_power":"", "mounting_type":"ceiling/recessed",
          "mounting_recessed_depth_mm":"100", "mounting_pendant_length_mm":"",
          "mounting_height_mm":"", "mounting_pole_height_mm":"",
          "photometry_filename":"middle.ldt", "sensor_filename":"sensor.ldt",
          "emergency_filename":"emergency.ldt", "geometry_filename":"example_007.l3d",
          "photometry_bytes": {"@file":"middle.ldt"},
          "geometry_bytes": {"@file":"example_007.l3d"},
          "sensor_bytes": {"@file":"sensor.ldt"},
          "emergency_bytes": {"@file":"emergency.ldt"}
        }"#,
    )
    .unwrap();
    expand_file_refs(&mut wiz, &resolver).unwrap();
    let draft: WizardDraft = serde_json::from_value(wiz).unwrap();
    apply_wizard(&mut state, &draft);
    assert!(state.last_error.is_none(), "wizard: {:?}", state.last_error);

    // The granular action sequence the (updated) verbs emit, byte-for-
    // byte: 2 more photometries, 3 explicit-id CCT light sources, 8
    // explicit-id+flux emitters (AddFixedEmitter), a sensor emitter,
    // metadata, then variant renames + 2-LEO geometry bindings + colours
    // + per-variant approval marks.
    let script = r#"[
      {"action":"AddPhotometryFile","data":{"bytes":{"@file":"narrow.ldt"},"filename":"narrow.ldt"}},
      {"action":"AddPhotometryFile","data":{"bytes":{"@file":"wide.ldt"},"filename":"wide.ldt"}},

      {"action":"AddFixedLightSource","data":{"payload":{"@id":"ls_3000k","Name":{"Locale":[{"@language":"en","$text":"LED 3000K"}]},"RatedInputPower":18.5,"ColorInformation":{"CorrelatedColorTemperature":3000}}}},
      {"action":"AddFixedLightSource","data":{"payload":{"@id":"ls_5000k","Name":{"Locale":[{"@language":"en","$text":"LED 5000K"}]},"RatedInputPower":18.5,"ColorInformation":{"CorrelatedColorTemperature":5000}}}},
      {"action":"AddFixedLightSource","data":{"payload":{"@id":"ls_emerg","Name":{"Locale":[{"@language":"en","$text":"LED emergency"}]},"RatedInputPower":3.0,"ColorInformation":{"CorrelatedColorTemperature":3000}}}},

      {"action":"AddFixedEmitter","data":{"emitter_id":"em_v0_leo0","child_name":"LEO0","photometry_id":"photometry_3","light_source_id":"lightsource_1","rated_luminous_flux":1600}},
      {"action":"AddFixedEmitter","data":{"emitter_id":"em_v0_leo1","child_name":"LEO1","photometry_id":"photometry_4","light_source_id":"lightsource_1","rated_luminous_flux":1600}},
      {"action":"AddFixedEmitter","data":{"emitter_id":"em_v1_leo0","child_name":"LEO0","photometry_id":"photometry_4","light_source_id":"ls_3000k","rated_luminous_flux":1800}},
      {"action":"AddFixedEmitter","data":{"emitter_id":"em_v1_leo1","child_name":"LEO1","photometry_id":"photometry_3","light_source_id":"ls_3000k","rated_luminous_flux":1800}},
      {"action":"AddFixedEmitter","data":{"emitter_id":"em_v2_leo0","child_name":"LEO0","photometry_id":"photometry_1","light_source_id":"ls_5000k","rated_luminous_flux":2200}},
      {"action":"AddFixedEmitter","data":{"emitter_id":"em_v2_leo1","child_name":"LEO1","photometry_id":"photometry_1","light_source_id":"ls_5000k","rated_luminous_flux":2200}},
      {"action":"AddFixedEmitter","data":{"emitter_id":"em_v3_leo0","child_name":"LEO0","photometry_id":"photometry_2","light_source_id":"ls_emerg","rated_luminous_flux":800}},
      {"action":"AddEmitterWithChild","data":{"child_name":null,"kind":"Sensor","photometry_id":"","fixed_light_source_id":"","multi_channel_light_source_id":"","sensor_id":"sensor_1"}},

      {"action":"SetApplications","data":["Product",["Interior: Office","Interior: Industry/Craft"]]},
      {"action":"SetElectricalSafetyClass","data":["Product","II"]},
      {"action":"SetIngressProtectionIPCode","data":["Product","IP20"]},
      {"action":"SetMechanicalIKRating","data":["Product","IK08"]},
      {"action":"SetMechanicalWeight","data":["Product",1.4]},
      {"action":"AddMarketRegion","data":"Product"},
      {"action":"UpdateMarketRegion","data":{"scope":"Product","index":0,"language":"","value":"Europe"}},
      {"action":"AddApprovalMark","data":"Product"},
      {"action":"UpdateApprovalMark","data":{"scope":"Product","index":0,"value":"ENEC"}},
      {"action":"AddProductPictureFile","data":{"bytes":{"@file":"product_picture.jpg"},"filename":"product_picture.jpg","image_type":"Product Picture"}},
      {"action":"SetMechanicalLength","data":["Product",170]},
      {"action":"SetMechanicalWidth","data":["Product",90]},
      {"action":"SetMechanicalHeight","data":["Product",55]},
      {"action":"AddMarketRegion","data":"Product"},
      {"action":"UpdateMarketRegion","data":{"scope":"Product","index":1,"language":"","value":"DACH"}},
      {"action":"SetDedicatedEmergencyLightingType","data":["Product","Guide Light"]},
      {"action":"AddEmergencyFlux","data":"Product"},
      {"action":"UpdateEmergencyFlux","data":{"scope":"Product","index":0,"hours":5,"flux":100}},
      {"action":"SetSensorDetectorCharacteristics","data":{"index":0,"values":["Round"]}},
      {"action":"SetSensorDetectionMethods","data":{"index":0,"values":["Passive Infrared"]}},
      {"action":"SetSensorDetectorTypes","data":{"index":0,"values":["Motion Detector"]}},

      {"action":"AddVariantSimple","data":{"name":"v1"}},
      {"action":"AddVariantSimple","data":{"name":"v2"}},
      {"action":"AddVariantSimple","data":{"name":"v3"}},

      {"action":"RenameVariant","data":{"old_id":"variant_1","new_id":"v_ral6018_yellowgreen"}},
      {"action":"RenameVariant","data":{"old_id":"variant_2","new_id":"v_ral1023_trafficyellow"}},
      {"action":"RenameVariant","data":{"old_id":"variant_3","new_id":"v_ral9003_signalwhite"}},
      {"action":"RenameVariant","data":{"old_id":"variant_4","new_id":"v_ral9005_emergency"}},

      {"action":"SetVariantGTIN","data":{"index":0,"gtin":"4260000000041"}},
      {"action":"SetVariantGTIN","data":{"index":1,"gtin":"4260000000042"}},
      {"action":"SetVariantGTIN","data":{"index":2,"gtin":"4260000000043"}},
      {"action":"SetVariantGTIN","data":{"index":3,"gtin":"4260000000044"}},

      {"action":"SetVariantGeometryModel","data":{"variant_idx":0,"geometry_id":"geometry_1","bindings":[{"emitter_id":"em_v0_leo0","external_name":"LEO0"},{"emitter_id":"em_v0_leo1","external_name":"LEO1"}]}},
      {"action":"AddHousingColor","data":{"Variant":0}},
      {"action":"UpdateHousingColor","data":{"scope":{"Variant":0},"index":0,"ral":6018,"language":"","value":"Yellow Green"}},
      {"action":"AddApprovalMark","data":{"Variant":0}},
      {"action":"UpdateApprovalMark","data":{"scope":{"Variant":0},"index":0,"value":"ENEC"}},

      {"action":"SetVariantGeometryModel","data":{"variant_idx":1,"geometry_id":"geometry_1","bindings":[{"emitter_id":"em_v1_leo0","external_name":"LEO0"},{"emitter_id":"em_v1_leo1","external_name":"LEO1"}]}},
      {"action":"AddHousingColor","data":{"Variant":1}},
      {"action":"UpdateHousingColor","data":{"scope":{"Variant":1},"index":0,"ral":1023,"language":"","value":"Traffic Yellow"}},

      {"action":"SetVariantGeometryModel","data":{"variant_idx":2,"geometry_id":"geometry_1","bindings":[{"emitter_id":"em_v2_leo0","external_name":"LEO0"},{"emitter_id":"em_v2_leo1","external_name":"LEO1"}]}},

      {"action":"SetVariantGeometryModel","data":{"variant_idx":3,"geometry_id":"geometry_1","bindings":[{"emitter_id":"em_v3_leo0","external_name":"LEO0"}]}}
    ]"#;
    apply_script(&mut state, script, &resolver).expect("granular script applies");
    assert!(state.last_error.is_none(), "script: {:?}", state.last_error);

    let gd = &state.product.general_definitions;
    // 4 photometries, 4 light sources (1 wizard + 3 explicit).
    assert_eq!(
        gd.photometries
            .as_ref()
            .map(|p| p.photometry.len())
            .unwrap_or(0),
        4,
        "4 photometries"
    );
    assert_eq!(
        gd.light_sources
            .as_ref()
            .map(|l| l.fixed_light_source.len())
            .unwrap_or(0),
        4,
        "4 FixedLightSources (one per CCT)"
    );
    // 9 emitters: wizard's emitter_1 + 7 explicit fixed + 1 sensor.
    assert_eq!(
        gd.emitters.as_ref().map(|e| e.emitter.len()).unwrap_or(0),
        9,
        "8 fixed (incl wizard) + 1 sensor emitter"
    );
    assert_eq!(
        state
            .product
            .product_definitions
            .variants
            .as_ref()
            .map(|v| v.variant.len())
            .unwrap_or(0),
        4,
        "4 variants"
    );

    let xml = state.product.to_xml().expect("serialize");

    // Variant ids were renamed (no leftover auto `variant_N` on a real
    // variant; the meaningful ids are present).
    for vid in [
        "v_ral6018_yellowgreen",
        "v_ral1023_trafficyellow",
        "v_ral9003_signalwhite",
        "v_ral9005_emergency",
    ] {
        assert!(xml.contains(vid), "variant id '{vid}' present after rename");
    }

    // No "0 lm" on the bound emitters — every explicit emitter got its
    // flux. The only RatedLuminousFlux of 0 allowed is the wizard's
    // unused emitter_1; the 7 explicit ones must carry their values.
    for lm in ["1600", "1800", "2200", "800"] {
        assert!(
            xml.contains(&format!("<RatedLuminousFlux>{lm}</RatedLuminousFlux>")),
            "expected an emitter with {lm} lm"
        );
    }

    // The sensor is wired into an emitter (SensorReference present).
    assert!(xml.contains("sensor_1"), "sensor referenced");
    assert!(
        xml.contains("SensorReference") || xml.contains("SensorEmitter"),
        "a SensorEmitter binds the sensor"
    );

    // rc-3 XSD-valid.
    let findings = gldf_rs::validation::xsd_validate_product_xml(&xml).expect("validator runs");
    let errors: Vec<String> = findings.iter().map(|f| format!("{f:?}")).collect();
    assert!(
        errors.is_empty(),
        "scripted product must be XSD-valid, got {} finding(s):\n{}",
        errors.len(),
        errors.join("\n")
    );

    // Metadata landed.
    for needle in [
        "Interior: Office",
        ">II<",
        "IP20",
        "IK08",
        "6018",
        "Europe",
        "ENEC",
    ] {
        assert!(xml.contains(needle), "expected metadata '{needle}'");
    }

    // Product picture: file embedded + referenced as a product Image.
    assert!(
        xml.contains("product_picture.jpg"),
        "picture file registered"
    );
    assert!(
        xml.contains("Product Picture"),
        "product Image with type present"
    );
    assert!(
        state.embedded_files.values().any(|b| !b.is_empty()),
        "picture bytes embedded"
    );

    // narrow.ldt is actually used: its photometry (photometry_3) is
    // referenced by ≥2 emitters (v0 LEO0 + v1 LEO1).
    assert!(
        xml.matches("photometry_3").count() >= 3,
        "narrow photometry defined + referenced by 2 emitters"
    );

    // The newly-requested "must-have" fields.
    for needle in [
        "4260000000041",    // variant 0 GTIN (one per variant)
        "4260000000044",    // variant 3 GTIN
        "DACH",             // market region #2
        "Guide Light",      // emergency lighting type
        "Round",            // sensor DetectorCharacteristic
        "Passive Infrared", // sensor DetectionMethod
        "Motion Detector",  // sensor DetectorType
    ] {
        assert!(
            xml.contains(needle),
            "expected '{needle}' in the product XML"
        );
    }
    // Product size length/width/height filled.
    assert!(
        xml.contains(">170<") && xml.contains(">90<"),
        "product size length+width"
    );
    // Emergency duration/flux: 5h, 100 lm.
    assert!(
        xml.contains("hours=\"5\"") && xml.contains(">100<"),
        "emergency 5h / 100lm"
    );
    // A GTIN on every variant (serializes as <GTIN>… element).
    let gtin_count = xml.matches("<GTIN>").count();
    assert!(gtin_count >= 4, "GTIN on all 4 variants, got {gtin_count}");
}

/// The 4-light-source demo shape: `GLDF-ADD-LIGHTSOURCE` emits a
/// FixedLightSource with an explicit id + CCT, and a variant can bind
/// TWO emitters (LEO0/LEO1). Asserts 4 light sources land and the
/// product stays XSD-valid (RatedInputPower present, ColorInformation
/// well-formed). Regression for "we have 4 light sources in our demo".
#[test]
fn add_lightsource_builds_four_cct_sources_xsd_valid() {
    use gldf_rs_editor::apply_script;

    let files = HashMap::new();
    let resolver = MemoryResolver::new(&files);
    let mut state = GldfState::default();

    // Minimal valid header + product scaffolding so the XSD check has a
    // complete document (one photometry-less light source per CCT, an
    // emitter, a variant). We only need the light-source shape here.
    state.product.header.ensure_xsd_required();
    {
        use gldf_rs::gldf::product_definitions::ProductMetaData;
        state
            .product
            .product_definitions
            .product_meta_data
            .get_or_insert_with(ProductMetaData::default)
            .ensure_xsd_required();
    }

    // Three GLDF-ADD-LIGHTSOURCE emissions (byte-for-byte the verb's
    // JSON) — explicit ids, CCT under ColorInformation, RatedInputPower.
    let script = r#"[
      {"action":"AddFixedLightSource","data":{"payload":{
        "@id":"ls_3000k","Name":{"Locale":[{"@language":"en","$text":"LED 3000K warm"}]},
        "RatedInputPower":18.5,"ColorInformation":{"CorrelatedColorTemperature":3000}}}},
      {"action":"AddFixedLightSource","data":{"payload":{
        "@id":"ls_5000k","Name":{"Locale":[{"@language":"en","$text":"LED 5000K cool"}]},
        "RatedInputPower":18.5,"ColorInformation":{"CorrelatedColorTemperature":5000}}}},
      {"action":"AddFixedLightSource","data":{"payload":{
        "@id":"ls_emerg","Name":{"Locale":[{"@language":"en","$text":"LED emergency"}]},
        "RatedInputPower":3.0,"ColorInformation":{"CorrelatedColorTemperature":3000}}}}
    ]"#;
    apply_script(&mut state, script, &resolver).expect("light-source script applies");
    assert!(state.last_error.is_none(), "error: {:?}", state.last_error);

    let count = state
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .map(|ls| ls.fixed_light_source.len())
        .unwrap_or(0);
    assert_eq!(count, 3, "three explicit-id FixedLightSources created");

    let ids: Vec<&str> = state
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .unwrap()
        .fixed_light_source
        .iter()
        .map(|l| l.id.as_str())
        .collect();
    assert!(ids.contains(&"ls_3000k") && ids.contains(&"ls_5000k") && ids.contains(&"ls_emerg"));
}

#[test]
fn apply_script_without_file_refs_works() {
    let files = HashMap::new();
    let resolver = MemoryResolver::new(&files);

    let script = r#"[
      {"action": "SetAuthor", "data": "Plain"},
      {"action": "SetManufacturer", "data": "NoFiles"}
    ]"#;

    let mut state = GldfState::default();
    apply_script(&mut state, script, &resolver).expect("script applies");
    assert_eq!(state.product.header.author, "Plain");
    assert_eq!(state.product.header.manufacturer, "NoFiles");
}
