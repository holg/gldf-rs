//! Regenerate `include/gldf_unreal.h` from the FFI surface.
//!
//! Run manually: `cargo run -p gldf-unreal --bin gen-header`.
//!
//! The generated header is committed to the repo so downstream UE projects
//! can vendor it without a Rust toolchain.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config_path = crate_dir.join("cbindgen.toml");
    let out_path = crate_dir.join("include").join("gldf_unreal.h");

    let config = match cbindgen::Config::from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to read {}: {e}", config_path.display());
            return ExitCode::FAILURE;
        }
    };

    let builder = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config);

    let bindings = match builder.generate() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("cbindgen failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    bindings.write_to_file(&out_path);
    println!("wrote {}", out_path.display());
    ExitCode::SUCCESS
}
