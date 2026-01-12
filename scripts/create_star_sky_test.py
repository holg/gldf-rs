#!/usr/bin/env python3
"""
Create a minimal star_sky.gldf test file with embedded WASM viewer plugin.

This demonstrates the plugin architecture where WASM modules can be
embedded inside GLDF files for self-contained operation.
"""

import json
import zipfile
import io
from datetime import datetime, timezone
from pathlib import Path


def create_star_sky_gldf() -> bytes:
    """Create a minimal star_sky.gldf with embedded starsky viewer plugin."""

    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    # Minimal star data for testing (brightest stars visible from mid-latitudes)
    sky_data = {
        "location": {
            "name": "Test Location",
            "lat": 51.5,
            "lng": 0.0
        },
        "time": timestamp,
        "stars": [
            {"name": "Sirius", "alt": 25.0, "az": 180.0, "mag": -1.46, "spectral": "A1V", "temp": 9940, "ra": 101.287, "dec": -16.716},
            {"name": "Vega", "alt": 65.0, "az": 45.0, "mag": 0.03, "spectral": "A0V", "temp": 9600, "ra": 279.235, "dec": 38.784},
            {"name": "Arcturus", "alt": 55.0, "az": 220.0, "mag": -0.05, "spectral": "K1.5III", "temp": 4286, "ra": 213.915, "dec": 19.182},
            {"name": "Capella", "alt": 70.0, "az": 320.0, "mag": 0.08, "spectral": "G5III", "temp": 4970, "ra": 79.172, "dec": 45.998},
            {"name": "Rigel", "alt": 35.0, "az": 200.0, "mag": 0.13, "spectral": "B8Ia", "temp": 12100, "ra": 78.634, "dec": -8.202},
            {"name": "Betelgeuse", "alt": 45.0, "az": 190.0, "mag": 0.42, "spectral": "M1Ia", "temp": 3500, "ra": 88.793, "dec": 7.407},
            {"name": "Polaris", "alt": 51.5, "az": 0.0, "mag": 2.02, "spectral": "F7Ib", "temp": 6015, "ra": 37.954, "dec": 89.264},
            {"name": "Deneb", "alt": 75.0, "az": 60.0, "mag": 1.25, "spectral": "A2Ia", "temp": 8525, "ra": 310.358, "dec": 45.280},
            {"name": "Altair", "alt": 50.0, "az": 120.0, "mag": 0.77, "spectral": "A7V", "temp": 7550, "ra": 297.696, "dec": 8.868},
            {"name": "Aldebaran", "alt": 40.0, "az": 280.0, "mag": 0.85, "spectral": "K5III", "temp": 3910, "ra": 68.980, "dec": 16.509},
        ]
    }

    # Minimal product.xml
    product_xml = f'''<?xml version="1.0" encoding="UTF-8"?>
<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="https://gldf.io/xsd/gldf/1.0.0/gldf.xsd">
    <Header>
        <Author>gldf-rs test</Author>
        <Manufacturer>Star Sky Test</Manufacturer>
        <FormatVersion major="1" minor="0" pre-release="3"/>
        <CreatedWithApplication>create_star_sky_test.py</CreatedWithApplication>
        <GldfCreationTimeCode>{timestamp}</GldfCreationTimeCode>
        <UniqueGldfId>star-sky-test-001</UniqueGldfId>
    </Header>
    <GeneralDefinitions>
        <Files>
            <File id="sky_data" contentType="other" type="localFileName">sky_data.json</File>
        </Files>
    </GeneralDefinitions>
    <ProductDefinitions>
        <ProductMetaData>
            <UniqueProductId>star-sky-test</UniqueProductId>
            <ProductNumber><Locale language="en">STARSKY-TEST</Locale></ProductNumber>
            <Name><Locale language="en">Star Sky Test</Locale></Name>
            <Description><Locale language="en">
                Test GLDF file with embedded Star Sky WASM viewer plugin.
                Demonstrates plugin architecture for self-contained operation.
            </Locale></Description>
        </ProductMetaData>
        <Variants>
            <Variant id="default">
                <Name><Locale language="en">Default</Locale></Name>
            </Variant>
        </Variants>
    </ProductDefinitions>
</Root>
'''

    # Create GLDF ZIP
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, 'w', zipfile.ZIP_DEFLATED) as zf:
        # Add product.xml
        zf.writestr("product.xml", product_xml)

        # Add sky data JSON
        zf.writestr("other/sky_data.json", json.dumps(sky_data, indent=2))

        # Embed starsky WASM viewer plugin
        script_dir = Path(__file__).parent
        project_dir = script_dir.parent
        starsky_pkg = project_dir / "crates" / "gldf-starsky-wasm" / "pkg"

        js_file = starsky_pkg / "gldf_starsky_wasm.js"
        wasm_file = starsky_pkg / "gldf_starsky_wasm_bg.wasm"

        if js_file.exists() and wasm_file.exists():
            # Plugin manifest - enhanced with data binding and entry point
            manifest = {
                "type": "starsky",
                "name": "Star Sky 2D Viewer",
                "version": "0.1.0",
                "description": "Lightweight 2D star sky visualization",
                "js": "gldf_starsky_wasm.js",
                "wasm": "gldf_starsky_wasm_bg.wasm",
                "data_file": "other/sky_data.json",
                "canvas_id": "starsky-canvas",
                "entry_point": "render_from_storage",
                "storage_key": "gldf_star_sky_json"
            }
            zf.writestr("other/viewer/starsky/manifest.json", json.dumps(manifest, indent=2))
            zf.writestr("other/viewer/starsky/gldf_starsky_wasm.js", js_file.read_bytes())
            zf.writestr("other/viewer/starsky/gldf_starsky_wasm_bg.wasm", wasm_file.read_bytes())
            print(f"Embedded starsky viewer: {wasm_file.stat().st_size:,} bytes")
        else:
            print(f"Warning: starsky WASM not found at {starsky_pkg}")
            print("Run: cd crates/gldf-starsky-wasm && wasm-pack build --target web")

    return buffer.getvalue()


def main():
    """Generate star_sky.gldf test file."""
    print("=== Creating star_sky.gldf test file ===")

    gldf_data = create_star_sky_gldf()

    # Save to tests/data
    script_dir = Path(__file__).parent
    project_dir = script_dir.parent

    test_data_dir = project_dir / "tests" / "data"
    test_data_dir.mkdir(parents=True, exist_ok=True)

    output_file = test_data_dir / "star_sky.gldf"
    output_file.write_bytes(gldf_data)
    print(f"Created: {output_file}")
    print(f"Size: {len(gldf_data):,} bytes ({len(gldf_data)/1024:.1f} KB)")

    # Also copy to static for web testing
    static_dir = project_dir / "crates" / "gldf-rs-wasm" / "src" / "static"
    if static_dir.exists():
        static_file = static_dir / "star_sky.gldf"
        static_file.write_bytes(gldf_data)
        print(f"Copied to: {static_file}")

    # Verify contents
    print("\n=== GLDF Contents ===")
    with zipfile.ZipFile(io.BytesIO(gldf_data), 'r') as zf:
        for info in zf.infolist():
            print(f"  {info.filename}: {info.file_size:,} bytes")


if __name__ == "__main__":
    main()
