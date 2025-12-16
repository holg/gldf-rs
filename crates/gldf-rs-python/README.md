[![CI](https://github.com/holg/gldf-rs-python/actions/workflows/CI.yml/badge.svg)](https://github.com/holg/gldf-rs-python/actions/workflows/CI.yml)
[![PyPI](https://img.shields.io/pypi/v/gldf-rs-python.svg)](https://pypi.org/project/gldf-rs-python/)

# gldf-rs-python

Python bindings for the [gldf-rs](https://crates.io/crates/gldf-rs) GLDF parsing library.

## Overview

gldf-rs-python provides Python access to GLDF (General Lighting Data Format) file parsing and manipulation. It wraps the high-performance Rust gldf-rs library using PyO3/maturin.

GLDF files are ZIP containers containing `product.xml` definitions along with embedded images, photometry files (Eulumdat/IES), and L3D 3D models.

Learn more at: https://gldf.io

## Installation

```bash
pip install gldf-rs-python
```
GLDF Viewer
## Quick Start

```python
import gldf_rs_python

# Load GLDF and convert to XML
xml = gldf_rs_python.gldf_to_xml('path/to/file.gldf')

# Load GLDF and convert to JSON
json = gldf_rs_python.gldf_to_json('path/to/file.gldf')

# Round-trip: JSON back to XML
xml2 = gldf_rs_python.xml_from_json(json)

assert xml == xml2  # True
```

## Converting LDT/IES to GLDF

```python
import gldf_rs_python

# Convert LDT to GLDF JSON
with open("luminaire.ldt", "rb") as f:
    gldf_json = gldf_rs_python.ldt_to_gldf_json(f.read(), "luminaire.ldt")

# Convert LDT to GLDF file
with open("luminaire.ldt", "rb") as f:
    gldf_bytes = gldf_rs_python.ldt_to_gldf_bytes(f.read(), "luminaire.ldt")
    with open("luminaire.gldf", "wb") as out:
        out.write(gldf_bytes)

# Works with IES files too
with open("luminaire.ies", "rb") as f:
    gldf_json = gldf_rs_python.ldt_to_gldf_json(f.read(), "luminaire.ies")
```

## Working with GLDF Bytes

```python
import gldf_rs_python

# Load GLDF from bytes (useful for web apps, streaming)
with open("product.gldf", "rb") as f:
    json_data = gldf_rs_python.gldf_from_bytes(f.read())

# Export GLDF JSON back to bytes
gldf_bytes = gldf_rs_python.gldf_json_to_bytes(json_data)
```

## API Reference

| Function | Description |
|----------|-------------|
| `gldf_to_xml(path)` | Load GLDF file and convert to XML string |
| `gldf_to_json(path)` | Load GLDF file and convert to JSON string |
| `json_from_xml_str(xml_str)` | Parse XML string to JSON |
| `xml_from_json(json_str)` | Parse JSON string to XML |
| `gldf_from_bytes(data)` | Load GLDF from bytes and return JSON |
| `ldt_to_gldf_json(data, filename)` | Convert LDT/IES to GLDF JSON |
| `ldt_to_gldf_bytes(data, filename)` | Convert LDT/IES to GLDF bytes (ZIP) |
| `gldf_json_to_bytes(json_str)` | Export GLDF JSON to bytes (ZIP) |

## Development

```bash
# Create virtual environment
python -m venv .venv
source .venv/bin/activate

# Install maturin
pip install maturin

# Build and install in development mode
maturin develop

# Build release wheel
maturin build --release
```

## Live Demo

Try the WASM-based GLDF viewer at: **https://gldf.icu**

## Related Crates

| Crate | Description |
|-------|-------------|
| [gldf-rs](https://crates.io/crates/gldf-rs) | Core Rust library |
| [gldf-rs-wasm](https://crates.io/crates/gldf-rs-wasm) | WebAssembly viewer application |
| [l3d-rs](https://crates.io/crates/l3d_rs) | L3D 3D model format parsing |
| [eulumdat](https://crates.io/crates/eulumdat) | Eulumdat/LDT photometry parsing |

## Release Notes

### 0.3.3
- **New**: `ldt_to_gldf_json()` - Convert LDT/IES photometry to GLDF JSON
- **New**: `ldt_to_gldf_bytes()` - Convert LDT/IES photometry to GLDF bytes (ZIP)
- **New**: `gldf_from_bytes()` - Load GLDF from bytes and return JSON
- **New**: `gldf_json_to_bytes()` - Export GLDF JSON to bytes (ZIP)
- Improved error handling with proper `PyValueError` exceptions
- Added `eulumdat` feature for LDT/IES conversion support

### 0.3.1
- Updated to gldf-rs 0.3.1 with quick-xml parser
- Part of gldf-rs monorepo restructure

### 0.2.0
- Support for URL file types
- Support for BOM-encoded UTF8 product.xml

## License

MIT License
