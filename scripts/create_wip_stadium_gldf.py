#!/usr/bin/env python3
"""
Create a work-in-progress GLDF file for the stadium plugin.

This generates a GLDF file containing:
- Stadium floodlight photometric data (LDT)
- Plugin manifest for the stadium viewer
- Stadium configuration JSON
"""

import json
import os
import zipfile
from datetime import datetime


def create_product_xml():
    """Create the GLDF product.xml content."""
    now = datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")
    return f'''<?xml version="1.0" encoding="UTF-8"?>
<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="https://gldf.io/xsd/gldf/1.0.0/gldf.xsd">
    <Header>
        <Author>gldf-rs stadium plugin</Author>
        <Manufacturer>GLDF Stadium Demo</Manufacturer>
        <FormatVersion major="1" minor="0" pre-release="3"/>
        <CreatedWithApplication>create_wip_stadium_gldf.py</CreatedWithApplication>
        <GldfCreationTimeCode>{now}</GldfCreationTimeCode>
        <UniqueGldfId>wip-stadium-plugin-001</UniqueGldfId>
    </Header>
    <GeneralDefinitions>
        <Files>
            <File id="stadium_config" contentType="other" type="localFileName">stadium_config.json</File>
            <File id="floodlight_ldt" contentType="ldc/eulumdat" type="localFileName">SL9178-480W-5700K.ldt</File>
        </Files>
        <Photometries>
            <Photometry id="floodlight_photometry">
                <PhotometryFileReference fileId="floodlight_ldt"/>
            </Photometry>
        </Photometries>
        <LightSources>
            <FixedLightSource id="floodlight_source">
                <Name>
                    <Locale language="en">Stadium Floodlight LED 480W 5700K</Locale>
                </Name>
                <RatedInputPower>480</RatedInputPower>
                <ColorInformation>
                    <ColorRenderingIndex>80</ColorRenderingIndex>
                    <CorrelatedColorTemperature>5700</CorrelatedColorTemperature>
                    <ColorTemperatureAdjustingRange>
                        <Lower>5700</Lower>
                        <Upper>5700</Upper>
                    </ColorTemperatureAdjustingRange>
                </ColorInformation>
                <LightSourceMaintenance>
                    <Cie97LampType>LED</Cie97LampType>
                </LightSourceMaintenance>
                <RatedLuminousFlux>66000</RatedLuminousFlux>
            </FixedLightSource>
        </LightSources>
        <Emitters>
            <Emitter id="floodlight_emitter">
                <FixedLightEmitter>
                    <PhotometryReference photometryId="floodlight_photometry"/>
                    <LightSourceReference fixedLightSourceId="floodlight_source"/>
                    <RatedLuminousFlux>66000</RatedLuminousFlux>
                </FixedLightEmitter>
            </Emitter>
        </Emitters>
    </GeneralDefinitions>
    <ProductDefinitions>
        <ProductMetaData>
            <UniqueProductId>wip-stadium-floodlight</UniqueProductId>
            <ProductNumber><Locale language="en">SL9178-480W-5700K-STADIUM</Locale></ProductNumber>
            <Name><Locale language="en">Stadium Floodlight 480W 5700K</Locale></Name>
            <Description><Locale language="en">
                Work-in-progress GLDF file for stadium floodlight visualization.
                Contains embedded stadium viewer plugin for Bevy-based 3D rendering.

                Features:
                - 4 or 6 floodlight tower configuration
                - Adjustable floodlight tilt (C/V keys)
                - Light intensity control (+/- keys)
                - Camera navigation (WASD, arrows, scroll)
                - Photometric data visualization
            </Locale></Description>
        </ProductMetaData>
        <Variants>
            <Variant id="stadium_4_towers">
                <Name><Locale language="en">4-Tower Configuration</Locale></Name>
                <EmitterReferences>
                    <EmitterReference emitterId="floodlight_emitter"/>
                </EmitterReferences>
            </Variant>
            <Variant id="stadium_6_towers">
                <Name><Locale language="en">6-Tower Configuration (with middle towers)</Locale></Name>
                <EmitterReferences>
                    <EmitterReference emitterId="floodlight_emitter"/>
                </EmitterReferences>
            </Variant>
        </Variants>
    </ProductDefinitions>
</Root>
'''


def create_stadium_config():
    """Create the stadium configuration JSON."""
    return {
        "stadium": {
            "name": "Demo Stadium",
            "field": {
                "length": 105.0,
                "width": 68.0,
                "surface": "grass"
            },
            "stands": {
                "enabled": True,
                "tiers": 5,
                "height_per_tier": 2.0,
                "depth": 15.0
            }
        },
        "towers": {
            "layout": "4_corners",
            "height": 45.0,
            "positions": {
                "corner_offset_x": 10.0,
                "corner_offset_z": 10.0,
                "middle_enabled": False
            }
        },
        "floodlights": {
            "per_tower": 1,
            "default_tilt": 35.0,
            "min_tilt": 15.0,
            "max_tilt": 60.0,
            "intensity_scale": 1.0,
            "shadows_enabled": True
        },
        "controls": {
            "tilt_decrease": "C",
            "tilt_increase": "V",
            "intensity_decrease": "-",
            "intensity_increase": "+",
            "toggle_layout": "T",
            "reset_view": "R",
            "toggle_shadows": "H",
            "toggle_night_mode": "N"
        },
        "camera": {
            "initial_position": [0.0, 80.0, 120.0],
            "look_at": [0.0, 0.0, 0.0],
            "orbit_speed": 1.0,
            "zoom_speed": 5.0
        },
        "plugin": {
            "type": "stadium",
            "name": "Stadium Floodlight Viewer",
            "version": "0.1.0",
            "description": "3D stadium floodlight visualization using Bevy and photometric data"
        }
    }


def create_plugin_manifest():
    """Create the plugin manifest JSON."""
    return {
        "type": "stadium",
        "name": "Stadium Floodlight Viewer",
        "version": "0.1.0",
        "description": "3D stadium floodlight visualization using Bevy",
        "native_binary": "gldf-stadium-viewer",
        "wasm_js": "gldf_stadium_plugin.js",
        "wasm_binary": "gldf_stadium_plugin_bg.wasm",
        "data_file": "other/stadium_config.json",
        "ldt_file": "ldc/SL9178-480W-5700K.ldt",
        "entry_point": "run_stadium_viewer",
        "controls": {
            "keyboard": [
                {"key": "C", "action": "Decrease floodlight tilt"},
                {"key": "V", "action": "Increase floodlight tilt"},
                {"key": "T", "action": "Toggle tower layout (4/6)"},
                {"key": "+", "action": "Increase light intensity"},
                {"key": "-", "action": "Decrease light intensity"},
                {"key": "R", "action": "Reset camera view"},
                {"key": "H", "action": "Toggle shadows"},
                {"key": "N", "action": "Toggle night/day mode"},
                {"key": "WASD", "action": "Move camera"},
                {"key": "Arrows", "action": "Orbit camera"},
                {"key": "QE", "action": "Zoom in/out"}
            ]
        }
    }


def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(script_dir)
    output_path = os.path.join(project_root, "tests", "data", "wip_stadium.gldf")

    # Try to find the LDT file
    ldt_paths = [
        "/Volumes/tb3ssd/downloads/SL9178-480W-5700K.ldt",
        os.path.join(project_root, "tests", "data", "SL9178-480W-5700K.ldt"),
        os.path.join(script_dir, "SL9178-480W-5700K.ldt"),
    ]

    ldt_content = None
    for ldt_path in ldt_paths:
        if os.path.exists(ldt_path):
            with open(ldt_path, "rb") as f:
                ldt_content = f.read()
            print(f"Found LDT file: {ldt_path}")
            break

    if ldt_content is None:
        print("Warning: LDT file not found, creating placeholder")
        ldt_content = b"# Placeholder LDT file - replace with actual photometric data\n"

    # Create the GLDF ZIP file
    with zipfile.ZipFile(output_path, "w", zipfile.ZIP_DEFLATED) as zf:
        # Add product.xml
        zf.writestr("product.xml", create_product_xml())

        # Add LDT file
        zf.writestr("ldc/SL9178-480W-5700K.ldt", ldt_content)

        # Add stadium configuration
        stadium_config = json.dumps(create_stadium_config(), indent=2)
        zf.writestr("other/stadium_config.json", stadium_config)

        # Add plugin manifest
        manifest = json.dumps(create_plugin_manifest(), indent=2)
        zf.writestr("other/viewer/stadium/manifest.json", manifest)

        # Add placeholder files for WASM build (to be replaced after compilation)
        zf.writestr("other/viewer/stadium/gldf_stadium_plugin.js",
                    "// Placeholder - build with: cargo build --target wasm32-unknown-unknown --release\n")
        zf.writestr("other/viewer/stadium/gldf_stadium_plugin_bg.wasm", b"")

    print(f"Created: {output_path}")

    # List contents
    print("\nGLDF contents:")
    with zipfile.ZipFile(output_path, "r") as zf:
        for info in zf.infolist():
            print(f"  {info.filename} ({info.file_size} bytes)")


if __name__ == "__main__":
    main()
