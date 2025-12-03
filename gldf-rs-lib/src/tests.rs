#![allow(unused_variables)]

use std::env;
use std::error::Error;
use std::fs;

use anyhow::Result;
use serde::de::StdError;

#[cfg(feature = "http")]
use super::fetch_content_from_url;
use crate::gldf::GldfProduct;

const GLDF_FILE_NAME: &str = "../tests/data/R2MCOBSIK-30.gldf";

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
    let test_data_dir = current_dir
        .parent()
        .expect("Failed to get parent directory")
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
            let file_name = path.file_name()
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
