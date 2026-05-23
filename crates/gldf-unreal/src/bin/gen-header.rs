//! Regenerate `include/gldf_unreal.h` from the FFI surface.
//!
//! Run manually: `cargo run -p gldf-unreal --bin gen-header`.
//!
//! The generated header is committed to the repo so downstream UE projects
//! can vendor it without a Rust toolchain.
//!
//! Phase 1 prints a placeholder so the binary compiles; Phase 4 wires up
//! `cbindgen::Builder` to walk `src/ffi.rs` and emit the header.

fn main() {
    eprintln!("gen-header phase-1 stub. Phase 4 will:");
    eprintln!("  cbindgen::Builder::new()");
    eprintln!("    .with_crate(env!(\"CARGO_MANIFEST_DIR\"))");
    eprintln!("    .with_config(cbindgen::Config::from_file(\"cbindgen.toml\").unwrap())");
    eprintln!("    .generate()?");
    eprintln!("    .write_to_file(\"include/gldf_unreal.h\");");
}
