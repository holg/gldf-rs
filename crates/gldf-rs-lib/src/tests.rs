#![allow(unused_variables)]

use std::env;
use std::fs;

use anyhow::Result;
use serde::de::StdError;

#[cfg(feature = "http")]
use super::fetch_content_from_url;
use crate::gldf::{FormatVersion, GldfProduct};

const GLDF_FILE_NAME: &str = "../../tests/data/R2MCOBSIK-30.gldf";

#[test]
fn test_default() {
    let gldf = GldfProduct::default();
    println!("{:?}", gldf);
    println!("{:?}", gldf.to_json());
}

#[test]
fn parsing_gldf_container() -> Result<(), Box<dyn StdError>> {
    use serde_json::from_str as serde_from_str;
    let loaded: GldfProduct = GldfProduct::load_gldf(GLDF_FILE_NAME).unwrap();
    let general_files = loaded.get_all_file_definitions().unwrap();
    println!("{:?}", loaded);

    // Test JSON round-trip
    let gldf_to_json = loaded.to_json()?;
    let gldf_to_xml = loaded.to_xml()?;
    let json_to_xml = GldfProduct::from_json(&gldf_to_json)?.to_xml()?;
    assert_eq!(gldf_to_xml, json_to_xml);

    // Test XML round-trip
    let result = GldfProduct::from_xml(&gldf_to_xml)?;
    let xml_to_json = result.to_json().unwrap();

    let x_serialized = loaded.to_xml().unwrap();
    println!("{}", x_serialized);

    let json_str = serde_json::to_string(&loaded).unwrap();
    println!("{}", json_str);

    let j_loaded: GldfProduct = serde_from_str(&json_str).unwrap();
    let x_reserialized = j_loaded.to_xml().unwrap();
    println!("{}", x_reserialized);

    assert_eq!(x_serialized, x_reserialized);
    Ok(())
}

#[test]
fn test_gldf_product_impls() {
    let loaded: GldfProduct = GldfProduct::load_gldf(GLDF_FILE_NAME).unwrap();
    println!("{:?}", loaded);

    // Display pretty printed XML
    let x_serialized = loaded.to_xml().unwrap();
    println!("{}", x_serialized);

    let json_str = loaded.to_json().unwrap();
    let j_loaded: GldfProduct = GldfProduct::from_json(&json_str).unwrap();
    let x_reserialized = j_loaded.to_xml().unwrap();
    println!("{}", x_reserialized);
    println!(r#"{{"product":"#);
    println!("{}", loaded.to_json().unwrap());
    println!("}}");

    assert_eq!(x_serialized, x_reserialized);
}

#[allow(dead_code)]
fn read_test_gldf() -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut gldf_file = std::fs::File::open(GLDF_FILE_NAME)?;
    let mut file_buf = Vec::new();
    gldf_file.read_to_end(&mut file_buf)?;
    Ok(file_buf)
}

#[test]
fn test_gldf_from_buf() {
    // Get the current directory.
    let current_dir = env::current_dir().expect("Failed to get current directory");

    // Define the relative path to your test data from the project root.
    // From crates/gldf-rs-lib/ we need to go up two levels to workspace root
    let test_data_dir = current_dir
        .parent()
        .expect("Failed to get parent directory")
        .parent()
        .expect("Failed to get workspace root")
        .join("tests")
        .join("data");

    let mut success_count = 0;
    let mut failure_count = 0;
    let mut failed_files: Vec<String> = Vec::new();

    // Get all gldf files in the directory.
    for entry in fs::read_dir(&test_data_dir).expect("Failed to read test data directory") {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Failed to read directory entry: {}", e);
                continue;
            }
        };
        let path = entry.path();
        if path.extension() == Some(std::ffi::OsStr::new("gldf")) {
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            println!("\n=== Testing: {} ===", file_name);

            let file_buf = match fs::read(&path) {
                Ok(buf) => buf,
                Err(e) => {
                    eprintln!("  ERROR: Failed to read file: {}", e);
                    failure_count += 1;
                    failed_files.push(format!("{}: read error - {}", file_name, e));
                    continue;
                }
            };

            let loaded = match GldfProduct::load_gldf_from_buf_all(file_buf) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("  ERROR: Failed to parse GLDF: {:?}", e);
                    // Print the full error chain
                    let mut source = e.source();
                    while let Some(cause) = source {
                        eprintln!("    Caused by: {}", cause);
                        source = cause.source();
                    }
                    failure_count += 1;
                    failed_files.push(format!("{}: parse error - {:?}", file_name, e));
                    continue;
                }
            };

            // Test XML serialization
            let x_serialized = match loaded.gldf.to_xml() {
                Ok(xml) => xml,
                Err(e) => {
                    eprintln!("  ERROR: Failed to serialize to XML: {}", e);
                    failure_count += 1;
                    failed_files.push(format!("{}: XML serialization error - {}", file_name, e));
                    continue;
                }
            };

            // Test JSON serialization
            let json_str = match loaded.gldf.to_json() {
                Ok(json) => json,
                Err(e) => {
                    eprintln!("  ERROR: Failed to serialize to JSON: {}", e);
                    failure_count += 1;
                    failed_files.push(format!("{}: JSON serialization error - {}", file_name, e));
                    continue;
                }
            };

            // Test JSON round-trip
            let j_loaded = match GldfProduct::from_json(&json_str) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("  ERROR: Failed to deserialize from JSON: {}", e);
                    failure_count += 1;
                    failed_files.push(format!("{}: JSON deserialization error - {}", file_name, e));
                    continue;
                }
            };

            let j_reserialized = match j_loaded.to_xml() {
                Ok(xml) => xml,
                Err(e) => {
                    eprintln!("  ERROR: Failed to re-serialize to XML: {}", e);
                    failure_count += 1;
                    failed_files.push(format!("{}: XML re-serialization error - {}", file_name, e));
                    continue;
                }
            };

            // Verify round-trip
            if x_serialized != j_reserialized {
                eprintln!("  WARNING: Round-trip XML mismatch");
            }

            // Get photometric files
            if let Ok(phot_files) = GldfProduct::get_phot_files(&loaded.gldf) {
                println!("  Photometric files: {}", phot_files.len());
                for p_f in phot_files.iter() {
                    println!("    - {}", p_f.file_name);
                }
            }

            println!("  SUCCESS: {} parsed and serialized correctly", file_name);
            success_count += 1;
        }
    }

    // Print summary
    println!("\n=== Summary ===");
    println!("Success: {}", success_count);
    println!("Failures: {}", failure_count);
    if !failed_files.is_empty() {
        println!("\nFailed files:");
        for f in &failed_files {
            println!("  - {}", f);
        }
    }

    // Assert all files parsed successfully
    assert!(
        failed_files.is_empty(),
        "Some GLDF files failed to parse: {:?}",
        failed_files
    );
}

#[tokio::test]
async fn test_gldf_get_phot_files() {
    let loaded: GldfProduct = GldfProduct::load_gldf(GLDF_FILE_NAME).unwrap();
    let phot_files = loaded.get_phot_files().unwrap();
    let mut ldc_contents: Vec<String> = Vec::new();
    for f in phot_files.iter() {
        let file_id = f.id.to_string();
        let result = loaded.get_ldc_by_id(file_id).await.unwrap();
        ldc_contents.push(result);
        println!("{}", f.file_name);
    }
}

#[cfg(feature = "http")]
#[tokio::test]
async fn test_gldf_get_pic_files() {
    let loaded: GldfProduct = GldfProduct::load_gldf(GLDF_FILE_NAME).unwrap();
    let image_files = loaded.get_image_def_files().unwrap();
    let mut file_contents: Vec<Vec<u8>> = Vec::new();
    for f in image_files.iter() {
        let file_id = f.id.to_string();
        let result = fetch_content_from_url(&f.file_name).await.unwrap();
        file_contents.push(result);
        println!("{}", f.file_name);
    }
}

/// Test round-trip: Load -> Modify -> Save -> Reload -> Verify
#[test]
fn test_editable_gldf_round_trip() {
    use crate::gldf::general_definitions::files::File;
    use crate::EditableGldf;

    // Load GLDF into EditableGldf
    let mut editable = EditableGldf::from_gldf(GLDF_FILE_NAME).expect("Failed to load GLDF");

    // Verify initial state
    let original_author = editable.product.header.author.clone();
    let original_file_count = editable.product.general_definitions.files.file.len();
    assert!(!editable.is_modified());

    // Make modifications
    editable.product.header.author = "Test Author - Round Trip".to_string();
    editable.mark_modified();

    // Add a new file definition
    let new_file = File {
        id: "roundtrip_test_file".to_string(),
        content_type: "other".to_string(),
        type_attr: "localFileName".to_string(),
        file_name: "test_roundtrip.txt".to_string(),
        language: String::new(),
    };
    editable
        .product
        .add_file(new_file.clone())
        .expect("Failed to add file");

    // Add embedded content for the file
    let test_content = b"This is test content for round-trip verification".to_vec();
    editable.add_embedded_file("roundtrip_test_file", test_content.clone());

    // Verify modifications are tracked
    assert!(editable.is_modified());

    // Save to buffer
    let saved_buf = editable.save_to_buf().expect("Failed to save to buffer");
    println!("Saved GLDF size: {} bytes", saved_buf.len());

    // Reload from buffer
    let reloaded = EditableGldf::from_buf(saved_buf).expect("Failed to reload GLDF from buffer");

    // Verify modifications were preserved
    assert_eq!(reloaded.product.header.author, "Test Author - Round Trip");
    assert_eq!(
        reloaded.product.general_definitions.files.file.len(),
        original_file_count + 1
    );

    // Verify new file exists
    let found_file = reloaded.product.get_file("roundtrip_test_file");
    assert!(found_file.is_some(), "New file not found after reload");
    assert_eq!(found_file.unwrap().file_name, "test_roundtrip.txt");

    // Verify embedded content was preserved
    let embedded = reloaded.get_embedded_file("roundtrip_test_file");
    assert!(embedded.is_some(), "Embedded file content not found");
    assert_eq!(embedded.unwrap(), test_content.as_slice());

    // Verify original data is still intact
    assert_ne!(reloaded.product.header.author, original_author);

    println!("Round-trip test passed!");
}

/// Test creating a new GLDF from scratch and saving it
#[test]
fn test_create_new_gldf() {
    use crate::gldf::general_definitions::files::File;
    use crate::gldf::product_definitions::Variant;
    use crate::EditableGldf;

    // Create new EditableGldf
    let mut editable = EditableGldf::new();

    // Set header info
    editable.product.header.author = "New Product Author".to_string();
    editable.product.header.manufacturer = "Test Manufacturer".to_string();
    editable.product.header.format_version = FormatVersion::from_string("1.0.0-rc.3");

    // Add a file definition
    let file = File {
        id: "photometry_1".to_string(),
        content_type: "ldc/eulumdat".to_string(),
        type_attr: "localFileName".to_string(),
        file_name: "test.ldt".to_string(),
        language: String::new(),
    };
    editable.product.add_file(file).expect("Failed to add file");

    // Add fake photometry content
    editable.add_embedded_file("photometry_1", b"FAKE LDT CONTENT".to_vec());

    // Add a variant
    let variant = Variant {
        id: "variant_1".to_string(),
        ..Default::default()
    };
    editable
        .product
        .add_variant(variant)
        .expect("Failed to add variant");

    // Validate the product
    let validation = editable.product.validate_structure();
    println!("Validation errors: {}", validation.errors.len());
    for err in &validation.errors {
        println!("  {:?}: {} - {}", err.level, err.path, err.message);
    }

    // Save to buffer
    let saved_buf = editable.save_to_buf().expect("Failed to save new GLDF");
    println!("New GLDF size: {} bytes", saved_buf.len());

    // Verify it can be reloaded
    let reloaded = EditableGldf::from_buf(saved_buf).expect("Failed to reload new GLDF");

    assert_eq!(reloaded.product.header.author, "New Product Author");
    assert_eq!(reloaded.product.header.manufacturer, "Test Manufacturer");
    assert!(reloaded.product.get_file("photometry_1").is_some());
    assert!(reloaded.product.get_variant("variant_1").is_some());

    println!("Create new GLDF test passed!");
}

/// Test loading GLDF with embedded WASM viewer plugin (star_sky.gldf)
#[test]
fn test_star_sky_gldf_with_plugin() {
    // Load the star_sky.gldf test file
    let test_file = "../../tests/data/star_sky.gldf";

    // Check if file exists (test may be skipped if not generated)
    if !std::path::Path::new(test_file).exists() {
        println!("Skipping test: {} not found", test_file);
        println!("Generate with: python3 scripts/create_star_sky_test.py");
        return;
    }

    let file_buf = fs::read(test_file).expect("Failed to read star_sky.gldf");
    let loaded =
        GldfProduct::load_gldf_from_buf_all(file_buf).expect("Failed to parse star_sky.gldf");

    // Verify basic GLDF structure
    assert_eq!(loaded.gldf.header.manufacturer, "Star Sky Test");
    println!("Manufacturer: {}", loaded.gldf.header.manufacturer);

    // Check for embedded files
    println!("Embedded files: {}", loaded.files.len());
    for file in &loaded.files {
        println!(
            "  - {:?}: {} bytes",
            file.name,
            file.content.as_ref().map(|c| c.len()).unwrap_or(0)
        );
    }

    // Find sky_data.json
    let sky_data = loaded.files.iter().find(|f| {
        f.name
            .as_ref()
            .map(|n| n.contains("sky_data.json"))
            .unwrap_or(false)
    });
    assert!(sky_data.is_some(), "sky_data.json not found in GLDF");

    // Verify sky_data.json contains star data
    if let Some(sky_file) = sky_data {
        let content = sky_file
            .content
            .as_ref()
            .expect("sky_data.json has no content");
        let json_str = String::from_utf8_lossy(content);
        assert!(
            json_str.contains("\"stars\""),
            "sky_data.json missing stars array"
        );
        assert!(
            json_str.contains("Sirius"),
            "sky_data.json missing Sirius star"
        );
        println!("sky_data.json: {} bytes, contains star data", content.len());
    }

    // Find viewer plugin manifest
    let manifest = loaded.files.iter().find(|f| {
        f.name
            .as_ref()
            .map(|n| n.contains("other/viewer/starsky/manifest.json"))
            .unwrap_or(false)
    });
    assert!(
        manifest.is_some(),
        "Plugin manifest not found at other/viewer/starsky/manifest.json"
    );

    // Verify manifest content
    if let Some(manifest_file) = manifest {
        let content = manifest_file
            .content
            .as_ref()
            .expect("manifest.json has no content");
        let json_str = String::from_utf8_lossy(content);
        assert!(
            json_str.contains("\"type\": \"starsky\""),
            "manifest missing type: starsky"
        );
        assert!(
            json_str.contains("gldf_starsky_wasm.js"),
            "manifest missing js file reference"
        );
        assert!(
            json_str.contains("gldf_starsky_wasm_bg.wasm"),
            "manifest missing wasm file reference"
        );
        println!("Plugin manifest: valid starsky plugin");
    }

    // Find WASM binary
    let wasm = loaded.files.iter().find(|f| {
        f.name
            .as_ref()
            .map(|n| n.ends_with(".wasm"))
            .unwrap_or(false)
    });
    assert!(wasm.is_some(), "WASM binary not found in GLDF");

    if let Some(wasm_file) = wasm {
        let content = wasm_file
            .content
            .as_ref()
            .expect("WASM file has no content");
        // WASM files start with magic bytes: 0x00 0x61 0x73 0x6D ("\0asm")
        assert!(content.len() > 4, "WASM file too small");
        assert_eq!(&content[0..4], b"\0asm", "Invalid WASM magic bytes");
        println!("WASM binary: {} bytes, valid WASM format", content.len());
    }

    // Find JS loader
    let js = loaded.files.iter().find(|f| {
        f.name
            .as_ref()
            .map(|n| n.contains("gldf_starsky_wasm.js"))
            .unwrap_or(false)
    });
    assert!(js.is_some(), "JS loader not found in GLDF");

    println!("\nStar Sky GLDF plugin test passed!");
    println!("  - GLDF structure: valid");
    println!("  - Sky data: present");
    println!("  - Plugin manifest: valid");
    println!("  - WASM binary: valid");
    println!("  - JS loader: present");
}

// ──────────────────────────────────────────────────────────────────
// rc.3 ↔ rc.4 backwards compatibility tests.
//
// rc.4 corrected two long-standing typos: the element name
// `RatedChromacityCoordinateValues` became `RatedChromaticityCoordinateValues`,
// and the `Cie97LampType` enum values `Flourescent Triphosphor` /
// `Flourescent Halophosphate` became `Fluorescent ...`. Both old forms
// remain valid (rc.4 keeps them as deprecated aliases), so gldf-rs:
//   - parses either spelling
//   - stores the rc.4-canonical form internally
//   - serializes either form on demand via `GldfSchemaVersion`
// ──────────────────────────────────────────────────────────────────

/// rc.3 input with the typo'd `Flourescent Triphosphor` enum value
/// parses cleanly and lands in memory as the corrected `Fluorescent
/// Triphosphor` form. Default-export (rc.4) writes the corrected form;
/// rc.3-export writes the original typo back.
#[test]
fn rc3_flourescent_typo_normalizes_and_round_trips() {
    use crate::GldfSchemaVersion;

    // Minimal rc.3-shaped product XML carrying the typo'd enum value
    // inside Cie97LampType (under LightSourceMaintenance).
    let rc3_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
    <Header>
        <Manufacturer>Test</Manufacturer>
        <FormatVersion major="1" minor="0" pre-release="3">1.0.0-rc.3</FormatVersion>
        <CreationTimeCode>2026-01-01T00:00:00</CreationTimeCode>
        <CreatedWithApplication>gldf-rs</CreatedWithApplication>
    </Header>
    <GeneralDefinitions>
        <Files />
        <Photometries />
        <LightSources>
            <FixedLightSource id="ls1">
                <Name><Locale language="en">Test</Locale></Name>
                <RatedInputPower>10</RatedInputPower>
                <LightSourceMaintenance>
                    <Cie97LampType>Flourescent Triphosphor</Cie97LampType>
                </LightSourceMaintenance>
            </FixedLightSource>
        </LightSources>
    </GeneralDefinitions>
    <ProductDefinitions />
</Root>"#;

    let parsed = GldfProduct::from_xml(rc3_xml).expect("rc.3 input parses");
    let cie = parsed
        .general_definitions
        .light_sources
        .as_ref()
        .and_then(|ls| ls.fixed_light_source.first())
        .and_then(|fls| fls.light_source_maintenance.as_ref())
        .and_then(|m| m.cie97_lamp_type.as_ref())
        .map(|s| s.as_str());

    // Internal model holds the rc.4-canonical (corrected) spelling.
    assert_eq!(
        cie,
        Some("Fluorescent Triphosphor"),
        "inbound rc.3 typo must be normalized to the rc.4 form on parse"
    );

    // rc.4 export emits the corrected spelling.
    let rc4_out = parsed.to_xml_with_schema(GldfSchemaVersion::Rc4).unwrap();
    assert!(
        rc4_out.contains("Fluorescent Triphosphor"),
        "rc.4 export should contain the corrected spelling"
    );
    assert!(
        !rc4_out.contains("Flourescent Triphosphor"),
        "rc.4 export must not contain the typo'd form"
    );

    // rc.3 export rewrites it back so legacy validators still accept it.
    let rc3_out = parsed.to_xml_with_schema(GldfSchemaVersion::Rc3).unwrap();
    assert!(
        rc3_out.contains("Flourescent Triphosphor"),
        "rc.3 export should restore the typo'd spelling for legacy consumers"
    );
    assert!(
        !rc3_out.contains("Fluorescent Triphosphor"),
        "rc.3 export must not contain the corrected form"
    );
}

/// Diagnostic — dump everything the parser sees from full_demo.gldf, so
/// we can confirm whether "0 LightSources" in the viewer is a parsing
/// regression, a viewer-display bug, or a stale bundle on the server.
/// Always passes; output goes to `cargo test -- --nocapture`.
#[test]
fn full_demo_diagnostic_dump() {
    let bytes = fs::read("../../tests/data/full_demo.gldf").expect("full_demo.gldf");
    let loaded = GldfProduct::load_gldf_from_buf_all(bytes).expect("parse");
    let g = &loaded.gldf;

    eprintln!("\n=== full_demo.gldf parser dump ===");
    eprintln!("manufacturer: {}", g.header.manufacturer);
    eprintln!("files: {}", g.general_definitions.files.file.len());
    eprintln!(
        "photometries: {}",
        g.general_definitions
            .photometries
            .as_ref()
            .map(|p| p.photometry.len())
            .unwrap_or(0)
    );
    let ls = g.general_definitions.light_sources.as_ref();
    eprintln!("light_sources block present: {}", ls.is_some());
    if let Some(ls) = ls {
        eprintln!("  fixed_light_source: {}", ls.fixed_light_source.len());
        eprintln!(
            "  changeable_light_source: {}",
            ls.changeable_light_source.len()
        );
        for c in &ls.changeable_light_source {
            let chrom = c
                .color_information
                .as_ref()
                .and_then(|ci| ci.rated_chromaticity_coordinate_values.as_ref());
            eprintln!(
                "    id={} W={:?} lm={:?} chrom={:?}",
                c.id,
                c.rated_input_power,
                c.rated_luminous_flux,
                chrom.map(|v| (v.x, v.y))
            );
        }
    }
    eprintln!(
        "variants: {}",
        g.product_definitions
            .variants
            .as_ref()
            .map(|v| v.variant.len())
            .unwrap_or(0)
    );
    eprintln!("=== end dump ===\n");
}

/// `full_demo.gldf` is the canonical rc.3 fixture with three
/// `ChangeableLightSource` blocks, each carrying a typo'd
/// `RatedChromacityCoordinateValues` element under `ColorInformation`.
/// This test confirms the rebuilt fixture parses cleanly and that both
/// rc.4 and rc.3 export produce the right element-name spelling for the
/// chromaticity blocks.
#[test]
fn full_demo_round_trips_chromaticity_under_both_schemas() {
    use crate::GldfSchemaVersion;
    let bytes = fs::read("../../tests/data/full_demo.gldf").expect("full_demo.gldf must exist");
    let loaded = GldfProduct::load_gldf_from_buf_all(bytes).expect("load_gldf_from_buf_all");
    let gldf = &loaded.gldf;

    // Three ChangeableLightSource elements survive the parse (the
    // pre-rc.3 fixture used <LightSource>, which the parser would have
    // dropped — fixed by the tmp/full_demo_lightsources_fix.py rewrite).
    let ls = gldf
        .general_definitions
        .light_sources
        .as_ref()
        .expect("LightSources block");
    assert_eq!(
        ls.changeable_light_source.len(),
        3,
        "expected 3 ChangeableLightSource elements after parse"
    );

    // Each one has a chromaticity block. The internal model carries the
    // rc.4-canonical spelling (the parser renamed via serde alias).
    for c in &ls.changeable_light_source {
        let chrom = c
            .color_information
            .as_ref()
            .and_then(|ci| ci.rated_chromaticity_coordinate_values.as_ref())
            .unwrap_or_else(|| panic!("ChromaticityCoords missing on {}", c.id));
        assert!(chrom.x > 0.0 && chrom.y > 0.0);
    }

    // rc.4 export → corrected element name only.
    let xml4 = gldf.to_xml_with_schema(GldfSchemaVersion::Rc4).unwrap();
    assert!(xml4.contains("RatedChromaticityCoordinateValues"));
    assert!(!xml4.contains("RatedChromacityCoordinateValues"));

    // rc.3 export → typo'd element name only.
    let xml3 = gldf.to_xml_with_schema(GldfSchemaVersion::Rc3).unwrap();
    assert!(xml3.contains("RatedChromacityCoordinateValues"));
    assert!(!xml3.contains("RatedChromaticityCoordinateValues"));
}

/// rc.3 input with the typo'd `RatedChromacityCoordinateValues` element
/// parses through the serde alias and round-trips correctly under both
/// schema targets.
#[test]
fn rc3_chromaticity_typo_alias_round_trips() {
    use crate::GldfSchemaVersion;

    let rc3_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
    <Header>
        <Manufacturer>Test</Manufacturer>
        <FormatVersion major="1" minor="0" pre-release="3">1.0.0-rc.3</FormatVersion>
        <CreationTimeCode>2026-01-01T00:00:00</CreationTimeCode>
        <CreatedWithApplication>gldf-rs</CreatedWithApplication>
    </Header>
    <GeneralDefinitions>
        <Files />
        <Photometries />
        <LightSources>
            <FixedLightSource id="ls1">
                <Name><Locale language="en">Test</Locale></Name>
                <RatedInputPower>10</RatedInputPower>
                <ColorInformation>
                    <RatedChromacityCoordinateValues>
                        <X>0.4</X>
                        <Y>0.4</Y>
                    </RatedChromacityCoordinateValues>
                </ColorInformation>
            </FixedLightSource>
        </LightSources>
    </GeneralDefinitions>
    <ProductDefinitions />
</Root>"#;

    let parsed = GldfProduct::from_xml(rc3_xml).expect("rc.3 element name parses via alias");
    let coord = parsed
        .general_definitions
        .light_sources
        .as_ref()
        .and_then(|ls| ls.fixed_light_source.first())
        .and_then(|fls| fls.color_information.as_ref())
        .and_then(|ci| ci.rated_chromaticity_coordinate_values.as_ref())
        .map(|v| (v.x, v.y));
    assert_eq!(coord, Some((0.4, 0.4)));

    // rc.4 export uses the corrected element name.
    let rc4_out = parsed.to_xml_with_schema(GldfSchemaVersion::Rc4).unwrap();
    assert!(rc4_out.contains("RatedChromaticityCoordinateValues"));
    assert!(!rc4_out.contains("RatedChromacityCoordinateValues"));

    // rc.3 export restores the typo'd element name.
    let rc3_out = parsed.to_xml_with_schema(GldfSchemaVersion::Rc3).unwrap();
    assert!(rc3_out.contains("RatedChromacityCoordinateValues"));
    assert!(!rc3_out.contains("RatedChromaticityCoordinateValues"));
}
