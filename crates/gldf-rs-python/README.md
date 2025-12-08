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

### 0.3.1
- Updated to gldf-rs 0.3.1 with quick-xml parser
- Part of gldf-rs monorepo restructure

### 0.2.0
- Support for URL file types
- Support for BOM-encoded UTF8 product.xml

## License

MIT License
