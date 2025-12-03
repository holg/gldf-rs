# GLDF-RS-WASM

A WebAssembly-based GLDF (General Lighting Data Format) viewer and editor.

## Overview

GLDF-RS-WASM provides an interactive web application for viewing and editing GLDF files directly in the browser. It leverages WebAssembly for high-performance parsing and rendering of lighting product data.

## Features

- **Modern UI**: Clean, responsive interface built with Tailwind CSS
- **GLDF Parsing**: Load and parse GLDF container files client-side
- **L3D 3D Viewer**: WebGL-based 3D rendering of L3D luminaire models with orbit/zoom controls
- **LDT/IES Diagrams**: Interactive photometry polar diagrams with C-plane visualization
- **Tabbed Editor**: Edit header, variants, light sources, and files in organized tabs
- **File Browser**: View and download embedded assets (images, photometry, 3D models)
- **Drag & Drop**: Easy file upload via drag and drop
- **Multi-L3D Support**: Render GLDF files containing multiple L3D geometries

## Dependencies

This crate relies on excellent Rust libraries for the heavy lifting:

| Crate | Purpose |
|-------|---------|
| [gldf-rs](https://crates.io/crates/gldf-rs) | Core GLDF parsing and manipulation |
| [l3d-rs](https://crates.io/crates/l3d_rs) | L3D 3D model format parsing |
| [eulumdat](https://crates.io/crates/eulumdat) | Eulumdat/LDT photometry file parsing |
| [three-d](https://crates.io/crates/three-d) | WebGL 3D rendering |
| [yew](https://crates.io/crates/yew) | Reactive web framework |

## Live Demo

Try it at: **https://gldf.icu**

Embedded Eulumdat files can be opened directly in **https://eulumdat.icu** - a WASM-based Eulumdat editor and viewer that can also export to IESNA format.

## Running Locally

```bash
# Install trunk (WASM bundler)
cargo install trunk

# Build and serve
cd gldf-rs-wasm
trunk serve

# Open http://127.0.0.1:8080
```

## Release Notes

### 0.3.0
- Complete rewrite with modern UI using Yew framework and Tailwind CSS
- New tabbed interface for editing header, variants, light sources, and files
- L3D 3D viewer with WebGL rendering via three-d
- Interactive orbit/zoom controls for 3D models
- LDT/IES photometry polar diagram viewer with C-plane visualization
- Support for GLDF files containing multiple L3D geometries
- Fixed L3D rendering for OBJ files with missing MTL materials (auto-generates stub materials)
- File browser with image preview and download support

### 0.2.1
- Usage of new gldf-rs 0.2.1
- Inheritance and overwriting of properties (needed for reqwest)

## License

MIT License
