[![Rust](https://github.com/holg/gldf-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/holg/gldf-rs/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/gldf-rs.svg)](https://crates.io/crates/gldf-rs)
[![Documentation](https://docs.rs/gldf-rs/badge.svg)](https://docs.rs/gldf-rs)

# gldf-rs

A cross-platform GLDF (General Lighting Data Format) processing library for Rust.

## Overview

gldf-rs provides comprehensive tools for working with GLDF files - the modern container format for luminaire and sensor data defined by the lighting industry (ISO 7127).

GLDF files are ZIP containers containing:
- `product.xml` - Product definitions and specifications
- Photometry files (Eulumdat/LDT, IES)
- Images and documentation
- L3D 3D model files

Learn more at: https://gldf.io

## Features

- Parse GLDF containers and `product.xml` definitions
- Convert between XML and JSON representations
- Extract and process embedded files (images, photometry, 3D models)
- Support for meta-information.xml
- Optional HTTP support for URL-based file references
- WASM-compatible (disable `http` feature)

## Installation

```toml
[dependencies]
gldf-rs = "0.3"
```

For WASM builds, disable HTTP support:
```toml
[dependencies]
gldf-rs = { version = "0.3", default-features = false }
```

## Quick Start

```rust
use gldf_rs::GldfProduct;

// Load from file
let loaded = GldfProduct::load_gldf("./tests/data/test.gldf").unwrap();

// Convert to XML
let xml = loaded.to_xml().unwrap();
println!("{}", xml);

// Convert to JSON
let json = loaded.to_json().unwrap();
println!("{}", json);

// Round-trip: JSON back to XML
let from_json = GldfProduct::from_json(&json).unwrap();
assert_eq!(loaded.to_xml().unwrap(), from_json.to_xml().unwrap());
```

## Working with Embedded Files

```rust
// Get photometry files
let phot_files = loaded.get_phot_files().unwrap();
for f in phot_files.iter() {
    println!("Photometry: {}", f.file_name);
}

// Get image files
let images = loaded.get_image_def_files().unwrap();
for img in images.iter() {
    println!("Image: {}", img.file_name);
}

// Load from buffer (useful for WASM)
let buffer = std::fs::read("test.gldf").unwrap();
let loaded = GldfProduct::load_gldf_from_buf(buffer).unwrap();
```

## Live Demo

Try the WASM-based GLDF viewer at: **https://gldf.icu**

## Related Crates

This crate is part of the gldf-rs ecosystem:

| Crate | Description |
|-------|-------------|
| [gldf-rs](https://crates.io/crates/gldf-rs) | Core library (this crate) |
| [gldf-rs-wasm](https://crates.io/crates/gldf-rs-wasm) | WebAssembly viewer application |
| [l3d-rs](https://crates.io/crates/l3d_rs) | L3D 3D model format parsing |
| [eulumdat](https://crates.io/crates/eulumdat) | Eulumdat/LDT photometry parsing |

## Release Notes

### 0.3.0
- Major refactor using quick-xml for XML parsing (replaces yaserde)
- Restructured as part of monorepo
- Optional HTTP feature for URL file references
- WASM-compatible build support
- Improved error handling with anyhow

### 0.2.2
- Added support for meta-information.xml

### 0.2.1
- Refactored for WASM support

### 0.2.0
- Refactored into submodules
- Added BOM-encoded UTF8 support
- Added URL file type support

## License

MIT License
