#!/usr/bin/env python3
"""
Astral Sky Demo - Generate star photometry data for 3D sky visualization

This script:
1. Downloads bright star catalog (Hipparcos/Yale BSC)
2. Extracts spectral data (temperature, color, magnitude)
3. Generates synthetic LDT/IES files based on stellar spectra
4. Calculates Alt/Az positions for a given location and time
5. Packages everything into a GLDF file for the viewer

Tribute to Astrophysics!
"""

import json
import math
import os
import sys
import zipfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

# Try to import astronomy libraries
try:
    from astropy import units as u
    from astropy.coordinates import EarthLocation, AltAz, SkyCoord
    from astropy.time import Time
    HAVE_ASTROPY = True
except ImportError:
    HAVE_ASTROPY = False
    print("Note: astropy not installed. Install with: pip install astropy")
    print("      Will use simplified calculations instead.")

try:
    import requests
    HAVE_REQUESTS = True
except ImportError:
    HAVE_REQUESTS = False
    print("Note: requests not installed. Install with: pip install requests")


@dataclass
class Star:
    """Star data from catalog"""
    name: str
    hip_id: Optional[int]  # Hipparcos ID
    ra: float  # Right Ascension in degrees
    dec: float  # Declination in degrees
    mag: float  # Apparent magnitude
    spectral_type: str  # e.g., "G2V", "A0", "M5III"
    color_index: Optional[float]  # B-V color index

    @property
    def temperature(self) -> int:
        """Estimate temperature from spectral type or color index"""
        if self.color_index is not None:
            # B-V to temperature approximation
            bv = self.color_index
            if bv < -0.3:
                return 30000
            elif bv < 0.0:
                return int(10000 - bv * 10000)
            elif bv < 0.6:
                return int(7500 - bv * 5000)
            elif bv < 1.5:
                return int(6000 - (bv - 0.6) * 2500)
            else:
                return 3000

        # Fallback: estimate from spectral type
        spec = self.spectral_type.upper() if self.spectral_type else "G2"
        if spec.startswith("O"):
            return 35000
        elif spec.startswith("B"):
            return 20000
        elif spec.startswith("A"):
            return 9000
        elif spec.startswith("F"):
            return 7000
        elif spec.startswith("G"):
            return 5500
        elif spec.startswith("K"):
            return 4500
        elif spec.startswith("M"):
            return 3200
        return 5500  # Default to Sun-like


def load_hyg_catalogue(mag_limit: float = 6.5) -> list[Star]:
    """
    Load stars from HYG v4.2 catalogue CSV file.

    Args:
        mag_limit: Maximum apparent magnitude (default 6.5 = naked eye limit)

    Returns:
        List of Star objects
    """
    import csv
    import gzip

    script_dir = Path(__file__).parent.parent
    csv_path = script_dir / "data" / "hygdata_v42.csv"
    gz_path = script_dir / "data" / "hygdata_v42.csv.gz"

    stars = []

    # Try CSV first, then gzipped
    if csv_path.exists():
        f = open(csv_path, 'r', encoding='utf-8')
    elif gz_path.exists():
        f = gzip.open(gz_path, 'rt', encoding='utf-8')
    else:
        print(f"Warning: HYG catalogue not found at {csv_path}")
        print("Download from: https://www.astronexus.com/downloads/catalogs/hygdata_v42.csv.gz")
        return get_fallback_stars()

    try:
        reader = csv.DictReader(f)
        for row in reader:
            try:
                mag = float(row['mag'])
                if mag > mag_limit:
                    continue

                # Parse B-V color index
                ci = row.get('ci', '')
                color_index = float(ci) if ci else None

                # Parse Hipparcos ID
                hip = row.get('hip', '')
                hip_id = int(hip) if hip else None

                stars.append(Star(
                    name=row.get('proper', '') or f"HIP {hip_id}" if hip_id else f"HYG {row['id']}",
                    hip_id=hip_id,
                    ra=float(row['ra']) * 15.0,  # Convert hours to degrees
                    dec=float(row['dec']),
                    mag=mag,
                    spectral_type=row.get('spect', ''),
                    color_index=color_index
                ))
            except (ValueError, KeyError) as e:
                continue  # Skip malformed rows
    finally:
        f.close()

    print(f"Loaded {len(stars)} stars from HYG catalogue (mag < {mag_limit})")
    return stars


def get_fallback_stars() -> list[Star]:
    """Fallback: hardcoded bright stars if HYG not available"""
    BRIGHT_STARS = [
        ("Sirius", 101.287, -16.716, -1.46, "A1V", 0.00),
        ("Canopus", 95.988, -52.696, -0.74, "F0Ib", 0.15),
        ("Arcturus", 213.915, 19.182, -0.05, "K1.5III", 1.23),
        ("Vega", 279.235, 38.784, 0.03, "A0V", 0.00),
        ("Capella", 79.172, 45.998, 0.08, "G5III", 0.80),
        ("Rigel", 78.634, -8.202, 0.13, "B8Ia", -0.03),
        ("Procyon", 114.825, 5.225, 0.34, "F5IV-V", 0.42),
        ("Betelgeuse", 88.793, 7.407, 0.42, "M1Ia", 1.85),
        ("Altair", 297.696, 8.868, 0.77, "A7V", 0.22),
        ("Aldebaran", 68.980, 16.509, 0.85, "K5III", 1.54),
        ("Antares", 247.352, -26.432, 0.96, "M1Ib", 1.83),
        ("Spica", 201.298, -11.161, 0.97, "B1V", -0.23),
        ("Pollux", 116.329, 28.026, 1.14, "K0III", 1.00),
        ("Deneb", 310.358, 45.280, 1.25, "A2Ia", 0.09),
        ("Polaris", 37.954, 89.264, 2.02, "F7Ib", 0.60),
    ]
    stars = []
    for name, ra, dec, mag, spec, bv in BRIGHT_STARS:
        stars.append(Star(name=name, hip_id=None, ra=ra, dec=dec, mag=mag,
                         spectral_type=spec, color_index=bv))
    return stars


def get_stars(mag_limit: float = 6.5) -> list[Star]:
    """Get list of stars (from HYG catalogue or fallback)"""
    return load_hyg_catalogue(mag_limit)


def star_to_id(star: Star) -> str:
    """Convert star name to valid XML ID (alphanumeric + underscore, starts with letter)"""
    import re
    # Use proper name or HIP ID
    name = star.name.lower().strip()
    # Replace spaces with underscores, remove non-alphanumeric except underscore
    safe_name = re.sub(r'[^a-z0-9_]', '', name.replace(' ', '_').replace('-', '_'))
    # Ensure starts with letter (prefix with 'star_' if it starts with number)
    if safe_name and safe_name[0].isdigit():
        safe_name = f"star_{safe_name}"
    return safe_name or f"star_{star.hip_id or id(star)}"


def generate_star_spectrum(temperature: int) -> list[tuple[int, float]]:
    """
    Generate a blackbody-like spectrum for a star.
    Returns list of (wavelength_nm, relative_intensity) pairs.
    """
    # Planck's law approximation for relative intensity
    h = 6.626e-34  # Planck constant
    c = 3e8  # Speed of light
    k = 1.381e-23  # Boltzmann constant

    spectrum = []
    for wl_nm in range(380, 781, 5):
        wl_m = wl_nm * 1e-9
        # Planck function (simplified, normalized)
        try:
            exponent = (h * c) / (wl_m * k * temperature)
            if exponent > 700:  # Prevent overflow
                intensity = 0.0
            else:
                intensity = (1.0 / (wl_m ** 5)) / (math.exp(exponent) - 1)
        except (OverflowError, ZeroDivisionError):
            intensity = 0.0
        spectrum.append((wl_nm, intensity))

    # Normalize to max = 1.0
    max_intensity = max(i for _, i in spectrum) if spectrum else 1.0
    if max_intensity > 0:
        spectrum = [(wl, i / max_intensity) for wl, i in spectrum]

    return spectrum


def generate_tm33_for_star(star: Star) -> str:
    """Generate TM-33-23 IESXML content for a star"""
    temp = star.temperature
    spectrum = generate_star_spectrum(temp)

    wavelengths = " ".join(str(wl) for wl, _ in spectrum)
    values = " ".join(f"{v:.4f}" for _, v in spectrum)

    # For stars, we use a simple point source distribution
    # All intensity in nadir direction (gamma=0)
    intensity_data = ""
    for gamma in range(0, 91, 5):
        # Point source: all light at gamma=0
        intensity = 1000.0 if gamma == 0 else 0.0
        intensity_data += f"            <IntensityData horz=\"0\" vert=\"{gamma}\">{intensity:.1f}</IntensityData>\n"

    return f'''<?xml version="1.0" encoding="UTF-8"?>
<!--
  Stellar Photometry - {star.name}
  Spectral Type: {star.spectral_type}
  Temperature: {temp}K
  Magnitude: {star.mag}

  Generated for Astral Sky Demo - Tribute to Astrophysics
-->
<IESTM33-22>
    <Version>1.1</Version>
    <Header>
        <Manufacturer>Astral Sky Demo</Manufacturer>
        <CatalogNumber>{star.name.replace(" ", "_")}</CatalogNumber>
        <Description>Star: {star.name} ({star.spectral_type})</Description>
        <Laboratory>Stellar Photometry Generator</Laboratory>
        <ReportNumber>STAR-{star.name.replace(" ", "-")}</ReportNumber>
        <ReportDate>{datetime.now().strftime("%Y-%m-%d")}</ReportDate>
        <DocumentCreator>astral_sky_demo.py</DocumentCreator>
        <DocumentCreationDate>{datetime.now().strftime("%Y-%m-%d")}</DocumentCreationDate>
    </Header>
    <Luminaire>
        <Dimensions>
            <Length>0</Length>
            <Width>0</Width>
            <Height>0</Height>
        </Dimensions>
        <Mounting>Celestial</Mounting>
        <NumEmitters>1</NumEmitters>
    </Luminaire>
    <Emitter>
        <ID>star-{star.name.lower().replace(" ", "-")}</ID>
        <Description>{star.name} - {star.spectral_type} star</Description>
        <CatalogNumber>{star.name}</CatalogNumber>
        <Quantity>1</Quantity>
        <RatedLumens>{int(10 ** ((0 - star.mag) / 2.5) * 1000)}</RatedLumens>
        <InputWattage>0</InputWattage>
        <FixedCCT>{temp}</FixedCCT>
        <Duv>0.000</Duv>
        <ColorRendering>
            <Ra>100</Ra>
        </ColorRendering>
        <LuminousData>
            <PhotometryType>CIE _ C</PhotometryType>
            <Metric>Luminous</Metric>
            <SymmType>Symm _ Full</SymmType>
            <Multiplier>1.0</Multiplier>
{intensity_data}        </LuminousData>
        <SpectralDistribution>
            <Wavelengths>{wavelengths}</Wavelengths>
            <Values>{values}</Values>
        </SpectralDistribution>
    </Emitter>
</IESTM33-22>
'''


def calculate_alt_az(star: Star, lat: float, lng: float, time: datetime) -> tuple[float, float]:
    """
    Calculate altitude and azimuth for a star at given location and time.

    Returns (altitude, azimuth) in degrees.
    """
    if HAVE_ASTROPY:
        # Use astropy for accurate calculation
        location = EarthLocation(lat=lat * u.deg, lon=lng * u.deg)
        obs_time = Time(time)
        coord = SkyCoord(ra=star.ra * u.deg, dec=star.dec * u.deg)
        altaz = coord.transform_to(AltAz(obstime=obs_time, location=location))
        return float(altaz.alt.deg), float(altaz.az.deg)
    else:
        # Simplified calculation (less accurate)
        # Convert to radians
        ra_rad = math.radians(star.ra)
        dec_rad = math.radians(star.dec)
        lat_rad = math.radians(lat)

        # Calculate Local Sidereal Time (simplified)
        # This is a rough approximation!
        j2000 = datetime(2000, 1, 1, 12, 0, 0, tzinfo=timezone.utc)
        days_since_j2000 = (time - j2000).total_seconds() / 86400.0
        lst_deg = (280.46 + 360.9856474 * days_since_j2000 + lng) % 360
        lst_rad = math.radians(lst_deg)

        # Hour angle
        ha_rad = lst_rad - ra_rad

        # Altitude
        sin_alt = (math.sin(dec_rad) * math.sin(lat_rad) +
                   math.cos(dec_rad) * math.cos(lat_rad) * math.cos(ha_rad))
        alt = math.degrees(math.asin(max(-1, min(1, sin_alt))))

        # Azimuth
        cos_az = ((math.sin(dec_rad) - math.sin(lat_rad) * sin_alt) /
                  (math.cos(lat_rad) * math.cos(math.radians(alt))))
        cos_az = max(-1, min(1, cos_az))
        az = math.degrees(math.acos(cos_az))

        if math.sin(ha_rad) > 0:
            az = 360 - az

        return alt, az


def generate_star_lisp(visible_stars: list, location_name: str, lat: float, lng: float, obs_time: datetime) -> str:
    """Generate AutoLISP code to draw all visible stars.

    This creates a complete LISP program that can be loaded in AutoCAD
    or run in the embedded AcadLISP interpreter.
    """
    # Header with documentation
    lisp = f'''; ============================================
; Star Sky Visualization - AutoLISP
; Astral Sky Demo - Tribute to Astrophysics
; ============================================
; Location: {location_name} ({lat:.2f}°N, {lng:.2f}°E)
; Time: {obs_time.strftime("%Y-%m-%d %H:%M UTC")}
; Stars: {len(visible_stars)} visible above horizon
;
; This code was auto-generated from real astronomical data.
; Run it to create a polar projection star map.

; --- Configuration ---
(setq PI 3.14159265359)
(setq DEG2RAD (/ PI 180.0))
(setq RADIUS 100.0)  ; Sky dome radius in drawing units

; --- Helper Functions ---

(defun deg-to-rad (deg)
  "Convert degrees to radians"
  (* deg DEG2RAD))

(defun polar-to-xy (az alt / r x y)
  "Convert azimuth/altitude to X/Y coordinates.
   Uses stereographic projection (polar view).
   Zenith at center, horizon at edge."
  (setq r (* RADIUS (- 1.0 (/ alt 90.0))))
  (setq x (* r (sin (deg-to-rad az))))
  (setq y (* r (cos (deg-to-rad az))))
  (list x y))

(defun mag-to-size (mag)
  "Convert magnitude to circle size.
   Brighter stars (lower mag) are larger."
  (max 0.5 (- 6.0 (* mag 1.0))))

; --- Drawing Functions ---

(defun draw-star (name az alt mag / pos size)
  "Draw a star at the given position"
  (setq pos (polar-to-xy az alt))
  (setq size (mag-to-size mag))
  (command "CIRCLE" pos size)
  ; Label only bright stars (mag < 2.0)
  (if (< mag 2.0)
    (command "TEXT"
      (list (+ (car pos) (+ size 1))
            (+ (cadr pos) 0.5))
      1.5 0 name)))

(defun draw-grid ()
  "Draw horizon circle and cardinal directions"
  ; Horizon circle
  (command "CIRCLE" (list 0 0) RADIUS)
  ; Altitude circles at 30° and 60°
  (command "CIRCLE" (list 0 0) (* RADIUS 0.667))  ; 30°
  (command "CIRCLE" (list 0 0) (* RADIUS 0.333))  ; 60°
  ; Cardinal directions
  (command "TEXT" (list -2 (+ RADIUS 3)) 3 0 "N")
  (command "TEXT" (list -2 (- (- RADIUS) 6)) 3 0 "S")
  (command "TEXT" (list (+ RADIUS 2) -1) 3 0 "E")
  (command "TEXT" (list (- (- RADIUS) 8) -1) 3 0 "W"))

(defun draw-title ()
  "Draw title and info"
  (command "TEXT" (list -95 (+ RADIUS 15)) 4 0
    "ASTRAL SKY - Polar Star Map")
  (command "TEXT" (list -95 (+ RADIUS 10)) 2.5 0
    "{location_name}")
  (command "TEXT" (list -95 (- (- RADIUS) 12)) 2 0
    "Generated with AutoLISP - Tribute to Astrophysics"))

; =============================================
; MAIN PROGRAM - Draw the night sky
; =============================================

(princ "\\n=== Astral Sky Drawing ===")
(princ "\\nDrawing sky grid...")
(draw-grid)
(draw-title)
(princ "\\nDrawing {len(visible_stars)} stars...")

; --- Star Data ---
; Each star: (draw-star "Name" Azimuth Altitude Magnitude)
'''

    # Add all visible stars sorted by magnitude (brightest first)
    sorted_stars = sorted(visible_stars, key=lambda x: x[0].mag)

    # Group stars by brightness for better organization
    bright_stars = [(s, alt, az) for s, alt, az in sorted_stars if s.mag < 2.0]
    medium_stars = [(s, alt, az) for s, alt, az in sorted_stars if 2.0 <= s.mag < 3.5]
    dim_stars = [(s, alt, az) for s, alt, az in sorted_stars if s.mag >= 3.5]

    lisp += f'''
; --- Bright Stars (mag < 2.0) - {len(bright_stars)} stars ---
'''
    for star, alt, az in bright_stars:
        name = star.name.replace('"', '\\"')
        lisp += f'(draw-star "{name}" {az:.1f} {alt:.1f} {star.mag:.2f})\n'

    lisp += f'''
; --- Medium Stars (2.0 <= mag < 3.5) - {len(medium_stars)} stars ---
'''
    for star, alt, az in medium_stars:
        name = star.name.replace('"', '\\"')
        lisp += f'(draw-star "{name}" {az:.1f} {alt:.1f} {star.mag:.2f})\n'

    lisp += f'''
; --- Dim Stars (mag >= 3.5) - {len(dim_stars)} stars ---
'''
    for star, alt, az in dim_stars:
        name = star.name.replace('"', '\\"')
        lisp += f'(draw-star "{name}" {az:.1f} {alt:.1f} {star.mag:.2f})\n'

    # Footer
    lisp += f'''
; --- Complete ---
(princ "\\n=== Drawing Complete ===")
(princ "\\n{len(visible_stars)} stars rendered")
(princ "\\nLocation: {location_name}")
(princ "\\nTribute to Astrophysics!")
(princ)
'''
    return lisp


def generate_gldf_for_sky(lat: float, lng: float, location_name: str, time: datetime, mag_limit: float = 4.0, json_mag_limit: float = 6.5) -> bytes:
    """
    Generate a GLDF file containing visible stars for a location.

    The GLDF includes:
    - TM-33 spectral files for bright stars (mag_limit)
    - Embedded sky_data.json in other/ folder with all visible stars (json_mag_limit)

    Args:
        lat: Observer latitude
        lng: Observer longitude
        location_name: Name for the location
        time: Observation time
        mag_limit: Maximum magnitude for TM-33 files (default 4.0 = ~600 brightest stars)
        json_mag_limit: Maximum magnitude for sky JSON (default 6.5 = naked eye limit)
    """
    stars = get_stars(mag_limit=mag_limit)

    # Calculate positions and filter visible stars (alt > 0)
    visible_stars = []
    for star in stars:
        alt, az = calculate_alt_az(star, lat, lng, time)
        if alt > 0:
            visible_stars.append((star, alt, az))

    print(f"Visible stars from {location_name}: {len(visible_stars)}/{len(stars)}")

    # Generate TM-33 files for each star
    tm33_files = {}
    for star, _, _ in visible_stars:
        star_id = star_to_id(star)
        filename = f"{star_id}.iesxml"
        tm33_files[star_id] = (filename, generate_tm33_for_star(star))

    # Generate product.xml
    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    # Build mapping from star to its ID for consistent references
    star_ids = [(star, star_to_id(star)) for star, _, _ in visible_stars]

    # File definitions - use star name as ID
    files_xml = ""
    for star, star_id in star_ids:
        filename, _ = tm33_files[star_id]
        files_xml += f'            <File id="file_{star_id}" contentType="ldc/iesxml" type="localFileName">{filename}</File>\n'

    # Photometry definitions
    photometries_xml = ""
    for star, star_id in star_ids:
        photometries_xml += f'''            <Photometry id="photometry_{star_id}">
                <PhotometryFileReference fileId="file_{star_id}"/>
            </Photometry>
'''

    # Light sources with spectral references
    light_sources_xml = ""
    for (star, _, _), (_, star_id) in zip(visible_stars, star_ids):
        light_sources_xml += f'''            <FixedLightSource id="lightsource_{star_id}">
                <Name><Locale language="en">{star.name}</Locale></Name>
                <Description><Locale language="en">{star.spectral_type} star, mag {star.mag}</Locale></Description>
                <RatedInputPower>0</RatedInputPower>
                <ColorInformation>
                    <CorrelatedColorTemperature>{star.temperature}</CorrelatedColorTemperature>
                </ColorInformation>
            </FixedLightSource>
'''

    # Emitters
    emitters_xml = ""
    for (star, _, _), (_, star_id) in zip(visible_stars, star_ids):
        emitters_xml += f'''            <Emitter id="emitter_{star_id}">
                <FixedLightEmitter>
                    <Name><Locale language="en">{star.name}</Locale></Name>
                    <PhotometryReference photometryId="photometry_{star_id}"/>
                    <LightSourceReference fixedLightSourceId="lightsource_{star_id}"/>
                    <RatedLuminousFlux>{min(int(10 ** ((0 - star.mag) / 2.5) * 1000), 2000000000)}</RatedLuminousFlux>
                </FixedLightEmitter>
            </Emitter>
'''

    # Variants with position data (as custom properties)
    variants_xml = ""
    for (star, alt, az), (_, star_id) in zip(visible_stars, star_ids):
        variants_xml += f'''        <Variant id="variant_{star_id}">
            <Name><Locale language="en">{star.name}</Locale></Name>
            <Description><Locale language="en">Alt: {alt:.1f}, Az: {az:.1f}, Mag: {star.mag}</Locale></Description>
            <DescriptiveAttributes>
                <CustomProperties>
                    <Property id="altitude">{alt:.2f}</Property>
                    <Property id="azimuth">{az:.2f}</Property>
                    <Property id="magnitude">{star.mag}</Property>
                    <Property id="ra">{star.ra:.4f}</Property>
                    <Property id="dec">{star.dec:.4f}</Property>
                    <Property id="spectral_type">{star.spectral_type}</Property>
                </CustomProperties>
            </DescriptiveAttributes>
        </Variant>
'''

    product_xml = f'''<?xml version="1.0" encoding="UTF-8"?>
<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="https://gldf.io/xsd/gldf/1.0.0/gldf.xsd">
    <Header>
        <Author>holg</Author>
        <Manufacturer>Astral Sky Demo</Manufacturer>
        <FormatVersion major="1" minor="0" pre-release="3"/>
        <CreatedWithApplication>astral_sky_demo.py</CreatedWithApplication>
        <GldfCreationTimeCode>{timestamp}</GldfCreationTimeCode>
        <UniqueGldfId>astral-sky-{location_name.lower().replace(" ", "-")}</UniqueGldfId>
    </Header>
    <GeneralDefinitions>
        <Files>
{files_xml}        </Files>
        <Photometries>
{photometries_xml}        </Photometries>
        <LightSources>
{light_sources_xml}        </LightSources>
        <Emitters>
{emitters_xml}        </Emitters>
    </GeneralDefinitions>
    <ProductDefinitions>
        <ProductMetaData>
            <UniqueProductId>astral-sky-{location_name.lower().replace(" ", "-")}</UniqueProductId>
            <ProductNumber><Locale language="en">Astral Sky</Locale></ProductNumber>
            <Name><Locale language="en">Night Sky over {location_name}</Locale></Name>
            <Description><Locale language="en">
                Visible stars from {location_name} ({lat:.2f}N, {lng:.2f}E)
                at {time.strftime("%Y-%m-%d %H:%M UTC")}.
                Contains {len(visible_stars)} stars with TM-33 spectral data.

                Tribute to Astrophysics!
            </Locale></Description>
            <DescriptiveAttributes>
                <CustomProperties>
                    <Property id="default_emitter_view"><Value>spectral</Value></Property>
                    <Property id="sky_location"><Value>{location_name}</Value></Property>
                    <Property id="sky_latitude"><Value>{lat}</Value></Property>
                    <Property id="sky_longitude"><Value>{lng}</Value></Property>
                </CustomProperties>
            </DescriptiveAttributes>
        </ProductMetaData>
        <Variants>
{variants_xml}        </Variants>
    </ProductDefinitions>
</Root>
'''

    # Generate sky_data.json for 3D viewer (all naked-eye visible stars)
    all_stars = get_stars(mag_limit=json_mag_limit)
    sky_data = {
        "location": {
            "name": location_name,
            "lat": lat,
            "lng": lng
        },
        "time": time.isoformat(),
        "stars": []
    }

    for star in all_stars:
        alt, az = calculate_alt_az(star, lat, lng, time)
        if alt > 0:
            sky_data["stars"].append({
                "name": star.name,
                "alt": round(alt, 2),
                "az": round(az, 2),
                "mag": star.mag,
                "spectral": star.spectral_type,
                "temp": star.temperature,
                "ra": star.ra,
                "dec": star.dec
            })

    sky_json = json.dumps(sky_data, indent=2)
    print(f"Embedded sky_data.json: {len(sky_data['stars'])} stars ({len(sky_json):,} bytes)")

    # Generate AutoLISP code for star visualization
    lisp_code = generate_star_lisp(visible_stars, location_name, lat, lng, time)

    # Create GLDF (ZIP) file
    import io
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, 'w', zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("product.xml", product_xml)
        for star_id, (filename, content) in tm33_files.items():
            zf.writestr(f"ldc/{filename}", content)
        # Embed sky data JSON in other/ folder
        zf.writestr("other/sky_data.json", sky_json)
        # Embed AutoLISP code for CAD export
        zf.writestr("other/autolisp/star_sky.lsp", lisp_code)
        print(f"Embedded AutoLISP: star_sky.lsp ({len(lisp_code):,} bytes, {len(visible_stars)} stars)")

        # Embed WASM viewers in other/viewer/ folder
        embed_wasm_viewers(zf)

    return buffer.getvalue()


def embed_wasm_viewers(zf: zipfile.ZipFile):
    """Embed WASM viewers in the GLDF for self-contained operation.

    Embeds:
    1. Star Sky 2D viewer (~123KB) - lightweight canvas-based star visualization
    2. AcadLISP/XLisp engine (~1MB) - AutoLISP interpreter for CAD export

    This makes the GLDF file self-contained without external network requests.
    """
    script_dir = Path(__file__).parent
    project_dir = script_dir.parent

    # 1. Star Sky 2D viewer from gldf-starsky-wasm
    starsky_pkg = project_dir / "crates" / "gldf-starsky-wasm" / "pkg"
    if starsky_pkg.exists():
        js_file = starsky_pkg / "gldf_starsky_wasm.js"
        wasm_file = starsky_pkg / "gldf_starsky_wasm_bg.wasm"

        if js_file.exists() and wasm_file.exists():
            starsky_manifest = {
                "type": "starsky",
                "name": "Star Sky 2D Viewer",
                "version": "0.1.0",
                "description": "Lightweight 2D star sky visualization - Tribute to Astrophysics",
                "js": "gldf_starsky_wasm.js",
                "wasm": "gldf_starsky_wasm_bg.wasm"
            }
            zf.writestr("other/viewer/starsky/manifest.json", json.dumps(starsky_manifest, indent=2))
            zf.writestr("other/viewer/starsky/gldf_starsky_wasm.js", js_file.read_bytes())
            zf.writestr("other/viewer/starsky/gldf_starsky_wasm_bg.wasm", wasm_file.read_bytes())
            print(f"Embedded Star Sky viewer: {wasm_file.stat().st_size:,} bytes (lightweight 2D canvas)")
        else:
            print(f"Warning: Star Sky viewer files not found in {starsky_pkg}")
    else:
        print(f"Warning: Star Sky pkg folder not found at {starsky_pkg}")

    # 2. AcadLISP/XLisp WASM from acadlisp dist
    acadlisp_dist = Path.home() / "Documents" / "develeop" / "rust" / "acadlisp" / "dist"
    if acadlisp_dist.exists():
        # Find the hashed JS and WASM files
        js_files = list(acadlisp_dist.glob("acadlisp-*.js"))
        wasm_files = list(acadlisp_dist.glob("acadlisp-*_bg.wasm"))
        xlisp_js = acadlisp_dist / "xlisp.js"
        xlisp_wasm = acadlisp_dist / "xlisp.wasm"

        if js_files and wasm_files:
            js_file = js_files[0]
            wasm_file = wasm_files[0]
            # Create manifest for acadlisp
            acadlisp_manifest = {
                "type": "acadlisp",
                "name": "AcadLISP Engine",
                "version": "0.1.0",
                "description": "AutoLISP interpreter with SVG/DXF export for CAD integration",
                "js": js_file.name,
                "wasm": wasm_file.name,
                "xlisp_js": "xlisp.js" if xlisp_js.exists() else None,
                "xlisp_wasm": "xlisp.wasm" if xlisp_wasm.exists() else None
            }
            zf.writestr("other/viewer/acadlisp/manifest.json", json.dumps(acadlisp_manifest, indent=2))
            zf.writestr(f"other/viewer/acadlisp/{js_file.name}", js_file.read_bytes())
            zf.writestr(f"other/viewer/acadlisp/{wasm_file.name}", wasm_file.read_bytes())
            print(f"Embedded AcadLISP: {js_file.name} ({wasm_file.stat().st_size:,} bytes)")

            # Also embed xlisp.js and xlisp.wasm (the core LISP engine)
            if xlisp_js.exists() and xlisp_wasm.exists():
                zf.writestr("other/viewer/acadlisp/xlisp.js", xlisp_js.read_bytes())
                zf.writestr("other/viewer/acadlisp/xlisp.wasm", xlisp_wasm.read_bytes())
                print(f"Embedded XLisp core: xlisp.wasm ({xlisp_wasm.stat().st_size:,} bytes)")
        else:
            print(f"Warning: AcadLISP files not found in {acadlisp_dist}")
    else:
        print(f"Warning: AcadLISP dist folder not found at {acadlisp_dist}")


def main():
    """Main entry point"""
    import argparse

    parser = argparse.ArgumentParser(description='Generate GLDF sky with real star data')
    parser.add_argument('--lat', type=float, default=51.77, help='Latitude (default: Lüdinghausen)')
    parser.add_argument('--lng', type=float, default=7.44, help='Longitude')
    parser.add_argument('--location', type=str, default='Lüdinghausen', help='Location name')
    parser.add_argument('--mag', type=float, default=4.0, help='Magnitude limit for GLDF (default: 4.0 = ~600 stars)')
    parser.add_argument('--json-mag', type=float, default=6.5, help='Magnitude limit for JSON (default: 6.5 = naked eye)')
    args = parser.parse_args()

    lat = args.lat
    lng = args.lng
    location_name = args.location
    gldf_mag_limit = args.mag
    json_mag_limit = args.json_mag

    # Current time (or specified time)
    obs_time = datetime.now(timezone.utc)

    print(f"=== Astral Sky Demo ===")
    print(f"Location: {location_name} ({lat}°N, {lng}°E)")
    print(f"Time: {obs_time.strftime('%Y-%m-%d %H:%M UTC')}")
    print(f"GLDF mag limit: {gldf_mag_limit} (brighter stars with TM-33 spectral data)")
    print(f"JSON mag limit: {json_mag_limit} (all visible stars for sky rendering)")
    print()

    # Generate GLDF with TM-33 spectral files and embedded sky_data.json
    gldf_data = generate_gldf_for_sky(lat, lng, location_name, obs_time,
                                       mag_limit=gldf_mag_limit, json_mag_limit=json_mag_limit)

    # Save to file
    output_dir = Path(__file__).parent.parent / "tests" / "data"
    output_file = output_dir / f"astral_sky_{location_name.lower().replace(' ', '_')}.gldf"
    output_file.write_bytes(gldf_data)

    print(f"Generated: {output_file}")
    print(f"Size: {len(gldf_data):,} bytes ({len(gldf_data)/1024:.1f} KB)")

    # Also copy to dist and static for web access
    for subdir in ["dist", "src/static"]:
        target_dir = Path(__file__).parent.parent / "crates" / "gldf-rs-wasm" / subdir
        if target_dir.exists():
            target_file = target_dir / f"astral_sky_{location_name.lower().replace(' ', '_')}.gldf"
            target_file.write_bytes(gldf_data)
            print(f"Copied to: {target_file}")


if __name__ == "__main__":
    main()
