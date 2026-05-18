//! Fix-up utilities for legacy / non-conforming GLDF archives.
//!
//! These functions take raw GLDF bytes, parse the embedded `product.xml`
//! with the structured `GldfProduct` model (so we get real XML editing
//! rather than string substitution), apply targeted corrections, and
//! return a new GLDF byte buffer.
//!
//! Currently handled:
//!
//! * `contentType="geometry/l3d"` → `contentType="geo/l3d"`
//!   (the XSD-allowed canonical value). The corresponding embedded files
//!   are also moved from the `geometry/` zip folder to `geo/` so the
//!   archive layout matches the conventional folder implied by the
//!   corrected `contentType`.

use crate::gldf::GldfProduct;
use anyhow::{Context, Result};
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

/// Legacy → canonical `contentType` substitutions to apply.
const CONTENT_TYPE_REWRITES: &[(&str, &str)] = &[("geometry/l3d", "geo/l3d")];

/// Embedded-folder renames to apply alongside the `contentType` fix,
/// so the archive layout matches the conventional folder implied by
/// the corrected `contentType` prefix.
const FOLDER_RENAMES: &[(&str, &str)] = &[("geometry/", "geo/")];

/// Rewrite a GLDF archive in memory, applying every legacy content-type
/// fix-up. Returns `(new_bytes, changed)`; if no fix-up was needed the
/// original bytes are returned and `changed` is `false`.
///
/// This is a pure function on bytes — call sites that want to overwrite
/// the file on disk should handle the file I/O themselves.
pub fn fix_legacy_content_types(gldf_bytes: Vec<u8>) -> Result<(Vec<u8>, bool)> {
    let cursor = Cursor::new(&gldf_bytes);
    let mut zip = ZipArchive::new(cursor).context("open GLDF as zip")?;

    let mut xml_str = String::new();
    {
        let mut xmlfile = zip
            .by_name("product.xml")
            .context("product.xml not found in GLDF archive")?;
        xmlfile
            .read_to_string(&mut xml_str)
            .context("read product.xml")?;
    }

    let mut product = GldfProduct::from_xml(&xml_str).context("parse product.xml")?;
    let mut changed = false;
    for f in &mut product.general_definitions.files.file {
        for (from, to) in CONTENT_TYPE_REWRITES {
            if f.content_type == *from {
                f.content_type = (*to).to_string();
                changed = true;
            }
        }
    }
    if !changed {
        return Ok((gldf_bytes, false));
    }
    let new_xml = product
        .to_xml()
        .context("serialize updated product.xml")?;

    let mut out_buf: Vec<u8> = Vec::new();
    {
        let mut writer = ZipWriter::new(Cursor::new(&mut out_buf));
        let opts = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        writer
            .start_file("product.xml", opts)
            .context("start product.xml")?;
        writer
            .write_all(new_xml.as_bytes())
            .context("write product.xml")?;

        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).context("read zip entry")?;
            let orig_name = entry.name().to_string();
            if orig_name == "product.xml" {
                continue;
            }

            let new_name = rename_zip_path(&orig_name);

            if entry.is_dir() {
                // Skip bare directory entries whose target name is empty
                // (e.g. the source "geometry/" — the destination "geo/"
                // will be created implicitly by file entries).
                if new_name.ends_with('/') && !has_content_after_prefix(&new_name) {
                    continue;
                }
                writer
                    .add_directory(&new_name, opts)
                    .context("add directory")?;
                continue;
            }

            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).context("read file entry")?;
            writer
                .start_file(&new_name, opts)
                .context("start file entry")?;
            writer.write_all(&buf).context("write file entry")?;
        }
        writer.finish().context("finish zip")?;
    }
    Ok((out_buf, true))
}

fn rename_zip_path(path: &str) -> String {
    for (from, to) in FOLDER_RENAMES {
        if let Some(rest) = path.strip_prefix(from) {
            return format!("{}{}", to, rest);
        }
    }
    path.to_string()
}

fn has_content_after_prefix(name: &str) -> bool {
    name.trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .count()
        > 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_minimal_gldf_with_legacy_content_type() -> Vec<u8> {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="https://gldf.io/xsd/gldf/1.0.0/gldf.xsd"><Header><Manufacturer>Test</Manufacturer><FormatVersion major="1" minor="0"/><CreatedWithApplication>fix-test</CreatedWithApplication><GldfCreationTimeCode>2026-01-01T00:00:00Z</GldfCreationTimeCode></Header><GeneralDefinitions><Files><File id="geo1" contentType="geometry/l3d" type="localFileName">model_1.l3d</File></Files><Photometries/><LightSources/><Emitters/><Geometries><ModelGeometry id="g1"><GeometryFileReference fileId="geo1" levelOfDetail="High"/></ModelGeometry></Geometries></GeneralDefinitions><ProductDefinitions><ProductMetaData><Name><Locale language="en">T</Locale></Name></ProductMetaData></ProductDefinitions></Root>"#;

        let mut out = Vec::new();
        {
            let mut w = ZipWriter::new(Cursor::new(&mut out));
            let opts = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            w.start_file("product.xml", opts).unwrap();
            w.write_all(xml.as_bytes()).unwrap();
            w.start_file("geometry/model_1.l3d", opts).unwrap();
            w.write_all(b"\0L3D\0fake\0").unwrap();
            w.finish().unwrap();
        }
        out
    }

    #[test]
    fn fixes_geometry_l3d_to_geo_l3d() {
        let input = build_minimal_gldf_with_legacy_content_type();
        let (output, changed) = fix_legacy_content_types(input).unwrap();
        assert!(changed, "expected changed=true");

        let mut z = ZipArchive::new(Cursor::new(&output)).unwrap();
        let mut xml = String::new();
        z.by_name("product.xml")
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();
        assert!(xml.contains("contentType=\"geo/l3d\""));
        assert!(!xml.contains("contentType=\"geometry/l3d\""));

        // File moved from geometry/ to geo/.
        assert!(z.by_name("geo/model_1.l3d").is_ok());
        assert!(z.by_name("geometry/model_1.l3d").is_err());
    }

    #[test]
    fn noop_when_no_legacy_content_type() {
        let input = {
            let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="https://gldf.io/xsd/gldf/1.0.0/gldf.xsd"><Header><Manufacturer>Test</Manufacturer><FormatVersion major="1" minor="0"/><CreatedWithApplication>fix-test</CreatedWithApplication><GldfCreationTimeCode>2026-01-01T00:00:00Z</GldfCreationTimeCode></Header><GeneralDefinitions><Files><File id="img" contentType="image/jpg" type="localFileName">p.jpg</File></Files><Photometries/><LightSources/><Emitters/></GeneralDefinitions><ProductDefinitions><ProductMetaData><Name><Locale language="en">T</Locale></Name></ProductMetaData></ProductDefinitions></Root>"#;
            let mut out = Vec::new();
            let mut w = ZipWriter::new(Cursor::new(&mut out));
            let opts = SimpleFileOptions::default();
            w.start_file("product.xml", opts).unwrap();
            w.write_all(xml.as_bytes()).unwrap();
            w.finish().unwrap();
            out
        };
        let (_, changed) = fix_legacy_content_types(input).unwrap();
        assert!(!changed, "expected changed=false");
    }
}
