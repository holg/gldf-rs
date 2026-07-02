//! Batch-conversion pipeline tests: template parsing, placeholder
//! substitution + autofill in `process_one`, and the end-to-end
//! directory runner (`collect_inputs` + `run_batch`) over the real
//! `tests/test-create/` LDT assets.

use gldf_rs_editor::batch::{
    collect_inputs, process_one, run_batch, BatchEvent, BatchJob, BatchOptions, BatchTemplate,
    TemplateDraft,
};
use gldf_rs_editor::MemoryResolver;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn test_create_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/test-create")
}

fn narrow_ldt() -> Vec<u8> {
    std::fs::read(test_create_dir().join("narrow.ldt")).expect("read narrow.ldt")
}

#[test]
fn template_parse_classifies_by_shape() {
    assert!(matches!(
        BatchTemplate::parse("{}").unwrap(),
        BatchTemplate::Wizard(_)
    ));
    assert!(matches!(
        BatchTemplate::parse("[]").unwrap(),
        BatchTemplate::Script(_)
    ));
    assert!(BatchTemplate::parse("42").is_err());
}

/// A `{}` template must already yield a complete GLDF: the input LDT
/// embedded as photometry, a light source, an emitter, and a variant.
#[test]
fn empty_template_autofills_from_ldt() {
    let ldt = narrow_ldt();
    let template = BatchTemplate::parse("{}").unwrap();
    let files = HashMap::new();
    let resolver = MemoryResolver::new(&files);
    let out = process_one(
        &ldt,
        "narrow.ldt",
        &template,
        &resolver,
        &BatchOptions {
            write_gldf: true,
            write_ifc: true,
        },
    )
    .expect("process_one");

    let gldf_bytes = out.gldf.expect("gldf bytes");
    let findings =
        gldf_rs::validation::xsd_validate_gldf_archive(&gldf_bytes).expect("xsd validate");
    assert!(
        findings.is_empty(),
        "batch output must be XSD-clean, got: {findings:?}"
    );
    let loaded = gldf_rs::GldfProduct::load_gldf_from_buf_all(gldf_bytes).expect("re-load gldf");
    let gd = &loaded.gldf.general_definitions;
    assert_eq!(gd.files.file.len(), 1, "one embedded photometry file");
    assert!(gd.photometries.as_ref().is_some_and(|p| !p.photometry.is_empty()));
    assert!(gd
        .light_sources
        .as_ref()
        .is_some_and(|ls| !ls.fixed_light_source.is_empty()));
    assert!(gd.emitters.as_ref().is_some_and(|e| !e.emitter.is_empty()));
    assert!(loaded
        .gldf
        .product_definitions
        .variants
        .as_ref()
        .is_some_and(|v| !v.variant.is_empty()));

    let ifc = out.ifc.expect("ifc text");
    assert!(ifc.contains("IFCLIGHTFIXTURETYPE"), "IFC has a light fixture");
}

#[test]
fn placeholders_substitute_ldt_metadata() {
    let ldt = narrow_ldt();
    let template = BatchTemplate::parse(
        r#"{
            "header_author": "batch: {input.stem}",
            "header_manufacturer": "{ldt.manufacturer}",
            "variant_name": "{ldt.name} / {ldt.watts} W"
        }"#,
    )
    .unwrap();
    let files = HashMap::new();
    let resolver = MemoryResolver::new(&files);
    let out = process_one(
        &ldt,
        "narrow.ldt",
        &template,
        &resolver,
        &BatchOptions::default(),
    )
    .expect("process_one");

    let loaded =
        gldf_rs::GldfProduct::load_gldf_from_buf_all(out.gldf.expect("gldf")).expect("re-load");
    assert_eq!(loaded.gldf.header.author, "batch: narrow");
    assert!(
        !loaded.gldf.header.manufacturer.is_empty(),
        "manufacturer substituted from LDT line 1"
    );
    let variants = loaded.gldf.product_definitions.variants.expect("variants");
    let vname = &variants.variant[0].name;
    let vname_value = vname
        .as_ref()
        .and_then(|n| n.locale.first())
        .map(|l| l.value.clone())
        .unwrap_or_default();
    assert!(
        vname_value.contains("W") && !vname_value.contains("{ldt."),
        "placeholders resolved in variant name, got: {vname_value}"
    );
}

#[test]
fn run_batch_converts_directory_to_gldf_and_ifc() {
    let inputs = collect_inputs(&[test_create_dir()]);
    assert!(
        inputs.len() >= 3,
        "expected several .ldt in tests/test-create, got {}",
        inputs.len()
    );

    let out_dir = std::env::temp_dir().join(format!(
        "gldf-batch-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&out_dir);

    let job = BatchJob {
        template: BatchTemplate::parse(r#"{"header_author": "batch runner"}"#).unwrap(),
        template_dir: test_create_dir(),
        inputs: inputs.clone(),
        out_dir: out_dir.clone(),
        options: BatchOptions {
            write_gldf: true,
            write_ifc: true,
        },
    };

    let mut started = 0usize;
    let mut finished = 0usize;
    let reports = run_batch(&job, &mut |ev| {
        match ev {
            BatchEvent::Started { .. } => started += 1,
            BatchEvent::Finished { .. } => finished += 1,
        }
        true
    })
    .expect("run_batch");

    assert_eq!(reports.len(), inputs.len());
    assert_eq!(started, inputs.len());
    assert_eq!(finished, inputs.len());
    for r in &reports {
        assert!(
            r.error.is_none(),
            "{} failed: {:?}",
            r.input.display(),
            r.error
        );
        let gldf = r.gldf_path.as_ref().expect("gldf written");
        let ifc = r.ifc_path.as_ref().expect("ifc written");
        assert!(gldf.exists() && std::fs::metadata(gldf).unwrap().len() > 0);
        assert!(ifc.exists() && std::fs::metadata(ifc).unwrap().len() > 0);
    }

    let _ = std::fs::remove_dir_all(&out_dir);
}

/// The template-editor form model round-trips through template JSON,
/// including `@file` refs for the shared byte slots, and the produced
/// JSON both parses as a batch template and drives a conversion.
#[test]
fn template_draft_roundtrips_and_converts() {
    let mut form = TemplateDraft::default();
    form.draft.header_author = "Form author".to_string();
    form.draft.variant_name = "{ldt.name}".to_string();
    form.geometry_file = "example_007.l3d".to_string();

    let json = form.to_template_json();
    assert!(json.contains("\"@file\""), "geometry saved as @file ref");
    assert!(!json.contains("photometry_bytes"), "no byte-slot noise");

    let reparsed = TemplateDraft::from_template_value(
        &serde_json::from_str(&json).unwrap(),
    )
    .expect("round-trip back into the form");
    assert_eq!(reparsed.draft.header_author, "Form author");
    assert_eq!(reparsed.geometry_file, "example_007.l3d");

    // And the emitted JSON works as an actual batch template, with the
    // @file geometry resolved relative to the template dir.
    let template = BatchTemplate::parse(&json).expect("parse as batch template");
    let ldt = narrow_ldt();
    let geometry = std::fs::read(test_create_dir().join("example_007.l3d"))
        .expect("read example_007.l3d");
    let mut files = HashMap::new();
    files.insert("example_007.l3d".to_string(), geometry);
    let resolver = MemoryResolver::new(&files);
    let out = process_one(
        &ldt,
        "narrow.ldt",
        &template,
        &resolver,
        &BatchOptions::default(),
    )
    .expect("process_one with @file geometry");
    let loaded =
        gldf_rs::GldfProduct::load_gldf_from_buf_all(out.gldf.expect("gldf")).expect("re-load");
    assert!(
        loaded
            .gldf
            .general_definitions
            .geometries
            .as_ref()
            .is_some_and(|g| !g.model_geometry.is_empty()),
        "shared L3D geometry embedded via @file ref"
    );
}

/// With the `uld` feature, a DIALux .uld container is silently accepted:
/// converted to a complete GLDF (geometry included) with the wizard
/// template contributing only header overrides.
#[cfg(feature = "uld")]
#[test]
fn uld_input_converts_with_header_overrides() {
    let uld = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/test_1.uld"),
    )
    .expect("read tests/data/test_1.uld");
    let template = BatchTemplate::parse(
        r#"{"header_author": "batch: {input.stem}", "header_created_with": "uld-batch"}"#,
    )
    .unwrap();
    let files = HashMap::new();
    let resolver = MemoryResolver::new(&files);
    let out = process_one(
        &uld,
        "test_1.uld",
        &template,
        &resolver,
        &BatchOptions {
            write_gldf: true,
            write_ifc: true,
        },
    )
    .expect("process_one on .uld");

    let loaded =
        gldf_rs::GldfProduct::load_gldf_from_buf_all(out.gldf.expect("gldf")).expect("re-load");
    assert_eq!(loaded.gldf.header.author, "batch: test_1");
    assert_eq!(loaded.gldf.header.created_with_application, "uld-batch");
    assert!(
        !loaded.files.is_empty(),
        "converted GLDF carries embedded files (geometry etc.)"
    );
    assert!(out.ifc.expect("ifc").contains("IFCLIGHTFIXTURETYPE"));
}

/// Returning `false` from the callback cancels the run after the
/// current file.
#[test]
fn run_batch_cancels_via_callback() {
    let inputs = collect_inputs(&[test_create_dir()]);
    assert!(inputs.len() >= 2);

    let out_dir = std::env::temp_dir().join(format!(
        "gldf-batch-cancel-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&out_dir);

    let job = BatchJob {
        template: BatchTemplate::parse("{}").unwrap(),
        template_dir: test_create_dir(),
        inputs,
        out_dir: out_dir.clone(),
        options: BatchOptions::default(),
    };

    let reports = run_batch(&job, &mut |ev| {
        !matches!(ev, BatchEvent::Finished { index: 0, .. })
    })
    .expect("run_batch");
    assert_eq!(reports.len(), 1, "cancelled after the first file");

    let _ = std::fs::remove_dir_all(&out_dir);
}
