# gldf-bevy

Bevy-based 3D scene viewer for GLDF (Global Lighting Data Format) files with L3D models and photometric lighting visualization.

## Features

- Load and display GLDF files with embedded L3D 3D geometry
- Visualize photometric data from EULUMDAT/IES files
- Real-time 3D rendering using Bevy game engine
- WebAssembly support for browser-based viewing

## Usage

```rust
use gldf_bevy::GldfViewerPlugin;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GldfViewerPlugin)
        .run();
}
```

## License

GPL-3.0-or-later

## Links

- [Repository](https://github.com/holg/gldf-rs)
- [GLDF Specification](https://gldf.io)
