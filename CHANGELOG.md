# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.3] - 2024-12-16

### Added

- **Python bindings**: New functions for LDT/IES to GLDF conversion
  - `ldt_to_gldf_json(data, filename)` - Convert LDT/IES to GLDF JSON
  - `ldt_to_gldf_bytes(data, filename)` - Convert LDT/IES to GLDF bytes (ZIP)
  - `gldf_from_bytes(data)` - Load GLDF from bytes and return JSON
  - `gldf_json_to_bytes(json_str)` - Export GLDF JSON to bytes (ZIP)
- **WASM Viewer**: Clear and Help buttons in the toolbar
- **WASM Viewer**: Context-sensitive help overlay
- **Photometry Editor**: Dual-value display showing both GLDF values (blue) and calculated values from LDT/IES (orange)
- **LDT Viewer**: Multiple diagram tabs (Polar, Cartesian, Heatmap, 3D Butterfly, BUG, LCS)
- **LDT Viewer**: Click-to-zoom modal for diagram inspection
- **Variants View**: Readonly mounting badges in viewer mode

### Changed

- **Python bindings**: Improved error handling with proper `PyValueError` exceptions
- **Python bindings**: Added `eulumdat` feature for LDT/IES conversion support
- **CI**: Updated `test-ci-locally.sh` to exclude external path dependencies from fmt/clippy checks
- **CI**: Added MIT-0 and CC0-1.0 to allowed licenses in `deny.toml`

### Fixed

- Fixed SVG overlap issue in LDT viewer
- Fixed file ID matching for photometry references
- Fixed wasm-opt compatibility by preferring homebrew binaryen
- Fixed clippy warnings across all crates
- Fixed rustdoc broken intra-doc links

## [0.3.2] - 2024-12-03

### Added

- Initial WASM-based GLDF viewer at gldf.icu
- Bevy-based 3D L3D model rendering
- Photometry diagram rendering using eulumdat-rs
- EditableGldf wrapper for editing and saving GLDF files
- Validation engine for GLDF files

### Changed

- Migrated WASM frontend from Leptos to Yew framework

## [0.3.1] - 2024-09-18

### Added

- Python bindings via PyO3
- FFI bindings for Swift/Kotlin
- Native iOS/macOS app via Swift Package Manager

## [0.3.0] - 2024-08-01

### Added

- Initial public release
- Core GLDF parsing and serialization
- XML/JSON conversion
- L3D 3D model support via l3d-rs
- Eulumdat photometry support via eulumdat-rs
