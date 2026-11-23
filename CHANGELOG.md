# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-11-23

Everything since 0.4.0 in one release: the feature branch was merged into
`main` as a whole.

### Added

- **Editor API (`gldf-rs-editor`)**: one `GldfAction` → `GldfState` reducer shared by the web editor, the `gldf-edit` terminal UI, `gldf-cli` and the Python bindings; product wizard (incl. driver step), variant editor with mountings and product series, marketing editor (housing colours, markets, RAL ↔ name), `.mtl` sync for L3D.
- **GLDF builder API** in the library: spectra, photometry, variants; `load_gldf_from_buf_all` cross-references file ids from the XML.
- **Selectable rc-3 / rc-4 XSD validation** with the rc.4 schema and tests.
- **Sensors**: each sensor file shows its detection areas (plan, table per motion type, height, SensXML planning shapes, warnings) through `eulumdat::sensor::SensorFile`; `SensorObject` mounts from l3d_rs.
- **Batch processing**: LDT / IES / ULD + template → GLDF / IFC, with an egui batch app.
- **gldf-bevy**: native scene loading from a `.gldf`, command bridge for Lisp scene commands.
- **gldf-unreal** (not published): exporter, C ABI, Interchange translator plugin for Unreal Engine.
- Web editor: GLDF builder, SPD viewer, IFC viewer, URL deep links, validation banner, mobile layout.
- New crates on crates.io: `gldf-rs-i18n`, `gldf-rs-editor`.

- **Validation in every desktop frontend**: `gldf-cli validate --schema rc3|rc4` (and `batch --validate --schema …`), a *Validation* page in `gldf-editor-egui` with an rc-3/rc-4 switch, and a *Validation* section in the `gldf-edit` TUI (`t` toggles the schema). All three render the new frontend-neutral `gldf_rs_editor::validation_report` (engine + canonical + XSD, behind the editor crate's `xsd` feature) — the same bucketing as the web banner.

### Changed

- **eulumdat 0.8, l3d_rs 0.2.4, Bevy 0.19** (published versions; the Bevy fork is retired). Local builds take the sibling checkouts through `[patch.crates-io]`.
- **Luminous flux and wattage of an LDT** in the GLDF metadata, the IFC export, the Bevy lights and the stadium plugin now come from the primary lamp set (`luminous_flux()` / `wattage()`); results differ only for files with more than one lamp set.
- **Desktop GUI stack bumped**: eframe/egui 0.33 → 0.36 (`App::ui`, unified `egui::Panel`), three-d 0.18 → 0.19, rfd 0.15 → 0.17, ratatui 0.29 → 0.30 (+ ratatui-explorer 0.3, crossterm 0.29), resvg 0.45 → 0.48. `wasm-bindgen` pin moved to `=0.2.126` (required by eframe 0.36's js-sys); a manual `wasm-bindgen` CLI must match.
- **Release workflow** builds the binaries that actually exist (`gldf-editor-egui`, `gldf-l3d-viewer`, `gldf-cli`, `gldf-edit`, `gldf-bevy-viewer`); the stale `gldf-viewer-egui` target is gone.

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
