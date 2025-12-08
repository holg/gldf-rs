# GLDF Viewer (egui)

Cross-platform GLDF (Global Lighting Data Format) viewer built with [egui](https://github.com/emilk/egui).

## Features

- **Cross-platform**: Works on Windows, macOS, Linux, and WebAssembly (WASM)
- **Native performance**: Built with Rust and egui for fast, responsive UI
- **File support**: View GLDF files with embedded photometry, images, and 3D geometry
- **Export**: Export to JSON or XML formats
- **Dark/Light mode**: Toggle between themes

## Running

### Native (Desktop)

```bash
cargo run -p gldf-rs-egui --release
```

Or build the release binary:

```bash
cargo build -p gldf-rs-egui --release
./target/release/gldf-viewer-egui
```

### Web (WASM)

First, install [Trunk](https://trunkrs.dev/):

```bash
cargo install trunk
```

Then build and serve:

```bash
cd gldf-rs-egui
trunk serve
```

Open http://127.0.0.1:8080 in your browser.

For a release build:

```bash
trunk build --release
```

## Project Structure

```
gldf-rs-egui/
├── src/
│   ├── main.rs          # Native entry point
│   ├── lib.rs           # Library exports
│   ├── app.rs           # Application state and logic
│   ├── web.rs           # WASM entry point
│   └── ui/              # UI components
│       ├── mod.rs       # UI module with menu/status bar
│       ├── sidebar.rs   # Navigation sidebar
│       ├── welcome.rs   # Welcome/drop zone view
│       ├── overview.rs  # Overview dashboard
│       ├── header.rs    # Header information view
│       ├── files.rs     # Files list view
│       ├── light_sources.rs  # Light sources view
│       ├── variants.rs  # Variants view
│       ├── statistics.rs    # Statistics view
│       ├── raw_data.rs  # Raw JSON view
│       └── file_viewer.rs   # Embedded file viewer
├── index.html           # WASM HTML template
├── Trunk.toml          # Trunk configuration
└── Cargo.toml          # Dependencies
```

## Usage

1. **Open a file**:
   - Drag and drop a `.gldf` file onto the window
   - Use File > Open... menu (native only)

2. **Navigate**:
   - Use the sidebar to switch between views
   - Overview shows a summary dashboard
   - Raw Data shows the JSON representation

3. **Export**:
   - File > Export JSON...
   - File > Export XML...

## Dependencies

- [egui](https://github.com/emilk/egui) - Immediate mode GUI
- [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) - Framework for egui
- [gldf-rs](https://github.com/holg/gldf-rs) - GLDF parsing library
- [rfd](https://github.com/PolyMeilex/rfd) - Native file dialogs

## License

GPL-3.0-or-later
