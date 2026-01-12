# GLDF Stadium Plugin

Stadium floodlight visualization plugin for GLDF files using Bevy.

## Overview

This plugin renders a 3D FIFA-standard football stadium with 6 floodlight towers positioned at:
- 4 corner positions
- 2 midfield positions (behind each goal)

Each tower has 4 stacked floodlights that illuminate the field using photometric data from GLDF/LDT files.

## Stadium Layout

```
    [FL1]                    [FL2]                    [FL3]
      \                        |                        /
       \                       |                       /
        \                      |                      /
         +--------------------+--------------------+
         |                    |                    |
         |                    |                    |
         |        FIELD       |       FIELD        |
         |       (105x68m)    |                    |
         |                    |                    |
         +--------------------+--------------------+
       /                       |                       \
      /                        |                        \
    [FL4]                    [FL5]                    [FL6]
```

## Features

- **Realistic Stadium Geometry**: FIFA-standard 105x68m field with pitch markings, running track, and spectator stands
- **6 Floodlight Towers**: 45m high towers with 4 floodlights each (24 total lights)
- **Photometric Lighting**: Uses GLDF/LDT data for realistic light distribution
- **Camera Controls**: Orbit around the stadium, adjust height
- **Day/Night Mode**: Toggle between day and night ambient lighting
- **Adjustable Intensity**: Scale light intensity up or down

## Controls

| Key | Action |
|-----|--------|
| Arrow Left/Right | Orbit camera around stadium |
| Arrow Up/Down | Raise/lower camera height |
| O (hold) | Auto-orbit camera |
| H | Toggle shadows |
| N | Toggle night/day mode |
| +/- | Increase/decrease light intensity |

## Running Native

```bash
cargo run -p gldf-stadium-plugin --bin gldf-stadium-viewer
```

## WASM Entry Points

For web builds, use:

```javascript
// Initialize on a canvas element
gldf_stadium_plugin.run_stadium_on_canvas("#stadium-canvas");
```

## Configuration

The stadium can be configured via `StadiumSettings`:

```rust
StadiumSettings {
    field_length: 105.0,      // FIFA standard
    field_width: 68.0,        // FIFA standard
    track_width: 8.0,         // Running track
    stand_height: 15.0,       // Spectator stands
    tower_height: 45.0,       // Floodlight towers
    lights_per_tower: 4,      // Lights per tower
    default_flux: 150000.0,   // Lumens per floodlight
    default_color_temp: 5600.0, // Kelvin (daylight)
    night_mode: true,         // Start in night mode
    shadows_enabled: false,   // Shadows off by default
}
```

## Integration with GLDF

When loading a GLDF file containing floodlight luminaire data:

1. L3D geometry (optional) - Visual representation of the floodlight housing
2. LDT photometry - Light distribution data including:
   - Luminous flux (lumens)
   - Color temperature (Kelvin)
   - Beam angle and distribution

The plugin will use this data to create realistic lighting across the field.

## License

GPL-3.0-or-later
