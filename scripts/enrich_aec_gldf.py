#!/usr/bin/env python3
"""
Generate two enriched GLDF files to demonstrate different use cases:

1. aec_ga15_enriched.gldf - ONE base LDT file, many variants with parameter overrides
   - Demonstrates storage efficiency: 600 variants sharing 1 geometry + 1 photometry
   - No spectrum XML files (all variants use same base photometry)

2. aec_ga15_enriched_spectral.gldf - 5 spectrum XML files as base photometry
   - Demonstrates spectral data handling with TM-30/TM-33 data
   - Each CCT has its own spectrum file with full SPD (380-780nm)
   - Light sources reference specific spectrums
"""

import zipfile
import os
import math
from datetime import datetime, timezone

# Configuration for variants
WATTAGES = [30, 50, 75, 100, 150, 200]  # 6 wattages
CCTS = [3000, 3500, 4000, 5000, 6500]   # 5 CCTs
OPTICS = ["Narrow", "Medium", "Wide", "Asymmetric"]  # 4 beam angles
LENGTHS = [600, 900, 1200, 1500, 1800]  # 5 lengths in mm
COLORS = [("RAL9003", "White"), ("RAL9005", "Black"), ("RAL9006", "Silver")]  # 3 colors


def generate_fake_spectrum(cct: int) -> list[tuple[int, float]]:
    """
    Generate fake spectral power distribution (SPD) data for a given CCT.
    Returns list of (wavelength_nm, relative_intensity) tuples from 380-780nm in 5nm steps.
    """
    wavelengths = list(range(380, 785, 5))  # 380-780nm in 5nm steps (81 bins)
    spectrum = []

    for wl in wavelengths:
        # Blue LED pump peak around 450nm (Gaussian)
        blue_peak = 0.6 * math.exp(-((wl - 450) ** 2) / (2 * 15 ** 2))

        # Phosphor emission - shifts based on CCT
        phosphor_peak_wl = 580 - (cct - 4000) * 0.02
        phosphor_width = 80 + (cct - 3000) * 0.01

        phosphor = 0.8 * math.exp(-((wl - phosphor_peak_wl) ** 2) / (2 * phosphor_width ** 2))

        # Red phosphor component (more prominent in warm white)
        if cct <= 4000:
            red_boost = (4000 - cct) / 3000
            red_phosphor = red_boost * 0.4 * math.exp(-((wl - 620) ** 2) / (2 * 40 ** 2))
        else:
            red_phosphor = 0

        intensity = blue_peak + phosphor + red_phosphor
        intensity *= (1 + 0.02 * math.sin(wl * 0.1))
        spectrum.append((wl, max(0, intensity)))

    # Normalize to max = 1.0
    max_intensity = max(s[1] for s in spectrum)
    if max_intensity > 0:
        spectrum = [(wl, intensity / max_intensity) for wl, intensity in spectrum]

    return spectrum


def generate_tm30_values(cct: int) -> tuple[int, int]:
    """Generate fake but realistic TM-30 Rf and Rg values."""
    base_rf = 85
    if cct <= 3000:
        rf = base_rf + 5
    elif cct <= 4000:
        rf = base_rf + 3
    elif cct <= 5000:
        rf = base_rf
    else:
        rf = base_rf - 2

    rf += int(math.sin(cct * 0.001) * 3)
    rf = max(70, min(95, rf))
    rg = 100 - int((cct - 4000) * 0.002)
    rg = max(95, min(108, rg))

    return rf, rg


def generate_cie_xy(cct: int) -> tuple[float, float]:
    """Calculate approximate CIE 1931 chromaticity coordinates for a given CCT."""
    if cct < 4000:
        x = -0.2661239e9 / cct**3 - 0.2343589e6 / cct**2 + 0.8776956e3 / cct + 0.179910
    else:
        x = -3.0258469e9 / cct**3 + 2.1070379e6 / cct**2 + 0.2226347e3 / cct + 0.240390

    if cct < 2222:
        y = -1.1063814 * x**3 - 1.34811020 * x**2 + 2.18555832 * x - 0.20219683
    elif cct < 4000:
        y = -0.9549476 * x**3 - 1.37418593 * x**2 + 2.09137015 * x - 0.16748867
    else:
        y = 3.0817580 * x**3 - 5.87338670 * x**2 + 3.75112997 * x - 0.37001483

    return round(x, 4), round(y, 4)


def generate_fake_photometry(cct: int) -> list[list[float]]:
    """
    Generate fake angular photometric data (C/γ matrix).
    Returns intensity values for C-planes (0-360) and gamma angles (0-180).

    This simulates a typical LED downlight distribution.
    """
    # C-planes: 0, 15, 30, 45, 60, 75, 90 (symmetric, so we only need 0-90)
    c_planes = list(range(0, 91, 15))  # 7 C-planes
    # Gamma angles: 0-90 in 5 degree steps (downlight, no uplight)
    gamma_angles = list(range(0, 91, 5))  # 19 gamma angles

    # Generate intensity matrix
    # Peak at gamma=0 (nadir), decreasing with angle
    # Slight variation with CCT (warmer = slightly wider beam)
    beam_width = 35 + (cct - 3000) / 500  # 35-42 degrees half-angle

    intensities = []
    for gamma in gamma_angles:
        row = []
        for c in c_planes:
            # Gaussian-like distribution
            intensity = 1000 * math.exp(-(gamma ** 2) / (2 * beam_width ** 2))
            # Add slight asymmetry based on C-plane
            intensity *= (1 + 0.02 * math.sin(math.radians(c * 4)))
            row.append(max(0, intensity))
        intensities.append(row)

    return intensities, c_planes, gamma_angles


def generate_iesxml_content(cct: int, wattage: int = 50) -> str:
    """
    Generate IES TM-33-23 (IESTM33-22) XML content with:
    - Angular photometric data (C/γ matrix) - like LDT/IES
    - Spectral data (SPD, 380-780nm)
    - Colorimetric data (CCT, CRI, TM-30 Rf/Rg)

    TM-33 is the complete successor to IES/EULUMDAT, not just spectral data!
    Uses IESTM33-22 root element which is properly parsed by atla crate.
    """
    spectrum = generate_fake_spectrum(cct)
    rf, rg = generate_tm30_values(cct)
    x, y = generate_cie_xy(cct)
    intensities, c_planes, gamma_angles = generate_fake_photometry(cct)

    # Calculate luminous flux from intensity distribution (simplified)
    efficacy = 150 - (wattage - 30) * 0.3
    lumens = int(wattage * efficacy)

    # Generate spectral wavelengths and values as space-separated strings (TM-33-23 format)
    wavelengths_str = " ".join(str(wl) for wl, _ in spectrum)
    values_str = " ".join(f"{intensity:.4f}" for _, intensity in spectrum)

    # Generate intensity data elements (TM-33-23 format: horz/vert attributes)
    intensity_data = ""
    for gamma_idx, gamma in enumerate(gamma_angles):
        for c_idx, c in enumerate(c_planes):
            intensity_data += f"            <IntensityData horz=\"{c}\" vert=\"{gamma}\">{intensities[gamma_idx][c_idx]:.1f}</IntensityData>\n"

    return f'''<?xml version="1.0" encoding="UTF-8"?>
<!--
  IES TM-33-23 Complete Photometric Data File

  This file contains EVERYTHING - it's a complete replacement for IES/LDT:
  - Angular photometric data (C/γ intensity matrix)
  - Spectral power distribution (380-780nm)
  - Color metrics (CCT, CRI, TM-30 Rf/Rg, Duv)
  - Luminaire metadata

  CCT: {cct}K | Rf: {rf} | Rg: {rg} | {wattage}W | {lumens}lm
-->
<IESTM33-22>
    <Version>1.1</Version>
    <Header>
        <Manufacturer>AEC Illuminazione</Manufacturer>
        <CatalogNumber>GA15-{cct}K</CatalogNumber>
        <Description>GA15 Industrial LED Module - {cct}K CCT</Description>
        <Laboratory>Demo Data Generator</Laboratory>
        <ReportNumber>GA15-TM33-{cct}K</ReportNumber>
        <ReportDate>2024-01-15</ReportDate>
        <DocumentCreator>GLDF Demo Generator</DocumentCreator>
        <DocumentCreationDate>2024-01-15</DocumentCreationDate>
    </Header>
    <Luminaire>
        <Dimensions>
            <Length>300</Length>
            <Width>300</Width>
            <Height>120</Height>
        </Dimensions>
        <Mounting>Recessed</Mounting>
        <NumEmitters>1</NumEmitters>
    </Luminaire>
    <Emitter>
        <ID>led-{cct}k</ID>
        <Description>LED Module {cct}K</Description>
        <CatalogNumber>GA15-LED-{cct}K</CatalogNumber>
        <Quantity>1</Quantity>
        <RatedLumens>{lumens}</RatedLumens>
        <InputWattage>{wattage}</InputWattage>
        <PowerFactor>0.95</PowerFactor>
        <BallastFactor>1.0</BallastFactor>
        <FixedCCT>{cct}</FixedCCT>
        <Duv>0.000</Duv>
        <ColorRendering>
            <Ra>90</Ra>
            <R9>50</R9>
            <Rf>{rf}</Rf>
            <Rg>{rg}</Rg>
        </ColorRendering>
        <LuminousData>
            <PhotometryType>CIE _ C</PhotometryType>
            <Metric>Luminous</Metric>
            <SymmType>Symm _ Quad</SymmType>
            <Multiplier>1.0</Multiplier>
{intensity_data}        </LuminousData>
        <SpectralDistribution>
            <Wavelengths>{wavelengths_str}</Wavelengths>
            <Values>{values_str}</Values>
        </SpectralDistribution>
    </Emitter>
</IESTM33-22>
'''


# =============================================================================
# VERSION 1: aec_ga15_enriched.gldf - ONE base LDT, many variants
# =============================================================================

def generate_enriched_gldf(input_gldf: str, output_gldf: str):
    """Generate enriched GLDF with ONE base LDT and many parameter-override variants."""

    # Generate light sources (different wattages/CCTs but all use same base photometry)
    light_sources = []
    for wattage in WATTAGES:
        for cct in CCTS:
            source_id = f"led_{wattage}W_{cct}K"
            efficacy = 150 - (wattage - 30) * 0.3
            lumens = int(wattage * efficacy)
            rf, rg = generate_tm30_values(cct)
            x, y = generate_cie_xy(cct)
            cri = 80 if cct >= 5000 else 90

            light_sources.append({
                "id": source_id,
                "wattage": wattage,
                "cct": cct,
                "lumens": lumens,
                "cri": cri,
                "rf": rf,
                "rg": rg,
                "cie_x": x,
                "cie_y": y,
            })

    # Generate emitters
    emitters = []
    for ls in light_sources:
        emitters.append({
            "id": f"emitter_{ls['id']}",
            "light_source_id": ls["id"],
            "name": f"LED Module {ls['wattage']}W {ls['cct']}K",
            "lumens": ls["lumens"],
        })

    # Generate variants
    variants = []
    variant_num = 1
    for length in LENGTHS:
        for optic in OPTICS:
            for ral, color_name in COLORS:
                for ls in light_sources[:10]:  # First 10 light sources
                    variants.append({
                        "id": f"variant_{variant_num}",
                        "product_number": f"GA15-{length}-{optic[0]}-{ls['cct']}K-{ls['wattage']}W-{ral}",
                        "name": f"GA15 {length}mm {optic} {ls['cct']}K {ls['wattage']}W {color_name}",
                        "length": length,
                        "width": 120,
                        "height": 80,
                        "weight": 2.0 + (length / 1000) * 1.5,
                        "emitter_id": f"emitter_{ls['id']}",
                        "ral": ral,
                        "color_name": color_name,
                    })
                    variant_num += 1

    # Generate XML
    light_sources_xml = ""
    for ls in light_sources:
        light_sources_xml += f'''
        <FixedLightSource id="{ls['id']}">
            <Name><Locale language="en">LED {ls['wattage']}W {ls['cct']}K CRI{ls['cri']}</Locale></Name>
            <RatedInputPower>{ls['wattage']}</RatedInputPower>
            <RatedLuminousFlux>{ls['lumens']}</RatedLuminousFlux>
            <ColorInformation>
                <ColorRenderingIndex>{ls['cri']}</ColorRenderingIndex>
                <CorrelatedColorTemperature>{ls['cct']}</CorrelatedColorTemperature>
                <RatedChromacityCoordinateValues>
                    <X>{ls['cie_x']}</X>
                    <Y>{ls['cie_y']}</Y>
                </RatedChromacityCoordinateValues>
                <IES-TM-30-15>
                    <Rf>{ls['rf']}</Rf>
                    <Rg>{ls['rg']}</Rg>
                </IES-TM-30-15>
            </ColorInformation>
        </FixedLightSource>'''

    emitters_xml = ""
    for em in emitters:
        emitters_xml += f'''
        <Emitter id="{em['id']}">
            <FixedLightEmitter>
                <Name><Locale language="en">{em['name']}</Locale></Name>
                <PhotometryReference photometryId="photometry"/>
                <LightSourceReference fixedLightSourceId="{em['light_source_id']}" lightSourceCount="1"/>
                <RatedLuminousFlux>{em['lumens']}</RatedLuminousFlux>
            </FixedLightEmitter>
        </Emitter>'''

    variants_xml = ""
    for v in variants:
        variants_xml += f'''
        <Variant id="{v['id']}">
            <ProductNumber><Locale language="en">{v['product_number']}</Locale></ProductNumber>
            <Name><Locale language="en">{v['name']}</Locale></Name>
            <Geometry>
                <ModelGeometryReference geometryId="geometry">
                    <EmitterReference emitterId="{v['emitter_id']}">
                        <EmitterObjectExternalName>LEO</EmitterObjectExternalName>
                    </EmitterReference>
                </ModelGeometryReference>
            </Geometry>
            <DescriptiveAttributes>
                <Mechanical>
                    <ProductSize>
                        <Length>{v['length']}</Length>
                        <Width>{v['width']}</Width>
                        <Height>{v['height']}</Height>
                    </ProductSize>
                    <Weight>{v['weight']:.1f}</Weight>
                </Mechanical>
                <Electrical>
                    <ElectricalSafetyClass>ClassI</ElectricalSafetyClass>
                    <IngressProtectionIPCode>IP66</IngressProtectionIPCode>
                    <PowerFactor>0.95</PowerFactor>
                    <ConstantLightOutput>true</ConstantLightOutput>
                </Electrical>
                <Marketing>
                    <HousingColors>
                        <HousingColor ral="{v['ral'].replace('RAL', '')}">
                            <Locale language="en">{v['color_name']}</Locale>
                        </HousingColor>
                    </HousingColors>
                </Marketing>
            </DescriptiveAttributes>
        </Variant>'''

    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    product_xml = f'''<?xml version="1.0" encoding="UTF-8"?>
<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="https://gldf.io/xsd/gldf/1.0.0/gldf.xsd">
    <Header>
        <Author>holg</Author>
        <Manufacturer>AEC Illuminazione</Manufacturer>
        <FormatVersion major="1" minor="0" pre-release="3"/>
        <CreatedWithApplication>gldf-rs variant demo</CreatedWithApplication>
        <GldfCreationTimeCode>{timestamp}</GldfCreationTimeCode>
        <UniqueGldfId>aec-ga15-enriched-variants</UniqueGldfId>
    </Header>
    <GeneralDefinitions>
        <Files>
            <!-- ONE 3D model shared by ALL {len(variants)} variants -->
            <File id="geometry_file" contentType="geometry/l3d" type="localFileName">model.l3d</File>
            <!-- ONE photometry file - variants use different light sources with scaled lumen output -->
            <File id="photometry_file" contentType="ldc/eulumdat" type="localFileName">photometry.ldt</File>
            <File id="image_file" contentType="image/jpg" type="localFileName">product.jpg</File>
        </Files>
        <Photometries>
            <Photometry id="photometry">
                <PhotometryFileReference fileId="photometry_file"/>
            </Photometry>
        </Photometries>
        <LightSources>
            <!-- {len(light_sources)} light source definitions (wattage/CCT combinations) -->
            <!-- All reference the SAME base photometry - lumen output is scaled -->{light_sources_xml}
        </LightSources>
        <Emitters>{emitters_xml}
        </Emitters>
        <Geometries>
            <ModelGeometry id="geometry">
                <GeometryFileReference fileId="geometry_file" levelOfDetail="High"/>
            </ModelGeometry>
        </Geometries>
    </GeneralDefinitions>
    <ProductDefinitions>
        <ProductMetaData>
            <UniqueProductId>aec-ga15-series</UniqueProductId>
            <ProductNumber><Locale language="en">GA15 Series</Locale></ProductNumber>
            <Name><Locale language="en">GA15 Industrial LED Luminaire</Locale></Name>
            <Description><Locale language="en">GA15 Series - Demonstrating GLDF variant efficiency.

This file contains {len(variants)} product variants sharing:
- 1 geometry file (3D model)
- 1 photometry file (base LDT)
- {len(light_sources)} light source configurations

Traditional approach: {len(variants)} separate files
GLDF approach: 1 file with shared assets

Key point: All variants use the SAME base photometry.
The light source definitions provide different CCT/wattage/lumen values,
but the light distribution pattern comes from the single LDT file.
</Locale></Description>
        </ProductMetaData>
        <Variants>{variants_xml}
        </Variants>
    </ProductDefinitions>
</Root>'''

    # Extract files from original and create new GLDF
    with zipfile.ZipFile(input_gldf, 'r') as zin:
        geometry_data = zin.read("geometry/model.l3d")
        photometry_data = zin.read("ldc/photometry.ldt")
        image_data = zin.read("image/product.jpg")

    with zipfile.ZipFile(output_gldf, 'w', zipfile.ZIP_DEFLATED) as zout:
        zout.writestr("product.xml", product_xml.encode('utf-8'))
        zout.writestr("geometry/model.l3d", geometry_data)
        zout.writestr("ldc/photometry.ldt", photometry_data)
        zout.writestr("image/product.jpg", image_data)

    return len(variants), len(light_sources)


# =============================================================================
# VERSION 2: aec_ga15_enriched_spectral.gldf - TM-33 IESXML as complete photometry
# =============================================================================

def generate_spectral_gldf(input_gldf: str, output_gldf: str):
    """
    Generate GLDF with IES TM-33 XML files as the COMPLETE photometry source.

    TM-33/IESXML contains EVERYTHING:
    - Angular photometric data (C/γ intensity matrix) - replaces LDT/IES
    - Spectral power distribution (380-780nm)
    - Color metrics (CCT, CRI, TM-30 Rf/Rg, Duv)

    NO separate LDT file is needed - the IESXML files ARE the photometry!
    """

    # Generate TM-33 file references - these are the PHOTOMETRY files
    tm33_files_xml = ""
    for cct in CCTS:
        tm33_files_xml += f'''
            <File id="photometry_{cct}K" contentType="ldc/iesxml" type="localFileName">GA15_{cct}K.iesxml</File>'''

    # Generate photometry definitions - each CCT has its own photometry from TM-33
    photometries_xml = ""
    for cct in CCTS:
        photometries_xml += f'''
            <Photometry id="photometry_{cct}K">
                <PhotometryFileReference fileId="photometry_{cct}K"/>
            </Photometry>'''

    # Generate spectrums with inline intensity data (extracted from TM-33 for display)
    spectrums_xml = ""
    for cct in CCTS:
        spectrum = generate_fake_spectrum(cct)
        intensities_xml = ""
        for wl, intensity in spectrum:
            intensities_xml += f'\n                <Intensity wavelength="{wl}">{intensity:.4f}</Intensity>'

        spectrums_xml += f'''
            <Spectrum id="spectrum_{cct}K">
                <SpectrumFileReference fileId="photometry_{cct}K"/>{intensities_xml}
            </Spectrum>'''

    # Generate light sources - each CCT has its own
    light_sources = []
    for wattage in WATTAGES[:3]:  # 3 wattages for demo
        for cct in CCTS:
            source_id = f"led_{wattage}W_{cct}K"
            efficacy = 150 - (wattage - 30) * 0.3
            lumens = int(wattage * efficacy)
            rf, rg = generate_tm30_values(cct)
            x, y = generate_cie_xy(cct)
            cri = 80 if cct >= 5000 else 90

            light_sources.append({
                "id": source_id,
                "wattage": wattage,
                "cct": cct,
                "lumens": lumens,
                "cri": cri,
                "rf": rf,
                "rg": rg,
                "cie_x": x,
                "cie_y": y,
                "spectrum_id": f"spectrum_{cct}K",
                "photometry_id": f"photometry_{cct}K",  # Each CCT uses its own photometry!
            })

    light_sources_xml = ""
    for ls in light_sources:
        light_sources_xml += f'''
        <FixedLightSource id="{ls['id']}">
            <Name><Locale language="en">LED {ls['wattage']}W {ls['cct']}K Rf{ls['rf']}</Locale></Name>
            <RatedInputPower>{ls['wattage']}</RatedInputPower>
            <RatedLuminousFlux>{ls['lumens']}</RatedLuminousFlux>
            <SpectrumReference spectrumId="{ls['spectrum_id']}"/>
            <ColorInformation>
                <ColorRenderingIndex>{ls['cri']}</ColorRenderingIndex>
                <CorrelatedColorTemperature>{ls['cct']}</CorrelatedColorTemperature>
                <RatedChromacityCoordinateValues>
                    <X>{ls['cie_x']}</X>
                    <Y>{ls['cie_y']}</Y>
                </RatedChromacityCoordinateValues>
                <IES-TM-30-15>
                    <Rf>{ls['rf']}</Rf>
                    <Rg>{ls['rg']}</Rg>
                </IES-TM-30-15>
                <MelanopicFactor>{0.7 + (ls['cct'] - 3000) / 10000:.2f}</MelanopicFactor>
            </ColorInformation>
        </FixedLightSource>'''

    # Generate emitters - each references its CCT's TM-33 photometry
    emitters_xml = ""
    for ls in light_sources:
        emitters_xml += f'''
        <Emitter id="emitter_{ls['id']}">
            <FixedLightEmitter>
                <Name><Locale language="en">LED Module {ls['wattage']}W {ls['cct']}K</Locale></Name>
                <PhotometryReference photometryId="{ls['photometry_id']}"/>
                <LightSourceReference fixedLightSourceId="{ls['id']}" lightSourceCount="1"/>
                <RatedLuminousFlux>{ls['lumens']}</RatedLuminousFlux>
            </FixedLightEmitter>
        </Emitter>'''

    # Generate variants
    variants = []
    variant_num = 1
    for length in LENGTHS[:2]:
        for ls in light_sources:
            variants.append({
                "id": f"variant_{variant_num}",
                "product_number": f"GA15-{length}-{ls['cct']}K-{ls['wattage']}W",
                "name": f"GA15 {length}mm {ls['cct']}K {ls['wattage']}W",
                "length": length,
                "width": 120,
                "height": 80,
                "weight": 2.0 + (length / 1000) * 1.5,
                "emitter_id": f"emitter_{ls['id']}",
                "cct": ls['cct'],
                "wattage": ls['wattage'],
            })
            variant_num += 1

    variants_xml = ""
    for v in variants:
        variants_xml += f'''
        <Variant id="{v['id']}">
            <ProductNumber><Locale language="en">{v['product_number']}</Locale></ProductNumber>
            <Name><Locale language="en">{v['name']}</Locale></Name>
            <Geometry>
                <ModelGeometryReference geometryId="geometry">
                    <EmitterReference emitterId="{v['emitter_id']}">
                        <EmitterObjectExternalName>LEO</EmitterObjectExternalName>
                    </EmitterReference>
                </ModelGeometryReference>
            </Geometry>
            <DescriptiveAttributes>
                <Mechanical>
                    <ProductSize>
                        <Length>{v['length']}</Length>
                        <Width>{v['width']}</Width>
                        <Height>{v['height']}</Height>
                    </ProductSize>
                </Mechanical>
            </DescriptiveAttributes>
        </Variant>'''

    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    product_xml = f'''<?xml version="1.0" encoding="UTF-8"?>
<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="https://gldf.io/xsd/gldf/1.0.0/gldf.xsd">
    <Header>
        <Author>holg</Author>
        <Manufacturer>AEC Illuminazione</Manufacturer>
        <FormatVersion major="1" minor="0" pre-release="3"/>
        <CreatedWithApplication>gldf-rs TM-33 demo</CreatedWithApplication>
        <GldfCreationTimeCode>{timestamp}</GldfCreationTimeCode>
        <UniqueGldfId>aec-ga15-tm33-spectral</UniqueGldfId>
    </Header>
    <GeneralDefinitions>
        <Files>
            <File id="geometry_file" contentType="geometry/l3d" type="localFileName">model.l3d</File>
            <File id="image_file" contentType="image/jpg" type="localFileName">product.jpg</File>
            <!-- IES TM-33 IESXML files - COMPLETE photometry (angular + spectral + color) -->
            <!-- NO separate LDT file - TM-33 is the complete replacement! -->{tm33_files_xml}
        </Files>
        <Photometries>
            <!-- Each CCT has its own photometry from TM-33 file -->{photometries_xml}
        </Photometries>
        <Spectrums>
            <!-- Spectrum data extracted from TM-33 for inline display -->{spectrums_xml}
        </Spectrums>
        <LightSources>{light_sources_xml}
        </LightSources>
        <Emitters>{emitters_xml}
        </Emitters>
        <Geometries>
            <ModelGeometry id="geometry">
                <GeometryFileReference fileId="geometry_file" levelOfDetail="High"/>
            </ModelGeometry>
        </Geometries>
    </GeneralDefinitions>
    <ProductDefinitions>
        <ProductMetaData>
            <UniqueProductId>aec-ga15-tm33</UniqueProductId>
            <ProductNumber><Locale language="en">GA15 Series (TM-33)</Locale></ProductNumber>
            <Name><Locale language="en">GA15 Industrial LED - IES TM-33 Demo</Locale></Name>
            <Description><Locale language="en">GA15 Series - Using IES TM-33 (IESXML) as COMPLETE photometry.

NO LDT/IES files! This file uses {len(CCTS)} TM-33 IESXML files:
{', '.join(f'GA15_{cct}K.iesxml' for cct in CCTS)}

Each TM-33 file contains EVERYTHING:
- Angular photometric data (C/γ intensity matrix) - same as LDT/IES
- Spectral Power Distribution (380-780nm in 5nm bins)
- TM-30 Rf (Fidelity Index) and Rg (Gamut Index)
- CIE colorimetric data (CCT, CRI, Duv, x/y coordinates)
- Melanopic factor for circadian lighting

TM-33 is the complete successor to IES/EULUMDAT, not just a spectral extension!
</Locale></Description>
        </ProductMetaData>
        <Variants>{variants_xml}
        </Variants>
    </ProductDefinitions>
</Root>'''

    # Extract geometry and image from original
    with zipfile.ZipFile(input_gldf, 'r') as zin:
        geometry_data = zin.read("geometry/model.l3d")
        image_data = zin.read("image/product.jpg")

    # Create GLDF with TM-33 files (NO LDT!)
    with zipfile.ZipFile(output_gldf, 'w', zipfile.ZIP_DEFLATED) as zout:
        zout.writestr("product.xml", product_xml.encode('utf-8'))
        zout.writestr("geometry/model.l3d", geometry_data)
        zout.writestr("image/product.jpg", image_data)

        # Add TM-33 IESXML files - these ARE the photometry!
        for cct in CCTS:
            # Use typical wattage for the demo
            wattage = 50
            iesxml_content = generate_iesxml_content(cct, wattage)
            zout.writestr(f"ldc/GA15_{cct}K.iesxml", iesxml_content.encode('utf-8'))

    return len(variants), len(light_sources), len(CCTS)


def main():
    input_gldf = "tests/data/aec_ga15.gldf"

    # Generate Version 1: Variants with ONE base LDT
    output1 = "tests/data/aec_ga15_enriched.gldf"
    v1_variants, v1_sources = generate_enriched_gldf(input_gldf, output1)
    size1 = os.path.getsize(output1)

    print("=" * 60)
    print("VERSION 1: aec_ga15_enriched.gldf")
    print("=" * 60)
    print(f"  - {v1_variants} variants")
    print(f"  - {v1_sources} light source definitions")
    print(f"  - 1 base LDT file (shared by all)")
    print(f"  - NO spectrum XML files")
    print(f"  - Size: {size1:,} bytes ({size1/1024:.1f} KB)")
    print(f"\nUse case: Efficient storage for many product variants")
    print(f"          All variants share the same light distribution pattern")
    print()

    # Generate Version 2: Spectral data demo
    output2 = "tests/data/aec_ga15_enriched_spectral.gldf"
    v2_variants, v2_sources, v2_spectrums = generate_spectral_gldf(input_gldf, output2)
    size2 = os.path.getsize(output2)

    print("=" * 60)
    print("VERSION 2: aec_ga15_enriched_spectral.gldf")
    print("=" * 60)
    print(f"  - {v2_variants} variants")
    print(f"  - {v2_sources} light source definitions")
    print(f"  - {v2_spectrums} spectrum XML files (one per CCT)")
    print(f"  - Full SPD data (380-780nm)")
    print(f"  - TM-30 Rf/Rg values")
    print(f"  - Size: {size2:,} bytes ({size2/1024:.1f} KB)")
    print(f"\nUse case: Products with different spectral characteristics")
    print(f"          Each CCT has its own spectrum file")
    print()

    # List contents
    print("=" * 60)
    print("FILE CONTENTS")
    print("=" * 60)
    for gldf_path in [output1, output2]:
        print(f"\n{os.path.basename(gldf_path)}:")
        with zipfile.ZipFile(gldf_path, 'r') as z:
            for info in z.infolist():
                print(f"  {info.filename}: {info.file_size:,} bytes")


if __name__ == "__main__":
    main()
