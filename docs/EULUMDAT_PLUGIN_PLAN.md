# Eulumdat Plugin Plan

## Overview

Create a self-describing WASM plugin for the full eulumdat library that can be embedded in GLDF files and discovered by the plugin system.

## Current State

### Existing Components
- **eulumdat** (core library): Parsing, calculations, diagrams, BUG rating
- **eulumdat-wasm**: Full Leptos app with UI components (~7.2MB)
- **eulumdat-ffi**: UniFFI bindings for Swift/Kotlin/Python

### Challenge
The current `eulumdat-wasm` is a complete web application, not a library. For the plugin system, we need a lightweight library-only WASM that exposes functions without UI.

## Proposed Solution

### Option A: New Crate `eulumdat-plugin` (Recommended)

Create a new crate specifically for the plugin architecture:

```
eulumdat-rs/crates/eulumdat-plugin/
├── Cargo.toml
├── src/
│   ├── lib.rs          # wasm_bindgen exports
│   └── manifest.rs     # Plugin manifest generation
└── dist/
    ├── manifest.json
    ├── eulumdat-plugin-*.js
    └── eulumdat-plugin-*_bg.wasm
```

**Advantages:**
- Clean separation from existing WASM app
- Smaller bundle (no UI framework)
- Follows plugin manifest pattern
- Can be embedded in GLDF files

### Option B: Add Plugin Exports to Existing Crate

Add `#[wasm_bindgen]` functions to `eulumdat-wasm` with a feature flag.

**Disadvantages:**
- Larger bundle due to Leptos framework
- Mixing app and library concerns

## Plugin API Design

### Manifest (manifest.json)

```json
{
  "id": "eulumdat",
  "name": "Eulumdat Photometric Engine",
  "version": "0.6.0",
  "description": "Parse, analyze, and visualize LDT/IES photometric files",
  "js": "eulumdat-plugin-*.js",
  "wasm": "eulumdat-plugin-*_bg.wasm",
  "capabilities": {
    "functions": {
      "parse_ldt": {
        "args": ["content"],
        "returns": "json",
        "description": "Parse LDT file content"
      },
      "parse_ies": {
        "args": ["content"],
        "returns": "json",
        "description": "Parse IES file content"
      },
      "export_ldt": {
        "args": ["json_data"],
        "returns": "string",
        "description": "Export to LDT format"
      },
      "export_ies": {
        "args": ["json_data"],
        "returns": "string",
        "description": "Export to IES format"
      },
      "calculate_beam_angle": {
        "args": ["json_data"],
        "returns": "number",
        "description": "Calculate beam angle (50% intensity)"
      },
      "calculate_field_angle": {
        "args": ["json_data"],
        "returns": "number",
        "description": "Calculate field angle (10% intensity)"
      },
      "calculate_bug_rating": {
        "args": ["json_data"],
        "returns": "json",
        "description": "Calculate BUG rating (B/U/G values)"
      },
      "calculate_zone_lumens": {
        "args": ["json_data"],
        "returns": "json",
        "description": "Calculate IESNA zone lumens"
      },
      "calculate_flux": {
        "args": ["json_data"],
        "returns": "json",
        "description": "Calculate flux fractions (up/down/total)"
      },
      "calculate_cu_table": {
        "args": ["json_data"],
        "returns": "json",
        "description": "Calculate coefficient of utilization table"
      },
      "calculate_ugr_table": {
        "args": ["json_data"],
        "returns": "json",
        "description": "Calculate UGR table"
      },
      "generate_polar_svg": {
        "args": ["json_data", "width", "height", "theme"],
        "returns": "string",
        "description": "Generate polar diagram SVG"
      },
      "generate_cartesian_svg": {
        "args": ["json_data", "width", "height", "theme"],
        "returns": "string",
        "description": "Generate Cartesian diagram SVG"
      },
      "generate_butterfly_svg": {
        "args": ["json_data", "width", "height", "theme"],
        "returns": "string",
        "description": "Generate butterfly diagram SVG"
      },
      "generate_heatmap_svg": {
        "args": ["json_data", "width", "height", "theme"],
        "returns": "string",
        "description": "Generate intensity heatmap SVG"
      },
      "generate_bug_svg": {
        "args": ["json_data", "width", "height", "theme"],
        "returns": "string",
        "description": "Generate BUG rating diagram SVG"
      },
      "generate_lcs_svg": {
        "args": ["json_data", "width", "height", "theme"],
        "returns": "string",
        "description": "Generate LCS classification diagram SVG"
      },
      "generate_cone_svg": {
        "args": ["json_data", "width", "height", "theme"],
        "returns": "string",
        "description": "Generate cone diagram SVG"
      },
      "generate_beam_angle_svg": {
        "args": ["json_data", "width", "height", "theme"],
        "returns": "string",
        "description": "Generate beam/field angle diagram SVG"
      },
      "validate": {
        "args": ["json_data"],
        "returns": "json",
        "description": "Validate photometric data with warnings"
      },
      "get_summary": {
        "args": ["json_data"],
        "returns": "json",
        "description": "Get photometric summary (all key metrics)"
      }
    },
    "inputFormats": ["ldt", "ies", "json"],
    "outputFormats": ["svg", "json", "ldt", "ies"],
    "diagrams": [
      "polar", "cartesian", "butterfly", "heatmap",
      "bug", "lcs", "cone", "beam_angle"
    ],
    "ui": {
      "icon": "📊",
      "color": "#4a90d9",
      "primaryAction": "parse_ldt",
      "showExamples": false,
      "editorLanguage": null
    }
  }
}
```

### Rust Implementation Structure

```rust
// eulumdat-plugin/src/lib.rs
use wasm_bindgen::prelude::*;
use eulumdat::{Eulumdat, PhotometricCalculations, BugRating, ZoneLumens};
use eulumdat::diagram::{PolarDiagram, CartesianDiagram, ButterflyDiagram, ...};

/// Plugin engine holding parsed data
#[wasm_bindgen]
pub struct EulumdatEngine {
    data: Option<Eulumdat>,
}

#[wasm_bindgen]
impl EulumdatEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { data: None }
    }

    /// Parse LDT content
    pub fn parse_ldt(&mut self, content: &str) -> Result<String, JsValue> {
        let ldt = Eulumdat::parse(content).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let json = serde_json::to_string(&ldt).unwrap();
        self.data = Some(ldt);
        Ok(json)
    }

    /// Parse IES content
    pub fn parse_ies(&mut self, content: &str) -> Result<String, JsValue> {
        let ldt = IesParser::parse(content).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let json = serde_json::to_string(&ldt).unwrap();
        self.data = Some(ldt);
        Ok(json)
    }

    /// Calculate beam angle
    pub fn calculate_beam_angle(&self) -> Result<f64, JsValue> {
        let ldt = self.data.as_ref().ok_or(JsValue::from_str("No data loaded"))?;
        Ok(PhotometricCalculations::beam_angle(ldt))
    }

    /// Calculate field angle
    pub fn calculate_field_angle(&self) -> Result<f64, JsValue> {
        let ldt = self.data.as_ref().ok_or(JsValue::from_str("No data loaded"))?;
        Ok(PhotometricCalculations::field_angle(ldt))
    }

    /// Calculate BUG rating
    pub fn calculate_bug_rating(&self) -> Result<String, JsValue> {
        let ldt = self.data.as_ref().ok_or(JsValue::from_str("No data loaded"))?;
        let zones = ZoneLumens::from_eulumdat(ldt);
        let rating = BugRating::from_zone_lumens(&zones);
        Ok(serde_json::to_string(&rating).unwrap())
    }

    /// Generate polar diagram SVG
    pub fn generate_polar_svg(&self, width: u32, height: u32, theme: &str) -> Result<String, JsValue> {
        let ldt = self.data.as_ref().ok_or(JsValue::from_str("No data loaded"))?;
        let diagram = PolarDiagram::from_eulumdat(ldt);
        let theme = DiagramTheme::from_str(theme);
        Ok(diagram.to_svg(width, height, &theme))
    }

    // ... more methods
}

// Static functions for stateless operations
#[wasm_bindgen]
pub fn parse_ldt_static(content: &str) -> Result<String, JsValue> {
    let ldt = Eulumdat::parse(content).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(serde_json::to_string(&ldt).unwrap())
}

#[wasm_bindgen]
pub fn validate_ldt(content: &str) -> String {
    match Eulumdat::parse(content) {
        Ok(ldt) => {
            let warnings = eulumdat::validate(&ldt);
            serde_json::to_string(&warnings).unwrap()
        }
        Err(e) => serde_json::to_string(&vec![e.to_string()]).unwrap()
    }
}
```

## Integration with GLDF Plugin System

### Embedding in GLDF

```
other/viewer/eulumdat/
├── manifest.json
├── eulumdat-plugin-xxx.js
└── eulumdat-plugin-xxx_bg.wasm
```

### Usage in DynamicPluginViewer

The existing `DynamicPluginViewer` will automatically:
1. Discover the plugin when GLDF is loaded
2. Show available functions in the UI
3. Allow calling functions with file content
4. Display SVG outputs in the viewer

### New EulumdatPluginViewer Component

A specialized viewer component that:
- Accepts LDT/IES file drops
- Shows all diagram types in tabs
- Displays calculated metrics
- Allows format conversion

```rust
#[function_component(EulumdatPluginViewer)]
pub fn eulumdat_plugin_viewer(props: &EulumdatPluginViewerProps) -> Html {
    // Uses Plugin::get("eulumdat") to call functions
    // Provides UI specific to photometric data
}
```

## Implementation Steps

### Phase 1: Create eulumdat-plugin crate
1. Create new crate in eulumdat-rs workspace
2. Add wasm-bindgen dependencies
3. Implement core parsing functions
4. Implement calculation functions
5. Implement diagram generation functions
6. Generate manifest.json

### Phase 2: Build system
1. Add build script for WASM
2. Add manifest generation
3. Add to CI/CD pipeline

### Phase 3: GLDF integration
1. Update gldf-rs to recognize eulumdat plugin
2. Add EulumdatPluginViewer component to gldf-rs-wasm
3. Test embedding in GLDF files

### Phase 4: Enhanced features
1. Add spectral analysis (ATLA support)
2. Add batch processing
3. Add i18n support

## Estimated Bundle Size

- **eulumdat-plugin (library only)**: ~500KB - 1MB compressed
- **Current eulumdat-wasm (full app)**: ~7MB compressed

The plugin approach should be 7-10x smaller since it excludes:
- Leptos framework
- UI components
- Router
- Internationalization strings

## File Locations

```
eulumdat-rs/
└── crates/
    └── eulumdat-plugin/     # NEW
        ├── Cargo.toml
        ├── build.rs         # Manifest generation
        └── src/
            ├── lib.rs
            └── manifest.rs

gldf-rs/
└── crates/
    └── gldf-rs-wasm/
        └── src/
            └── components/
                └── eulumdat_viewer.rs  # NEW (optional specialized viewer)
```

## Open Questions

1. Should the plugin maintain state (parsed data) or be stateless?
   - **Recommendation**: Stateful with `EulumdatEngine` class for efficiency

2. Should we support multiple files loaded simultaneously?
   - **Recommendation**: Single file for simplicity, batch via separate calls

3. How to handle large photometric files (>1MB)?
   - **Recommendation**: Streaming parser in future, standard for now

4. Should diagram themes be configurable?
   - **Recommendation**: Yes, pass theme name ("light", "dark", "print")
