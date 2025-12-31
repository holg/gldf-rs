//! HYG Star Catalogue parser
//!
//! Parses the HYG (Hipparcos-Yale-Gliese) star catalogue v4.2
//! containing ~120,000 stars with positions, magnitudes, and spectral data.
//!
//! Data source: https://www.astronexus.com/hyg
//! License: CC BY-SA 4.0

use serde::Deserialize;
use std::io::Read;

/// Star data from HYG catalogue
#[derive(Debug, Clone, Deserialize)]
pub struct HygStar {
    /// Database ID
    pub id: u32,
    /// Hipparcos catalogue ID
    #[serde(deserialize_with = "deserialize_option_u32")]
    pub hip: Option<u32>,
    /// Henry Draper catalogue ID
    #[serde(deserialize_with = "deserialize_option_u32")]
    pub hd: Option<u32>,
    /// Harvard Revised catalogue ID
    #[serde(deserialize_with = "deserialize_option_u32")]
    pub hr: Option<u32>,
    /// Gliese catalogue ID
    #[serde(default)]
    pub gl: String,
    /// Bayer/Flamsteed designation
    #[serde(default)]
    pub bf: String,
    /// Proper name (Sirius, Vega, etc.)
    #[serde(default)]
    pub proper: String,
    /// Right Ascension in degrees (0-360)
    pub ra: f64,
    /// Declination in degrees (-90 to +90)
    pub dec: f64,
    /// Distance in parsecs
    pub dist: f64,
    /// Proper motion in RA (mas/yr)
    #[serde(default)]
    pub pmra: f64,
    /// Proper motion in Dec (mas/yr)
    #[serde(default)]
    pub pmdec: f64,
    /// Radial velocity (km/s)
    #[serde(deserialize_with = "deserialize_option_f64")]
    pub rv: Option<f64>,
    /// Apparent magnitude
    pub mag: f32,
    /// Absolute magnitude
    pub absmag: f32,
    /// Spectral type (e.g., "G2V", "A0", "M5III")
    #[serde(default)]
    pub spect: String,
    /// Color index (B-V)
    #[serde(deserialize_with = "deserialize_option_f32")]
    pub ci: Option<f32>,
    /// Cartesian X coordinate
    pub x: f64,
    /// Cartesian Y coordinate
    pub y: f64,
    /// Cartesian Z coordinate
    pub z: f64,
    /// Velocity X component
    #[serde(default)]
    pub vx: f64,
    /// Velocity Y component
    #[serde(default)]
    pub vy: f64,
    /// Velocity Z component
    #[serde(default)]
    pub vz: f64,
    /// RA in radians
    pub rarad: f64,
    /// Dec in radians
    pub decrad: f64,
    /// Proper motion RA in radians/yr
    #[serde(default)]
    pub pmrarad: f64,
    /// Proper motion Dec in radians/yr
    #[serde(default)]
    pub pmdecrad: f64,
    /// Bayer designation
    #[serde(default)]
    pub bayer: String,
    /// Flamsteed number
    #[serde(default)]
    pub flam: String,
    /// Constellation abbreviation
    #[serde(default)]
    pub con: String,
    /// Component identifier for multiple star systems
    #[serde(default)]
    pub comp: u32,
    /// Primary component ID
    #[serde(default)]
    pub comp_primary: u32,
    /// Base star ID
    #[serde(default)]
    pub base: String,
    /// Luminosity in solar luminosities
    #[serde(deserialize_with = "deserialize_option_f64")]
    pub lum: Option<f64>,
    /// Variable star designation
    #[serde(default)]
    pub var: String,
    /// Variable minimum magnitude
    #[serde(deserialize_with = "deserialize_option_f32")]
    pub var_min: Option<f32>,
    /// Variable maximum magnitude
    #[serde(deserialize_with = "deserialize_option_f32")]
    pub var_max: Option<f32>,
}

// Custom deserializers for optional numeric fields
fn deserialize_option_u32<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) if !s.is_empty() => s.parse().map(Some).map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

fn deserialize_option_f32<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) if !s.is_empty() => s.parse().map(Some).map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

fn deserialize_option_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) if !s.is_empty() => s.parse().map(Some).map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

impl HygStar {
    /// Get display name (proper name, Bayer designation, or HD number)
    pub fn display_name(&self) -> String {
        if !self.proper.is_empty() {
            self.proper.clone()
        } else if !self.bf.is_empty() {
            self.bf.clone()
        } else if let Some(hd) = self.hd {
            format!("HD {}", hd)
        } else if let Some(hip) = self.hip {
            format!("HIP {}", hip)
        } else {
            format!("HYG {}", self.id)
        }
    }

    /// Calculate temperature from B-V color index using Ballesteros formula
    pub fn temperature(&self) -> f32 {
        if let Some(bv) = self.ci {
            // Ballesteros 2012 formula
            4600.0 * (1.0 / (0.92 * bv + 1.7) + 1.0 / (0.92 * bv + 0.62))
        } else {
            // Estimate from spectral type
            self.temperature_from_spectral()
        }
    }

    /// Estimate temperature from spectral type
    fn temperature_from_spectral(&self) -> f32 {
        let spec = self.spect.to_uppercase();
        if spec.starts_with('O') {
            35000.0
        } else if spec.starts_with('B') {
            20000.0
        } else if spec.starts_with('A') {
            9000.0
        } else if spec.starts_with('F') {
            7000.0
        } else if spec.starts_with('G') {
            5500.0
        } else if spec.starts_with('K') {
            4500.0
        } else if spec.starts_with('M') {
            3200.0
        } else {
            5500.0 // Default to Sun-like
        }
    }

    /// Convert temperature to RGB color (sRGB, 0-1 range)
    pub fn color_rgb(&self) -> [f32; 3] {
        temperature_to_rgb(self.temperature())
    }

    /// Calculate relative brightness (1.0 = magnitude 0)
    pub fn relative_brightness(&self) -> f32 {
        // Pogson's equation: each magnitude is 2.512x brightness difference
        10.0_f32.powf(-0.4 * self.mag)
    }

    /// Check if star is visible to naked eye (mag < 6.5)
    pub fn is_naked_eye(&self) -> bool {
        self.mag < 6.5
    }

    /// Check if star has valid distance (not placeholder 100000 pc)
    pub fn has_valid_distance(&self) -> bool {
        self.dist > 0.0 && self.dist < 10000.0
    }
}

/// Convert color temperature (Kelvin) to RGB using Tanner Helland's algorithm
pub fn temperature_to_rgb(kelvin: f32) -> [f32; 3] {
    let temp = (kelvin / 100.0).clamp(10.0, 400.0);

    let r = if temp <= 66.0 {
        1.0
    } else {
        let r = 329.698727446 * (temp - 60.0).powf(-0.1332047592);
        (r / 255.0).clamp(0.0, 1.0)
    };

    let g = if temp <= 66.0 {
        let g = 99.4708025861 * temp.ln() - 161.1195681661;
        (g / 255.0).clamp(0.0, 1.0)
    } else {
        let g = 288.1221695283 * (temp - 60.0).powf(-0.0755148492);
        (g / 255.0).clamp(0.0, 1.0)
    };

    let b = if temp >= 66.0 {
        1.0
    } else if temp <= 19.0 {
        0.0
    } else {
        let b = 138.5177312231 * (temp - 10.0).ln() - 305.0447927307;
        (b / 255.0).clamp(0.0, 1.0)
    };

    [r, g, b]
}

/// HYG Star Catalogue
#[derive(Debug, Clone)]
pub struct HygCatalogue {
    pub stars: Vec<HygStar>,
}

impl HygCatalogue {
    /// Parse HYG catalogue from CSV reader
    pub fn from_csv<R: Read>(reader: R) -> Result<Self, csv::Error> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(reader);

        let mut stars = Vec::with_capacity(120000);
        for result in rdr.deserialize() {
            match result {
                Ok(star) => stars.push(star),
                Err(e) => {
                    // Log but continue on parse errors
                    eprintln!("Warning: Failed to parse star record: {}", e);
                }
            }
        }

        Ok(Self { stars })
    }

    /// Parse from gzipped CSV
    pub fn from_gzip<R: Read>(reader: R) -> Result<Self, Box<dyn std::error::Error>> {
        let decoder = flate2::read::GzDecoder::new(reader);
        Ok(Self::from_csv(decoder)?)
    }

    /// Filter to naked-eye visible stars (mag < 6.5)
    pub fn naked_eye_stars(&self) -> impl Iterator<Item = &HygStar> {
        self.stars.iter().filter(|s| s.is_naked_eye())
    }

    /// Filter to bright stars (mag < threshold)
    pub fn stars_brighter_than(&self, mag_limit: f32) -> impl Iterator<Item = &HygStar> {
        self.stars.iter().filter(move |s| s.mag < mag_limit)
    }

    /// Get stars with proper names
    pub fn named_stars(&self) -> impl Iterator<Item = &HygStar> {
        self.stars.iter().filter(|s| !s.proper.is_empty())
    }

    /// Find star by proper name
    pub fn find_by_name(&self, name: &str) -> Option<&HygStar> {
        let name_lower = name.to_lowercase();
        self.stars
            .iter()
            .find(|s| s.proper.to_lowercase() == name_lower)
    }

    /// Find star by Hipparcos ID
    pub fn find_by_hip(&self, hip_id: u32) -> Option<&HygStar> {
        self.stars.iter().find(|s| s.hip == Some(hip_id))
    }

    /// Get brightest N stars
    pub fn brightest(&self, n: usize) -> Vec<&HygStar> {
        let mut sorted: Vec<_> = self.stars.iter().collect();
        sorted.sort_by(|a, b| a.mag.partial_cmp(&b.mag).unwrap());
        sorted.truncate(n);
        sorted
    }

    /// Get stars in a constellation
    pub fn stars_in_constellation(&self, con: &str) -> impl Iterator<Item = &HygStar> {
        let con_upper = con.to_uppercase();
        self.stars
            .iter()
            .filter(move |s| s.con.to_uppercase() == con_upper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temperature_to_rgb() {
        // Hot blue star
        let rgb = temperature_to_rgb(30000.0);
        assert!(rgb[2] > rgb[0], "Hot star should be bluish");

        // Sun-like
        let rgb = temperature_to_rgb(5778.0);
        assert!(rgb[0] > 0.9 && rgb[1] > 0.9, "Sun should be yellowish-white");

        // Red giant
        let rgb = temperature_to_rgb(3500.0);
        assert!(rgb[0] > rgb[2], "Cool star should be reddish");
    }

    #[test]
    fn test_relative_brightness() {
        let star = HygStar {
            id: 1,
            hip: None,
            hd: None,
            hr: None,
            gl: String::new(),
            bf: String::new(),
            proper: "Test".to_string(),
            ra: 0.0,
            dec: 0.0,
            dist: 10.0,
            pmra: 0.0,
            pmdec: 0.0,
            rv: None,
            mag: 0.0,
            absmag: 0.0,
            spect: "G2V".to_string(),
            ci: Some(0.65),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
            rarad: 0.0,
            decrad: 0.0,
            pmrarad: 0.0,
            pmdecrad: 0.0,
            bayer: String::new(),
            flam: String::new(),
            con: String::new(),
            comp: 0,
            comp_primary: 0,
            base: String::new(),
            lum: None,
            var: String::new(),
            var_min: None,
            var_max: None,
        };

        // Magnitude 0 star should have brightness 1.0
        assert!((star.relative_brightness() - 1.0).abs() < 0.001);
    }
}
