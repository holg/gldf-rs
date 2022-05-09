use serde::Serialize;
use serde::Deserialize;
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
#[yaserde(rename = "Root")]
pub struct GldfProduct {
  #[yaserde(rename = "Header")]
  #[yaserde(child)]
  pub header: Header,
  #[yaserde(child)]
  #[yaserde(rename = "GeneralDefinitions")]
  pub general_definitions: GeneralDefinitions,
  #[yaserde(child)]
  #[yaserde(rename = "ProductDefinitions")]
  pub product_definitions: ProductDefinitions,
}

// LicenseKey ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
#[yaserde(rename = "LicenseKey")]
pub struct LicenseKey {
  #[yaserde(attribute)]
  #[yaserde(rename = "application")]
  pub application: String,
  #[yaserde(text)]
  pub license_key: String,
}

// LicenseKeys ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LicenseKeys {
  #[yaserde(child)]
  #[yaserde(rename = "LicenseKey")]
  pub license_key: Vec<LicenseKey>,
}

// EMail ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EMail {
  #[yaserde(attribute)]
  #[yaserde(rename = "mailto")]
  pub mailto: String,
  #[yaserde(text)]
  pub value: String,
}

// EMailAddresses ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EMailAddresses {
  #[yaserde(child)]
  #[yaserde(rename = "EMail")]
  pub e_mail: Vec<EMail>,
}

// Address ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Address {
  #[yaserde(rename = "FirstName")]
  pub first_name: String,
  #[yaserde(rename = "Name")]
  pub name: String,
  #[yaserde(rename = "Street")]
  pub street: String,
  // #[yaserde(skip_serializing_if="Option::is_none")] // TODO enable this
  // #[yaserde(rename = "Number")]
  // pub number: Option<String>,
  #[yaserde(rename = "ZIPCode")]
  pub zip_code: String,
  #[yaserde(rename = "City")]
  pub city: String,
  #[yaserde(rename = "Country")]
  pub country: String,
  #[yaserde(rename = "Phone")]
  pub phone: String,
  #[yaserde(child)]
  #[yaserde(rename = "EMailAddresses")]
  pub e_mail_addresses: EMailAddresses,
  // #[yaserde(skip_serializing_if="Option::is_none")] // TODO enable this
  // #[yaserde(rename = "Websites")]
  // pub websites: Option<Hyperlinks>,
  // #[yaserde(skip_serializing_if="Option::is_none")]
  // #[yaserde(rename = "AdditionalInfo")]
  // pub additional_info: Option<String>,
}

// Contact ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Contact {
  #[yaserde(child)]
  #[yaserde(rename = "Address")]
  pub address: Vec<Address>,
}

// Header is Software used to create this file
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Header {
  #[yaserde(rename = "Author")]
  pub author: String,
  #[yaserde(rename = "Manufacturer")]
  pub manufacturer: String,
  #[yaserde(rename = "CreationTimeCode")]
  pub creation_time_code: String,
  #[yaserde(rename = "CreatedWithApplication")]
  pub created_with_application: String,
  #[yaserde(rename = "FormatVersion")]
  pub format_version: String,
  #[yaserde(rename = "DefaultLanguage")]
  pub default_language: Option<String>,
  #[yaserde(rename = "LicenseKeys")]
  #[yaserde(child)]
  pub license_keys: Option<LicenseKeys>,
  #[yaserde(rename = "ReluxMemberId")]
  pub relux_member_id: Option<String>,
  #[yaserde(rename = "DIALuxMemberId")]
  pub dia_lux_member_id: Option<String>,
  #[yaserde(child)]
  #[yaserde(rename = "Contact")]
  pub contact: Contact,
}

// GeneralDefinitions ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct GeneralDefinitions {
  #[yaserde(child)]
  #[yaserde(rename = "Files")]
  pub files: Files,
  #[yaserde(child)]
  #[yaserde(rename = "Sensors")]
  pub sensors: Option<Sensors>,
  #[yaserde(child)]
  #[yaserde(rename = "Photometries")]
  pub photometries: Option<Photometries>,
  #[yaserde(child)]
  #[yaserde(rename = "Spectrums")]
  pub spectrums: Option<Spectrums>,
  #[yaserde(child)]
  #[yaserde(rename = "LightSources")]
  pub light_sources: Option<LightSources>,
  #[yaserde(child)]
  #[yaserde(rename = "ControlGears")]
  pub control_gears: Option<ControlGears>,
  #[yaserde(child)]
  #[yaserde(rename = "Equipments")]
  pub equipments: Option<Equipments>,
  #[yaserde(child)]
  #[yaserde(rename = "Emitters")]
  pub emitters: Option<Emitters>,
  #[yaserde(rename = "Geometries")]
  #[yaserde(child)]
  pub geometries: Option<Geometries>,
}

// ProductDefinitions ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductDefinitions {
  #[yaserde(child)]
  #[yaserde(rename = "ProductMetaData")]
  pub product_meta_data: Option<ProductMetaData>,
  #[yaserde(child)]
  #[yaserde(rename = "Variants")]
  pub variants: Option<Variants>,
}

// File ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct File {
  #[yaserde(attribute)]
  #[yaserde(rename = "id")]
  pub id: String,
  #[yaserde(attribute)]
  #[yaserde(rename = "contentType")]
  pub content_type: String,
  #[yaserde(attribute)]
  #[yaserde(rename = "type")]
  pub type_attr: String,
  #[yaserde(attribute)]
  #[yaserde(rename = "language")]
  pub language: Option<String>,
  #[yaserde(text)]
  pub value: String,
}

// Files ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Files {
  #[yaserde(child)]
  #[yaserde(rename = "File")]
  pub file: Vec<File>,
}

// SensorFileReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SensorFileReference {
  #[yaserde(rename = "fileId")]
  pub file_id: String,
}

// DetectorCharacteristics ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DetectorCharacteristics {
  #[yaserde(rename = "DetectorCharacteristic")]
  pub detector_characteristic: Vec<String>,
}

// DetectionMethods ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DetectionMethods {
  #[yaserde(rename = "DetectionMethod")]
  pub detection_method: Vec<String>,
}

// DetectorTypes ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DetectorTypes {
  #[yaserde(rename = "DetectorType")]
  pub detector_type: Vec<String>,
}

// Sensor ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Sensor {
  #[yaserde(rename = "id")]
  pub id: String,
  #[yaserde(child)]
  #[yaserde(rename = "SensorFileReference")]
  pub sensor_file_reference: SensorFileReference,
  #[yaserde(child)]
  #[yaserde(rename = "DetectorCharacteristics")]
  pub detector_characteristics: DetectorCharacteristics,
  #[yaserde(child)]
  #[yaserde(rename = "DetectionMethods")]
  pub detection_methods: DetectionMethods,
  #[yaserde(child)]
  #[yaserde(rename = "DetectorTypes")]
  pub detector_types: DetectorTypes,
}

// Sensors ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Sensors {
  #[yaserde(child)]
  #[yaserde(rename = "Sensor")]
  pub sensor: Vec<Sensor>,
}

// PhotometryFileReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PhotometryFileReference {
  #[yaserde(rename = "fileId")]
  pub file_id: String,
}

// TenthPeakDivergence ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct TenthPeakDivergence {
  #[yaserde(rename = "C0-C180")]
  pub c0_c180: f64,
  #[yaserde(rename = "C90-C270")]
  pub c90_c270: f64,
}

// HalfPeakDivergence ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct HalfPeakDivergence {
  #[yaserde(rename = "C0-C180")]
  pub c0_c180: f64,
  #[yaserde(rename = "C90-C270")]
  pub c90_c270: f64,
}

// UGR4H8H705020LQ ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct UGR4H8H705020LQ {
  #[yaserde(rename = "X")]
  pub x: f64,
  #[yaserde(rename = "Y")]
  pub y: f64,
}

// DescriptivePhotometry ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DescriptivePhotometry {
  #[yaserde(rename = "LuminaireLuminance")]
  pub luminaire_luminance: Option<i32>,
  #[yaserde(rename = "LightOutputRatio")]
  pub light_output_ratio: Option<f64>,
  #[yaserde(rename = "LuminousEfficacy")]
  pub luminous_efficacy: Option<f64>,
  #[yaserde(rename = "DownwardFluxFraction")]
  pub downward_flux_fraction: Option<f64>,
  #[yaserde(rename = "DownwardLightOutputRatio")]
  pub downward_light_output_ratio: Option<f64>,
  #[yaserde(rename = "UpwardLightOutputRatio")]
  pub upward_light_output_ratio: Option<f64>,
  #[yaserde(rename = "TenthPeakDivergence")]
  pub tenth_peak_divergence: Option<TenthPeakDivergence>,
  #[yaserde(rename = "HalfPeakDivergence")]
  pub half_peak_divergence: Option<HalfPeakDivergence>,
  #[yaserde(rename = "PhotometricCode")]
  pub photometric_code: Option<String>,
  #[yaserde(rename = "CIE-FluxCode")]
  pub cie_flux_code: Option<String>,
  #[yaserde(rename = "CutOffAngle")]
  pub cut_off_angle: Option<f64>,
  #[yaserde(rename = "UGR-4H8H-70-50-20-LQ")]
  pub ugr4_h8_h705020_lq: Option<UGR4H8H705020LQ>,
  #[yaserde(rename = "IESNA-LightDistributionDefinition")]
  pub iesna_light_distribution_definition: Option<String>,
  #[yaserde(rename = "LightDistributionBUG-Rating")]
  pub light_distribution_bug_rating: Option<String>,
}

// Photometry ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Photometry {
  #[yaserde(rename = "id")]
  pub id: String,
  #[yaserde(rename = "PhotometryFileReference")]
  pub photometry_file_reference: PhotometryFileReference,
  #[yaserde(rename = "DescriptivePhotometry")]
  pub descriptive_photometry: DescriptivePhotometry,
}

// Photometries ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Photometries {
  #[yaserde(rename = "Photometry")]
  pub photometry: Vec<Photometry>,
}

// SpectrumFileReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SpectrumFileReference {
  #[yaserde(rename = "fileId")]
  pub file_id: String,
}

// Intensity ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Intensity {
  #[yaserde(rename = "wavelength")]
  pub wavelength: i32,
  #[yaserde(rename = "$value")]
  pub value: f64,
}

// Spectrum ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Spectrum {
  #[yaserde(rename = "id")]
  pub id: String,
  #[yaserde(rename = "SpectrumFileReference")]
  pub spectrum_file_reference: SpectrumFileReference,
  #[yaserde(rename = "Intensity")]
  pub intensity: Vec<Intensity>,
}

// Spectrums ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Spectrums {
  #[yaserde(rename = "Spectrum")]
  pub spectrum: Vec<Spectrum>,
}

// PowerRange ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PowerRange {
  #[yaserde(rename = "Lower")]
  pub lower: f64,
  #[yaserde(rename = "Upper")]
  pub upper: f64,
  #[yaserde(rename = "DefaultLightSourcePower")]
  pub default_light_source_power: f64,
}

// SpectrumReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SpectrumReference {
  #[yaserde(rename = "spectrumId")]
  pub spectrum_id: String,
}

// FluxFactor ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FluxFactor {
  #[yaserde(rename = "inputPower")]
  pub input_power: String,
  #[yaserde(rename = "flickerPstLM")]
  pub flicker_pst_lm: String,
  #[yaserde(rename = "stroboscopicEffectsSVM")]
  pub stroboscopic_effects_svm: String,
  #[yaserde(rename = "description")]
  pub description: String,
  #[yaserde(attribute)]
  pub value: f64,
}

// ActivePowerTable ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ActivePowerTable {
  #[yaserde(rename = "type")]
  pub type_attr: String,
  #[yaserde(rename = "DefaultLightSourcePower")]
  pub default_light_source_power: String,
}

// ColorTemperatureAdjustingRange ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ColorTemperatureAdjustingRange {
  #[yaserde(rename = "Lower")]
  pub lower: i32,
  #[yaserde(rename = "Upper")]
  pub upper: i32,
}

// Cie1931ColorAppearance ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Cie1931ColorAppearance {
  #[yaserde(rename = "X")]
  pub x: f64,
  #[yaserde(rename = "Y")]
  pub y: f64,
  #[yaserde(rename = "Z")]
  pub z: f64,
}

// RatedChromacityCoordinateValues ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct RatedChromacityCoordinateValues {
  #[yaserde(rename = "X")]
  pub x: f64,
  #[yaserde(rename = "Y")]
  pub y: f64,
}

// IESTM3015 ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct IESTM3015 {
  #[yaserde(rename = "Rf")]
  pub rf: i32,
  #[yaserde(rename = "Rg")]
  pub rg: i32,
}

// ColorInformation ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ColorInformation {
  #[yaserde(rename = "ColorRenderingIndex")]
  pub color_rendering_index: i32,
  #[yaserde(rename = "CorrelatedColorTemperature")]
  pub correlated_color_temperature: i32,
  #[yaserde(rename = "ColorTemperatureAdjustingRange")]
  pub color_temperature_adjusting_range: ColorTemperatureAdjustingRange,
  #[yaserde(rename = "Cie1931ColorAppearance")]
  pub cie1931_color_appearance: Cie1931ColorAppearance,
  #[yaserde(rename = "InitialColorTolerance")]
  pub initial_color_tolerance: String,
  #[yaserde(rename = "MaintainedColorTolerance")]
  pub maintained_color_tolerance: String,
  #[yaserde(rename = "RatedChromacityCoordinateValues")]
  pub rated_chromacity_coordinate_values: RatedChromacityCoordinateValues,
  #[yaserde(rename = "TLCI")]
  pub tlci: i32,
  #[yaserde(rename = "IES-TM-30-15")]
  pub iestm3015: IESTM3015,
  #[yaserde(rename = "MelanopicFactor")]
  pub melanopic_factor: f64,
}

// PhotometryReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PhotometryReference {
  #[yaserde(rename = "photometryId")]
  pub photometry_id: String,
}

// CieLampMaintenanceFactor ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CieLampMaintenanceFactor {
  #[yaserde(rename = "burningTime")]
  pub burning_time: String,
  #[yaserde(rename = "LampLumenMaintenanceFactor")]
  pub lamp_lumen_maintenance_factor: f64,
  #[yaserde(rename = "LampSurvivalFactor")]
  pub lamp_survival_factor: i32,
}

// CieLampMaintenanceFactors ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CieLampMaintenanceFactors {
  #[yaserde(rename = "CieLampMaintenanceFactor")]
  pub cie_lamp_maintenance_factor: Vec<CieLampMaintenanceFactor>,
}

// LedMaintenanceFactor ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LedMaintenanceFactor {
  #[yaserde(rename = "hours")]
  pub hours: String,
  #[yaserde(attribute)]
  pub value: f64,
}

// LightSourceMaintenance ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LightSourceMaintenance {
  #[yaserde(rename = "lifetime")]
  pub lifetime: Option<String>,
  #[yaserde(rename = "Cie97LampType")]
  pub cie97_lamp_type: Option<String>,
  #[yaserde(rename = "CieLampMaintenanceFactors")]
  pub cie_lamp_maintenance_factors: Option<CieLampMaintenanceFactors>,
  #[yaserde(rename = "LampSurvivalFactor")]
  pub lamp_survival_factor: Option<i32>,
}

// ChangeableLightSource ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ChangeableLightSource {
  #[yaserde(rename = "id")]
  pub id: String,
  #[yaserde(child)]
  #[yaserde(rename = "Name")]
  pub name: Locale,
  #[yaserde(rename = "Description")]
  pub description: Locale,
  #[yaserde(rename = "Manufacturer")]
  pub manufacturer: String,
  #[yaserde(rename = "GTIN")]
  pub gtin: String,
  #[yaserde(rename = "RatedInputPower")]
  pub rated_input_power: f64,
  #[yaserde(rename = "RatedInputVoltage")]
  pub rated_input_voltage: Voltage,
  #[yaserde(rename = "PowerRange")]
  pub power_range: PowerRange,
  #[yaserde(rename = "LightSourcePositionOfUsage")]
  pub light_source_position_of_usage: String,
  #[yaserde(rename = "EnergyLabels")]
  pub energy_labels: EnergyLabels,
  #[yaserde(rename = "SpectrumReference")]
  pub spectrum_reference: SpectrumReference,
  #[yaserde(rename = "ActivePowerTable")]
  pub active_power_table: ActivePowerTable,
  #[yaserde(rename = "ColorInformation")]
  pub color_information: ColorInformation,
  #[yaserde(rename = "LightSourceImages")]
  pub light_source_images: Images,
  #[yaserde(rename = "ZVEI")]
  pub zvei: Option<String>,
  #[yaserde(rename = "Socket")]
  pub socket: Option<String>,
  #[yaserde(rename = "ILCOS")]
  pub ilcos: Option<String>,
  #[yaserde(rename = "RatedLuminousFlux")]
  pub rated_luminous_flux: Option<i32>,
  #[yaserde(rename = "RatedLuminousFlux>RGB")]
  pub rated_luminous_flux_rgb: Option<i32>,
  #[yaserde(rename = "PhotometryReference")]
  pub photometry_reference: PhotometryReference,
  #[yaserde(rename = "LightSourceMaintenance")]
  pub light_source_maintenance: LightSourceMaintenance,
}

// FixedLightSource ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FixedLightSource {
  #[yaserde(rename = "id")]
  pub id: String,
  #[yaserde(rename = "Name")]
  pub name: Locale,
  #[yaserde(rename = "Description")]
  pub description: Locale,
  #[yaserde(rename = "Manufacturer")]
  pub manufacturer: String,
  #[yaserde(rename = "GTIN")]
  pub gtin: String,
  #[yaserde(rename = "RatedInputPower")]
  pub rated_input_power: f64,
  #[yaserde(rename = "RatedInputVoltage")]
  pub rated_input_voltage: Voltage,
  #[yaserde(rename = "PowerRange")]
  pub power_range: PowerRange,
  #[yaserde(rename = "LightSourcePositionOfUsage")]
  pub light_source_position_of_usage: String,
  #[yaserde(rename = "EnergyLabels")]
  pub energy_labels: EnergyLabels,
  #[yaserde(rename = "SpectrumReference")]
  pub spectrum_reference: SpectrumReference,
  #[yaserde(rename = "ActivePowerTable")]
  pub active_power_table: ActivePowerTable,
  #[yaserde(rename = "ColorInformation")]
  pub color_information: ColorInformation,
  #[yaserde(rename = "LightSourceImages")]
  pub light_source_images: Images,
  #[yaserde(rename = "LightSourceMaintenance")]
  pub light_source_maintenance: Option<LightSourceMaintenance>,
  #[yaserde(rename = "ZhagaStandard")]
  pub zhaga_standard: Option<bool>,
}

// LightSources ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LightSources {
  #[yaserde(child)]
  #[yaserde(rename = "ChangeableLightSource")]
  pub changeable_light_source: Vec<ChangeableLightSource>,
  #[yaserde(child)]
  #[yaserde(rename = "FixedLightSource")]
  pub fixed_light_source: Vec<FixedLightSource>,
}

// Interfaces ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Interfaces {
  #[yaserde(rename = "Interface")]
  pub interface: Vec<String>,
}

// ControlGear ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ControlGear {
  #[yaserde(rename = "id")]
  pub id: String,
  #[yaserde(rename = "Name")]
  pub name: Locale,
  #[yaserde(child)]
  #[yaserde(rename = "Description")]
  pub description: Locale,
  #[yaserde(child)]
  #[yaserde(rename = "NominalVoltage")]
  pub nominal_voltage: Voltage,
  #[yaserde(rename = "StandbyPower")]
  pub standby_power: f64,
  #[yaserde(rename = "ConstantLightOutputStartPower")]
  pub constant_light_output_start_power: f64,
  #[yaserde(rename = "ConstantLightOutputEndPower")]
  pub constant_light_output_end_power: f64,
  #[yaserde(rename = "PowerConsumptionControls")]
  pub power_consumption_controls: f64,
  #[yaserde(rename = "Dimmable")]
  pub dimmable: bool,
  #[yaserde(rename = "ColorControllable")]
  pub color_controllable: bool,
  #[yaserde(child)]
  #[yaserde(rename = "Interfaces")]
  pub interfaces: Interfaces,
  #[yaserde(child)]
  #[yaserde(rename = "EnergyLabels")]
  pub energy_labels: EnergyLabels,
}

// ControlGears ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ControlGears {
  #[yaserde(child)]
  #[yaserde(rename = "ControlGear")]
  pub control_gear: Vec<ControlGear>,
}

// LightSourceReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LightSourceReference {
  #[yaserde(child)]
  #[yaserde(rename = "changeableLightSourceId")]
  pub changeable_light_source_id: String,
  #[yaserde(child)]
  #[yaserde(rename = "lightSourceCount")]
  pub light_source_count: Option<i32>,
}

// ControlGearReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ControlGearReference {
  #[yaserde(rename = "controlGearId")]
  pub control_gear_id: String,
  #[yaserde(rename = "controlGearCount")]
  pub control_gear_count: i32,
}

// Equipment ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Equipment {
  #[yaserde(rename = "id")]
  pub id: String,
  #[yaserde(rename = "LightSourceReference")]
  pub light_source_reference: LightSourceReference,
  #[yaserde(rename = "ControlGearReference")]
  pub control_gear_reference: ControlGearReference,
  #[yaserde(rename = "RatedInputPower")]
  pub rated_input_power: f64,
  #[yaserde(rename = "EmergencyBallastLumenFactor")]
  pub emergency_ballast_lumen_factor: f64,
  #[yaserde(rename = "EmergencyRatedLuminousFlux")]
  pub emergency_rated_luminous_flux: i32,
}

// Equipments ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Equipments {
  #[yaserde(rename = "Equipment")]
  pub equipment: Vec<Equipment>,
}

// Rotation ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Rotation {
  #[yaserde(rename = "X")]
  pub x: i32,
  #[yaserde(rename = "Y")]
  pub y: i32,
  #[yaserde(rename = "Z")]
  pub z: i32,
  #[yaserde(rename = "G0")]
  pub g0: i32,
}

// EquipmentReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EquipmentReference {
  #[yaserde(rename = "equipmentId")]
  pub equipment_id: String,
}

// ChangeableLightEmitter ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ChangeableLightEmitter {
  #[yaserde(rename = "emergencyBehaviour")]
  pub emergency_behaviour: Option<String>,
  #[yaserde(rename = "Name")]
  pub name: Locale,
  #[yaserde(rename = "Rotation")]
  pub rotation: Rotation,
  #[yaserde(rename = "PhotometryReference")]
  pub photometry_reference: PhotometryReference,
  #[yaserde(rename = "G0")]
  pub g0: String,
}

// FixedLightEmitter ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FixedLightEmitter {
  #[yaserde(rename = "emergencyBehaviour")]
  pub emergency_behaviour: Option<String>,
  #[yaserde(rename = "Name")]
  pub name: Locale,
  #[yaserde(rename = "Rotation")]
  pub rotation: Rotation,
  #[yaserde(rename = "PhotometryReference")]
  pub photometry_reference: PhotometryReference,
  #[yaserde(rename = "LightSourceReference")]
  pub light_source_reference: LightSourceReference,
  #[yaserde(rename = "ControlGearReference")]
  pub control_gear_reference: ControlGearReference,
  #[yaserde(rename = "RatedLuminousFlux")]
  pub rated_luminous_flux: i32,
  #[yaserde(rename = "RatedLuminousFluxRGB")]
  pub rated_luminous_flux_rgb: i32,
  #[yaserde(rename = "EmergencyBallastLumenFactor")]
  pub emergency_ballast_lumen_factor: f64,
  #[yaserde(rename = "EmergencyRatedLuminousFlux")]
  pub emergency_rated_luminous_flux: String,
}

// Emitter ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Emitter {
  #[yaserde(rename = "id")]
  pub id: String,
  #[yaserde(rename = "ChangeableLightEmitter")]
  pub changeable_light_emitter: Vec<ChangeableLightEmitter>,
  #[yaserde(rename = "FixedLightEmitter")]
  pub fixed_light_emitter: Vec<FixedLightEmitter>,
  #[yaserde(rename = "Sensor")]
  pub sensor: Vec<Sensor>,
}

// Emitters ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Emitters {
  #[yaserde(rename = "Emitter")]
  pub emitter: Emitter,
}

// Cuboid ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Cuboid {
  #[yaserde(rename = "Width")]
  pub width: Vec<i32>,
  #[yaserde(rename = "Length")]
  pub length: Vec<i32>,
  #[yaserde(rename = "Height")]
  pub height: Vec<i32>,
}

// Cylinder ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Cylinder {
  #[yaserde(rename = "plane")]
  pub plane: String,
  #[yaserde(rename = "Diameter")]
  pub diameter: Vec<i32>,
  #[yaserde(rename = "Height")]
  pub height: Vec<String>,
}

// RectangularEmitter ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct RectangularEmitter {
  #[yaserde(rename = "Width")]
  pub width: Vec<i32>,
  #[yaserde(rename = "Length")]
  pub length: Vec<i32>,
}

// CircularEmitter ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CircularEmitter {
  #[yaserde(rename = "Diameter")]
  pub diameter: Vec<i32>,
}

// CHeights ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CHeights {
  #[yaserde(rename = "C0")]
  pub c0: Vec<i32>,
  #[yaserde(rename = "C90")]
  pub c90: Vec<i32>,
  #[yaserde(rename = "C180")]
  pub c180: Vec<i32>,
  #[yaserde(rename = "C270")]
  pub c270: Vec<i32>,
}

// SimpleGeometry ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SimpleGeometry {
  #[yaserde(rename = "id")]
  pub id: String,
  #[yaserde(rename = "Cuboid")]
  pub cuboid: Vec<Cuboid>,
  #[yaserde(rename = "Cylinder")]
  pub cylinder: Vec<Cylinder>,
  #[yaserde(rename = "RectangularEmitter")]
  pub rectangular_emitter: Vec<RectangularEmitter>,
  #[yaserde(rename = "CircularEmitter")]
  pub circular_emitter: Vec<CircularEmitter>,
  #[yaserde(rename = "C-Heights")]
  pub c_heights: Vec<CHeights>,
}

// GeometryFileReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct GeometryFileReference {
  #[yaserde(rename = "fileId")]
  pub file_id: String,
  #[yaserde(rename = "levelOfDetail")]
  pub level_of_detail: Option<String>,
}

// ModelGeometry ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ModelGeometry {
  #[yaserde(rename = "id")]
  pub id: String,
  #[yaserde(rename = "GeometryFileReference")]
  pub geometry_file_reference: Vec<GeometryFileReference>,
}

// Geometries ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Geometries {
  #[yaserde(rename = "SimpleGeometry")]
  pub simple_geometry: Vec<SimpleGeometry>,
  #[yaserde(rename = "ModelGeometry")]
  pub model_geometry: Vec<ModelGeometry>,
}

// LuminaireMaintenanceFactor ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LuminaireMaintenanceFactor {
  #[yaserde(rename = "years")]
  pub years: String,
  #[yaserde(rename = "roomCondition")]
  pub room_condition: String,
  #[yaserde(rename = "$value")]
  pub value: f64,
}

// CieLuminaireMaintenanceFactors ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CieLuminaireMaintenanceFactors {
  #[yaserde(rename = "LuminaireMaintenanceFactor")]
  pub luminaire_maintenance_factor: Vec<LuminaireMaintenanceFactor>,
}

// LuminaireDirtDepreciation ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LuminaireDirtDepreciation {
  #[yaserde(rename = "years")]
  pub years: i32,
  #[yaserde(rename = "roomCondition")]
  pub room_condition: String,
  #[yaserde(rename = "$value")]
  pub value: f64,
}

// IesLuminaireLightLossFactors ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct IesLuminaireLightLossFactors {
  #[yaserde(rename = "LuminaireDirtDepreciation")]
  pub luminaire_dirt_depreciation: Vec<LuminaireDirtDepreciation>,
}

// JiegMaintenanceFactors ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct JiegMaintenanceFactors {
  #[yaserde(rename = "LuminaireMaintenanceFactor")]
  pub luminaire_maintenance_factor: Vec<LuminaireMaintenanceFactor>,
}

// LuminaireMaintenance ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LuminaireMaintenance {
  #[yaserde(rename = "Cie97LuminaireType")]
  pub cie97_luminaire_type: String,
  #[yaserde(rename = "CieLuminaireMaintenanceFactors")]
  pub cie_luminaire_maintenance_factors: CieLuminaireMaintenanceFactors,
  #[yaserde(rename = "IesLuminaireLightLossFactors")]
  pub ies_luminaire_light_loss_factors: IesLuminaireLightLossFactors,
  #[yaserde(rename = "JiegMaintenanceFactors")]
  pub jieg_maintenance_factors: JiegMaintenanceFactors,
}

// ProductMetaData ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductMetaData {
  #[yaserde(rename = "ProductNumber")]
  pub product_number: Locale,
  #[yaserde(rename = "Name")]
  pub name: Locale,
  #[yaserde(rename = "Description")]
  pub description: Locale,
  #[yaserde(rename = "TenderText")]
  pub tender_text: Locale,
  #[yaserde(rename = "ProductSeries")]
  pub product_series: ProductSeries,
  #[yaserde(rename = "Pictures")]
  pub pictures: Images,
  #[yaserde(rename = "LuminaireMaintenance")]
  pub luminaire_maintenance: LuminaireMaintenance,
  #[yaserde(rename = "DescriptiveAttributes")]
  pub descriptive_attributes: DescriptiveAttributes,
}

// RectangularCutout ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct RectangularCutout {
  #[yaserde(rename = "Width")]
  pub width: i32,
  #[yaserde(rename = "Length")]
  pub length: i32,
  #[yaserde(rename = "Depth")]
  pub depth: i32,
}

// CircularCutout ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CircularCutout {
  #[yaserde(rename = "Diameter")]
  pub diameter: i32,
  #[yaserde(rename = "Depth")]
  pub depth: i32,
}

// Recessed ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Recessed {
  #[yaserde(rename = "recessedDepth")]
  pub recessed_depth: i32,
  #[yaserde(rename = "RectangularCutout")]
  pub rectangular_cutout: RectangularCutout,
  #[yaserde(rename = "Depth")]
  pub depth: i32,
}

// SurfaceMounted ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SurfaceMounted {}

// Pendant ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Pendant {
  #[yaserde(rename = "pendantLength")]
  pub pendant_length: f64,
}

// Ceiling ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Ceiling {
  #[yaserde(rename = "Recessed")]
  pub recessed: Recessed,
  #[yaserde(rename = "SurfaceMounted")]
  pub surface_mounted: SurfaceMounted,
  #[yaserde(rename = "Pendant")]
  pub pendant: Pendant,
}

// Wall ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Wall {
  #[yaserde(rename = "mountingHeight")]
  pub mounting_height: i32,
  #[yaserde(rename = "Recessed")]
  pub recessed: Recessed,
  #[yaserde(rename = "Depth")]
  pub depth: i32,
}

// FreeStanding ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FreeStanding {}

// WorkingPlane ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct WorkingPlane {
  #[yaserde(rename = "FreeStanding")]
  pub free_standing: FreeStanding,
}

// PoleTop ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PoleTop {
  #[yaserde(rename = "poleHeight")]
  pub pole_height: i32,
}

// PoleIntegrated ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PoleIntegrated {
  #[yaserde(rename = "poleHeight")]
  pub pole_height: i32,
}

// Ground ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Ground {
  #[yaserde(rename = "PoleTop")]
  pub pole_top: PoleTop,
  #[yaserde(rename = "PoleIntegrated")]
  pub pole_integrated: PoleIntegrated,
  #[yaserde(rename = "FreeStanding")]
  pub free_standing: FreeStanding,
  #[yaserde(rename = "SurfaceMounted")]
  pub surface_mounted: SurfaceMounted,
  #[yaserde(rename = "Recessed")]
  pub recessed: Recessed,
}

// Mountings ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Mountings {
  #[yaserde(rename = "Ceiling")]
  pub ceiling: Ceiling,
  #[yaserde(rename = "Wall")]
  pub wall: Wall,
  #[yaserde(rename = "WorkingPlane")]
  pub working_plane: WorkingPlane,
  #[yaserde(rename = "Ground")]
  pub ground: Ground,
}

// EmitterReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EmitterReference {
  #[yaserde(rename = "emitterId")]
  pub emitter_id: String,
}

// SimpleGeometryReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SimpleGeometryReference {
  #[yaserde(rename = "geometryId")]
  pub geometry_id: String,
  #[yaserde(rename = "emitterId")]
  pub emitter_id: String,
}

// ModelGeometryReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ModelGeometryReference {
  #[yaserde(rename = "geometryId")]
  pub geometry_id: String,
  #[yaserde(rename = "EmitterReference")]
  pub emitter_reference: Vec<EmitterReference>,
}

// GeometryReferences ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct GeometryReferences {
  #[yaserde(rename = "SimpleGeometryReference")]
  pub simple_geometry_reference: SimpleGeometryReference,
  #[yaserde(rename = "ModelGeometryReference")]
  pub model_geometry_reference: ModelGeometryReference,
}

// Geometry ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Geometry {
  #[yaserde(rename = "EmitterReference")]
  pub emitter_reference: EmitterReference,
  #[yaserde(rename = "SimpleGeometryReference")]
  pub simple_geometry_reference: SimpleGeometryReference,
  #[yaserde(rename = "ModelGeometryReference")]
  pub model_geometry_reference: ModelGeometryReference,
  #[yaserde(rename = "GeometryReferences")]
  pub geometry_references: GeometryReferences,
}

// Symbol ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Symbol {
  #[yaserde(rename = "fileId")]
  pub file_id: String,
}

// Variant ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Variant {
  #[yaserde(rename = "id")]
  pub id: String,
  #[yaserde(rename = "sortOrder")]
  pub sort_order: Option<i32>,
  #[yaserde(rename = "ProductNumber")]
  pub product_number: Locale,
  #[yaserde(rename = "Name")]
  pub name: Locale,
  #[yaserde(rename = "Description")]
  pub description: Locale,
  #[yaserde(rename = "TenderText")]
  pub tender_text: Locale,
  #[yaserde(rename = "GTIN")]
  pub gtin: String,
  #[yaserde(rename = "Mountings")]
  pub mountings: Mountings,
  #[yaserde(rename = "Geometry")]
  pub geometry: Geometry,
  #[yaserde(rename = "ProductSeries")]
  pub product_series: ProductSeries,
  #[yaserde(rename = "Pictures")]
  pub pictures: Images,
  #[yaserde(rename = "Symbol")]
  pub symbol: Symbol,
  #[yaserde(rename = "DescriptiveAttributes")]
  pub descriptive_attributes: DescriptiveAttributes,
}

// Variants ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Variants {
  #[yaserde(rename = "Variant")]
  pub variant: Vec<Variant>,
}

// ProductSize ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductSize {
  #[yaserde(rename = "Length")]
  pub length: i32,
  #[yaserde(rename = "Width")]
  pub width: i32,
  #[yaserde(rename = "Height")]
  pub height: i32,
}

// Adjustabilities ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Adjustabilities {
  #[yaserde(rename = "Adjustability")]
  pub adjustability: Vec<String>,
}

// ProtectiveAreas ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProtectiveAreas {
  #[yaserde(rename = "Area")]
  pub area: Vec<String>,
}

// Mechanical ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Mechanical {
  #[yaserde(rename = "ProductSize")]
  pub product_size: ProductSize,
  #[yaserde(rename = "ProductForm")]
  pub product_form: String,
  #[yaserde(rename = "SealingMaterial")]
  pub sealing_material: Locale,
  #[yaserde(rename = "Adjustabilities")]
  pub adjustabilities: Adjustabilities,
  #[yaserde(rename = "IKRating")]
  pub ik_rating: String,
  #[yaserde(rename = "ProtectiveAreas")]
  pub protective_areas: ProtectiveAreas,
  #[yaserde(rename = "Weight")]
  pub weight: f64,
}

// ClampingRange ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ClampingRange {
  #[yaserde(rename = "Lower")]
  pub lower: f64,
  #[yaserde(rename = "Upper")]
  pub upper: f64,
}

// Electrical ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Electrical {
  #[yaserde(rename = "ClampingRange")]
  pub clamping_range: ClampingRange,
  #[yaserde(rename = "SwitchingCapacity")]
  pub switching_capacity: String,
  #[yaserde(rename = "ElectricalSafetyClass")]
  pub electrical_safety_class: String,
  #[yaserde(rename = "IngressProtectionIPCode")]
  pub ingress_protection_ip_code: String,
  #[yaserde(rename = "PowerFactor")]
  pub power_factor: f64,
  #[yaserde(rename = "ConstantLightOutput")]
  pub constant_light_output: bool,
  #[yaserde(rename = "LightDistribution")]
  pub light_distribution: String,
}

// Flux ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Flux {
  #[yaserde(rename = "hours")]
  pub hours: i32,
  #[yaserde(rename = "$value")]
  pub value: i32,
}

// DurationTimeAndFlux ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DurationTimeAndFlux {
  #[yaserde(rename = "Flux")]
  pub flux: Vec<Flux>,
}

// Emergency ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Emergency {
  #[yaserde(rename = "DurationTimeAndFlux")]
  pub duration_time_and_flux: DurationTimeAndFlux,
  #[yaserde(rename = "DedicatedEmergencyLightingType")]
  pub dedicated_emergency_lighting_type: String,
}

// ListPrice ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ListPrice {
  #[yaserde(rename = "currency")]
  pub currency: String,
  #[yaserde(rename = "$value")]
  pub value: f64,
}

// ListPrices ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ListPrices {
  #[yaserde(rename = "ListPrice")]
  pub list_price: Vec<ListPrice>,
}

// HousingColor ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct HousingColor {
  #[yaserde(rename = "ral")]
  pub ral: Option<i32>,
  #[yaserde(flatten)]
  pub locale: Locale,
}

// HousingColors ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct HousingColors {
  #[yaserde(rename = "HousingColor")]
  pub housing_color: Vec<HousingColor>,
}

// Markets ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Markets {
  #[yaserde(rename = "Region")]
  pub region: Vec<Locale>,
}

// ApprovalMarks ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ApprovalMarks {
  #[yaserde(rename = "ApprovalMark")]
  pub approval_mark: Vec<String>,
}

// DesignAwards ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DesignAwards {
  #[yaserde(rename = "DesignAward")]
  pub design_award: Vec<String>,
}

// Labels ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Labels {
  #[yaserde(rename = "Label")]
  pub label: Vec<String>,
}

// Applications ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Applications {
  #[yaserde(rename = "Application")]
  pub application: Vec<String>,
}

// Marketing ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Marketing {
  #[yaserde(rename = "ListPrices")]
  pub list_prices: ListPrices,
  #[yaserde(rename = "HousingColors")]
  pub housing_colors: HousingColors,
  #[yaserde(rename = "Markets")]
  pub markets: Markets,
  #[yaserde(rename = "Hyperlinks")]
  pub hyperlinks: Hyperlinks,
  #[yaserde(rename = "Designer")]
  pub designer: String,
  #[yaserde(rename = "ApprovalMarks")]
  pub approval_marks: ApprovalMarks,
  #[yaserde(rename = "DesignAwards")]
  pub design_awards: DesignAwards,
  #[yaserde(rename = "Labels")]
  pub labels: Labels,
  #[yaserde(rename = "Applications")]
  pub applications: Applications,
}

// UsefulLifeTimes ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct UsefulLifeTimes {
  #[yaserde(rename = "UsefulLife")]
  pub useful_life: Vec<String>,
}

// MedianUsefulLifeTimes ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct MedianUsefulLifeTimes {
  #[yaserde(rename = "MedianUsefulLife")]
  pub median_useful_life: Vec<String>,
}

// Directives ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Directives {
  #[yaserde(rename = "Directive")]
  pub directive: Vec<String>,
}

// Classes ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Classes {
  #[yaserde(rename = "Class")]
  pub class: Vec<String>,
}

// Divisions ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Divisions {
  #[yaserde(rename = "Division")]
  pub division: Vec<String>,
}

// Gas ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Gas {
  #[yaserde(rename = "Group")]
  pub group: Vec<String>,
}

// Dust ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Dust {
  #[yaserde(rename = "Group")]
  pub group: Vec<String>,
}

// DivisionGroups ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DivisionGroups {
  #[yaserde(rename = "Gas")]
  pub gas: Gas,
  #[yaserde(rename = "Dust")]
  pub dust: Dust,
}

// Zones ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Zones {
  #[yaserde(rename = "Gas")]
  pub gas: Gas,
  #[yaserde(rename = "Dust")]
  pub dust: Dust,
}

// ZoneGroups ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ZoneGroups {
  #[yaserde(rename = "Gas")]
  pub gas: Gas,
  #[yaserde(rename = "Dust")]
  pub dust: Dust,
}

// TemperatureClasses ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct TemperatureClasses {
  #[yaserde(rename = "TemperatureClass")]
  pub temperature_class: Vec<String>,
}

// ExCodes ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ExCodes {
  #[yaserde(rename = "ExCode")]
  pub ex_code: Vec<String>,
}

// EquipmentProtectionLevels ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EquipmentProtectionLevels {
  #[yaserde(rename = "EquipmentProtectionLevel")]
  pub equipment_protection_level: Vec<String>,
}

// EquipmentGroups ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EquipmentGroups {
  #[yaserde(rename = "EquipmentGroup")]
  pub equipment_group: Vec<String>,
}

// EquipmentCategories ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EquipmentCategories {
  #[yaserde(rename = "EquipmentCategory")]
  pub equipment_category: Vec<String>,
}

// Atmospheres ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Atmospheres {
  #[yaserde(rename = "Atmosphere")]
  pub atmosphere: Vec<String>,
}

// Groups ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Groups {
  #[yaserde(rename = "Group")]
  pub group: Vec<String>,
}

// ATEX ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ATEX {
  #[yaserde(rename = "Directives")]
  pub directives: Directives,
  #[yaserde(rename = "Classes")]
  pub classes: Classes,
  #[yaserde(rename = "Divisions")]
  pub divisions: Divisions,
  #[yaserde(rename = "DivisionGroups")]
  pub division_groups: DivisionGroups,
  #[yaserde(rename = "Zones")]
  pub zones: Zones,
  #[yaserde(rename = "ZoneGroups")]
  pub zone_groups: ZoneGroups,
  #[yaserde(rename = "MaximumSurfaceTemperature")]
  pub maximum_surface_temperature: String,
  #[yaserde(rename = "TemperatureClasses")]
  pub temperature_classes: TemperatureClasses,
  #[yaserde(rename = "ExCodes")]
  pub ex_codes: ExCodes,
  #[yaserde(rename = "EquipmentProtectionLevels")]
  pub equipment_protection_levels: EquipmentProtectionLevels,
  #[yaserde(rename = "EquipmentGroups")]
  pub equipment_groups: EquipmentGroups,
  #[yaserde(rename = "EquipmentCategories")]
  pub equipment_categories: EquipmentCategories,
  #[yaserde(rename = "Atmospheres")]
  pub atmospheres: Atmospheres,
  #[yaserde(rename = "Groups")]
  pub groups: Groups,
}

// AbsorptionRate ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct AbsorptionRate {
  #[yaserde(rename = "hertz")]
  pub hertz: i32,
  #[yaserde(rename = "$value")]
  pub value: f64,
}

// AcousticAbsorptionRates ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct AcousticAbsorptionRates {
  #[yaserde(rename = "AbsorptionRate")]
  pub absorption_rate: Vec<AbsorptionRate>,
}

// OperationsAndMaintenance ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct OperationsAndMaintenance {
  #[yaserde(rename = "UsefulLifeTimes")]
  pub useful_life_times: UsefulLifeTimes,
  #[yaserde(rename = "MedianUsefulLifeTimes")]
  pub median_useful_life_times: MedianUsefulLifeTimes,
  #[yaserde(rename = "OperatingTemperature")]
  pub operating_temperature: TemperatureRange,
  #[yaserde(rename = "AmbientTemperature")]
  pub ambient_temperature: TemperatureRange,
  #[yaserde(rename = "RatedAmbientTemperature")]
  pub rated_ambient_temperature: i32,
  #[yaserde(rename = "ATEX")]
  pub atex: ATEX,
  #[yaserde(rename = "AcousticAbsorptionRates")]
  pub acoustic_absorption_rates: AcousticAbsorptionRates,
}

// FileReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FileReference {
  #[yaserde(rename = "fileId")]
  pub file_id: String,
}

// Property ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Property {
  #[yaserde(rename = "id")]
  pub id: String,
  #[yaserde(rename = "Name")]
  pub name: Locale,
  #[yaserde(rename = "PropertySource")]
  pub property_source: String,
  #[yaserde(rename = "Value")]
  pub value: String,
  #[yaserde(rename = "FileReference")]
  pub file_reference: FileReference,
}

// CustomProperties ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CustomProperties {
  #[yaserde(rename = "Property")]
  pub property:Vec<Property>,
}

// DescriptiveAttributes ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DescriptiveAttributes {
  #[yaserde(rename = "Mechanical")]
  pub mechanical: Option<Mechanical>,
  #[yaserde(rename = "Electrical")]
  pub electrical: Option<Electrical>,
  #[yaserde(rename = "Emergency")]
  pub emergency: Option<Emergency>,
  #[yaserde(rename = "Marketing")]
  pub marketing: Option<Marketing>,
  #[yaserde(rename = "OperationsAndMaintenance")]
  pub operations_and_maintenance: Option<OperationsAndMaintenance>,
  #[yaserde(rename = "CustomProperties")]
  pub custom_properties: Option<CustomProperties>,
}

// Locale ...
//#[yaserde_with::serde_as]
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Locale {
  #[yaserde(rename = "language")]
  pub language: String,
  #[yaserde(rename = "$value")]
  pub value: String,
}

// Hyperlink ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Hyperlink {
  #[yaserde(rename = "href")]
  pub href: String,
  #[yaserde(rename = "language")]
  pub language: Option<String>,
  #[yaserde(rename = "region")]
  pub region: Option<String>,
  #[yaserde(rename = "countryCode")]
  pub country_code: Option<String>,
  #[yaserde(rename = "$value")]
  pub value: String,
}

// Hyperlinks ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Hyperlinks {
  #[yaserde(rename = "Hyperlink")]
  pub hyperlink: Vec<Hyperlink>,
}

// EnergyLabel ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EnergyLabel {
  #[yaserde(rename = "region")]
  pub region: String,
  #[yaserde(rename = "$value")]
  pub value: String,
}

// EnergyLabels ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EnergyLabels {
  #[yaserde(rename = "EnergyLabel")]
  pub energy_label: Vec<EnergyLabel>,
}

// ProductSerie ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductSerie {
  #[yaserde(rename = "Name")]
  pub name: Locale,
  #[yaserde(rename = "Description")]
  pub description: Locale,
  #[yaserde(rename = "Pictures")]
  pub pictures: Images,
  #[yaserde(rename = "Hyperlinks")]
  pub hyperlinks: Hyperlinks,
}

// ProductSeries ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductSeries {
  #[yaserde(rename = "ProductSerie")]
  pub product_serie: Vec<ProductSerie>,
}

// Image ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Image {
  #[yaserde(rename = "fileId")]
  pub file_id: String,
  #[yaserde(rename = "imageType")]
  pub image_type: String,
}

// Images ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Images {
  #[yaserde(rename = "Image")]
  pub image: Vec<Image>,
}

// TemperatureRange ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct TemperatureRange {
  #[yaserde(rename = "Lower")]
  pub lower: i32,
  #[yaserde(rename = "Upper")]
  pub upper: i32,
}

// VoltageRange ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct VoltageRange {
  #[yaserde(rename = "Min")]
  pub min: f64,
  #[yaserde(rename = "Max")]
  pub max: f64,
}

// Voltage ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Voltage {
  #[yaserde(rename = "VoltageRange")]
  pub voltage_range: VoltageRange,
  #[yaserde(rename = "FixedVoltage")]
  pub fixed_voltage: f64,
  #[yaserde(rename = "Type")]
  pub type_attr: String,
  #[yaserde(rename = "Frequency")]
  pub frequency: String,
}

