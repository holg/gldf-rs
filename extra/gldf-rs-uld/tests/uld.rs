use gldf_rs_editor::MemoryResolver;
use gldf_rs_editor::{process_one, BatchOptions, BatchTemplate};
use std::collections::HashMap;
use std::path::Path;

/// A DIALux .uld container is accepted once the converter is registered:
/// converted to a complete GLDF (geometry included) with the wizard
/// template contributing only header overrides.
#[test]
fn uld_input_converts_with_header_overrides() {
    gldf_rs_uld::register();
    let uld =
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/test_1.uld"))
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
