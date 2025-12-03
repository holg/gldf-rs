#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

use serde::{Deserialize, Serialize};

use super::electrical::{ControlGearReference, EnergyLabels, PowerRange, Voltage};
use super::geometries::Rotation;
use super::{Locale, LocaleFoo};
use super::sensors::Sensor;

/// Represents a factor used to adjust flux values for various parameters.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FluxFactor {
    /// The input power value for the flux factor.
    #[serde(rename = "@inputPower", default)]
    pub input_power: String,

    /// The flickerPstLM value for the flux factor.
    #[serde(rename = "@flickerPstLM", default, skip_serializing_if = "Option::is_none")]
    pub flicker_pst_lm: Option<String>,

    /// The stroboscopicEffectsSVM value for the flux factor.
    #[serde(rename = "@stroboscopicEffectsSVM", default, skip_serializing_if = "Option::is_none")]
    pub stroboscopic_effects_svm: Option<String>,

    /// The description of the flux factor.
    #[serde(rename = "@description", default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The value of the flux factor.
    #[serde(rename = "$text", default)]
    pub value: f64,
}

/// Represents a table of active power values.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivePowerTable {
    /// The type attribute of the active power table.
    #[serde(rename = "@type", default)]
    pub type_attr: String,

    /// The flux factors in the table.
    #[serde(rename = "FluxFactor", default)]
    pub flux_factor: Vec<FluxFactor>,
}

/// Represents a range of color temperature adjusting values.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorTemperatureAdjustingRange {
    /// The lower bound of the color temperature adjusting range.
    #[serde(rename = "Lower", skip_serializing_if = "Option::is_none")]
    pub lower: Option<i32>,

    /// The upper bound of the color temperature adjusting range.
    #[serde(rename = "Upper", skip_serializing_if = "Option::is_none")]
    pub upper: Option<i32>,
}

/// Represents CIE 1931 color appearance values.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cie1931ColorAppearance {
    /// The X value of the CIE 1931 color appearance.
    #[serde(rename = "X", skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,

    /// The Y value of the CIE 1931 color appearance.
    #[serde(rename = "Y", skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,

    /// The Z value of the CIE 1931 color appearance.
    #[serde(rename = "Z", skip_serializing_if = "Option::is_none")]
    pub z: Option<f64>,
}

/// Represents rated chromaticity coordinate values.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatedChromacityCoordinateValues {
    /// The X value of the rated chromaticity coordinate.
    #[serde(rename = "X")]
    pub x: f64,

    /// The Y value of the rated chromaticity coordinate.
    #[serde(rename = "Y")]
    pub y: f64,
}

/// Represents data conforming to the IES TM-30-15 method.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IESTM3015 {
    /// The Rf (Fidelity Index) value according to IES TM-30-15.
    #[serde(rename = "Rf")]
    pub rf: i32,

    /// The Rg (Gamut Index) value according to IES TM-30-15.
    #[serde(rename = "Rg")]
    pub rg: i32,
}

/// Represents color information related to a light source.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorInformation {
    /// The color rendering index (CRI) indicating the quality of color rendering.
    #[serde(rename = "ColorRenderingIndex", skip_serializing_if = "Option::is_none")]
    pub color_rendering_index: Option<i32>,

    /// The correlated color temperature (CCT) representing the color appearance of the light source.
    #[serde(rename = "CorrelatedColorTemperature", skip_serializing_if = "Option::is_none")]
    pub correlated_color_temperature: Option<i32>,

    /// The lower and upper bounds of the color temperature adjusting range.
    #[serde(rename = "ColorTemperatureAdjustingRange", skip_serializing_if = "Option::is_none")]
    pub color_temperature_adjusting_range: Option<ColorTemperatureAdjustingRange>,

    /// The CIE 1931 color appearance values representing the color of the light source.
    #[serde(rename = "Cie1931ColorAppearance", skip_serializing_if = "Option::is_none")]
    pub cie1931_color_appearance: Option<Cie1931ColorAppearance>,

    /// The initial color tolerance of the light source as a textual description.
    #[serde(rename = "InitialColorTolerance", skip_serializing_if = "Option::is_none")]
    pub initial_color_tolerance: Option<String>,

    /// The maintained color tolerance of the light source as a textual description.
    #[serde(rename = "MaintainedColorTolerance", skip_serializing_if = "Option::is_none")]
    pub maintained_color_tolerance: Option<String>,

    /// The rated chromaticity coordinate values of the light source.
    #[serde(rename = "RatedChromacityCoordinateValues", skip_serializing_if = "Option::is_none")]
    pub rated_chromacity_coordinate_values: Option<RatedChromacityCoordinateValues>,

    /// The Television Lighting Consistency Index (TLCI) indicating color rendering accuracy.
    #[serde(rename = "TLCI", skip_serializing_if = "Option::is_none")]
    pub tlci: Option<i32>,

    /// Data conforming to the IES TM-30-15 method providing additional color information.
    #[serde(rename = "IES-TM-30-15", skip_serializing_if = "Option::is_none")]
    pub iestm3015: Option<IESTM3015>,

    /// The melanopic factor representing the impact of light on the human circadian system.
    #[serde(rename = "MelanopicFactor", skip_serializing_if = "Option::is_none")]
    pub melanopic_factor: Option<f64>,
}

/// Represents a photometry reference.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotometryReference {
    /// The File ID of the referenced photometry File reference.
    #[serde(rename = "@photometryId")]
    pub photometry_id: String,
}

/// Represents lamp maintenance factor information based on CIE recommendations.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CieLampMaintenanceFactor {
    /// The duration of burning time for the lamp (hours).
    #[serde(rename = "@burningTime")]
    pub burning_time: u32,

    /// The lamp lumen maintenance factor.
    #[serde(rename = "LampLumenMaintenanceFactor")]
    pub lamp_lumen_maintenance_factor: f64,

    /// The lamp survival factor.
    #[serde(rename = "LampSurvivalFactor")]
    pub lamp_survival_factor: i32,
}

/// Represents a collection of CIE lamp maintenance factors.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CieLampMaintenanceFactors {
    /// A list of CIE lamp maintenance factors.
    #[serde(rename = "CieLampMaintenanceFactor", default)]
    pub cie_lamp_maintenance_factor: Vec<CieLampMaintenanceFactor>,
}

/// Represents the maintenance factor for LED lighting sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedMaintenanceFactor {
    /// The number of hours corresponding to the maintenance factor.
    #[serde(rename = "@hours")]
    pub hours: i32,

    /// The value of the maintenance factor.
    #[serde(rename = "$text")]
    pub value: f64,
}

impl Default for LedMaintenanceFactor {
    fn default() -> Self {
        Self {
            hours: 0,
            value: 1.0,
        }
    }
}

/// Represents maintenance information for a light source.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightSourceMaintenance {
    /// The expected lifetime of the light source in hours.
    #[serde(rename = "@lifetime", skip_serializing_if = "Option::is_none")]
    pub lifetime: Option<i32>,

    /// The type of lamp as per the CIE97 standard.
    #[serde(rename = "Cie97LampType", skip_serializing_if = "Option::is_none")]
    pub cie97_lamp_type: Option<String>,

    /// Maintenance factors specific to lamp types.
    #[serde(rename = "CieLampMaintenanceFactors", skip_serializing_if = "Option::is_none")]
    pub cie_lamp_maintenance_factors: Option<CieLampMaintenanceFactors>,

    /// LED maintenance factor information.
    #[serde(rename = "LedMaintenanceFactor", skip_serializing_if = "Option::is_none")]
    pub led_maintenance_factor: Option<LedMaintenanceFactor>,

    /// The lamp survival factor.
    #[serde(rename = "LampSurvivalFactor", skip_serializing_if = "Option::is_none")]
    pub lamp_survival_factor: Option<i32>,
}

/// Represents a reference to a spectrum.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectrumReference {
    /// The ID of the referenced spectrum.
    #[serde(rename = "@spectrumId")]
    pub spectrum_id: String,
}

/// Represents images associated with a light source.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Images {
    /// Image references.
    #[serde(rename = "Image", default)]
    pub image: Vec<ImageReference>,
}

/// Represents an image reference.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageReference {
    /// The file ID of the image.
    #[serde(rename = "@fileId")]
    pub file_id: String,

    /// The image type.
    #[serde(rename = "@imageType", skip_serializing_if = "Option::is_none")]
    pub image_type: Option<String>,
}

/// Represents a changeable light source in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeableLightSource {
    /// The unique identifier for the changeable light source.
    #[serde(rename = "@id", default)]
    pub id: String,

    /// The localized name of the changeable light source.
    #[serde(rename = "Name", default)]
    pub name: Locale,

    /// The localized description of the changeable light source.
    #[serde(rename = "Description", default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Locale>,

    /// The manufacturer of the changeable light source.
    #[serde(rename = "Manufacturer", default, skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,

    /// The photometric reference data associated with the changeable light source.
    #[serde(rename = "PhotometryReference", default, skip_serializing_if = "Option::is_none")]
    pub photometry_reference: Option<PhotometryReference>,

    /// Information about the maintenance of the light source.
    #[serde(rename = "LightSourceMaintenance", default, skip_serializing_if = "Option::is_none")]
    pub light_source_maintenance: Option<LightSourceMaintenance>,
}

/// Represents a fixed light source in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixedLightSource {
    /// Identifier for the fixed light source.
    #[serde(rename = "@id", default)]
    pub id: String,

    /// Name of the fixed light source.
    #[serde(rename = "Name", default)]
    pub name: LocaleFoo,

    /// Description of the fixed light source.
    #[serde(rename = "Description", default, skip_serializing_if = "Option::is_none")]
    pub description: Option<LocaleFoo>,

    /// Manufacturer of the fixed light source.
    #[serde(rename = "Manufacturer", skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,

    /// Global Trade Item Number (GTIN) of the fixed light source.
    #[serde(rename = "GTIN", skip_serializing_if = "Option::is_none")]
    pub gtin: Option<String>,

    /// Rated input power of the fixed light source.
    #[serde(rename = "RatedInputPower", skip_serializing_if = "Option::is_none")]
    pub rated_input_power: Option<f64>,

    /// Rated input voltage of the fixed light source.
    #[serde(rename = "RatedInputVoltage", skip_serializing_if = "Option::is_none")]
    pub rated_input_voltage: Option<Voltage>,

    /// Power range of the fixed light source.
    #[serde(rename = "PowerRange", skip_serializing_if = "Option::is_none")]
    pub power_range: Option<PowerRange>,

    /// Position of usage of the fixed light source.
    #[serde(rename = "LightSourcePositionOfUsage", skip_serializing_if = "Option::is_none")]
    pub light_source_position_of_usage: Option<String>,

    /// Energy labels of the fixed light source.
    #[serde(rename = "EnergyLabels", skip_serializing_if = "Option::is_none")]
    pub energy_labels: Option<EnergyLabels>,

    /// Spectrum reference data of the fixed light source.
    #[serde(rename = "SpectrumReference", skip_serializing_if = "Option::is_none")]
    pub spectrum_reference: Option<SpectrumReference>,

    /// Active power table of the fixed light source.
    #[serde(rename = "ActivePowerTable", skip_serializing_if = "Option::is_none")]
    pub active_power_table: Option<ActivePowerTable>,

    /// Color information of the fixed light source.
    #[serde(rename = "ColorInformation", skip_serializing_if = "Option::is_none")]
    pub color_information: Option<ColorInformation>,

    /// Images of the fixed light source.
    #[serde(rename = "LightSourceImages", skip_serializing_if = "Option::is_none")]
    pub light_source_images: Option<Images>,

    /// Maintenance data of the fixed light source.
    #[serde(rename = "LightSourceMaintenance", skip_serializing_if = "Option::is_none")]
    pub light_source_maintenance: Option<LightSourceMaintenance>,

    /// Indicates if the fixed light source adheres to the Zhaga standard.
    #[serde(rename = "ZhagaStandard", skip_serializing_if = "Option::is_none")]
    pub zhaga_standard: Option<bool>,
}

/// Represents a collection of light sources in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightSources {
    /// A vector of changeable light sources present in the luminaire.
    #[serde(rename = "ChangeableLightSource", default)]
    pub changeable_light_source: Vec<ChangeableLightSource>,

    /// A vector of fixed light sources integrated into the luminaire.
    #[serde(rename = "FixedLightSource", default)]
    pub fixed_light_source: Vec<FixedLightSource>,
}

/// Represents a reference to a light source in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightSourceReference {
    /// The ID of the referenced fixed light source.
    #[serde(rename = "@fixedLightSourceId", skip_serializing_if = "Option::is_none")]
    pub fixed_light_source_id: Option<String>,

    /// The ID of the referenced changeable light source.
    #[serde(rename = "@changeableLightSourceId", skip_serializing_if = "Option::is_none")]
    pub changeable_light_source_id: Option<String>,

    /// The count of light sources associated with this reference.
    #[serde(rename = "@lightSourceCount", skip_serializing_if = "Option::is_none")]
    pub light_source_count: Option<i32>,
}

/// Represents a changeable light emitter in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeableLightEmitter {
    /// The emergency behavior of the light emitter.
    #[serde(rename = "@emergencyBehaviour", skip_serializing_if = "Option::is_none")]
    pub emergency_behaviour: Option<String>,

    /// The localized name of the changeable light emitter.
    #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
    pub name: Option<Locale>,

    /// The rotation of the light emitter.
    #[serde(rename = "Rotation", skip_serializing_if = "Option::is_none")]
    pub rotation: Option<Rotation>,

    /// The photometry reference associated with the light emitter.
    #[serde(rename = "PhotometryReference")]
    pub photometry_reference: PhotometryReference,

    /// The global rotation value G0.
    #[serde(rename = "G0", skip_serializing_if = "Option::is_none")]
    pub g0: Option<String>,
}

/// Represents a fixed light emitter in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixedLightEmitter {
    /// The emergency behavior of the light emitter.
    #[serde(rename = "@emergencyBehaviour", skip_serializing_if = "Option::is_none")]
    pub emergency_behaviour: Option<String>,

    /// The localized name of the fixed light emitter.
    #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
    pub name: Option<LocaleFoo>,

    /// The rotation of the light emitter.
    #[serde(rename = "Rotation", skip_serializing_if = "Option::is_none")]
    pub rotation: Option<Rotation>,

    /// The photometry reference associated with the light emitter.
    #[serde(rename = "PhotometryReference")]
    pub photometry_reference: PhotometryReference,

    /// The reference to the light source associated with the fixed light emitter.
    #[serde(rename = "LightSourceReference")]
    pub light_source_reference: LightSourceReference,

    /// The reference to the control gear associated with the fixed light emitter.
    #[serde(rename = "ControlGearReference", skip_serializing_if = "Option::is_none")]
    pub control_gear_reference: Option<ControlGearReference>,

    /// The rated luminous flux of the light emitter.
    #[serde(rename = "RatedLuminousFlux", skip_serializing_if = "Option::is_none")]
    pub rated_luminous_flux: Option<i32>,

    /// The rated luminous flux in RGB of the light emitter.
    #[serde(rename = "RatedLuminousFluxRGB", skip_serializing_if = "Option::is_none")]
    pub rated_luminous_flux_rgb: Option<i32>,

    /// The emergency ballast lumen factor of the light emitter.
    #[serde(rename = "EmergencyBallastLumenFactor", skip_serializing_if = "Option::is_none")]
    pub emergency_ballast_lumen_factor: Option<f64>,

    /// The emergency rated luminous flux of the light emitter.
    #[serde(rename = "EmergencyRatedLuminousFlux", skip_serializing_if = "Option::is_none")]
    pub emergency_rated_luminous_flux: Option<String>,
}

/// Represents an emitter in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Emitter {
    /// The unique identifier of the emitter.
    #[serde(rename = "@id")]
    pub id: String,

    /// Collection of changeable light emitters.
    #[serde(rename = "ChangeableLightEmitter", default)]
    pub changeable_light_emitter: Vec<ChangeableLightEmitter>,

    /// Collection of fixed light emitters.
    #[serde(rename = "FixedLightEmitter", default)]
    pub fixed_light_emitter: Vec<FixedLightEmitter>,

    /// Collection of sensors associated with the emitter.
    #[serde(rename = "Sensor", default)]
    pub sensor: Vec<Sensor>,
}

/// Represents a collection of emitters in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Emitters {
    /// Collection of emitters.
    #[serde(rename = "Emitter", default)]
    pub emitter: Vec<Emitter>,
}

/// Represents a rectangular emitter in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RectangularEmitter {
    /// The width values for the rectangular emitter.
    #[serde(rename = "Width", default)]
    pub width: Vec<i32>,

    /// The length values for the rectangular emitter.
    #[serde(rename = "Length", default)]
    pub length: Vec<i32>,
}

/// Represents a circular emitter in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircularEmitter {
    /// The diameter values for the circular emitter.
    #[serde(rename = "Diameter", default)]
    pub diameter: Vec<i32>,
}
