# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-05-19

### Added

- **rc.3 ↔ rc.4 schema versioning**: new `GldfSchemaVersion` enum and `to_xml_with_schema()` method. Internal model always stores the rc.4-canonical form; rc.3 export rewrites the typo'd element names and enum values back for compatibility with pre-rc.4 consumers.
- **`fix` module**: `fix_legacy_content_types` rewrites legacy `contentType="geometry/l3d"` archives to the XSD-canonical `geo/l3d` form, including moving embedded files between zip folders.
- **`validation` submodule split**: `validation.rs` is now `validation/` with `engine` (schema-rule validator), `canonical` (closed-XSD-enum value validation), and `xsd_enums` (canonical value tables for IK ratings, IP codes, etc.). Public surface (`ValidationError`, `ValidationLevel`, `ValidationResult`) re-exported flat — no breaking change.
- **IFC integration improvements** and viewer locale handling.

### Changed

- **`RatedChromaticityCoordinateValues`**: renamed from `RatedChromacityCoordinateValues` per rc.4 typo cleanup. Old name kept as deprecated type alias.
- **`Cie97LampType` enum values**: `Fluorescent Triphosphor` / `Fluorescent Halophosphate` replace the rc.3-typo'd `Flourescent ...` spellings. Both forms parse; rc.4 is written by default.

### Fixed

- **`RectangularCutout` XSD compliance**: `Width`, `Length`, `Depth` now serialize as child elements (per XSD) rather than attributes.
- **Empty `<Files/>` skip**: omit the element when no files are present, matching XSD `minOccurs=1` constraint on `<File>` children.

## [0.3.4] - 2026-05-18

### Added

- **IFC integration**: Import/export IFC files with STEP parser and GLDF generator
- **Plugin system**: Extensible architecture with stadium and star sky example plugins
- **Bevy 3D scene renderer**: Room, road, parking, and outdoor scenes with scene switching (keys 1-4); correct emitter positioning (mm→m for L3D); scene-aware mounting (outdoor pole height vs room ceiling clamp); dynamic light range/intensity scaling based on mounting height
- **New photometry diagrams**: Cone, Isocandela, Isolux, Floodlight Cartesian (V-H)
- **Combined multi-variant GLDF support**: Load and inspect multi-variant files with correct file matching
- **C# / .NET bindings** via uniffi-bindgen-cs
- **JSON/XML export**: Browser download for exported files
- **Local CI script**: `scripts/test-ci-locally.sh` for pre-push validation

### Changed

- **eulumdat**: bumped workspace dep to 0.7
- **eulumdat-i18n**: added at 0.7 for viewer i18n support

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
