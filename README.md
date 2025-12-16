[![Rust](https://github.com/holg/gldf-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/holg/gldf-rs/actions/workflows/rust.yml)
[![PyPI](https://img.shields.io/pypi/v/gldf-rs-python.svg)](https://pypi.org/project/gldf-rs-python/)
[![crates.io](https://img.shields.io/crates/v/gldf-rs.svg)](https://crates.io/crates/gldf-rs)

# gldf-rs

A cross-platform GLDF (General Lighting Data Format) processing library and suite of applications.

![gldf-rs logo](gldf-rs-logo.png)

## Overview

gldf-rs provides comprehensive tools for working with GLDF files - the modern container format for luminaire and sensor data. GLDF files are ZIP containers containing `product.xml` definitions along with associated binaries like images, Eulumdat/IES photometry files, and L3D 3D models.

Learn more at: https://gldf.io

## Project Structure

```
gldf-rs/
├── crates/
│   ├── gldf-rs-lib/      # Core Rust library
│   ├── gldf-rs-egui/     # Desktop GUI (egui)
│   ├── gldf-rs-wasm/     # WebAssembly viewer
│   ├── gldf-rs-python/   # Python bindings (PyO3)
│   └── gldf-rs-ffi/      # FFI bindings (Swift/Kotlin)
├── GldfApp/              # Native iOS/macOS/Android apps
├── tests/                # Shared test data
└── scripts/              # Build scripts
```

## Packages

| Package | Description | Published |
|---------|-------------|-----------|
| `gldf-rs` | Core Rust library for GLDF parsing and manipulation | [crates.io](https://crates.io/crates/gldf-rs) |
| `gldf-rs-egui` | Desktop GUI application with 3D L3D viewer | - |
| `gldf-rs-wasm` | WebAssembly app with interactive GLDF viewer | [gldf.icu](https://gldf.icu) |
| `gldf-rs-python` | Python bindings via PyO3 | [PyPI](https://pypi.org/project/gldf-rs-python/) |
| `gldf-rs-ffi` | FFI bindings for Swift/Kotlin (iOS, macOS, Android) | - |
| `GldfApp` | Native applications for iOS, macOS, and Android | - |

## Features

- Parse GLDF containers and `product.xml` definitions
- Convert between XML and JSON representations
- Extract and process embedded files (images, photometry, 3D models)
- **Convert LDT/IES photometry files to GLDF** (Rust, Python, WASM)
- Support for meta-information.xml
- WebGL-based L3D 3D model viewer
- LDT/IES photometry diagram rendering (Polar, Cartesian, Heatmap, BUG, LCS)
- Native apps with Swift Package Manager support

## Live Demo

Try the WASM-based GLDF viewer at: **https://gldf.icu**

## Quick Start

### Rust Library

```rust
use gldf_rs::GldfProduct;

let loaded = GldfProduct::load_gldf("./tests/data/test.gldf").unwrap();

// Display pretty printed XML
let x_serialized = loaded.to_xml().unwrap();
println!("{}", x_serialized);

// Convert to JSON
let json_str = loaded.to_json().unwrap();
println!("{}", json_str);

// Round-trip: JSON back to XML
let j_loaded = GldfProduct::from_json(&json_str).unwrap();
let x_reserialized = j_loaded.to_xml().unwrap();
assert_eq!(x_serialized, x_reserialized);
```

### WASM Web Viewer

```bash
cd crates/gldf-rs-wasm
trunk serve
# Open http://127.0.0.1:8080
```

### Desktop GUI (egui)

```bash
cargo run -p gldf-rs-egui --release
```

### Native Apps

```bash
# macOS
cd GldfApp
./scripts/build_macos.sh

# iOS/macOS via Swift Package Manager
./scripts/build_spm_package.sh
```

## Working with Photometry Files

```rust
let phot_files = loaded.get_phot_files().unwrap();
for f in phot_files.iter() {
    let file_id = f.id.to_string();
    let ldc_content = loaded.get_ldc_by_id(file_id).unwrap();
    println!("{}: {} bytes", f.file_name, ldc_content.len());
}
```

## Release Notes

### 0.3.3
- **LDT/IES to GLDF conversion**: New Python bindings for converting photometry files
  - `ldt_to_gldf_json()`, `ldt_to_gldf_bytes()`, `gldf_from_bytes()`, `gldf_json_to_bytes()`
- **WASM Viewer improvements**:
  - Clear and Help buttons in toolbar
  - Context-sensitive help overlay
  - Photometry editor with dual-value display (GLDF vs calculated from LDT/IES)
  - LDT diagram tabs (Polar, Cartesian, Heatmap, 3D Butterfly, BUG, LCS)
  - Click-to-zoom for diagram inspection
- **CI improvements**: Updated deny.toml, fixed clippy warnings

### 0.3.1
- **Workspace reorganization**: All crates moved to `crates/` directory
- **New desktop GUI**: `gldf-rs-egui` - cross-platform desktop viewer using egui
  - Interactive 3D L3D model viewer (spawns subprocess for macOS compatibility)
  - Automatic URL asset downloading
  - PDF viewing via system viewer
  - Dark mode support
- **Improved CI/CD**: Strict clippy and rustdoc checks
- **Code quality**: Fixed all clippy warnings across workspace

### 0.3.0
- **Major refactor**: Restructured as monorepo with separate packages
- **WASM Web App**: Full-featured GLDF viewer with Yew framework
- **L3D 3D Viewer**: WebGL-based rendering using three-d
  - Fixed: L3D files with missing MTL materials now render correctly
  - Auto-generates stub materials for OBJ files referencing missing MTL files
- **LDT Diagram Viewer**: Interactive photometry visualization
- **Native Apps**: iOS, macOS, and Android applications
- **FFI Bindings**: Swift and Kotlin bindings via UniFFI
- **Python Bindings**: PyO3-based Python package
- **Swift Package Manager**: XCFramework distribution for Apple platforms

### 0.2.2
- Added support for meta-information.xml

### 0.2.1
- Added better documentation for the main page
- Refactored for WASM support using reqwest::blocking

### 0.2.0
- Refactored gldf.rs into submodules
- Added support for BOM encoded UTF8 product.xml
- Added support for URL file_types
- Added better documentation

## License

GPL-3.0-or-later
