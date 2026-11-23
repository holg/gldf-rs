//! Smoke test for `save_gldf` — reads one midea .gldf, sets
//! `<Manufacturer>` to "Midea", saves to a temp path, re-loads, and
//! checks the patched value plus that all non-XML entries survived.
use zip::ZipArchive;

fn main() -> anyhow::Result<()> {
    let src = std::env::args().nth(1).expect("argv[1]: source .gldf path");

    // 1. Snapshot source-zip entries for the round-trip check.
    let src_entries: Vec<(String, u64)> = {
        let f = std::fs::File::open(&src)?;
        let mut z = ZipArchive::new(f)?;
        (0..z.len())
            .map(|i| {
                let e = z.by_index(i).unwrap();
                (e.name().to_string(), e.size())
            })
            .collect()
    };

    // 2. Load, patch <Manufacturer>, save to a temp sibling.
    let mut p = gldf_rs::gldf::GldfProduct::load_gldf(&src)?;
    let before = p.header.manufacturer.clone();
    p.header.manufacturer = "Midea".to_string();
    let tmp = format!("{src}.smoke.gldf");
    p.save_gldf(&tmp)?;

    // 3. Re-load the patched copy through the normal path.
    let q = gldf_rs::gldf::GldfProduct::load_gldf(&tmp)?;
    println!("manufacturer: {before:?} -> {:?}", q.header.manufacturer);
    assert_eq!(q.header.manufacturer, "Midea");

    // 4. Confirm every original entry (besides product.xml, which we
    //    rewrote) is present in the new zip with the same uncompressed size.
    let dst_entries: std::collections::HashMap<String, u64> = {
        let f = std::fs::File::open(&tmp)?;
        let mut z = ZipArchive::new(f)?;
        (0..z.len())
            .map(|i| {
                let e = z.by_index(i).unwrap();
                (e.name().to_string(), e.size())
            })
            .collect()
    };
    for (name, size) in &src_entries {
        if name == "product.xml" {
            continue;
        }
        match dst_entries.get(name) {
            Some(&dst_size) => assert_eq!(
                *size, dst_size,
                "entry {name} changed size: {size} -> {dst_size}"
            ),
            None => panic!("entry {name} missing from patched gldf"),
        }
    }
    println!(
        "round-trip OK: {} entries preserved (product.xml rewritten)",
        src_entries.len() - 1
    );

    // 5. Show the byte delta on disk so we know patch sizes are sane.
    //    The entry-by-entry size check in step 4 already proves the zip
    //    is intact; the extractor's LDC re-read happens in the
    //    gldf-search-side smoke once we re-extract the corpus.
    let src_len = std::fs::metadata(&src)?.len();
    let dst_len = std::fs::metadata(&tmp)?.len();
    println!(
        "size: {src_len} B -> {dst_len} B (delta {})",
        dst_len as i64 - src_len as i64
    );

    std::fs::remove_file(&tmp).ok();
    Ok(())
}
