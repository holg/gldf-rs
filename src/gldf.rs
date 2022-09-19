use serde::Serialize;
use serde::Deserialize;

fn get_xsnonamespaceschemalocation() -> String {
  "https://gldf.io/xsd/gldf/1.0.0-rc.1/gldf.xsd".to_string()
}
fn get_xmlns_xsi() -> String {
  "http://www.w3.org/2001/XMLSchema-instance".to_string()
}


#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
#[yaserde(strict, rename = "Root", root = "Root")]
#[serde(rename = "Root")]
//#[yaserde(namespace: "xsi: \"http://www.w3.org/2001/XMLSchema-instance\" xsi:noNamespaceSchemaLocation=\"https://gldf.io/xsd/gldf/1.0.0-rc.1/gldf.xsd\"") ]
pub struct GldfProduct {
  #[serde(skip_serializing, skip_deserializing)]
  #[yaserde(skip_serializing, skip_deserializing)]
  pub path: String,
  #[yaserde(attribute, rename = "xmlns:xsi", default="get_xmlns_xsi")]
  #[serde(rename = "@xmlns:xsi")]
  pub xmlns_xsi: String,
  #[serde(rename = "@xsi:noNamespaceSchemaLocation")]
  #[yaserde(attribute, rename = "xsi:noNamespaceSchemaLocation", default="get_xsnonamespaceschemalocation", prefix=xsi, text)]
  pub xsnonamespaceschemalocation: String,
  //"@xsi:noNamespaceSchemaLocation": "https://gldf.io/xsd/gldf/1.0.0-rc.1/gldf.xsd",
  #[yaserde(rename = "Header")]
  #[serde(rename = "Header")]
  pub header: Header,
  #[yaserde(child)]
  #[yaserde(rename = "GeneralDefinitions")]
  #[serde(rename = "GeneralDefinitions")]
  pub general_definitions: GeneralDefinitions,
  #[yaserde(child)]
  #[yaserde(rename = "ProductDefinitions")]
  #[serde(rename = "ProductDefinitions")]
  pub product_definitions: ProductDefinitions,
}

// LicenseKey ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
#[yaserde(rename = "LicenseKey")]
pub struct LicenseKey {
  #[yaserde(attribute)]
  #[yaserde(rename = "application")]
  #[serde(rename = "@application")]
  pub application: String,
  #[yaserde(text)]
  #[serde(rename = "$")]
  pub license_key: String,
}

// LicenseKeys ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LicenseKeys {
  #[yaserde(child)]
  #[yaserde(rename = "LicenseKey")]
  #[serde(rename = "LicenseKey")]
  pub license_key: Vec<LicenseKey>,
}

// EMail ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EMail {
  #[yaserde(attribute)]
  #[yaserde(rename = "mailto")]
  #[serde(rename = "@mailto")]
  pub mailto: String,
  #[yaserde(text)]
  #[serde(rename = "$")]
  pub value: String,
}

// EMailAddresses ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EMailAddresses {
  #[yaserde(child)]
  #[yaserde(rename = "EMail")]
  #[serde(rename = "EMail")]
  pub e_mail: Vec<EMail>,
}

// Address ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Address {
  #[yaserde(rename = "FirstName")]
  #[serde(rename = "FirstName")]
  pub first_name: String,
  #[yaserde(rename = "Name")]
  #[serde(rename = "Name")]
  pub name: String,
  #[yaserde(rename = "Street")]
  #[serde(rename = "Street")]
  pub street: String,
  // #[yaserde(skip_serializing_if="Option::is_none")] // TODO enable this
  // #[yaserde(rename = "Number")]
  // pub number: Option<String>,
  #[yaserde(rename = "ZIPCode")]
  #[serde(rename = "ZIPCode")]
  pub zip_code: String,
  #[yaserde(rename = "City")]
  #[serde(rename = "City")]
  pub city: String,
  #[yaserde(rename = "Country")]
  #[serde(rename = "Country")]
  pub country: String,
  #[yaserde(rename = "Phone")]
  #[serde(rename = "Phone")]
  pub phone: String,
  #[yaserde(child)]
  #[yaserde(rename = "EMailAddresses")]
  #[serde(rename = "EMailAddresses")]
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
  #[serde(rename = "Address")]
  pub address: Vec<Address>,
}

// Header is Software used to create this file
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Header {
  #[yaserde(rename = "Author")]
  #[serde(rename = "Author")]
  pub author: String,
  #[yaserde(rename = "Manufacturer")]
  #[serde(rename = "Manufacturer")]
  pub manufacturer: String,
  #[yaserde(rename = "CreationTimeCode")]
  #[serde(rename = "CreationTimeCode")]
  pub creation_time_code: String,
  #[yaserde(rename = "CreatedWithApplication")]
  #[serde(rename = "CreatedWithApplication")]
  pub created_with_application: String,
  #[yaserde(rename = "FormatVersion")]
  #[serde(rename = "FormatVersion")]
  pub format_version: String,
  #[yaserde(rename = "DefaultLanguage")]
  #[serde(rename = "DefaultLanguage")]
  pub default_language: Option<String>,
  #[yaserde(rename = "LicenseKeys")]
  #[serde(rename = "LicenseKeys")]
  #[yaserde(child)]
  pub license_keys: Option<LicenseKeys>,
  #[yaserde(rename = "ReluxMemberId")]
  #[serde(rename = "ReluxMemberId")]
  pub relux_member_id: Option<String>,
  #[yaserde(rename = "DIALuxMemberId")]
  #[serde(rename = "DIALuxMemberId")]
  pub dia_lux_member_id: Option<String>,
  #[yaserde(child)]
  #[yaserde(rename = "Contact")]
  #[serde(rename = "Contact")]
  pub contact: Contact,
}

// GeneralDefinitions ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct GeneralDefinitions {
  #[yaserde(child)]
  #[yaserde(rename = "Files")]
  #[serde(rename = "Files")]
  pub files: Files,
  #[yaserde(child)]
  #[yaserde(rename = "Sensors")]
  #[serde(rename = "Sensors", skip_serializing_if = "Option::is_none")]
  pub sensors: Option<Sensors>,
  #[yaserde(child)]
  #[yaserde(rename = "Photometries")]
  #[serde(rename = "Photometries")]
  pub photometries: Option<Photometries>,
  #[yaserde(child)]
  #[yaserde(rename = "Spectrums")]
  #[serde(rename = "Spectrums", skip_serializing_if = "Option::is_none")]
  pub spectrums: Option<Spectrums>,
  #[yaserde(child)]
  #[yaserde(rename = "LightSources")]
  #[serde(rename = "LightSources")]
  pub light_sources: Option<LightSources>,
  #[yaserde(child)]
  #[yaserde(rename = "ControlGears")]
  #[serde(rename = "ControlGears", skip_serializing_if = "Option::is_none")]
  pub control_gears: Option<ControlGears>,
  #[yaserde(child)]
  #[yaserde(rename = "Equipments")]
  #[serde(rename = "Equipments", skip_serializing_if = "Option::is_none")]
  pub equipments: Option<Equipments>,
  #[yaserde(child)]
  #[yaserde(rename = "Emitters")]
  #[serde(rename = "Emitters", skip_serializing_if = "Option::is_none")]
  pub emitters: Option<Emitters>,
  #[yaserde(rename = "Geometries")]
  #[serde(rename = "Geometries", skip_serializing_if = "Option::is_none")]
  #[yaserde(child)]
  pub geometries: Option<Geometries>,
}

// ProductDefinitions ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductDefinitions {
  #[yaserde(child)]
  #[yaserde(rename = "ProductMetaData")]
  #[serde(rename = "ProductMetaData")]
  pub product_meta_data: Option<ProductMetaData>,
  #[yaserde(child)]
  #[yaserde(rename = "Variants")]
  #[serde(rename = "Variants", skip_serializing_if = "Option::is_none")]
  pub variants: Option<Variants>,
}

// File ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct File {
  #[yaserde(attribute)]
  #[yaserde(rename = "id")]
  #[serde(rename = "@id")]
  pub id: String,
  #[yaserde(attribute)]
  #[yaserde(rename = "contentType")]
  #[serde(rename = "@contentType")]
  pub content_type: String,
  #[yaserde(attribute)]
  #[yaserde(rename = "type")]
  #[serde(rename = "@type")]
  pub type_attr: String,
  // #[yaserde(attribute)]
  // #[yaserde(rename = "language")]
  // #[serde(rename = "language")]
  // pub language: Option<String>,
  #[yaserde(text)]
  #[serde(rename = "$")]
  pub file_name: String,
}

// Files ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Files {
  #[yaserde(child)]
  #[yaserde(rename = "File")]
  #[serde(rename = "File")]
  pub file: Vec<File>,
}

// SensorFileReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SensorFileReference {
  #[yaserde(rename = "fileId")]
  #[serde(rename = "fileId")]
  pub file_id: String,
}

// DetectorCharacteristics ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DetectorCharacteristics {
  #[yaserde(rename = "DetectorCharacteristic")]
  #[serde(rename = "DetectorCharacteristic")]
  pub detector_characteristic: Vec<String>,
}

// DetectionMethods ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DetectionMethods {
  #[yaserde(rename = "DetectionMethod")]
  #[serde(rename = "DetectionMethod")]
  pub detection_method: Vec<String>,
}

// DetectorTypes ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DetectorTypes {
  #[yaserde(rename = "DetectorType")]
  #[serde(rename = "DetectorType")]
  pub detector_type: Vec<String>,
}

// Sensor ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Sensor {
  #[yaserde(rename = "id")]
  #[serde(rename = "id")]
  pub id: String,
  #[yaserde(child)]
  #[yaserde(rename = "SensorFileReference")]
  #[serde(rename = "SensorFileReference", skip_serializing_if = "Option::is_none")]
  pub sensor_file_reference: Option<SensorFileReference>,
  #[yaserde(child)]
  #[yaserde(rename = "DetectorCharacteristics")]
  #[serde(rename = "DetectorCharacteristics", skip_serializing_if = "Option::is_none")]
  pub detector_characteristics: Option<DetectorCharacteristics>,
  #[yaserde(child)]
  #[yaserde(rename = "DetectionMethods")]
  #[serde(rename = "DetectionMethods", skip_serializing_if = "Option::is_none")]
  pub detection_methods: Option<DetectionMethods>,
  #[yaserde(child)]
  #[yaserde(rename = "DetectorTypes")]
  #[serde(rename = "DetectorTypes", skip_serializing_if = "Option::is_none")]
  pub detector_types: Option<DetectorTypes>,
}

// Sensors ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Sensors {
  #[yaserde(child)]
  #[yaserde(rename = "Sensor")]
  #[serde(rename = "Sensor")]
  pub sensor: Vec<Sensor>,
}

// PhotometryFileReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PhotometryFileReference {
  #[yaserde(rename = "fileId", attribute)]
  #[serde(rename = "@fileId")]
  pub file_id: String,
}

// TenthPeakDivergence ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct TenthPeakDivergence {
  #[yaserde(rename = "C0-C180")]
  #[serde(rename = "C0-C180", skip_serializing_if = "Option::is_none")]
  pub c0_c180: Option<f64>,
  #[yaserde(rename = "C90-C270")]
  #[serde(rename = "C90-C270", skip_serializing_if = "Option::is_none")]
  pub c90_c270: Option<f64>,
}

// HalfPeakDivergence ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct HalfPeakDivergence {
  #[yaserde(rename = "C0-C180")]
  #[serde(rename = "C0-C180", skip_serializing_if = "Option::is_none")]
  pub c0_c180: Option<f64>,
  #[yaserde(rename = "C90-C270")]
  #[serde(rename = "C90-C270", skip_serializing_if = "Option::is_none")]
  pub c90_c270: Option<f64>,
}

// UGR4H8H705020LQ ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct UGR4H8H705020LQ {
  #[yaserde(rename = "X")]
  #[serde(rename = "X", skip_serializing_if = "Option::is_none")]
  pub x: Option<f64>,
  #[yaserde(rename = "Y")]
  #[serde(rename = "Y", skip_serializing_if = "Option::is_none")]
  pub y: Option<f64>,
}

// DescriptivePhotometry ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DescriptivePhotometry {
  #[yaserde(rename = "LuminaireLuminance")]
  #[serde(rename = "LuminaireLuminance", skip_serializing_if = "Option::is_none")]
  pub luminaire_luminance: Option<i32>,
  #[yaserde(rename = "LightOutputRatio")]
  #[serde(rename = "LightOutputRatio", skip_serializing_if = "Option::is_none")]
  pub light_output_ratio: Option<f64>,
  #[yaserde(rename = "LuminousEfficacy")]
  #[serde(rename = "LuminousEfficacy", skip_serializing_if = "Option::is_none")]
  pub luminous_efficacy: Option<f64>,
  #[yaserde(rename = "DownwardFluxFraction")]
  #[serde(rename = "DownwardFluxFraction", skip_serializing_if = "Option::is_none")]
  pub downward_flux_fraction: Option<f64>,
  #[yaserde(rename = "DownwardLightOutputRatio")]
  #[serde(rename = "DownwardLightOutputRatio", skip_serializing_if = "Option::is_none")]
  pub downward_light_output_ratio: Option<f64>,
  #[yaserde(rename = "UpwardLightOutputRatio")]
  #[serde(rename = "UpwardLightOutputRatio", skip_serializing_if = "Option::is_none")]
  pub upward_light_output_ratio: Option<f64>,
  #[yaserde(rename = "TenthPeakDivergence")]
  #[serde(rename = "TenthPeakDivergence", skip_serializing_if = "Option::is_none")]
  pub tenth_peak_divergence: Option<TenthPeakDivergence>,
  #[yaserde(rename = "HalfPeakDivergence")]
  #[serde(rename = "HalfPeakDivergence", skip_serializing_if = "Option::is_none")]
  pub half_peak_divergence: Option<HalfPeakDivergence>,
  #[yaserde(rename = "PhotometricCode")]
  #[serde(rename = "PhotometricCode", skip_serializing_if = "Option::is_none")]
  pub photometric_code: Option<String>,
  #[yaserde(rename = "CIE-FluxCode")]
  #[serde(rename = "CIE-FluxCode", skip_serializing_if = "Option::is_none")]
  pub cie_flux_code: Option<String>,
  #[yaserde(rename = "CutOffAngle")]
  #[serde(rename = "CutOffAngle", skip_serializing_if = "Option::is_none")]
  pub cut_off_angle: Option<f64>,
  #[yaserde(rename = "UGR-4H8H-70-50-20-LQ")]
  #[serde(rename = "UGR-4H8H-70-50-20-LQ", skip_serializing_if = "Option::is_none")]
  pub ugr4_h8_h705020_lq: Option<UGR4H8H705020LQ>,
  #[yaserde(rename = "IESNA-LightDistributionDefinition")]
  #[serde(rename = "IESNA-LightDistributionDefinition", skip_serializing_if = "Option::is_none")]
  pub iesna_light_distribution_definition: Option<String>,
  #[yaserde(rename = "LightDistributionBUG-Rating")]
  #[serde(rename = "LightDistributionBUG-Rating", skip_serializing_if = "Option::is_none")]
  pub light_distribution_bug_rating: Option<String>,
}

// Photometry ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Photometry {
  #[yaserde(rename = "id", attribute)]
  #[serde(rename = "@id")]
  pub id: String,
  #[yaserde(rename = "PhotometryFileReference")]
  #[serde(rename = "PhotometryFileReference", skip_serializing_if = "Option::is_none")]
  pub photometry_file_reference: Option<PhotometryFileReference>,
  #[yaserde(rename = "DescriptivePhotometry")]
  #[serde(rename = "DescriptivePhotometry", skip_serializing_if = "Option::is_none")]
  pub descriptive_photometry: Option<DescriptivePhotometry>,
}

// Photometries ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Photometries {
  #[yaserde(rename = "Photometry")]
  #[serde(rename = "Photometry")]
  pub photometry: Vec<Photometry>,
}

// SpectrumFileReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SpectrumFileReference {
  #[yaserde(rename = "fileId")]
  #[serde(rename = "fileId")]
  pub file_id: String,
}

// Intensity ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Intensity {
  #[yaserde(rename = "wavelength")]
  #[serde(rename = "wavelength", skip_serializing_if = "Option::is_none")]
  pub wavelength: Option<i32>,
  #[yaserde(rename = "$value")]
  #[serde(rename = "$value", skip_serializing_if = "Option::is_none")]
  pub value: Option<f64>,
}

// Spectrum ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Spectrum {
  #[yaserde(rename = "id")]
  #[serde(rename = "id")]
  pub id: String,
  #[yaserde(rename = "SpectrumFileReference")]
  #[serde(rename = "SpectrumFileReference")]
  pub spectrum_file_reference: SpectrumFileReference,
  #[yaserde(rename = "Intensity")]
  #[serde(rename = "Intensity")]
  pub intensity: Vec<Intensity>,
}

// Spectrums ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Spectrums {
  #[yaserde(rename = "Spectrum")]
  #[serde(rename = "Spectrum")]
  pub spectrum: Vec<Spectrum>,
}

// PowerRange ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PowerRange {
  #[yaserde(rename = "Lower")]
  #[serde(rename = "Lower")]
  pub lower: f64,
  #[yaserde(rename = "Upper")]
  #[serde(rename = "Upper")]
  pub upper: f64,
  #[yaserde(rename = "DefaultLightSourcePower")]
  #[serde(rename = "DefaultLightSourcePower")]
  pub default_light_source_power: f64,
}

// SpectrumReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SpectrumReference {
  #[yaserde(rename = "spectrumId")]
  #[serde(rename = "spectrumId")]
  pub spectrum_id: String,
}

// FluxFactor ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FluxFactor {
  #[yaserde(rename = "inputPower")]
  #[serde(rename = "inputPower")]
  pub input_power: String,
  #[yaserde(rename = "flickerPstLM")]
  #[serde(rename = "flickerPstLM")]
  pub flicker_pst_lm: String,
  #[yaserde(rename = "stroboscopicEffectsSVM")]
  #[serde(rename = "stroboscopicEffectsSVM")]
  pub stroboscopic_effects_svm: String,
  #[yaserde(rename = "description")]
  #[serde(rename = "description")]
  pub description: String,
  #[yaserde(attribute)]
  pub value: f64,
}

// ActivePowerTable ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ActivePowerTable {
  #[yaserde(rename = "type")]
  #[serde(rename = "type")]
  pub type_attr: String,
  #[yaserde(rename = "DefaultLightSourcePower")]
  #[serde(rename = "DefaultLightSourcePower")]
  pub default_light_source_power: String,
}

// ColorTemperatureAdjustingRange ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ColorTemperatureAdjustingRange {
  #[yaserde(rename = "Lower")]
  #[serde(rename = "Lower", skip_serializing_if = "Option::is_none")]
  pub lower: Option<i32>,
  #[yaserde(rename = "Upper")]
  #[serde(rename = "Upper", skip_serializing_if = "Option::is_none")]
  pub upper: Option<i32>,
}

// Cie1931ColorAppearance ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Cie1931ColorAppearance {
  #[yaserde(rename = "X")]
  #[serde(rename = "X", skip_serializing_if = "Option::is_none")]
  pub x: Option<f64>,
  #[yaserde(rename = "Y")]
  #[serde(rename = "Y", skip_serializing_if = "Option::is_none")]
  pub y: Option<f64>,
  #[yaserde(rename = "Z")]
  #[serde(rename = "Z", skip_serializing_if = "Option::is_none")]
  pub z: Option<f64>,
}

// RatedChromacityCoordinateValues ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct RatedChromacityCoordinateValues {
  #[yaserde(rename = "X")]
  #[serde(rename = "X")]
  pub x: f64,
  #[yaserde(rename = "Y")]
  #[serde(rename = "Y")]
  pub y: f64,
}

// IESTM3015 ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct IESTM3015 {
  #[yaserde(rename = "Rf")]
  #[serde(rename = "Rf")]
  pub rf: i32,
  #[yaserde(rename = "Rg")]
  #[serde(rename = "Rg")]
  pub rg: i32,
}

// ColorInformation ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ColorInformation {
  #[yaserde(rename = "ColorRenderingIndex")]
  #[serde(rename = "ColorRenderingIndex", skip_serializing_if = "Option::is_none")]
  pub color_rendering_index: Option<i32>,
  #[yaserde(rename = "CorrelatedColorTemperature")]
  #[serde(rename = "CorrelatedColorTemperature", skip_serializing_if = "Option::is_none")]
  pub correlated_color_temperature: Option<i32>,
  #[yaserde(rename = "ColorTemperatureAdjustingRange")]
  #[serde(rename = "ColorTemperatureAdjustingRange", skip_serializing_if = "Option::is_none")]
  pub color_temperature_adjusting_range: Option<ColorTemperatureAdjustingRange>,
  #[yaserde(rename = "Cie1931ColorAppearance")]
  #[serde(rename = "Cie1931ColorAppearance", skip_serializing_if = "Option::is_none")]
  pub cie1931_color_appearance: Option<Cie1931ColorAppearance>,
  #[yaserde(rename = "InitialColorTolerance")]
  #[serde(rename = "InitialColorTolerance", skip_serializing_if = "Option::is_none")]
  pub initial_color_tolerance: Option<String>,
  #[yaserde(rename = "MaintainedColorTolerance")]
  #[serde(rename = "MaintainedColorTolerance", skip_serializing_if = "Option::is_none")]
  pub maintained_color_tolerance: Option<String>,
  #[yaserde(rename = "RatedChromacityCoordinateValues")]
  #[serde(rename = "RatedChromacityCoordinateValues", skip_serializing_if = "Option::is_none")]
  pub rated_chromacity_coordinate_values: Option<RatedChromacityCoordinateValues>,
  #[yaserde(rename = "TLCI")]
  #[serde(rename = "TLCI", skip_serializing_if = "Option::is_none")]
  pub tlci: Option<i32>,
  #[yaserde(rename = "IES-TM-30-15")]
  #[serde(rename = "IES-TM-30-15", skip_serializing_if = "Option::is_none")]
  pub iestm3015: Option<IESTM3015>,
  #[yaserde(rename = "MelanopicFactor")]
  #[serde(rename = "MelanopicFactor", skip_serializing_if = "Option::is_none")]
  pub melanopic_factor: Option<f64>,
}

// PhotometryReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PhotometryReference {
  #[yaserde(attribute, rename = "photometryId")]
  #[serde(rename = "@photometryId")]
  pub photometry_id: String,
}

// CieLampMaintenanceFactor ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CieLampMaintenanceFactor {
  #[yaserde(rename = "burningTime")]
  #[serde(rename = "burningTime")]
  pub burning_time: String,
  #[yaserde(rename = "LampLumenMaintenanceFactor")]
  #[serde(rename = "LampLumenMaintenanceFactor")]
  pub lamp_lumen_maintenance_factor: f64,
  #[yaserde(rename = "LampSurvivalFactor")]
  #[serde(rename = "LampSurvivalFactor")]
  pub lamp_survival_factor: i32,
}

// CieLampMaintenanceFactors ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CieLampMaintenanceFactors {
  #[yaserde(rename = "CieLampMaintenanceFactor")]
  #[serde(rename = "CieLampMaintenanceFactor")]
  pub cie_lamp_maintenance_factor: Vec<CieLampMaintenanceFactor>,
}
fn get_f64_from_string(some:String) ->f64{
  return some.parse::<f64>().unwrap();
}

// LedMaintenanceFactor ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LedMaintenanceFactor {
  #[yaserde(rename = "hours", attribute)]
  #[serde(rename = "@hours")]
  pub hours: i32,
  #[yaserde(text, rename="$value")]
  #[serde(rename = "$")] // , deserialize_with="%r.as_f64")] //, getter = "get_f64_from_string")]
  pub value: String, //# TODO this shall be f64, but yaserde(text) must be it seems
}
// LightSourceMaintenance ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LightSourceMaintenance {
  #[yaserde(rename = "lifetime", attribute)]
  #[serde(rename = "@lifetime")]
  pub lifetime: Option<i32>,
  #[yaserde(rename = "Cie97LampType")]
  #[serde(rename = "Cie97LampType", skip_serializing_if = "Option::is_none")]
  pub cie97_lamp_type: Option<String>,
  #[yaserde(rename = "CieLampMaintenanceFactors")]
  #[serde(rename = "CieLampMaintenanceFactors", skip_serializing_if = "Option::is_none")]
  pub cie_lamp_maintenance_factors: Option<CieLampMaintenanceFactors>,
  #[yaserde(rename = "LedMaintenanceFactor")]
  #[serde(rename = "LedMaintenanceFactor")]
  pub led_maintenance_factor: Option<LedMaintenanceFactor>,
  #[yaserde(rename = "LampSurvivalFactor")]
  #[serde(rename = "LampSurvivalFactor", skip_serializing_if = "Option::is_none")]
  pub lamp_survival_factor: Option<i32>,
}

// ChangeableLightSource ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ChangeableLightSource {
  #[yaserde(rename = "id")]
  #[serde(rename = "id")]
  pub id: String,
  #[yaserde(child)]
  #[yaserde(rename = "Name")]
  #[serde(rename = "Name")]
  pub name: Locale,
  #[yaserde(rename = "Description")]
  #[serde(rename = "Description")]
  pub description: Locale,
  #[yaserde(rename = "Manufacturer")]
  #[serde(rename = "Manufacturer", skip_serializing_if = "Option::is_none")]
  pub manufacturer: Option<String>,
  #[yaserde(rename = "GTIN")]
  #[serde(rename = "GTIN", skip_serializing_if = "Option::is_none")]
  pub gtin: Option<String>,
  #[yaserde(rename = "RatedInputPower")]
  #[serde(rename = "RatedInputPower", skip_serializing_if = "Option::is_none")]
  pub rated_input_power: Option<f64>,
  #[yaserde(rename = "RatedInputVoltage")]
  #[serde(rename = "RatedInputVoltage", skip_serializing_if = "Option::is_none")]
  pub rated_input_voltage: Option<Voltage>,
  #[yaserde(rename = "PowerRange")]
  #[serde(rename = "PowerRange", skip_serializing_if = "Option::is_none")]
  pub power_range: Option<PowerRange>,
  #[yaserde(rename = "LightSourcePositionOfUsage")]
  #[serde(rename = "LightSourcePositionOfUsage", skip_serializing_if = "Option::is_none")]
  pub light_source_position_of_usage: Option<String>,
  #[yaserde(rename = "EnergyLabels")]
  #[serde(rename = "EnergyLabels", skip_serializing_if = "Option::is_none")]
  pub energy_labels: Option<EnergyLabels>,
  #[yaserde(rename = "SpectrumReference")]
  #[serde(rename = "SpectrumReference", skip_serializing_if = "Option::is_none")]
  pub spectrum_reference: Option<SpectrumReference>,
  #[yaserde(rename = "ActivePowerTable")]
  #[serde(rename = "ActivePowerTable", skip_serializing_if = "Option::is_none")]
  pub active_power_table: Option<ActivePowerTable>,
  #[yaserde(rename = "ColorInformation")]
  #[serde(rename = "ColorInformation", skip_serializing_if = "Option::is_none")]
  pub color_information: Option<ColorInformation>,
  #[yaserde(rename = "LightSourceImages")]
  #[serde(rename = "LightSourceImages", skip_serializing_if = "Option::is_none")]
  pub light_source_images: Option<Images>,
  #[yaserde(rename = "ZVEI")]
  #[serde(rename = "ZVEI", skip_serializing_if = "Option::is_none")]
  pub zvei: Option<String>,
  #[yaserde(rename = "Socket")]
  #[serde(rename = "Socket", skip_serializing_if = "Option::is_none")]
  pub socket: Option<String>,
  #[yaserde(rename = "ILCOS")]
  #[serde(rename = "ILCOS", skip_serializing_if = "Option::is_none")]
  pub ilcos: Option<String>,
  #[yaserde(rename = "RatedLuminousFlux")]
  #[serde(rename = "RatedLuminousFlux", skip_serializing_if = "Option::is_none")]
  pub rated_luminous_flux: Option<i32>,
  #[yaserde(rename = "RatedLuminousFlux>RGB")]
  #[serde(rename = "RatedLuminousFlux>RGB", skip_serializing_if = "Option::is_none")]
  pub rated_luminous_flux_rgb: Option<i32>,
  #[yaserde(rename = "PhotometryReference")]
  #[serde(rename = "PhotometryReference", skip_serializing_if = "Option::is_none")]
  pub photometry_reference: Option<PhotometryReference>,
  #[yaserde(rename = "LightSourceMaintenance")]
  #[serde(rename = "LightSourceMaintenance", skip_serializing_if = "Option::is_none")]
  pub light_source_maintenance: Option<LightSourceMaintenance>,
}

// FixedLightSource ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FixedLightSource {
  #[yaserde(rename = "id", attribute)]
  #[serde(rename = "@id")]
  pub id: String,
  #[yaserde(rename = "Name", attribute)]
  #[serde(rename = "Name")]
  pub name: LocaleFoo,
  #[yaserde(rename = "Description")]
  #[serde(rename = "Description")]
  pub description: LocaleFoo,
  #[yaserde(rename = "Manufacturer")]
  #[serde(rename = "Manufacturer", skip_serializing_if = "Option::is_none")]
  pub manufacturer: Option<String>,
  #[yaserde(rename = "GTIN")]
  #[serde(rename = "GTIN", skip_serializing_if = "Option::is_none")]
  pub gtin: Option<String>,
  #[yaserde(rename = "RatedInputPower")]
  #[serde(rename = "RatedInputPower", skip_serializing_if = "Option::is_none")]
  pub rated_input_power: Option<f64>,
  #[yaserde(rename = "RatedInputVoltage")]
  #[serde(rename = "RatedInputVoltage", skip_serializing_if = "Option::is_none")]
  pub rated_input_voltage: Option<Voltage>,
  #[yaserde(rename = "PowerRange")]
  #[serde(rename = "PowerRange", skip_serializing_if = "Option::is_none")]
  pub power_range: Option<PowerRange>,
  #[yaserde(rename = "LightSourcePositionOfUsage")]
  #[serde(rename = "LightSourcePositionOfUsage", skip_serializing_if = "Option::is_none")]
  pub light_source_position_of_usage: Option<String>,
  #[yaserde(rename = "EnergyLabels")]
  #[serde(rename = "EnergyLabels", skip_serializing_if = "Option::is_none")]
  pub energy_labels: Option<EnergyLabels>,
  #[yaserde(rename = "SpectrumReference")]
  #[serde(rename = "SpectrumReference", skip_serializing_if = "Option::is_none")]
  pub spectrum_reference: Option<SpectrumReference>,
  #[yaserde(rename = "ActivePowerTable")]
  #[serde(rename = "ActivePowerTable", skip_serializing_if = "Option::is_none")]
  pub active_power_table: Option<ActivePowerTable>,
  #[yaserde(rename = "ColorInformation")]
  #[serde(rename = "ColorInformation", skip_serializing_if = "Option::is_none")]
  pub color_information: Option<ColorInformation>,
  #[yaserde(rename = "LightSourceImages")]
  #[serde(rename = "LightSourceImages", skip_serializing_if = "Option::is_none")]
  pub light_source_images: Option<Images>,
  #[yaserde(rename = "LightSourceMaintenance")]
  #[serde(rename = "LightSourceMaintenance", skip_serializing_if = "Option::is_none")]
  pub light_source_maintenance: Option<LightSourceMaintenance>,
  #[yaserde(rename = "ZhagaStandard")]
  #[serde(rename = "ZhagaStandard", skip_serializing_if = "Option::is_none")]
  pub zhaga_standard: Option<bool>,
}

// LightSources ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct  LightSources {
  #[yaserde(child)]
  #[yaserde(rename = "ChangeableLightSource")]
  #[serde(rename = "ChangeableLightSource")] //, skip_serializing_if = "Vec::is_empty")]
  pub changeable_light_source: Vec<ChangeableLightSource>,
  #[yaserde(child)]
  #[yaserde(rename = "FixedLightSource")]
  #[serde(rename = "FixedLightSource")]
  pub fixed_light_source: Vec<FixedLightSource>,
}

// Interfaces ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Interfaces {
  #[yaserde(rename = "Interface")]
  #[serde(rename = "Interface")]
  pub interface: Vec<String>,
}

// ControlGear ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ControlGear {
  #[yaserde(attribute, rename = "id")]
  #[serde(rename = "@id")]
  pub id: String,
  #[yaserde(rename = "Name")]
  #[serde(rename = "Name")]
  pub name: LocaleFoo,
  #[yaserde(child)]
  #[yaserde(rename = "Description")]
  #[serde(rename = "Description")]
  pub description: LocaleFoo,
  #[yaserde(child)]
  #[yaserde(rename = "NominalVoltage")]
  #[serde(rename = "NominalVoltage", skip_serializing_if = "Option::is_none")]
  pub nominal_voltage: Option<Voltage>,
  #[yaserde(rename = "StandbyPower")]
  #[serde(rename = "StandbyPower", skip_serializing_if = "Option::is_none")]
  pub standby_power: Option<f64>,
  #[yaserde(rename = "ConstantLightOutputStartPower")]
  #[serde(rename = "ConstantLightOutputStartPower", skip_serializing_if = "Option::is_none")]
  pub constant_light_output_start_power: Option<f64>,
  #[yaserde(rename = "ConstantLightOutputEndPower")]
  #[serde(rename = "ConstantLightOutputEndPower", skip_serializing_if = "Option::is_none")]
  pub constant_light_output_end_power: Option<f64>,
  #[yaserde(rename = "PowerConsumptionControls")]
  #[serde(rename = "PowerConsumptionControls", skip_serializing_if = "Option::is_none")]
  pub power_consumption_controls: Option<f64>,
  #[yaserde(rename = "Dimmable")]
  #[serde(rename = "Dimmable", skip_serializing_if = "Option::is_none")]
  pub dimmable: Option<bool>,
  #[yaserde(rename = "ColorControllable")]
  #[serde(rename = "ColorControllable", skip_serializing_if = "Option::is_none")]
  pub color_controllable: Option<bool>,
  #[yaserde(child)]
  #[yaserde(rename = "Interfaces")]
  #[serde(rename = "Interfaces")]
  pub interfaces: Interfaces,
  #[yaserde(child)]
  #[yaserde(rename = "EnergyLabels")]
  #[serde(rename = "EnergyLabels", skip_serializing_if = "Option::is_none")]
  pub energy_labels: Option<EnergyLabels>,
}

// ControlGears ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ControlGears {
  #[yaserde(child)]
  #[yaserde(rename = "ControlGear")]
  #[serde(rename = "ControlGear")]
  pub control_gear: Vec<ControlGear>,
}

// LightSourceReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LightSourceReference {
  #[yaserde(child)]
  #[yaserde(attribute, rename = "fixedLightSourceId")]
  #[serde(rename = "@fixedLightSourceId", skip_serializing_if = "Option::is_none")]
  pub fixed_light_source_id: Option<String>,
  #[yaserde(child)]
  #[yaserde(attribute, rename = "changeableLightSourceId")]
  #[serde(rename = "@changeableLightSourceId", skip_serializing_if = "Option::is_none")]
  pub changeable_light_source_id: Option<String>,
  #[yaserde(child)]
  #[yaserde(attribute, rename = "lightSourceCount")]
  #[serde(rename = "@lightSourceCount", skip_serializing_if = "Option::is_none")]
  pub light_source_count: Option<i32>,
}

// ControlGearReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ControlGearReference {
  #[yaserde(attribute, rename = "controlGearId")]
  #[serde(rename = "@controlGearId")]
  pub control_gear_id: String,
  #[yaserde(rename = "controlGearCount")]
  #[serde(rename = "controlGearCount", skip_serializing_if = "Option::is_none")]
  pub control_gear_count: Option<i32>,
}

// Equipment ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Equipment {
  #[yaserde(rename = "id")]
  #[serde(rename = "id")]
  pub id: String,
  #[yaserde(rename = "LightSourceReference")]
  #[serde(rename = "LightSourceReference")]
  pub light_source_reference: LightSourceReference,
  #[yaserde(rename = "ControlGearReference")]
  #[serde(rename = "ControlGearReference")]
  pub control_gear_reference: ControlGearReference,
  #[yaserde(rename = "RatedInputPower")]
  #[serde(rename = "RatedInputPower")]
  pub rated_input_power: f64,
  #[yaserde(rename = "EmergencyBallastLumenFactor")]
  #[serde(rename = "EmergencyBallastLumenFactor")]
  pub emergency_ballast_lumen_factor: f64,
  #[yaserde(rename = "EmergencyRatedLuminousFlux")]
  #[serde(rename = "EmergencyRatedLuminousFlux")]
  pub emergency_rated_luminous_flux: i32,
}

// Equipments ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Equipments {
  #[yaserde(rename = "Equipment")]
  #[serde(rename = "Equipment")]
  pub equipment: Vec<Equipment>,
}

// Rotation ...

#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Rotation {
  #[yaserde(rename = "X")]
  #[serde(rename = "X")]
  pub x: i32,
  #[yaserde(rename = "Y")]
  #[serde(rename = "Y")]
  pub y: i32,
  #[yaserde(rename = "Z")]
  #[serde(rename = "Z")]
  pub z: i32,
  #[yaserde(rename = "G0")]
  #[serde(rename = "G0")]
  pub g0: i32,
}

// EquipmentReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EquipmentReference {
  #[yaserde(rename = "equipmentId")]
  #[serde(rename = "equipmentId")]
  pub equipment_id: String,
}

// ChangeableLightEmitter ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ChangeableLightEmitter {
  #[yaserde(rename = "emergencyBehaviour")]
  #[serde(rename = "emergencyBehaviour")]
  pub emergency_behaviour: Option<String>,
  #[yaserde(rename = "Name")]
  #[serde(rename = "Name")]
  pub name: Locale,
  #[yaserde(rename = "Rotation")]
  #[serde(rename = "Rotation", skip_serializing_if = "Option::is_none")]
  pub rotation: Option<Rotation>,
  #[yaserde(rename = "PhotometryReference")]
  #[serde(rename = "PhotometryReference")]
  pub photometry_reference: PhotometryReference,
  #[yaserde(rename = "G0")]
  #[serde(rename = "G0")]
  pub g0: String,
}

// FixedLightEmitter ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FixedLightEmitter {
  #[yaserde(attribute, rename = "emergencyBehaviour")]
  #[serde(rename = "@emergencyBehaviour")]
  pub emergency_behaviour: Option<String>,
  #[yaserde(rename = "Name")]
  #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
  pub name: Option<LocaleFoo>,
  #[yaserde(rename = "Rotation")]
  #[serde(rename = "Rotation", skip_serializing_if = "Option::is_none")]
  pub rotation: Option<Rotation>,
  #[yaserde(rename = "PhotometryReference")]
  #[serde(rename = "PhotometryReference")]
  pub photometry_reference: PhotometryReference,
  #[yaserde(rename = "LightSourceReference")]
  #[serde(rename = "LightSourceReference")]
  pub light_source_reference: LightSourceReference,
  #[yaserde(rename = "ControlGearReference")]
  #[serde(rename = "ControlGearReference", skip_serializing_if = "Option::is_none")]
  pub control_gear_reference: Option<ControlGearReference>,
  #[yaserde(rename = "RatedLuminousFlux")]
  #[serde(rename = "RatedLuminousFlux", skip_serializing_if = "Option::is_none")]
  pub rated_luminous_flux: Option<i32>,
  #[yaserde(rename = "RatedLuminousFluxRGB")]
  #[serde(rename = "RatedLuminousFluxRGB", skip_serializing_if = "Option::is_none")]
  pub rated_luminous_flux_rgb: Option<i32>,
  #[yaserde(rename = "EmergencyBallastLumenFactor")]
  #[serde(rename = "EmergencyBallastLumenFactor", skip_serializing_if = "Option::is_none")]
  pub emergency_ballast_lumen_factor: Option<f64>,
  #[yaserde(rename = "EmergencyRatedLuminousFlux")]
  #[serde(rename = "EmergencyRatedLuminousFlux", skip_serializing_if = "Option::is_none")]
  pub emergency_rated_luminous_flux: Option<String>,
}

// Emitter ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Emitter {
  #[yaserde(attribute, rename = "id")]
  #[serde(rename = "@id")]
  pub id: String,
  #[yaserde(rename = "ChangeableLightEmitter")]
  #[serde(rename = "ChangeableLightEmitter")]
  pub changeable_light_emitter: Vec<ChangeableLightEmitter>,
  #[yaserde(rename = "FixedLightEmitter")]
  #[serde(rename = "FixedLightEmitter")]
  pub fixed_light_emitter: Vec<FixedLightEmitter>,
  #[yaserde(rename = "Sensor")]
  #[serde(rename = "Sensor")]
  pub sensor: Vec<Sensor>,
}

// Emitters ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Emitters {
  #[yaserde(rename = "Emitter")]
  #[serde(rename = "Emitter")]
  pub emitter: Vec<Emitter>,
}

// Cuboid ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Cuboid {
  #[yaserde(rename = "Width")]
  #[serde(rename = "Width")]
  pub width: Vec<i32>,
  #[yaserde(rename = "Length")]
  #[serde(rename = "Length")]
  pub length: Vec<i32>,
  #[yaserde(rename = "Height")]
  #[serde(rename = "Height")]
  pub height: Vec<i32>,
}

// Cylinder ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Cylinder {
  #[yaserde(rename = "plane")]
  #[serde(rename = "plane")]
  pub plane: String,
  #[yaserde(rename = "Diameter")]
  #[serde(rename = "Diameter")]
  pub diameter: Vec<i32>,
  #[yaserde(rename = "Height")]
  #[serde(rename = "Height")]
  pub height: Vec<String>,
}

// RectangularEmitter ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct RectangularEmitter {
  #[yaserde(rename = "Width")]
  #[serde(rename = "Width")]
  pub width: Vec<i32>,
  #[yaserde(rename = "Length")]
  #[serde(rename = "Length")]
  pub length: Vec<i32>,
}

// CircularEmitter ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CircularEmitter {
  #[yaserde(rename = "Diameter")]
  #[serde(rename = "Diameter")]
  pub diameter: Vec<i32>,
}

// CHeights ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CHeights {
  #[yaserde(rename = "C0")]
  #[serde(rename = "C0")]
  pub c0: Vec<i32>,
  #[yaserde(rename = "C90")]
  #[serde(rename = "C90")]
  pub c90: Vec<i32>,
  #[yaserde(rename = "C180")]
  #[serde(rename = "C180")]
  pub c180: Vec<i32>,
  #[yaserde(rename = "C270")]
  #[serde(rename = "C270")]
  pub c270: Vec<i32>,
}

// SimpleGeometry ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SimpleGeometry {
  #[yaserde(atribute, rename = "id")]
  #[serde(rename = "id")]
  pub id: String,
  #[yaserde(rename = "Cuboid")]
  #[serde(rename = "Cuboid")]
  pub cuboid: Vec<Cuboid>,
  #[yaserde(rename = "Cylinder")]
  #[serde(rename = "Cylinder")]
  pub cylinder: Vec<Cylinder>,
  #[yaserde(rename = "RectangularEmitter")]
  #[serde(rename = "RectangularEmitter")]
  pub rectangular_emitter: Vec<RectangularEmitter>,
  #[yaserde(rename = "CircularEmitter")]
  #[serde(rename = "CircularEmitter")]
  pub circular_emitter: Vec<CircularEmitter>,
  #[yaserde(rename = "C-Heights")]
  pub c_heights: Vec<CHeights>,
}

// GeometryFileReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct GeometryFileReference {
  #[yaserde(attribute, rename = "fileId")]
  #[serde(rename = "@fileId")]
  pub file_id: String,
  #[yaserde(rename = "levelOfDetail")]
  #[serde(rename = "levelOfDetail", skip_serializing_if = "Option::is_none")]
  pub level_of_detail: Option<String>,
}

// ModelGeometry ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ModelGeometry {
  #[yaserde(attribute, rename = "id")]
  #[serde(rename = "@id")]
  pub id: String,
  #[yaserde(rename = "GeometryFileReference")]
  #[serde(rename = "GeometryFileReference")]
  pub geometry_file_reference: Vec<GeometryFileReference>,
}

// Geometries ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Geometries {
  #[yaserde(rename = "SimpleGeometry")]
  #[serde(rename = "SimpleGeometry")]
  pub simple_geometry: Vec<SimpleGeometry>,
  #[yaserde(rename = "ModelGeometry")]
  #[serde(rename = "ModelGeometry")]
  pub model_geometry: Vec<ModelGeometry>,
}

// LuminaireMaintenanceFactor ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LuminaireMaintenanceFactor {
  #[yaserde(attribute, rename = "years")]
  #[serde(rename = "@years", skip_serializing_if = "Option::is_none")]
  pub years: Option<String>,
  #[yaserde(attribute, rename = "roomCondition")]
  #[serde(rename = "@roomCondition", skip_serializing_if = "Option::is_none")]
  pub room_condition: Option<String>,
  #[yaserde(text, rename = "$value")]
  #[serde(rename = "$")]
  pub value: String, // TODO shall be f64, probles with needed text yaserde directive
}

// CieLuminaireMaintenanceFactors ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CieLuminaireMaintenanceFactors {
  #[yaserde(rename = "LuminaireMaintenanceFactor")]
  #[serde(rename = "LuminaireMaintenanceFactor")]
  pub luminaire_maintenance_factor: Vec<LuminaireMaintenanceFactor>,
}

// LuminaireDirtDepreciation ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LuminaireDirtDepreciation {
  #[yaserde(rename = "years")]
  #[serde(rename = "years")]
  pub years: i32,
  #[yaserde(rename = "roomCondition")]
  #[serde(rename = "roomCondition")]
  pub room_condition: String,
  #[yaserde(rename = "$value")]
  pub value: f64,
}

// IesLuminaireLightLossFactors ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct IesLuminaireLightLossFactors {
  #[yaserde(rename = "LuminaireDirtDepreciation")]
  #[serde(rename = "LuminaireDirtDepreciation")]
  pub luminaire_dirt_depreciation: Vec<LuminaireDirtDepreciation>,
}

// JiegMaintenanceFactors ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct JiegMaintenanceFactors {
  #[yaserde(rename = "LuminaireMaintenanceFactor")]
  #[serde(rename = "LuminaireMaintenanceFactor")]
  pub luminaire_maintenance_factor: Vec<LuminaireMaintenanceFactor>,
}

// LuminaireMaintenance ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LuminaireMaintenance {
  #[yaserde(rename = "Cie97LuminaireType")]
  #[serde(rename = "Cie97LuminaireType")]
  pub cie97_luminaire_type: String,
  #[yaserde(rename = "CieLuminaireMaintenanceFactors")]
  #[serde(rename = "CieLuminaireMaintenanceFactors")]
  pub cie_luminaire_maintenance_factors: CieLuminaireMaintenanceFactors,
  #[yaserde(rename = "IesLuminaireLightLossFactors")]
  #[serde(rename = "IesLuminaireLightLossFactors", skip_serializing_if = "Option::is_none")]
  pub ies_luminaire_light_loss_factors: Option<IesLuminaireLightLossFactors>,
  #[yaserde(rename = "JiegMaintenanceFactors")]
  #[serde(rename = "JiegMaintenanceFactors", skip_serializing_if = "Option::is_none")]
  pub jieg_maintenance_factors: Option<JiegMaintenanceFactors>,
}

// ProductMetaData ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductMetaData {
  #[yaserde(rename = "ProductNumber")]
  #[serde(rename = "ProductNumber", skip_serializing_if = "Option::is_none")]
  pub product_number: Option<LocaleFoo>,
  #[yaserde(rename = "Name")]
  #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
  pub name: Option<LocaleFoo>,
  #[yaserde(rename = "Description")]
  #[serde(rename = "Description", skip_serializing_if = "Option::is_none")]
  pub description: Option<LocaleFoo>,
  #[yaserde(rename = "TenderText")]
  #[serde(rename = "TenderText")]
  pub tender_text: Option<LocaleFoo>,
  #[yaserde(rename = "ProductSeries")]
  #[serde(rename = "ProductSeries", skip_serializing_if = "Option::is_none")]
  pub product_series: Option<ProductSeries>,
  #[yaserde(rename = "Pictures")]
  #[serde(rename = "Pictures", skip_serializing_if = "Option::is_none")]
  pub pictures: Option<Images>,
  #[yaserde(rename = "LuminaireMaintenance")]
  #[serde(rename = "LuminaireMaintenance", skip_serializing_if = "Option::is_none")]
  pub luminaire_maintenance: Option<LuminaireMaintenance>,
  #[yaserde(rename = "DescriptiveAttributes")]
  #[serde(rename = "DescriptiveAttributes", skip_serializing_if = "Option::is_none")]
  pub descriptive_attributes: Option<DescriptiveAttributes>,
}

// RectangularCutout ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct RectangularCutout {
  #[yaserde(rename = "Width")]
  #[serde(rename = "Width")]
  pub width: i32,
  #[yaserde(rename = "Length")]
  #[serde(rename = "Length")]
  pub length: i32,
  #[yaserde(rename = "Depth")]
  #[serde(rename = "Depth")]
  pub depth: i32,
}

// CircularCutout ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CircularCutout {
  #[yaserde(rename = "Diameter")]
  #[serde(rename = "Diameter")]
  pub diameter: i32,
  #[yaserde(rename = "Depth")]
  #[serde(rename = "Depth")]
  pub depth: i32,
}

// Recessed ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Recessed {
  #[yaserde(rename = "recessedDepth")]
  #[serde(rename = "recessedDepth")]
  pub recessed_depth: i32,
  #[yaserde(rename = "RectangularCutout")]
  #[serde(rename = "RectangularCutout")]
  pub rectangular_cutout: RectangularCutout,
  #[yaserde(rename = "Depth")]
  #[serde(rename = "Depth")]
  pub depth: i32,
}

// SurfaceMounted ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SurfaceMounted {}

// Pendant ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Pendant {
  #[yaserde(rename = "pendantLength")]
  #[serde(rename = "pendantLength")]
  pub pendant_length: f64,
}

// Ceiling ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Ceiling {
  #[yaserde(rename = "Recessed")]
  #[serde(rename = "Recessed")]
  pub recessed: Recessed,
  #[yaserde(rename = "SurfaceMounted")]
  #[serde(rename = "SurfaceMounted")]
  pub surface_mounted: SurfaceMounted,
  #[yaserde(rename = "Pendant")]
  #[serde(rename = "Pendant")]
  pub pendant: Pendant,
}

// Wall ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Wall {
  #[yaserde(rename = "mountingHeight")]
  #[serde(rename = "mountingHeight")]
  pub mounting_height: i32,
  #[yaserde(rename = "Recessed")]
  #[serde(rename = "Recessed")]
  pub recessed: Recessed,
  #[yaserde(rename = "Depth")]
  #[serde(rename = "Depth")]
  pub depth: i32,
}

// FreeStanding ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FreeStanding {}

// WorkingPlane ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct WorkingPlane {
  #[yaserde(rename = "FreeStanding")]
  #[serde(rename = "FreeStanding")]
  pub free_standing: FreeStanding,
}

// PoleTop ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PoleTop {
  #[yaserde(rename = "poleHeight")]
  #[serde(rename = "poleHeight")]
  pub pole_height: i32,
}

// PoleIntegrated ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PoleIntegrated {
  #[yaserde(rename = "poleHeight")]
  #[serde(rename = "poleHeight")]
  pub pole_height: i32,
}

// Ground ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Ground {
  #[yaserde(rename = "PoleTop")]
  #[serde(rename = "PoleTop")]
  pub pole_top: PoleTop,
  #[yaserde(rename = "PoleIntegrated")]
  #[serde(rename = "PoleIntegrated")]
  pub pole_integrated: PoleIntegrated,
  #[yaserde(rename = "FreeStanding")]
  #[serde(rename = "FreeStanding")]
  pub free_standing: FreeStanding,
  #[yaserde(rename = "SurfaceMounted")]
  #[serde(rename = "SurfaceMounted")]
  pub surface_mounted: SurfaceMounted,
  #[yaserde(rename = "Recessed")]
  #[serde(rename = "Recessed")]
  pub recessed: Recessed,
}

// Mountings ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Mountings {
  #[yaserde(rename = "Ceiling")]
  #[serde(rename = "Ceiling")]
  pub ceiling: Ceiling,
  #[yaserde(rename = "Wall")]
  #[serde(rename = "Wall")]
  pub wall: Wall,
  #[yaserde(rename = "WorkingPlane")]
  #[serde(rename = "WorkingPlane")]
  pub working_plane: WorkingPlane,
  #[yaserde(rename = "Ground")]
  #[serde(rename = "Ground")]
  pub ground: Ground,
}

// EmitterReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EmitterReference {
  #[yaserde(attribute, rename = "emitterId")]
  #[serde(rename = "@emitterId")]
  pub emitter_id: String,
  #[yaserde(rename = "EmitterObjectExternalName")]
  #[serde(rename = "EmitterObjectExternalName")]
  pub emitter_object_external_name: String,
}

// SimpleGeometryReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SimpleGeometryReference {
  #[yaserde(rename = "geometryId")]
  #[serde(rename = "geometryId")]
  pub geometry_id: String,
  #[yaserde(rename = "emitterId")]
  #[serde(rename = "emitterId")]
  pub emitter_id: String,
}

// ModelGeometryReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ModelGeometryReference {
  #[yaserde(attribute,rename = "geometryId")]
  #[serde(rename = "@geometryId")]
  pub geometry_id: String,
  #[yaserde(rename = "EmitterReference")]
  #[serde(rename = "EmitterReference")]
  pub emitter_reference: Vec<EmitterReference>,
}

// GeometryReferences ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct GeometryReferences {
  #[yaserde(rename = "SimpleGeometryReference")]
  #[serde(rename = "SimpleGeometryReference")]
  pub simple_geometry_reference: SimpleGeometryReference,
  #[yaserde(rename = "ModelGeometryReference")]
  #[serde(rename = "ModelGeometryReference")]
  pub model_geometry_reference: ModelGeometryReference,
}

// Geometry ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Geometry {
  // #[yaserde(rename = "EmitterReference")]
  // #[serde(rename = "EmitterReference")]
  // pub emitter_reference: EmitterReference,  // Moved to the Model / SimpleGeometry
  #[yaserde(rename = "SimpleGeometryReference")]
  #[serde(rename = "SimpleGeometryReference", skip_serializing_if = "Option::is_none")]
  pub simple_geometry_reference: Option<SimpleGeometryReference>,
  #[yaserde(rename = "ModelGeometryReference")]
  #[serde(rename = "ModelGeometryReference", skip_serializing_if = "Option::is_none")]
  pub model_geometry_reference: Option<ModelGeometryReference>,
  // #[yaserde(rename = "GeometryReferences")]
  // #[serde(rename = "GeometryReferences")]
  // pub geometry_references: GeometryReferences,
}

// Symbol ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Symbol {
  #[yaserde(rename = "fileId")]
  #[serde(rename = "fileId")]
  pub file_id: String,
}

// Variant ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Variant {
  #[yaserde(attribute, rename = "id")]
  #[serde(rename = "@id")]
  pub id: String,
  #[yaserde(rename = "sortOrder")]
  #[serde(rename = "sortOrder", skip_serializing_if = "Option::is_none")]
  pub sort_order: Option<i32>,
  #[yaserde(rename = "ProductNumber")]
  #[serde(rename = "ProductNumber", skip_serializing_if = "Option::is_none")]
  pub product_number: Option<LocaleFoo>,
  #[yaserde(rename = "Name")]
  #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
  pub name: Option<LocaleFoo>,
  #[yaserde(rename = "Description")]
  #[serde(rename = "Description", skip_serializing_if = "Option::is_none")]
  pub description: Option<LocaleFoo>,
  #[yaserde(rename = "TenderText")]
  #[serde(rename = "TenderText", skip_serializing_if = "Option::is_none")]
  pub tender_text: Option<LocaleFoo>,
  #[yaserde(rename = "GTIN")]
  #[serde(rename = "GTIN", skip_serializing_if = "Option::is_none")]
  pub gtin: Option<String>,
  #[yaserde(rename = "Mountings")]
  #[serde(rename = "Mountings", skip_serializing_if = "Option::is_none")]
  pub mountings: Option<Mountings>,
  #[yaserde(rename = "Geometry")]
  #[serde(rename = "Geometry", skip_serializing_if = "Option::is_none")]
  pub geometry: Option<Geometry>,
  #[yaserde(rename = "ProductSeries")]
  #[serde(rename = "ProductSeries", skip_serializing_if = "Option::is_none")]
  pub product_series: Option<ProductSeries>,
  #[yaserde(rename = "Pictures")]
  #[serde(rename = "Pictures", skip_serializing_if = "Option::is_none")]
  pub pictures: Option<Images>,
  #[yaserde(rename = "Symbol")]
  #[serde(rename = "Symbol", skip_serializing_if = "Option::is_none")]
  pub symbol: Option<Symbol>,
  #[yaserde(rename = "DescriptiveAttributes")]
  #[serde(rename = "DescriptiveAttributes", skip_serializing_if = "Option::is_none")]
  pub descriptive_attributes: Option<DescriptiveAttributes>,
}

// Variants ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Variants {
  #[yaserde(rename = "Variant")]
  #[serde(rename = "Variant")]
  pub variant: Vec<Variant>,
}

// ProductSize ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductSize {
  #[yaserde(rename = "Length")]
  #[serde(rename = "Length")]
  pub length: i32,
  #[yaserde(rename = "Width")]
  #[serde(rename = "Width")]
  pub width: i32,
  #[yaserde(rename = "Height")]
  #[serde(rename = "Height")]
  pub height: i32,
}

// Adjustabilities ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Adjustabilities {
  #[yaserde(rename = "Adjustability")]
  #[serde(rename = "Adjustability")]
  pub adjustability: Vec<String>,
}

// ProtectiveAreas ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProtectiveAreas {
  #[yaserde(rename = "Area")]
  #[serde(rename = "Area")]
  pub area: Vec<String>,
}

// Mechanical ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Mechanical {
  #[yaserde(rename = "ProductSize")]
  #[serde(rename = "ProductSize", skip_serializing_if = "Option::is_none")]
  pub product_size: Option<ProductSize>,
  #[yaserde(rename = "ProductForm")]
  #[serde(rename = "ProductForm", skip_serializing_if = "Option::is_none")]
  pub product_form: Option<String>,
  #[yaserde(rename = "SealingMaterial")]
  #[serde(rename = "SealingMaterial", skip_serializing_if = "Option::is_none")]
  pub sealing_material: Option<LocaleFoo>,
  #[yaserde(rename = "Adjustabilities")]
  #[serde(rename = "Adjustabilities", skip_serializing_if = "Option::is_none")]
  pub adjustabilities: Option<Adjustabilities>,
  #[yaserde(rename = "IKRating")]
  #[serde(rename = "IKRating", skip_serializing_if = "Option::is_none")]
  pub ik_rating: Option<String>,
  #[yaserde(rename = "ProtectiveAreas")]
  #[serde(rename = "ProtectiveAreas", skip_serializing_if = "Option::is_none")]
  pub protective_areas: Option<ProtectiveAreas>,
  #[yaserde(rename = "Weight")]
  #[serde(rename = "Weight", skip_serializing_if = "Option::is_none")]
  pub weight: Option<f64>,
}

// ClampingRange ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ClampingRange {
  #[yaserde(rename = "Lower")]
  #[serde(rename = "Lower")]
  pub lower: f64,
  #[yaserde(rename = "Upper")]
  #[serde(rename = "Upper")]
  pub upper: f64,
}

// Electrical ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Electrical {
  #[yaserde(rename = "ClampingRange")]
  #[serde(rename = "ClampingRange")]
  pub clamping_range: ClampingRange,
  #[yaserde(rename = "SwitchingCapacity")]
  #[serde(rename = "SwitchingCapacity")]
  pub switching_capacity: String,
  #[yaserde(rename = "ElectricalSafetyClass")]
  #[serde(rename = "ElectricalSafetyClass")]
  pub electrical_safety_class: String,
  #[yaserde(rename = "IngressProtectionIPCode")]
  #[serde(rename = "IngressProtectionIPCode")]
  pub ingress_protection_ip_code: String,
  #[yaserde(rename = "PowerFactor")]
  #[serde(rename = "PowerFactor")]
  pub power_factor: f64,
  #[yaserde(rename = "ConstantLightOutput")]
  #[serde(rename = "ConstantLightOutput")]
  pub constant_light_output: bool,
  #[yaserde(rename = "LightDistribution")]
  #[serde(rename = "LightDistribution", skip_serializing_if = "Option::is_none")]
  pub light_distribution: Option<String>,
}

// Flux ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Flux {
  #[yaserde(rename = "hours")]
  #[serde(rename = "hours")]
  pub hours: i32,
  #[yaserde(rename = "$value")]
  pub value: i32,
}

// DurationTimeAndFlux ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DurationTimeAndFlux {
  #[yaserde(rename = "Flux")]
  #[serde(rename = "Flux")]
  pub flux: Vec<Flux>,
}

// Emergency ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Emergency {
  #[yaserde(rename = "DurationTimeAndFlux")]
  #[serde(rename = "DurationTimeAndFlux", skip_serializing_if = "Option::is_none")]
  pub duration_time_and_flux: Option<DurationTimeAndFlux>,
  #[yaserde(rename = "DedicatedEmergencyLightingType")]
  #[serde(rename = "DedicatedEmergencyLightingType", skip_serializing_if = "Option::is_none")]
  pub dedicated_emergency_lighting_type: Option<String>,
}

// ListPrice ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ListPrice {
  #[yaserde(rename = "currency")]
  #[serde(rename = "currency", skip_serializing_if = "Option::is_none")]
  pub currency: Option<String>,
  #[yaserde(rename = "$value")]
  #[serde(rename = "$")]
  pub value: f64,
}

// ListPrices ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ListPrices {
  #[yaserde(rename = "ListPrice")]
  #[serde(rename = "ListPrice")]
  pub list_price: Vec<ListPrice>,
}

// HousingColor ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct HousingColor {
  #[yaserde(rename = "ral")]
  #[serde(rename = "ral")]
  pub ral: Option<i32>,
  #[yaserde(flatten)]
  pub locale: Locale,
}

// HousingColors ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct HousingColors {
  #[yaserde(rename = "HousingColor")]
  #[serde(rename = "HousingColor")]
  pub housing_color: Vec<HousingColor>,
}

// Markets ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Markets {
  #[yaserde(rename = "Region")]
  #[serde(rename = "Region")]
  pub region: Vec<Locale>,
}

// ApprovalMarks ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ApprovalMarks {
  #[yaserde(rename = "ApprovalMark")]
  #[serde(rename = "ApprovalMark")]
  pub approval_mark: Vec<String>,
}

// DesignAwards ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DesignAwards {
  #[yaserde(rename = "DesignAward")]
  #[serde(rename = "DesignAward")]
  pub design_award: Vec<String>,
}

// Labels ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Labels {
  #[yaserde(rename = "Label")]
  #[serde(rename = "Label")]
  pub label: Vec<String>,
}

// Applications ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Applications {
  #[yaserde(rename = "Application")]
  #[serde(rename = "Application")]
  pub application: Vec<String>,
}

// Marketing ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Marketing {
  #[yaserde(rename = "ListPrices")]
  #[serde(rename = "ListPrices", skip_serializing_if = "Option::is_none")]
  pub list_prices: Option<ListPrices>,
  #[yaserde(rename = "HousingColors")]
  #[serde(rename = "HousingColors", skip_serializing_if = "Option::is_none")]
  pub housing_colors: Option<HousingColors>,
  #[yaserde(rename = "Markets")]
  #[serde(rename = "Markets", skip_serializing_if = "Option::is_none")]
  pub markets: Option<Markets>,
  #[yaserde(rename = "Hyperlinks")]
  #[serde(rename = "Hyperlinks", skip_serializing_if = "Option::is_none")]
  pub hyperlinks: Option<Hyperlinks>,
  #[yaserde(rename = "Designer")]
  #[serde(rename = "Designer", skip_serializing_if = "Option::is_none")]
  pub designer: Option<String>,
  #[yaserde(rename = "ApprovalMarks")]
  #[serde(rename = "ApprovalMarks", skip_serializing_if = "Option::is_none")]
  pub approval_marks: Option<ApprovalMarks>,
  #[yaserde(rename = "DesignAwards")]
  #[serde(rename = "DesignAwards", skip_serializing_if = "Option::is_none")]
  pub design_awards: Option<DesignAwards>,
  #[yaserde(rename = "Labels")]
  #[serde(rename = "Labels", skip_serializing_if = "Option::is_none")]
  pub labels: Option<Labels>,
  #[yaserde(rename = "Applications")]
  #[serde(rename = "Applications", skip_serializing_if = "Option::is_none")]
  pub applications: Option<Applications>,
}

// UsefulLifeTimes ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct UsefulLifeTimes {
  #[yaserde(rename = "UsefulLife")]
  #[serde(rename = "UsefulLife")]
  pub useful_life: Vec<String>,
}

// MedianUsefulLifeTimes ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct MedianUsefulLifeTimes {
  #[yaserde(rename = "MedianUsefulLife")]
  #[serde(rename = "MedianUsefulLife")] //, skip_serializing_if = "Option::is_none")]
  pub median_useful_life: Vec<String>,
}

// Directives ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Directives {
  #[yaserde(rename = "Directive")]
  #[serde(rename = "Directive")]
  pub directive: Vec<String>,
}

// Classes ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Classes {
  #[yaserde(rename = "Class")]
  #[serde(rename = "Class")]
  pub class: Vec<String>,
}

// Divisions ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Divisions {
  #[yaserde(rename = "Division")]
  #[serde(rename = "Division")]
  pub division: Vec<String>,
}

// Gas ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Gas {
  #[yaserde(rename = "Group")]
  #[serde(rename = "Group")]
  pub group: Vec<String>,
}

// Dust ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Dust {
  #[yaserde(rename = "Group")]
  #[serde(rename = "Group")]
  pub group: Vec<String>,
}

// DivisionGroups ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DivisionGroups {
  #[yaserde(rename = "Gas")]
  #[serde(rename = "Gas")]
  pub gas: Gas,
  #[yaserde(rename = "Dust")]
  #[serde(rename = "Dust")]
  pub dust: Dust,
}

// Zones ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Zones {
  #[yaserde(rename = "Gas")]
  #[serde(rename = "Gas")]
  pub gas: Gas,
  #[yaserde(rename = "Dust")]
  #[serde(rename = "Dust")]
  pub dust: Dust,
}

// ZoneGroups ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ZoneGroups {
  #[yaserde(rename = "Gas")]
  #[serde(rename = "Gas")]
  pub gas: Gas,
  #[yaserde(rename = "Dust")]
  #[serde(rename = "Dust")]
  pub dust: Dust,
}

// TemperatureClasses ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct TemperatureClasses {
  #[yaserde(rename = "TemperatureClass")]
  #[serde(rename = "TemperatureClass")]
  pub temperature_class: Vec<String>,
}

// ExCodes ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ExCodes {
  #[yaserde(rename = "ExCode")]
  #[serde(rename = "ExCode")]
  pub ex_code: Vec<String>,
}

// EquipmentProtectionLevels ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EquipmentProtectionLevels {
  #[yaserde(rename = "EquipmentProtectionLevel")]
  #[serde(rename = "EquipmentProtectionLevel")]
  pub equipment_protection_level: Vec<String>,
}

// EquipmentGroups ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EquipmentGroups {
  #[yaserde(rename = "EquipmentGroup")]
  #[serde(rename = "EquipmentGroup")]
  pub equipment_group: Vec<String>,
}

// EquipmentCategories ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EquipmentCategories {
  #[yaserde(rename = "EquipmentCategory")]
  #[serde(rename = "EquipmentCategory")]
  pub equipment_category: Vec<String>,
}

// Atmospheres ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Atmospheres {
  #[yaserde(rename = "Atmosphere")]
  #[serde(rename = "Atmosphere")]
  pub atmosphere: Vec<String>,
}

// Groups ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Groups {
  #[yaserde(rename = "Group")]
  #[serde(rename = "Group")]
  pub group: Vec<String>,
}

// ATEX ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ATEX {
  #[yaserde(rename = "Directives")]
  #[serde(rename = "Directives")]
  pub directives: Directives,
  #[yaserde(rename = "Classes")]
  #[serde(rename = "Classes")]
  pub classes: Classes,
  #[yaserde(rename = "Divisions")]
  #[serde(rename = "Divisions")]
  pub divisions: Divisions,
  #[yaserde(rename = "DivisionGroups")]
  #[serde(rename = "DivisionGroups")]
  pub division_groups: DivisionGroups,
  #[yaserde(rename = "Zones")]
  #[serde(rename = "Zones")]
  pub zones: Zones,
  #[yaserde(rename = "ZoneGroups")]
  #[serde(rename = "ZoneGroups")]
  pub zone_groups: ZoneGroups,
  #[yaserde(rename = "MaximumSurfaceTemperature")]
  #[serde(rename = "MaximumSurfaceTemperature")]
  pub maximum_surface_temperature: String,
  #[yaserde(rename = "TemperatureClasses")]
  #[serde(rename = "TemperatureClasses")]
  pub temperature_classes: TemperatureClasses,
  #[yaserde(rename = "ExCodes")]
  #[serde(rename = "ExCodes")]
  pub ex_codes: ExCodes,
  #[yaserde(rename = "EquipmentProtectionLevels")]
  #[serde(rename = "EquipmentProtectionLevels")]
  pub equipment_protection_levels: EquipmentProtectionLevels,
  #[yaserde(rename = "EquipmentGroups")]
  #[serde(rename = "EquipmentGroups")]
  pub equipment_groups: EquipmentGroups,
  #[yaserde(rename = "EquipmentCategories")]
  #[serde(rename = "EquipmentCategories")]
  pub equipment_categories: EquipmentCategories,
  #[yaserde(rename = "Atmospheres")]
  #[serde(rename = "Atmospheres")]
  pub atmospheres: Atmospheres,
  #[yaserde(rename = "Groups")]
  #[serde(rename = "Groups")]
  pub groups: Groups,
}

// AbsorptionRate ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct AbsorptionRate {
  #[yaserde(rename = "hertz")]
  #[serde(rename = "hertz")]
  pub hertz: i32,
  #[yaserde(rename = "$value")]
  pub value: f64,
}

// AcousticAbsorptionRates ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct AcousticAbsorptionRates {
  #[yaserde(rename = "AbsorptionRate")]
  #[serde(rename = "AbsorptionRate")]
  pub absorption_rate: Vec<AbsorptionRate>,
}

// OperationsAndMaintenance ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct OperationsAndMaintenance {
  #[yaserde(rename = "UsefulLifeTimes")]
  #[serde(rename = "UsefulLifeTimes", skip_serializing_if = "Option::is_none")]
  pub useful_life_times: Option<UsefulLifeTimes>,
  #[yaserde(rename = "MedianUsefulLifeTimes")]
  #[serde(rename = "MedianUsefulLifeTimes", skip_serializing_if = "Option::is_none")]
  pub median_useful_life_times: Option<MedianUsefulLifeTimes>,
  #[yaserde(rename = "OperatingTemperature")]
  #[serde(rename = "OperatingTemperature", skip_serializing_if = "Option::is_none")]
  pub operating_temperature: Option<TemperatureRange>,
  #[yaserde(rename = "AmbientTemperature")]
  #[serde(rename = "AmbientTemperature", skip_serializing_if = "Option::is_none")]
  pub ambient_temperature: Option<TemperatureRange>,
  #[yaserde(rename = "RatedAmbientTemperature")]
  #[serde(rename = "RatedAmbientTemperature", skip_serializing_if = "Option::is_none")]
  pub rated_ambient_temperature: Option<i32>,
  #[yaserde(rename = "ATEX")]
  #[serde(rename = "ATEX", skip_serializing_if = "Option::is_none")]
  pub atex: Option<ATEX>,
  #[yaserde(rename = "AcousticAbsorptionRates")]
  #[serde(rename = "AcousticAbsorptionRates", skip_serializing_if = "Option::is_none")]
  pub acoustic_absorption_rates: Option<AcousticAbsorptionRates>,
}

// FileReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FileReference {
  #[yaserde(rename = "fileId")]
  #[serde(rename = "fileId")]
  pub file_id: String,
}

// Property ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Property {
  #[yaserde(rename = "id")]
  #[serde(rename = "id")]
  pub id: String,
  #[yaserde(rename = "Name")]
  #[serde(rename = "Name")]
  pub name: Locale,
  #[yaserde(rename = "PropertySource")]
  #[serde(rename = "PropertySource")]
  pub property_source: String,
  #[yaserde(rename = "Value")]
  #[serde(rename = "Value")]
  pub value: String,
  #[yaserde(rename = "FileReference")]
  #[serde(rename = "FileReference")]
  pub file_reference: FileReference,
}

// CustomProperties ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CustomProperties {
  #[yaserde(rename = "Property")]
  #[serde(rename = "Property")]
  pub property:Vec<Property>,
}

// DescriptiveAttributes ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DescriptiveAttributes {
  #[yaserde(rename = "Mechanical")]
  #[serde(rename = "Mechanical", skip_serializing_if = "Option::is_none")]
  pub mechanical: Option<Mechanical>,
  #[yaserde(rename = "Electrical")]
  #[serde(rename = "Electrical", skip_serializing_if = "Option::is_none")]
  pub electrical: Option<Electrical>,
  #[yaserde(rename = "Emergency")]
  #[serde(rename = "Emergency", skip_serializing_if = "Option::is_none")]
  pub emergency: Option<Emergency>,
  #[yaserde(rename = "Marketing")]
  #[serde(rename = "Marketing", skip_serializing_if = "Option::is_none")]
  pub marketing: Option<Marketing>,
  #[yaserde(rename = "OperationsAndMaintenance")]
  #[serde(rename = "OperationsAndMaintenance", skip_serializing_if = "Option::is_none")]
  pub operations_and_maintenance: Option<OperationsAndMaintenance>,
  #[yaserde(rename = "CustomProperties")]
  #[serde(rename = "CustomProperties", skip_serializing_if = "Option::is_none")]
  pub custom_properties: Option<CustomProperties>,
}

// Locale ...
//#[yaserde_with::serde_as]
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Locale {
  #[serde(rename = "@language")]
  #[yaserde(attribute)]
  pub language: String,
  #[yaserde(text)]
  #[serde(rename = "$")]
  pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LocaleFoo {
  #[yaserde(rename = "Locale")]
  #[serde(rename = "Locale")]
  pub locale: Vec<Locale>
}


// Hyperlink ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Hyperlink {
  #[yaserde(attribute, rename = "href")]
  #[serde(rename = "@href")]
  pub href: String,
  #[yaserde(rename = "language")]
  #[serde(rename = "language", skip_serializing_if = "Option::is_none")]
  pub language: Option<String>,
  #[yaserde(rename = "region")]
  #[serde(rename = "region", skip_serializing_if = "Option::is_none")]
  pub region: Option<String>,
  #[yaserde(rename = "countryCode")]
  #[serde(rename = "countryCode", skip_serializing_if = "Option::is_none")]
  pub country_code: Option<String>,
  #[yaserde(text)]
  #[yaserde(rename = "$value")]
  #[serde(rename = "$")]
  pub value: String,
}

// Hyperlinks ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Hyperlinks {
  #[yaserde(rename = "Hyperlink")]
  #[serde(rename = "Hyperlink")]
  pub hyperlink: Vec<Hyperlink>,
}

// EnergyLabel ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EnergyLabel {
  #[yaserde(rename = "region")]
  #[serde(rename = "region")]
  pub region: String,
  #[yaserde(rename = "$value")]
  pub value: String,
}

// EnergyLabels ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EnergyLabels {
  #[yaserde(rename = "EnergyLabel")]
  #[serde(rename = "EnergyLabel")]
  pub energy_label: Vec<EnergyLabel>,
}

// ProductSerie ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductSerie {
  #[yaserde(rename = "Name")]
  #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
  pub name: Option<LocaleFoo>,
  #[yaserde(rename = "Description")]
  #[serde(rename = "Description", skip_serializing_if = "Option::is_none")]
  pub description: Option<LocaleFoo>,
  #[yaserde(rename = "Pictures")]
  #[serde(rename = "Pictures", skip_serializing_if = "Option::is_none")]
  pub pictures: Option<Images>,
  #[yaserde(rename = "Hyperlinks")]
  #[serde(rename = "Hyperlinks", skip_serializing_if = "Option::is_none")]
  pub hyperlinks: Option<Hyperlinks>,
}

// ProductSeries ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductSeries {
  #[yaserde(rename = "ProductSerie")]
  #[serde(rename = "ProductSerie")]
  pub product_serie: Vec<ProductSerie>,
}

// Image ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Image {
  #[yaserde(attribute, rename = "imageType")]
  #[serde(rename = "@imageType")]
  pub image_type: String,
  #[yaserde(attribute, rename = "fileId")]
  #[serde(rename = "@fileId")]
  pub file_id: String,
}

// Images ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Images {
  #[yaserde(rename = "Image")]
  #[serde(rename = "Image")]
  pub image: Vec<Image>,
}

// TemperatureRange ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct TemperatureRange {
  #[yaserde(rename = "Lower")]
  #[serde(rename = "Lower")]
  pub lower: i32,
  #[yaserde(rename = "Upper")]
  #[serde(rename = "Upper")]
  pub upper: i32,
}

// VoltageRange ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct VoltageRange {
  #[yaserde(rename = "Min")]
  #[serde(rename = "Min")]
  pub min: f64,
  #[yaserde(rename = "Max")]
  #[serde(rename = "Max")]
  pub max: f64,
}

// Voltage ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Voltage {
  #[yaserde(rename = "VoltageRange")]
  #[serde(rename = "VoltageRange")]
  pub voltage_range: VoltageRange,
  #[yaserde(rename = "FixedVoltage")]
  #[serde(rename = "FixedVoltage")]
  pub fixed_voltage: f64,
  #[yaserde(rename = "Type")]
  #[serde(rename = "Type")]
  pub type_attr: String,
  #[yaserde(rename = "Frequency")]
  #[serde(rename = "Frequency")]
  pub frequency: String,
}

