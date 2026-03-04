#!/usr/bin/env python3
"""
Combine 11 canonical ROLF-exported GLDF files into one multi-variant
LEDVANCE_FLOODLIGHT_MAX.gldf with 11 beam-angle variants across 3 wattages.

Input:
  - 11 *WALName.gldf from scripts/ledvance_assets/floodlight_max/photometry/
  - 3 L3D files from scripts/ledvance_assets/floodlight_max/geometry/

Output:
  - scripts/ledvance_assets/combined_gldf/LEDVANCE_FLOODLIGHT_MAX.gldf
"""

import glob
import os
import re
import sys
import zipfile
from datetime import datetime, timezone

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_DIR = os.path.dirname(SCRIPT_DIR)

PHOTOMETRY_DIR = os.path.join(SCRIPT_DIR, "ledvance_assets", "floodlight_max", "photometry")
GEOMETRY_DIR = os.path.join(SCRIPT_DIR, "ledvance_assets", "floodlight_max", "geometry")
OUTPUT_DIR = os.path.join(SCRIPT_DIR, "ledvance_assets", "combined_gldf")
ENRICHED_TEMPLATE = os.path.join(PROJECT_DIR, "tests", "data",
                                  "FLOODLIGHT_MAX_1200W_757_SYM_10_WAL_enriched.gldf")

# Filename regex for canonical exports
FILENAME_RE = re.compile(
    r"FLOODLIGHT_MAX_(\d+)W_LUMINAIRE_HEAD_757_?(SYM_\d+|ASYM\d*x?\d*)_?WALName\.gldf$"
)

# Physical specs per wattage
WATTAGE_SPECS = {
    1200: {"length": 841, "width": 591, "height": 282, "weight": 31.9,
            "efficacy": 129, "psu_gtin": "4058075580749", "psu_id": "psu_1200w_900w"},
    900:  {"length": 841, "width": 591, "height": 282, "weight": 24.0,
            "efficacy": 130, "psu_gtin": "4058075580749", "psu_id": "psu_1200w_900w"},
    600:  {"length": 591, "width": 441, "height": 282, "weight": 16.5,
            "efficacy": 135, "psu_gtin": "4058075580756", "psu_id": "psu_600w"},
}

# Beam label normalization for IDs and display
def beam_id(beam_raw):
    """Normalize beam string to NCName-safe ID part, e.g. 'sym10', 'asym50x110'."""
    b = beam_raw.lower().replace("_", "").replace(" ", "")
    return b  # e.g. "sym10", "sym30", "sym60", "asym50x110"

def beam_display(beam_raw):
    """Pretty beam label, e.g. 'SYM 10' or 'ASYM 50x110'."""
    b = beam_raw.replace("_", " ").upper()
    # Ensure space after SYM/ASYM if not present
    b = re.sub(r"(SYM|ASYM)\s*(\d)", r"\1 \2", b)
    return b


def light_distribution(beam_raw):
    """Map beam to GLDF LightDistribution enum value."""
    bd = beam_display(beam_raw)
    if "ASYM" in bd:
        return "Laterally asymmetrical"
    angle = re.search(r"\d+", bd)
    if angle:
        a = int(angle.group())
        if a <= 15:
            return "Laterally symmetrical narrow"
        elif a <= 40:
            return "Laterally symmetrical medium"
        else:
            return "Laterally symmetrical wide"
    return "Laterally symmetrical medium"


def parse_ldt_header(ldt_bytes):
    """Parse EULUMDAT header, return dict with key fields."""
    text = ldt_bytes.decode("latin-1")
    lines = text.strip().split("\n")
    # Line indices are 0-based
    gtin = lines[9].strip().split("-")[0] if len(lines) > 9 else ""
    flux_str = lines[28].strip() if len(lines) > 28 else ""
    flux = int(float(flux_str)) if flux_str else 0
    power = int(float(lines[31].strip())) if len(lines) > 31 and lines[31].strip() else 0
    cct = int(float(lines[29].strip())) if len(lines) > 29 and lines[29].strip() else 5700
    cri = int(float(lines[30].strip())) if len(lines) > 30 and lines[30].strip() else 70
    return {"gtin": gtin, "flux": flux, "power": power, "cct": cct, "cri": cri}


def scan_canonical_gldfs():
    """Scan for canonical *WALName.gldf files and extract variant data."""
    pattern = os.path.join(PHOTOMETRY_DIR, "*WALName.gldf")
    variants = []
    for path in sorted(glob.glob(pattern)):
        basename = os.path.basename(path)
        m = FILENAME_RE.search(basename)
        if not m:
            print(f"  SKIP (no match): {basename}")
            continue
        wattage = int(m.group(1))
        beam_raw = m.group(2)

        z = zipfile.ZipFile(path)
        ldt_files = [n for n in z.namelist() if n.endswith(".ldt")]
        if not ldt_files:
            print(f"  SKIP (no LDT): {basename}")
            continue

        ldt_name = ldt_files[0]
        ldt_data = z.read(ldt_name)
        ldt_info = parse_ldt_header(ldt_data)

        # Extract just the filename part of the LDT (strip ldc/ prefix)
        ldt_filename = ldt_name.split("/")[-1]

        variants.append({
            "source_gldf": path,
            "wattage": wattage,
            "beam_raw": beam_raw,
            "beam_id": beam_id(beam_raw),
            "beam_display": beam_display(beam_raw),
            "ldt_zip_path": ldt_name,
            "ldt_filename": ldt_filename,
            "ldt_data": ldt_data,
            "gtin": ldt_info["gtin"],
            "flux": ldt_info["flux"],
            "power": ldt_info["power"],
            "cct": ldt_info["cct"],
            "cri": ldt_info["cri"],
        })

    # Sort by wattage descending, then beam
    variants.sort(key=lambda v: (-v["wattage"], v["beam_id"]))
    return variants


def xml_escape(s):
    """Escape special XML characters."""
    return (s.replace("&", "&amp;")
             .replace("<", "&lt;")
             .replace(">", "&gt;")
             .replace('"', "&quot;"))


def generate_product_xml(variants):
    """Generate the combined product.xml for all 11 variants."""
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    wattages_present = sorted(set(v["wattage"] for v in variants), reverse=True)

    # --- Files section ---
    file_elements = []
    # Geometry files (one per wattage)
    for w in wattages_present:
        file_elements.append(
            f'      <File id="geometry_file_{w}w" contentType="geo/l3d"'
            f' type="localFileName">geometry/model_{w}w.l3d</File>'
        )
    # Photometry files (one per variant)
    for v in variants:
        fid = f'photometry_file_{v["wattage"]}w_{v["beam_id"]}'
        file_elements.append(
            f'      <File id="{fid}" contentType="ldc/eulumdat"'
            f' type="localFileName">ldc/{xml_escape(v["ldt_filename"])}</File>'
        )
    files_xml = "\n".join(file_elements)

    # --- Photometries section ---
    photometry_elements = []
    for v in variants:
        pid = f'photometry_{v["wattage"]}w_{v["beam_id"]}'
        fid = f'photometry_file_{v["wattage"]}w_{v["beam_id"]}'
        specs = WATTAGE_SPECS[v["wattage"]]
        photometry_elements.append(f"""      <Photometry id="{pid}">
        <PhotometryFileReference fileId="{fid}" />
        <DescriptivePhotometry>
          <LuminousEfficacy>{specs["efficacy"]}</LuminousEfficacy>
          <DownwardFluxFraction>1.0</DownwardFluxFraction>
          <UpwardLightOutputRatio>0.0</UpwardLightOutputRatio>
        </DescriptivePhotometry>
      </Photometry>""")
    photometries_xml = "\n".join(photometry_elements)

    # --- LightSources section ---
    lightsource_elements = []
    for w in wattages_present:
        # Find max flux for this wattage (for description)
        w_variants = [v for v in variants if v["wattage"] == w]
        max_flux = max(v["flux"] for v in w_variants)
        lightsource_elements.append(f"""      <FixedLightSource id="lightsource_{w}w">
        <Name>
          <Locale language="en">LED Module {w}W 5700K</Locale>
          <Locale language="de">LED Modul {w}W 5700K</Locale>
        </Name>
        <Description>
          <Locale language="en">High-power LED module, 5700K daylight, CRI 70, up to {max_flux:,} lm initial flux</Locale>
          <Locale language="de">Hochleistungs-LED-Modul, 5700K Tageslicht, CRI 70, bis {max_flux:,} lm Anfangslichtstrom</Locale>
        </Description>
        <Manufacturer>LEDVANCE</Manufacturer>
        <RatedInputPower>{w}</RatedInputPower>
        <RatedInputVoltage>
          <FixedVoltage>180</FixedVoltage>
          <Type>DC</Type>
        </RatedInputVoltage>
        <ActivePowerTable type="Steps">
          <FluxFactor inputPower="0.5" description="Dimmed to 50% by wiring">0.5</FluxFactor>
          <FluxFactor inputPower="1.0" description="Full power">1.0</FluxFactor>
        </ActivePowerTable>
        <ColorInformation>
          <ColorRenderingIndex>70</ColorRenderingIndex>
          <CorrelatedColorTemperature>5700</CorrelatedColorTemperature>
        </ColorInformation>
        <LightSourceMaintenance lifetime="100000">
          <LedMaintenanceFactor hours="50000">0.9</LedMaintenanceFactor>
        </LightSourceMaintenance>
      </FixedLightSource>""")
    lightsources_xml = "\n".join(lightsource_elements)

    # --- ControlGears section ---
    # Two PSUs: one shared for 1200W/900W, one for 600W
    controlgears_xml = """      <ControlGear id="psu_1200w_900w">
        <Name>
          <Locale language="en">FLOODLIGHT MAX POWER SUPPLY 1200W/900W WAL</Locale>
          <Locale language="de">FLOODLIGHT MAX NETZTEIL 1200W/900W WAL</Locale>
        </Name>
        <Description>
          <Locale language="en">External wall-mount power supply unit for FL MAX 1200W and 900W luminaire heads. GTIN 4058075580749. AC 220-240V 50/60Hz input, 180V DC output. Max 40m cable distance. M20/M25 cable glands, quick-lock connector. Surge protection up to 10 kV (L/N-PE and L-N). Ordered separately.</Locale>
          <Locale language="de">Externes Wandmontage-Netzteil fuer FL MAX 1200W und 900W Leuchtenkoepfe. GTIN 4058075580749. AC 220-240V 50/60Hz Eingang, 180V DC Ausgang. Max 40m Kabelabstand. M20/M25 Kabelverschraubungen, Schnellverriegelungsstecker. Ueberspannungsschutz bis 10 kV (L/N-PE und L-N). Separat zu bestellen.</Locale>
        </Description>
        <NominalVoltage>
          <VoltageRange>
            <Min>220</Min>
            <Max>240</Max>
          </VoltageRange>
          <Type>AC</Type>
          <Frequency>50/60</Frequency>
        </NominalVoltage>
        <StandbyPower>0.5</StandbyPower>
        <Dimmable>true</Dimmable>
        <ColorControllable>false</ColorControllable>
      </ControlGear>
      <ControlGear id="psu_600w">
        <Name>
          <Locale language="en">FLOODLIGHT MAX POWER SUPPLY 600W WAL</Locale>
          <Locale language="de">FLOODLIGHT MAX NETZTEIL 600W WAL</Locale>
        </Name>
        <Description>
          <Locale language="en">External wall-mount power supply unit for FL MAX 600W luminaire heads. GTIN 4058075580756. AC 220-240V 50/60Hz input, 180V DC output. Max 40m cable distance. M20/M25 cable glands, quick-lock connector. Surge protection up to 10 kV (L/N-PE and L-N). Ordered separately.</Locale>
          <Locale language="de">Externes Wandmontage-Netzteil fuer FL MAX 600W Leuchtenkoepfe. GTIN 4058075580756. AC 220-240V 50/60Hz Eingang, 180V DC Ausgang. Max 40m Kabelabstand. M20/M25 Kabelverschraubungen, Schnellverriegelungsstecker. Ueberspannungsschutz bis 10 kV (L/N-PE und L-N). Separat zu bestellen.</Locale>
        </Description>
        <NominalVoltage>
          <VoltageRange>
            <Min>220</Min>
            <Max>240</Max>
          </VoltageRange>
          <Type>AC</Type>
          <Frequency>50/60</Frequency>
        </NominalVoltage>
        <StandbyPower>0.5</StandbyPower>
        <Dimmable>true</Dimmable>
        <ColorControllable>false</ColorControllable>
      </ControlGear>"""

    # --- Emitters section ---
    emitter_elements = []
    for v in variants:
        eid = f'emitter_{v["wattage"]}w_{v["beam_id"]}'
        pid = f'photometry_{v["wattage"]}w_{v["beam_id"]}'
        lsid = f'lightsource_{v["wattage"]}w'
        psu_id = WATTAGE_SPECS[v["wattage"]]["psu_id"]
        emitter_elements.append(f"""      <Emitter id="{eid}">
        <FixedLightEmitter>
          <Name>
            <Locale language="en">Main LED Array {v["beam_display"]}</Locale>
            <Locale language="de">Haupt-LED-Array {v["beam_display"]}</Locale>
          </Name>
          <PhotometryReference photometryId="{pid}" />
          <LightSourceReference fixedLightSourceId="{lsid}" />
          <ControlGearReference controlGearId="{psu_id}" />
          <RatedLuminousFlux>{v["flux"]}</RatedLuminousFlux>
        </FixedLightEmitter>
      </Emitter>""")
    emitters_xml = "\n".join(emitter_elements)

    # --- Geometries section ---
    geometry_elements = []
    for w in wattages_present:
        geometry_elements.append(f"""      <ModelGeometry id="geometry_{w}w">
        <GeometryFileReference fileId="geometry_file_{w}w" levelOfDetail="High" />
      </ModelGeometry>""")
    geometries_xml = "\n".join(geometry_elements)

    # --- Variant elements ---
    variant_elements = []
    for v in variants:
        vid = f'variant_{v["wattage"]}w_{v["beam_id"]}'
        eid = f'emitter_{v["wattage"]}w_{v["beam_id"]}'
        gid = f'geometry_{v["wattage"]}w'
        w = v["wattage"]
        specs = WATTAGE_SPECS[w]
        flux_klm = v["flux"] / 1000
        ld = light_distribution(v["beam_raw"])
        variant_elements.append(f"""      <Variant id="{vid}">
        <ProductNumber>
          <Locale language="en">{v["gtin"]}</Locale>
        </ProductNumber>
        <Name>
          <Locale language="en">FL MAX {w}W 757 {v["beam_display"]} WAL</Locale>
          <Locale language="de">FL MAX {w}W 757 {v["beam_display"]} WAL</Locale>
        </Name>
        <Description>
          <Locale language="en">{w}W luminaire head, 5700K, {v["beam_display"]} beam, {v["flux"]:,} lm, {specs["efficacy"]} lm/W. Wall/mast mounting.</Locale>
          <Locale language="de">{w}W Leuchtenkopf, 5700K, {v["beam_display"]} Lichtverteilung, {v["flux"]:,} lm, {specs["efficacy"]} lm/W. Wand-/Mastmontage.</Locale>
        </Description>
        <GTIN>{v["gtin"]}</GTIN>
        <Mountings>
          <Wall mountingHeight="20000">
            <SurfaceMounted />
          </Wall>
          <Ground>
            <PoleTop />
          </Ground>
        </Mountings>
        <Geometry>
          <ModelGeometryReference geometryId="{gid}">
            <EmitterReference emitterId="{eid}">
              <EmitterObjectExternalName>leo</EmitterObjectExternalName>
            </EmitterReference>
          </ModelGeometryReference>
        </Geometry>
      </Variant>""")
    variants_xml = "\n".join(variant_elements)

    # --- Assemble full product.xml ---
    # Use 1200W/900W dimensions for product-level (most common), note 600W differs
    product_xml = f"""<?xml version="1.0" encoding="UTF-8"?>
<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
      xsi:noNamespaceSchemaLocation="https://gldf.io/xsd/gldf/1.0.0-rc.3/gldf.xsd">

  <Header>
    <Manufacturer>LEDVANCE</Manufacturer>
    <FormatVersion major="1" minor="0" pre-release="3" />
    <CreatedWithApplication>gldf-rs</CreatedWithApplication>
    <GldfCreationTimeCode>{now}</GldfCreationTimeCode>
    <UniqueGldfId>LEDVANCE-FLOODLIGHT-MAX-COMBINED-001</UniqueGldfId>
    <DefaultLanguage>en</DefaultLanguage>
    <Author>Trahe Consult</Author>
  </Header>

  <GeneralDefinitions>

    <Files>
{files_xml}
    </Files>

    <Photometries>
{photometries_xml}
    </Photometries>

    <LightSources>
{lightsources_xml}
    </LightSources>

    <ControlGears>
{controlgears_xml}
    </ControlGears>

    <Emitters>
{emitters_xml}
    </Emitters>

    <Geometries>
{geometries_xml}
    </Geometries>

  </GeneralDefinitions>

  <ProductDefinitions>
    <ProductMetaData>
      <UniqueProductId>LEDVANCE-FLOODLIGHT-MAX</UniqueProductId>
      <ProductNumber>
        <Locale language="en">FLOODLIGHT-MAX</Locale>
      </ProductNumber>
      <Name>
        <Locale language="en">FLOODLIGHT MAX Luminaire Head Series</Locale>
        <Locale language="de">FLOODLIGHT MAX Leuchtenkopf-Serie</Locale>
      </Name>
      <Description>
        <Locale language="en">High-power floodlight for large area and sports lighting, up to 164 klm. Modular high-power floodlight with up to 135 lm/W. Joint or separate installation with Floodlight Max power supply up to 40m distance. Available in 600W, 900W, and 1200W wattages with SYM 10, SYM 30, SYM 60, and ASYM 50x110 beam angles. Mounting bracket with up to 180 degree tilting. Highly transparent, tempered glass diffuser. Energy savings up to 45% vs conventional discharge lamps. Dimming to 50% by wiring. ULOR 0% at 0 degree tilt. Low flicker light. 5 year guarantee.</Locale>
        <Locale language="de">Hochleistungsscheinwerfer fuer die Grossflaechen- und Sportstaettenbeleuchtung, bis 164 klm. Modularer Hochleistungsscheinwerfer mit bis zu 135 lm/W. Gemeinsame oder separate Installation mit Floodlight Max Netzteil mit bis zu 40 m Abstand. Erhaeltlich in 600W, 900W und 1200W mit SYM 10, SYM 30, SYM 60 und ASYM 50x110 Lichtverteilungen. Montagebuegel mit Rotationsbereich von bis zu 180 Grad. Abdeckscheibe aus hochtransparentem, gehaertetem Glas. Bis zu 45% Energieersparnis. Dimmen auf 50% mittels Verdrahtung. ULOR 0% bei 0 Grad Aufneigung. Flimmerarmes Licht. 5 Jahre Garantie.</Locale>
      </Description>
      <TenderText>
        <Locale language="en">LEDVANCE FLOODLIGHT MAX luminaire head series, 600W/900W/1200W, 5700K, CRI70, multiple beam angles (SYM 10/30/60, ASYM 50x110), up to 164,000lm, up to 135 lm/W, IP66, IK08, aluminium grey housing, tempered glass. External PSU required (220-240V AC 50/60Hz, surge protection 10kV). Max PSU distance 40m.</Locale>
      </TenderText>
      <ProductSeries>
        <ProductSerie id="FL-MAX-Series">
          <Name>
            <Locale language="en">FLOODLIGHT MAX</Locale>
            <Locale language="de">FLOODLIGHT MAX</Locale>
          </Name>
          <Description>
            <Locale language="en">High-power modular floodlight series for large area and sports lighting (600W/900W/1200W)</Locale>
            <Locale language="de">Modulare Hochleistungsscheinwerfer-Serie fuer Grossflaechen- und Sportstaettenbeleuchtung (600W/900W/1200W)</Locale>
          </Description>
          <Hyperlinks>
            <Hyperlink href="https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights" language="en">LEDVANCE Floodlight Product Range</Hyperlink>
          </Hyperlinks>
        </ProductSerie>
      </ProductSeries>
      <LuminaireMaintenance>
        <Cie97LuminaireType>Dust Proof IP5X</Cie97LuminaireType>
      </LuminaireMaintenance>
      <DescriptiveAttributes>
        <Mechanical>
          <ProductSize>
            <Length>841</Length>
            <Width>591</Width>
            <Height>282</Height>
          </ProductSize>
          <ProductForm>Square</ProductForm>
          <SealingMaterial>
            <Locale language="en">Silicone gasket</Locale>
          </SealingMaterial>
          <Adjustabilities>
            <Adjustability>Tilt</Adjustability>
          </Adjustabilities>
          <IKRating>IK08</IKRating>
          <Weight>31.9</Weight>
        </Mechanical>
        <Electrical>
          <ElectricalSafetyClass>I</ElectricalSafetyClass>
          <IngressProtectionIPCode>IP66</IngressProtectionIPCode>
          <PowerFactor>0.95</PowerFactor>
          <ConstantLightOutput>false</ConstantLightOutput>
          <LightDistribution>Laterally symmetrical narrow</LightDistribution>
        </Electrical>
        <Marketing>
          <HousingColors>
            <HousingColor>
              <Locale language="en">Grey aluminium</Locale>
              <Locale language="de">Grau Aluminium</Locale>
            </HousingColor>
          </HousingColors>
          <Hyperlinks>
            <Hyperlink href="https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights" language="en">LEDVANCE Floodlight Product Range</Hyperlink>
          </Hyperlinks>
          <Applications>
            <Application>Exterior: Sports Fields</Application>
            <Application>Exterior: Sports Fields: Spotlightings</Application>
            <Application>Exterior: General Areas (Exterior)</Application>
            <Application>Interior: Educational Facilities: Sports Halls</Application>
          </Applications>
        </Marketing>
        <OperationsAndMaintenance>
          <UsefulLifeTimes>
            <UsefulLife>L80B10 75000h 25°C</UsefulLife>
            <UsefulLife>L90B10 50000h 25°C</UsefulLife>
          </UsefulLifeTimes>
          <MedianUsefulLifeTimes>
            <MedianUsefulLife>L70B50 100000h 25°C</MedianUsefulLife>
          </MedianUsefulLifeTimes>
          <OperatingTemperature>
            <Lower>-30</Lower>
            <Upper>50</Upper>
          </OperatingTemperature>
          <RatedAmbientTemperature>50</RatedAmbientTemperature>
        </OperationsAndMaintenance>
        <CustomProperties>
          <Property id="sdcm">
            <Name><Locale language="en">SDCM</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>5</Value>
          </Property>
          <Property id="switching_cycles">
            <Name><Locale language="en">Switching Cycles</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>100000</Value>
          </Property>
          <Property id="diffuser_material">
            <Name><Locale language="en">Diffuser Material</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>Tempered glass, highly transparent</Value>
          </Property>
          <Property id="housing_material">
            <Name><Locale language="en">Housing Material</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>Aluminium</Value>
          </Property>
          <Property id="cable_length">
            <Name><Locale language="en">Cable Length (mm)</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>1300</Value>
          </Property>
          <Property id="psu_input">
            <Name><Locale language="en">PSU Input Voltage</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>220-240V AC 50/60Hz</Value>
          </Property>
          <Property id="psu_output">
            <Name><Locale language="en">PSU Output Voltage</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>180V DC</Value>
          </Property>
          <Property id="psu_max_distance">
            <Name><Locale language="en">Max PSU Distance (m)</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>40</Value>
          </Property>
          <Property id="psu_cable_glands">
            <Name><Locale language="en">PSU Cable Glands</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>M20 and M25 PG screw connections</Value>
          </Property>
          <Property id="surge_protection">
            <Name><Locale language="en">Surge Protection (kV)</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>10 (L-N and L/N-PE)</Value>
          </Property>
          <Property id="inrush_current">
            <Name><Locale language="en">Inrush Current</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>45A / 280us (via PSU)</Value>
          </Property>
          <Property id="thd">
            <Name><Locale language="en">THD (Total Harmonic Distortion)</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>10%</Value>
          </Property>
          <Property id="dimming">
            <Name><Locale language="en">Dimming Capability</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>50% by wiring</Value>
          </Property>
          <Property id="photobiological_safety">
            <Name><Locale language="en">Photobiological Safety (EN 62778)</Locale></Name>
            <PropertySource>EN 62778</PropertySource>
            <Value>RG2</Value>
          </Property>
          <Property id="min_staring_distance">
            <Name><Locale language="en">Min Safe Staring Distance (m)</Locale></Name>
            <PropertySource>LEDVANCE Installation Manual</PropertySource>
            <Value>4</Value>
          </Property>
          <Property id="glow_wire">
            <Name><Locale language="en">Glow Wire Test (IEC 60695-2-12)</Locale></Name>
            <PropertySource>IEC 60695-2-12</PropertySource>
            <Value>650</Value>
          </Property>
          <Property id="certifications">
            <Name><Locale language="en">Certifications</Locale></Name>
            <PropertySource>LEDVANCE</PropertySource>
            <Value>CE, CB, ENEC, EAC</Value>
          </Property>
          <Property id="epd">
            <Name><Locale language="en">Environmental Product Declaration</Locale></Name>
            <PropertySource>LEDVANCE</PropertySource>
            <Value>LEDV-00007-V01.01-EN</Value>
          </Property>
          <Property id="storage_temperature">
            <Name><Locale language="en">Storage Temperature Range</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>-40 to +70 degrees C</Value>
          </Property>
          <Property id="warranty">
            <Name><Locale language="en">Warranty (years)</Locale></Name>
            <PropertySource>LEDVANCE</PropertySource>
            <Value>5</Value>
          </Property>
          <Property id="mounting_tilt">
            <Name><Locale language="en">Mounting Bracket Tilt Range</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>180 degrees</Value>
          </Property>
          <Property id="ulor">
            <Name><Locale language="en">ULOR at 0 degree tilt</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>0%</Value>
          </Property>
          <Property id="connection">
            <Name><Locale language="en">Connection Type</Locale></Name>
            <PropertySource>LEDVANCE Datasheet</PropertySource>
            <Value>Terminal, 3-pole (L, N, PE) via quick-lock connector to PSU</Value>
          </Property>
          <Property id="country_of_origin">
            <Name><Locale language="en">Country of Origin</Locale></Name>
            <PropertySource>RS Components</PropertySource>
            <Value>CN</Value>
          </Property>
          <Property id="secondary_fixation">
            <Name><Locale language="en">Secondary Fixation Required</Locale></Name>
            <PropertySource>LEDVANCE Installation Manual</PropertySource>
            <Value>Yes, if single fixation point and height over 3m (restraining wire)</Value>
          </Property>
        </CustomProperties>
      </DescriptiveAttributes>
    </ProductMetaData>

    <Variants>
{variants_xml}
    </Variants>

  </ProductDefinitions>
</Root>"""

    return product_xml


def patch_l3d_with_leo(l3d_path, wattage):
    """Patch an L3D file to add LightEmittingObjects to structure.xml.

    The STEP-converted L3D files have valid housing geometry but no
    LightEmittingObjects defined. This adds a rectangular LEO named 'leo'
    (matching the GLDF EmitterObjectExternalName) at the glass aperture face.

    The LEO is placed at the center of the housing with the light direction
    pointing along -Y (the glass face normal in the STEP model coordinate system).
    L3D uses millimeters for position but meters for LEO rectangle dimensions.

    Returns the patched L3D as bytes.
    """
    specs = WATTAGE_SPECS[wattage]
    # Aperture rectangle: ~80% of housing face, in meters (L3D LEO uses meters)
    aperture_w = specs["width"] * 0.80 / 1000.0   # X dimension
    aperture_l = specs["length"] * 0.80 / 1000.0   # Z dimension

    # Read original L3D
    with zipfile.ZipFile(l3d_path, "r") as zin:
        entries = {}
        for name in zin.namelist():
            entries[name] = zin.read(name)

    # Patch structure.xml: add LightEmittingObjects inside <Geometry>
    structure_xml = entries["structure.xml"].decode("utf-8")

    # Find the OBJ bounding box to position the LEO at the glass face
    obj_data = entries.get("geom_1/housing.obj", b"").decode("latin-1")
    ys = []
    zs = []
    for line in obj_data.split("\n"):
        if line.startswith("v ") and not line.startswith("vn"):
            parts = line.split()
            ys.append(float(parts[2]))
            zs.append(float(parts[3]))

    # Position LEO at the glass face (minimum Y = front face of housing)
    # X=0 (centered), Y=min_y (glass face), Z=center of housing
    leo_y = min(ys) if ys else 0.0
    leo_z = (min(zs) + max(zs)) / 2 if zs else 0.0

    # Build the LightEmittingObjects XML block
    # Position is in mm (same units as geometry), rectangle size is in meters
    leo_xml = f"""      <LightEmittingObjects>
        <LightEmittingObject partName="leo">
          <Position x="0" y="{leo_y:.1f}" z="{leo_z:.1f}"/>
          <Rotation x="-90" y="0" z="0"/>
          <Rectangle sizeX="{aperture_w:.4f}" sizeY="{aperture_l:.4f}"/>
        </LightEmittingObject>
      </LightEmittingObjects>"""

    # Insert before </Geometry> closing tag
    structure_xml = structure_xml.replace(
        "    </Geometry>",
        leo_xml + "\n    </Geometry>"
    )

    entries["structure.xml"] = structure_xml.encode("utf-8")

    # Repackage L3D as ZIP
    import io
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_STORED) as zout:
        for name, data in entries.items():
            zout.writestr(name, data)
    return buf.getvalue()


def build_combined_gldf(variants):
    """Package combined GLDF as ZIP with stored compression."""
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    output_path = os.path.join(OUTPUT_DIR, "LEDVANCE_FLOODLIGHT_MAX.gldf")

    product_xml = generate_product_xml(variants)

    wattages_present = sorted(set(v["wattage"] for v in variants), reverse=True)

    with zipfile.ZipFile(output_path, "w", zipfile.ZIP_STORED) as zf:
        # product.xml
        zf.writestr("product.xml", product_xml)

        # Geometry files (one per wattage, patched with LightEmittingObjects)
        for w in wattages_present:
            l3d_path = os.path.join(GEOMETRY_DIR, f"FL_MAX_{w}W.l3d")
            if os.path.exists(l3d_path):
                patched_l3d = patch_l3d_with_leo(l3d_path, w)
                zf.writestr(f"geometry/model_{w}w.l3d", patched_l3d)
                print(f"  Patched L3D for {w}W with LightEmittingObject 'leo'")
            else:
                print(f"  WARNING: L3D not found: {l3d_path}")

        # LDT files (one per variant)
        for v in variants:
            zf.writestr(f"ldc/{v['ldt_filename']}", v["ldt_data"])

    return output_path, product_xml


def main():
    print("=== GLDF Combiner: LEDVANCE FLOODLIGHT MAX ===\n")

    print("Scanning canonical GLDF files...")
    variants = scan_canonical_gldfs()
    print(f"\nFound {len(variants)} variants:\n")

    for v in variants:
        print(f"  {v['wattage']}W {v['beam_display']:>15s}  "
              f"GTIN {v['gtin']}  {v['flux']:>7,} lm")

    if len(variants) != 11:
        print(f"\nWARNING: Expected 11 variants, found {len(variants)}")

    print(f"\nBuilding combined GLDF...")
    output_path, product_xml = build_combined_gldf(variants)
    print(f"  Output: {output_path}")

    # Verify
    with zipfile.ZipFile(output_path) as zf:
        names = zf.namelist()
        print(f"\nZIP contents ({len(names)} entries):")
        for n in sorted(names):
            info = zf.getinfo(n)
            print(f"  {n:60s} {info.file_size:>10,} bytes")

    print(f"\nDone. Combined GLDF with {len(variants)} variants written to:")
    print(f"  {output_path}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
