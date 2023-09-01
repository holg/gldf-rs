#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
pub use super::header::*;
pub use super::electrical::*;
use serde::{Serialize};
use serde::Deserialize;
use yaserde_derive::YaDeserialize;
use yaserde_derive::YaSerialize;
/// Represents a factor used to adjust flux values for various parameters.
///
/// The `FluxFactor` struct models a factor used to adjust flux values for different parameters
/// within the GLDF file. It includes information about input power, flicker, stroboscopic effects,
/// and a description. The factor value is specified as an attribute. It supports serialization
/// and deserialization of XML data for working with flux factors.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FluxFactor {
    /// The input power value for the flux factor.
    #[yaserde(rename = "inputPower")]
    #[serde(rename = "inputPower")]
    pub input_power: String,

    /// The flickerPstLM value for the flux factor.
    #[yaserde(rename = "flickerPstLM")]
    #[serde(rename = "flickerPstLM")]
    pub flicker_pst_lm: String,

    /// The stroboscopicEffectsSVM value for the flux factor.
    #[yaserde(rename = "stroboscopicEffectsSVM")]
    #[serde(rename = "stroboscopicEffectsSVM")]
    pub stroboscopic_effects_svm: String,

    /// The description of the flux factor.
    #[yaserde(rename = "description")]
    #[serde(rename = "description")]
    pub description: String,

    /// The value of the flux factor.
    #[yaserde(attribute)]
    pub value: f64,
}

/// Represents a table of active power values.
///
/// The `ActivePowerTable` struct models a table of active power values within the GLDF file. It
/// includes information about the type of the table and the default light source power. It supports
/// serialization and deserialization of XML data for working with active power tables.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ActivePowerTable {
    /// The type attribute of the active power table.
    #[yaserde(rename = "type")]
    #[serde(rename = "type")]
    pub type_attr: String,

    /// The default light source power value for the active power table.
    #[yaserde(rename = "DefaultLightSourcePower")]
    #[serde(rename = "DefaultLightSourcePower")]
    pub default_light_source_power: String,
}

/// Represents a range of color temperature adjusting values.
///
/// The `ColorTemperatureAdjustingRange` struct models a range of color temperature adjusting values
/// within the GLDF file. It includes the lower and upper bounds of the range. It supports
/// serialization and deserialization of XML data for working with color temperature adjusting ranges.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ColorTemperatureAdjustingRange {
    /// The lower bound of the color temperature adjusting range.
    #[yaserde(rename = "Lower")]
    #[serde(rename = "Lower", skip_serializing_if = "Option::is_none")]
    pub lower: Option<i32>,

    /// The upper bound of the color temperature adjusting range.
    #[yaserde(rename = "Upper")]
    #[serde(rename = "Upper", skip_serializing_if = "Option::is_none")]
    pub upper: Option<i32>,
}

/// Represents CIE 1931 color appearance values.
///
/// The `Cie1931ColorAppearance` struct models CIE 1931 color appearance values within the GLDF file.
/// It includes the X, Y, and Z values of the color appearance. It supports serialization and
/// deserialization of XML data for working with CIE 1931 color appearance.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Cie1931ColorAppearance {
    /// The X value of the CIE 1931 color appearance.
    #[yaserde(rename = "X")]
    #[serde(rename = "X", skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,

    /// The Y value of the CIE 1931 color appearance.
    #[yaserde(rename = "Y")]
    #[serde(rename = "Y", skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,

    /// The Z value of the CIE 1931 color appearance.
    #[yaserde(rename = "Z")]
    #[serde(rename = "Z", skip_serializing_if = "Option::is_none")]
    pub z: Option<f64>,
}

/// Represents rated chromaticity coordinate values.
///
/// The `RatedChromacityCoordinateValues` struct models rated chromaticity coordinate values within
/// the GLDF file. It includes the X and Y values of the rated chromaticity coordinate. It supports
/// serialization and deserialization of XML data for working with rated chromaticity coordinates.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct RatedChromacityCoordinateValues {
    /// The X value of the rated chromaticity coordinate.
    #[yaserde(rename = "X")]
    #[serde(rename = "X")]
    pub x: f64,

    /// The Y value of the rated chromaticity coordinate.
    #[yaserde(rename = "Y")]
    #[serde(rename = "Y")]
    pub y: f64,
}

/// Represents data conforming to the IES TM-30-15 method.
///
/// The `IESTM3015` struct models data conforming to the IES TM-30-15 method within the GLDF file. It
/// includes values for Rf (Fidelity Index) and Rg (Gamut Index). It supports serialization and
/// deserialization of XML data for working with IES TM-30-15 data.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct IESTM3015 {
    /// The Rf (Fidelity Index) value according to IES TM-30-15.
    #[yaserde(rename = "Rf")]
    #[serde(rename = "Rf")]
    pub rf: i32,

    /// The Rg (Gamut Index) value according to IES TM-30-15.
    #[yaserde(rename = "Rg")]
    #[serde(rename = "Rg")]
    pub rg: i32,
}

/// Example of how to construct a `RatedChromacityCoordinateValues` instance:
///
/// ```
/// use gldf_rs::gldf::RatedChromacityCoordinateValues;
///
/// let rated_chromacity = RatedChromacityCoordinateValues {
///     x: 0.35,
///     y: 0.42,
/// };
/// ```
///
/// Example of how to construct an `IESTM3015` instance:
///
/// ```
/// use gldf_rs::gldf::IESTM3015;
///
/// let iestm_data = IESTM3015 {
///     rf: 85,
///     rg: 103,
/// };
/// ```

/// Represents color information related to a light source.
///
/// The `ColorInformation` struct models color-related information within the GLDF file. It includes
/// various color attributes such as color rendering index, correlated color temperature, color
/// temperature adjusting range, color appearance, color tolerances, chromaticity coordinates, and
/// other color metrics. It supports serialization and deserialization of XML data for working with
/// color information.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ColorInformation {
    /// The color rendering index (CRI) indicating the quality of color rendering.
    #[yaserde(rename = "ColorRenderingIndex")]
    #[serde(rename = "ColorRenderingIndex", skip_serializing_if = "Option::is_none")]
    pub color_rendering_index: Option<i32>,

    /// The correlated color temperature (CCT) representing the color appearance of the light source.
    #[yaserde(rename = "CorrelatedColorTemperature")]
    #[serde(rename = "CorrelatedColorTemperature", skip_serializing_if = "Option::is_none")]
    pub correlated_color_temperature: Option<i32>,

    /// The lower and upper bounds of the color temperature adjusting range.
    #[yaserde(rename = "ColorTemperatureAdjustingRange")]
    #[serde(rename = "ColorTemperatureAdjustingRange", skip_serializing_if = "Option::is_none")]
    pub color_temperature_adjusting_range: Option<ColorTemperatureAdjustingRange>,

    /// The CIE 1931 color appearance values representing the color of the light source.
    #[yaserde(rename = "Cie1931ColorAppearance")]
    #[serde(rename = "Cie1931ColorAppearance", skip_serializing_if = "Option::is_none")]
    pub cie1931_color_appearance: Option<Cie1931ColorAppearance>,

    /// The initial color tolerance of the light source as a textual description.
    #[yaserde(rename = "InitialColorTolerance")]
    #[serde(rename = "InitialColorTolerance", skip_serializing_if = "Option::is_none")]
    pub initial_color_tolerance: Option<String>,

    /// The maintained color tolerance of the light source as a textual description.
    #[yaserde(rename = "MaintainedColorTolerance")]
    #[serde(rename = "MaintainedColorTolerance", skip_serializing_if = "Option::is_none")]
    pub maintained_color_tolerance: Option<String>,

    /// The rated chromaticity coordinate values of the light source.
    #[yaserde(rename = "RatedChromacityCoordinateValues")]
    #[serde(rename = "RatedChromacityCoordinateValues", skip_serializing_if = "Option::is_none")]
    pub rated_chromacity_coordinate_values: Option<RatedChromacityCoordinateValues>,

    /// The Television Lighting Consistency Index (TLCI) indicating color rendering accuracy.
    #[yaserde(rename = "TLCI")]
    #[serde(rename = "TLCI", skip_serializing_if = "Option::is_none")]
    pub tlci: Option<i32>,

    /// Data conforming to the IES TM-30-15 method providing additional color information.
    #[yaserde(rename = "IES-TM-30-15")]
    #[serde(rename = "IES-TM-30-15", skip_serializing_if = "Option::is_none")]
    pub iestm3015: Option<IESTM3015>,

    /// The melanopic factor representing the impact of light on the human circadian system.
    #[yaserde(rename = "MelanopicFactor")]
    #[serde(rename = "MelanopicFactor", skip_serializing_if = "Option::is_none")]
    pub melanopic_factor: Option<f64>,
}

/// Example of how to construct a `ColorInformation` instance:
///
/// ```
/// use gldf_rs::gldf::{ColorInformation, ColorTemperatureAdjustingRange, Cie1931ColorAppearance,
///                    RatedChromacityCoordinateValues, IESTM3015};
///
/// let color_info = ColorInformation {
///     color_rendering_index: Some(90),
///     correlated_color_temperature: Some(4000),
///     color_temperature_adjusting_range: Some(ColorTemperatureAdjustingRange {
///         lower: Some(3000),
///         upper: Some(6000),
///     }),
///     cie1931_color_appearance: Some(Cie1931ColorAppearance {
///         x: Some(0.35),
///         y: Some(0.42),
///         z: Some(0.23),
///     }),
///     initial_color_tolerance: Some("±0.005".to_string()),
///     maintained_color_tolerance: Some("±0.010".to_string()),
///     rated_chromacity_coordinate_values: Some(RatedChromacityCoordinateValues {
///         x: 0.313,
///         y: 0.333,
///     }),
///     tlci: Some(75),
///     iestm3015: Some(IESTM3015 { rf: 92, rg: 87 }),
///     melanopic_factor: Some(0.5),
/// };
/// ```

// PhotometryReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PhotometryReference {
    #[yaserde(attribute, rename = "photometryId")]
    #[serde(rename = "@photometryId")]
    /// The File ID of the referenced photometry File reference.
    pub photometry_id: String,
}

/// Represents lamp maintenance factor information based on CIE recommendations.
///
/// The `CieLampMaintenanceFactor` struct models lamp maintenance factor information within the
/// GLDF file. It includes the burning time, lamp lumen maintenance factor, and lamp survival
/// factor based on CIE recommendations. It supports serialization and deserialization of XML data
/// for working with lamp maintenance factor information.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CieLampMaintenanceFactor {
    /// The duration of burning time for the lamp, corresponds to IES "Burning Time Unit (hours)".
    #[yaserde(rename = "burningTime")]
    #[serde(rename = "burningTime")]
    pub burning_time: u32,

    /// The lamp lumen maintenance factor indicating how much the lumen output of the lamp
    /// decreases over time.
    #[yaserde(rename = "LampLumenMaintenanceFactor")]
    #[serde(rename = "LampLumenMaintenanceFactor")]
    pub lamp_lumen_maintenance_factor: f64,

    /// The lamp survival factor indicating the percentage of lamps that survive the given
    /// duration of burning time.
    #[yaserde(rename = "LampSurvivalFactor")]
    #[serde(rename = "LampSurvivalFactor")]
    pub lamp_survival_factor: i32,
}

/// Example of how to construct a `CieLampMaintenanceFactor` instance:
///
/// ```
/// use gldf_rs::gldf::CieLampMaintenanceFactor;
///
/// let lamp_maintenance = CieLampMaintenanceFactor {
///     burning_time: 6000, // Measured in hours
///     lamp_lumen_maintenance_factor: 0.75,
///     lamp_survival_factor: 85,
/// };
/// ```


/// Represents a collection of CIE lamp maintenance factors.
///
/// The `CieLampMaintenanceFactors` struct models a collection of lamp maintenance factors according to the
/// CIE standard. It contains a list of individual `CieLampMaintenanceFactor` instances. Lamp maintenance
/// factors provide information about how the lumen output of lamps changes over time due to various factors
/// such as burning time and survival rate.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CieLampMaintenanceFactors {
    /// A list of CIE lamp maintenance factors.
    #[yaserde(rename = "CieLampMaintenanceFactor")]
    #[serde(rename = "CieLampMaintenanceFactor")]
    pub cie_lamp_maintenance_factor: Vec<CieLampMaintenanceFactor>,
}

/// Example of how to construct a `CieLampMaintenanceFactors` instance:
///
/// ```
/// use gldf_rs::gldf::{CieLampMaintenanceFactors, CieLampMaintenanceFactor};
///
/// let maintenance_factors = CieLampMaintenanceFactors {
///     cie_lamp_maintenance_factor: vec![
///         CieLampMaintenanceFactor {
///             burning_time: 1000, // hours
///             lamp_lumen_maintenance_factor: 0.8,
///             lamp_survival_factor: 85,
///         },
///         // ... (add more maintenance factors as needed)
///     ],
/// };
/// ```

/// Represents the maintenance factor for LED lighting sources.
///
/// The `LedMaintenanceFactor` struct models the maintenance factor for LED lighting sources. Maintenance factor
/// is a measure of how the light output of an LED source decreases over time due to various factors such as
/// degradation of the LED components. It provides information about the expected reduction in light output
/// over a certain period of time.
///
/// This struct includes two fields: `hours` and `value`. The `hours` field indicates the number of hours for
/// which the maintenance factor is provided. The `value` field represents the actual maintenance factor value
/// corresponding to the specified number of hours. The maintenance factor value is typically a decimal number
/// between 0 and 1, where 1 represents no degradation and 0 represents complete loss of light output.
///
/// Example usage:
/// ```
/// use gldf_rs::gldf::LedMaintenanceFactor;
///
/// let maintenance_factor = LedMaintenanceFactor {
///     hours: 10000,
///     value: 0.85,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LedMaintenanceFactor {
    /// The number of hours corresponding to the maintenance factor.
    /// Handeled as an attribute in the XML.
    #[yaserde(rename = "hours", attribute)]
    #[serde(rename = "@hours")]
    pub hours: i32,

    /// The value of the maintenance factor.
    #[yaserde(textf64)]
    // #[yaserde(text)]
    #[serde(rename = "$")]
    // pub value: String,
    pub value: f64,
}
/// Represents maintenance information for a light source.
///
/// The `LightSourceMaintenance` struct models maintenance-related information for a light source. It includes
/// fields that provide details about the expected lifetime of the light source, the type of lamp as per the
/// CIE97 standard, maintenance factors specific to lamp types, LED maintenance factors, and lamp survival factors.
///
/// - The `lifetime` field specifies the expected lifetime of the light source in hours.
/// - The `cie97_lamp_type` field indicates the type of lamp according to the CIE97 standard.
/// - The `cie_lamp_maintenance_factors` field includes maintenance factors specific to lamp types.
/// - The `led_maintenance_factor` field provides information about the LED maintenance factor.
/// - The `lamp_survival_factor` field represents the lamp survival factor, which is the probability of
///   the lamp surviving beyond a specified time period.
///
/// Example usage:
/// ```
/// use gldf_rs::gldf::{LightSourceMaintenance, CieLampMaintenanceFactors, LedMaintenanceFactor};
///
/// let maintenance_info = LightSourceMaintenance {
///     lifetime: Some(20000),
///     cie97_lamp_type: Some("LED".to_string()),
///     cie_lamp_maintenance_factors: Some(CieLampMaintenanceFactors {
///         cie_lamp_maintenance_factor: vec![
///             // Fill in maintenance factor values for different lamp types
///         ],
///     }),
///     led_maintenance_factor: Some(LedMaintenanceFactor {
///         hours: 10000,
///         value: 0.85,
///     }),
///     lamp_survival_factor: Some(85),
/// };
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LightSourceMaintenance {
    /// The expected lifetime of the light source in hours.
    #[yaserde(rename = "lifetime", attribute)]
    #[serde(rename = "@lifetime")]
    pub lifetime: Option<i32>,

    /// The type of lamp as per the CIE97 standard.
    #[yaserde(rename = "Cie97LampType")]
    #[serde(rename = "Cie97LampType", skip_serializing_if = "Option::is_none")]
    pub cie97_lamp_type: Option<String>,

    /// Maintenance factors specific to lamp types.
    #[yaserde(rename = "CieLampMaintenanceFactors")]
    #[serde(rename = "CieLampMaintenanceFactors", skip_serializing_if = "Option::is_none")]
    pub cie_lamp_maintenance_factors: Option<CieLampMaintenanceFactors>,

    /// LED maintenance factor information.
    #[yaserde(rename = "LedMaintenanceFactor")]
    #[serde(rename = "LedMaintenanceFactor")]
    pub led_maintenance_factor: Option<LedMaintenanceFactor>,

    /// The lamp survival factor, indicating the probability of survival beyond a certain time period.
    #[yaserde(rename = "LampSurvivalFactor")]
    #[serde(rename = "LampSurvivalFactor", skip_serializing_if = "Option::is_none")]
    pub lamp_survival_factor: Option<i32>,
}

/// Represents a changeable light source in the GLDF data structure, conforming to ISO 7127:2017(E).
///
/// This struct defines the properties of a changeable light source that can be present
/// in a GLDF file. A changeable light source is one that can be exchanged or replaced
/// without altering the luminaire itself.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ChangeableLightSource {
    /// The unique identifier for the changeable light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.2.
    #[yaserde(rename = "id")]
    #[serde(rename = "id")]
    pub id: String,

    /// The localized name of the changeable light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.3.
    #[yaserde(child)]
    #[yaserde(rename = "Name")]
    #[serde(rename = "Name")]
    pub name: Locale,

    /// The localized description of the changeable light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.4.
    #[yaserde(rename = "Description")]
    #[serde(rename = "Description")]
    pub description: Locale,

    /// The manufacturer of the changeable light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.5.
    #[yaserde(rename = "Manufacturer")]
    #[serde(rename = "Manufacturer", skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,

    // Continue with similar documentation comments for each field...

    /// The photometric reference data associated with the changeable light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.15.
    #[yaserde(rename = "PhotometryReference")]
    #[serde(rename = "PhotometryReference", skip_serializing_if = "Option::is_none")]
    pub photometry_reference: Option<PhotometryReference>,

    /// Information about the maintenance of the light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.16.
    #[yaserde(rename = "LightSourceMaintenance")]
    #[serde(rename = "LightSourceMaintenance", skip_serializing_if = "Option::is_none")]
    pub light_source_maintenance: Option<LightSourceMaintenance>,
}


/// Represents a fixed light source in the GLDF data structure.
///
/// This struct defines the properties of a fixed light source that is permanently
/// integrated into the luminaire and cannot be exchanged or replaced.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FixedLightSource {
    /// Identifier for the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.2.1.
    #[yaserde(rename = "id", attribute)]
    #[serde(rename = "@id")]
    pub id: String,
    /// Name of the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.3.
    #[yaserde(rename = "Name")]
    #[serde(rename = "Name")]
    pub name: LocaleFoo,
    /// Description of the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.4.
    #[yaserde(rename = "Description")]
    #[serde(rename = "Description")]
    pub description: LocaleFoo,
    /// Manufacturer of the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.5.
    #[yaserde(rename = "Manufacturer")]
    #[serde(rename = "Manufacturer", skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    /// Global Trade Item Number (GTIN) of the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.6.
    #[yaserde(rename = "GTIN")]
    #[serde(rename = "GTIN", skip_serializing_if = "Option::is_none")]
    pub gtin: Option<String>,
    /// Rated input power of the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.7.
    #[yaserde(rename = "RatedInputPower")]
    #[serde(rename = "RatedInputPower", skip_serializing_if = "Option::is_none")]
    pub rated_input_power: Option<f64>,
    /// Rated input voltage of the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.8.
    #[yaserde(rename = "RatedInputVoltage")]
    #[serde(rename = "RatedInputVoltage", skip_serializing_if = "Option::is_none")]
    pub rated_input_voltage: Option<Voltage>,
    /// Power range of the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.9.
    #[yaserde(rename = "PowerRange")]
    #[serde(rename = "PowerRange", skip_serializing_if = "Option::is_none")]
    pub power_range: Option<PowerRange>,
    /// Position of usage of the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.10.
    #[yaserde(rename = "LightSourcePositionOfUsage")]
    #[serde(rename = "LightSourcePositionOfUsage", skip_serializing_if = "Option::is_none")]
    pub light_source_position_of_usage: Option<String>,
    /// Energy labels of the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.11.
    #[yaserde(rename = "EnergyLabels")]
    #[serde(rename = "EnergyLabels", skip_serializing_if = "Option::is_none")]
    pub energy_labels: Option<EnergyLabels>,
    /// Spectrum reference data of the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.12.
    #[yaserde(rename = "SpectrumReference")]
    #[serde(rename = "SpectrumReference", skip_serializing_if = "Option::is_none")]
    pub spectrum_reference: Option<SpectrumReference>,
    /// Active power table of the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.13.
    #[yaserde(rename = "ActivePowerTable")]
    #[serde(rename = "ActivePowerTable", skip_serializing_if = "Option::is_none")]
    pub active_power_table: Option<ActivePowerTable>,
    /// Color information of the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.14.
    #[yaserde(rename = "ColorInformation")]
    #[serde(rename = "ColorInformation", skip_serializing_if = "Option::is_none")]
    pub color_information: Option<ColorInformation>,
    /// Images of the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.14.
    #[yaserde(rename = "LightSourceImages")]
    #[serde(rename = "LightSourceImages", skip_serializing_if = "Option::is_none")]
    pub light_source_images: Option<Images>,
    /// Maintenance data of the fixed light source.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.14.
    #[yaserde(rename = "LightSourceMaintenance")]
    #[serde(rename = "LightSourceMaintenance", skip_serializing_if = "Option::is_none")]
    pub light_source_maintenance: Option<LightSourceMaintenance>,
    /// Indicates if the fixed light source adheres to the Zhaga standard.
    ///
    /// Corresponds to ISO 7127:2017(E) section 4.14.
    #[yaserde(rename = "ZhagaStandard")]
    #[serde(rename = "ZhagaStandard", skip_serializing_if = "Option::is_none")]
    pub zhaga_standard: Option<bool>,
}

/// Represents a reference to a spectrum.
///
/// The `SpectrumReference` struct models a reference to a spectrum within the GLDF file. It includes
/// the ID of the referenced spectrum. It supports serialization and deserialization of XML data
/// for working with spectrum references.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SpectrumReference {
    /// The ID of the referenced spectrum.
    #[yaserde(rename = "spectrumId")]
    #[serde(rename = "spectrumId")]
    pub spectrum_id: String,
}


/// Example of how to construct a `SpectrumReference` instance:
///
/// ```
/// use gldf_rs::gldf::SpectrumReference;
///
/// let spectrum_reference = SpectrumReference {
///     spectrum_id: "spectrum123".to_string(),
/// };
/// ```

/// Represents a collection of light sources in the GLDF data structure.
///
/// This struct holds both changeable and fixed light sources that are part of a luminaire.
/// It allows for modeling various light source configurations within a luminaire.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LightSources {
    /// A vector of changeable light sources present in the luminaire.
    ///
    /// Changeable light sources are those that can be exchanged or replaced without altering the luminaire itself.
    #[yaserde(child)]
    #[yaserde(rename = "ChangeableLightSource")]
    #[serde(rename = "ChangeableLightSource")]
    pub changeable_light_source: Vec<ChangeableLightSource>,

    /// A vector of fixed light sources integrated into the luminaire.
    ///
    /// Fixed light sources are permanently integrated into the luminaire and cannot be exchanged or replaced.
    #[yaserde(child)]
    #[yaserde(rename = "FixedLightSource")]
    #[serde(rename = "FixedLightSource")]
    pub fixed_light_source: Vec<FixedLightSource>,
}

/// Represents a reference to a light source in the GLDF data structure.
///
/// This struct holds references to fixed and changeable light sources by their IDs,
/// as well as the count of light sources associated with this reference.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LightSourceReference {
    /// The ID of the referenced fixed light source.
    #[yaserde(child)]
    #[yaserde(attribute, rename = "fixedLightSourceId")]
    #[serde(rename = "@fixedLightSourceId", skip_serializing_if = "Option::is_none")]
    pub fixed_light_source_id: Option<String>,

    /// The ID of the referenced changeable light source.
    #[yaserde(child)]
    #[yaserde(attribute, rename = "changeableLightSourceId")]
    #[serde(rename = "@changeableLightSourceId", skip_serializing_if = "Option::is_none")]
    pub changeable_light_source_id: Option<String>,

    /// The count of light sources associated with this reference.
    #[yaserde(child)]
    #[yaserde(attribute, rename = "lightSourceCount")]
    #[serde(rename = "@lightSourceCount", skip_serializing_if = "Option::is_none")]
    pub light_source_count: Option<i32>,
}
/// Represents a changeable light emitter in the GLDF data structure.
///
/// This struct defines the properties of a changeable light emitter that can be present
/// in a luminaire. A changeable light emitter is one that can be exchanged or replaced
/// without altering the luminaire itself.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ChangeableLightEmitter {
    /// The emergency behavior of the light emitter.
    #[yaserde(rename = "emergencyBehaviour")]
    #[serde(rename = "emergencyBehaviour")]
    pub emergency_behaviour: Option<String>,

    /// The localized name of the changeable light emitter.
    #[yaserde(rename = "Name")]
    #[serde(rename = "Name")]
    pub name: Locale,

    /// The rotation of the light emitter.
    #[yaserde(rename = "Rotation")]
    #[serde(rename = "Rotation", skip_serializing_if = "Option::is_none")]
    pub rotation: Option<Rotation>,

    /// The photometry reference associated with the light emitter.
    #[yaserde(rename = "PhotometryReference")]
    #[serde(rename = "PhotometryReference")]
    pub photometry_reference: PhotometryReference,

    /// The global rotation value G0.
    #[yaserde(rename = "G0")]
    #[serde(rename = "G0")]
    pub g0: String,
}

/// Represents a fixed light emitter in the GLDF data structure.
///
/// This struct defines the properties of a fixed light emitter that is permanently
/// integrated into the luminaire and cannot be exchanged or replaced.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FixedLightEmitter {
    /// The emergency behavior of the light emitter.
    #[yaserde(attribute, rename = "emergencyBehaviour")]
    #[serde(rename = "@emergencyBehaviour")]
    pub emergency_behaviour: Option<String>,

    /// The localized name of the fixed light emitter.
    #[yaserde(rename = "Name")]
    #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
    pub name: Option<LocaleFoo>,

    /// The rotation of the light emitter.
    #[yaserde(rename = "Rotation")]
    #[serde(rename = "Rotation", skip_serializing_if = "Option::is_none")]
    pub rotation: Option<Rotation>,

    /// The photometry reference associated with the light emitter.
    #[yaserde(rename = "PhotometryReference")]
    #[serde(rename = "PhotometryReference")]
    pub photometry_reference: PhotometryReference,

    /// The reference to the light source associated with the fixed light emitter.
    #[yaserde(rename = "LightSourceReference")]
    #[serde(rename = "LightSourceReference")]
    pub light_source_reference: LightSourceReference,

    /// The reference to the control gear associated with the fixed light emitter.
    #[yaserde(rename = "ControlGearReference")]
    #[serde(rename = "ControlGearReference", skip_serializing_if = "Option::is_none")]
    pub control_gear_reference: Option<ControlGearReference>,

    /// The rated luminous flux of the light emitter.
    #[yaserde(rename = "RatedLuminousFlux")]
    #[serde(rename = "RatedLuminousFlux", skip_serializing_if = "Option::is_none")]
    pub rated_luminous_flux: Option<i32>,

    /// The rated luminous flux in RGB of the light emitter.
    #[yaserde(rename = "RatedLuminousFluxRGB")]
    #[serde(rename = "RatedLuminousFluxRGB", skip_serializing_if = "Option::is_none")]
    pub rated_luminous_flux_rgb: Option<i32>,

    /// The emergency ballast lumen factor of the light emitter.
    #[yaserde(rename = "EmergencyBallastLumenFactor")]
    #[serde(rename = "EmergencyBallastLumenFactor", skip_serializing_if = "Option::is_none")]
    pub emergency_ballast_lumen_factor: Option<f64>,

    /// The emergency rated luminous flux of the light emitter.
    #[yaserde(rename = "EmergencyRatedLuminousFlux")]
    #[serde(rename = "EmergencyRatedLuminousFlux", skip_serializing_if = "Option::is_none")]
    pub emergency_rated_luminous_flux: Option<String>,
}

/// Represents an emitter in the GLDF data structure.
///
/// This struct defines a collection of changeable and fixed light emitters that are part of a luminaire.
/// It allows for modeling various light emitter configurations within a luminaire.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Emitter {
    /// The unique identifier of the emitter.
    #[yaserde(attribute, rename = "id")]
    #[serde(rename = "@id")]
    pub id: String,

    /// Collection of changeable light emitters.
    #[yaserde(rename = "ChangeableLightEmitter")]
    #[serde(rename = "ChangeableLightEmitter")]
    pub changeable_light_emitter: Vec<ChangeableLightEmitter>,

    /// Collection of fixed light emitters.
    #[yaserde(rename = "FixedLightEmitter")]
    #[serde(rename = "FixedLightEmitter")]
    pub fixed_light_emitter: Vec<FixedLightEmitter>,

    /// Collection of sensors associated with the emitter.
    #[yaserde(rename = "Sensor")]
    #[serde(rename = "Sensor")]
    pub sensor: Vec<Sensor>,
}

/// Represents a collection of emitters in the GLDF data structure.
///
/// This struct holds both changeable and fixed light emitters that are part of a luminaire.
/// It allows for modeling various emitter configurations within a luminaire.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Emitters {
    /// Collection of emitters.
    #[yaserde(rename = "Emitter")]
    #[serde(rename = "Emitter")]
    pub emitter: Vec<Emitter>,
}

/// Represents a rectangular emitter in the GLDF data structure.
///
/// This struct defines the properties of a rectangular emitter, including its width and length dimensions.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct RectangularEmitter {
    /// The width values for the rectangular emitter.
    #[yaserde(rename = "Width")]
    #[serde(rename = "Width")]
    pub width: Vec<i32>,

    /// The length values for the rectangular emitter.
    #[yaserde(rename = "Length")]
    #[serde(rename = "Length")]
    pub length: Vec<i32>,
}

/// Represents a circular emitter in the GLDF data structure.
///
/// This struct defines the properties of a circular emitter, including its diameter dimension.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CircularEmitter {
    /// The diameter values for the circular emitter.
    #[yaserde(rename = "Diameter")]
    #[serde(rename = "Diameter")]
    pub diameter: Vec<i32>,
}
