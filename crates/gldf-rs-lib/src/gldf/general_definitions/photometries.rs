#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

use serde::{Deserialize, Serialize};

/// Represents a reference to a photometry file.
///
/// The `PhotometryFileReference` struct models a reference to a photometry file within the GLDF file.
/// It includes the ID of the referenced file. It supports serialization and deserialization
/// of XML data for working with photometry file references.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotometryFileReference {
    /// The ID of the referenced file.
    #[serde(rename = "@fileId")]
    pub file_id: String,
}

/// Represents the tenth peak divergence of a photometry.
///
/// The `TenthPeakDivergence` struct models the tenth peak divergence of a photometry within the GLDF file.
/// It includes divergence values for angles `C0-C180` and `C90-C270`. It supports serialization and deserialization
/// of XML data for working with tenth peak divergence.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TenthPeakDivergence {
    /// The divergence value for angles `C0-C180`.
    #[serde(rename = "C0-C180", skip_serializing_if = "Option::is_none")]
    pub c0_c180: Option<f64>,

    /// The divergence value for angles `C90-C270`.
    #[serde(rename = "C90-C270", skip_serializing_if = "Option::is_none")]
    pub c90_c270: Option<f64>,
}

/// Represents the half peak divergence of a photometry.
///
/// The `HalfPeakDivergence` struct models the half peak divergence of a photometry within the GLDF file.
/// It includes divergence values for angles `C0-C180` and `C90-C270`. It supports serialization and deserialization
/// of XML data for working with half peak divergence.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HalfPeakDivergence {
    /// The divergence value for angles `C0-C180`.
    #[serde(rename = "C0-C180", skip_serializing_if = "Option::is_none")]
    pub c0_c180: Option<f64>,

    /// The divergence value for angles `C90-C270`.
    #[serde(rename = "C90-C270", skip_serializing_if = "Option::is_none")]
    pub c90_c270: Option<f64>,
}

/// Represents the Unified Glare Rating (UGR) photometric data.
///
/// The `UGR4H8H705020LQ` struct models the UGR4H8H705020LQ photometric data within the GLDF file.
/// Unified Glare Rating (UGR) is a metric used to assess the discomfort glare experienced by an
/// observer due to luminaires and their arrangement.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UGR4H8H705020LQ {
    /// The value of the `X` coordinate.
    #[serde(rename = "X", skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,

    /// The value of the `Y` coordinate.
    #[serde(rename = "Y", skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
}

/// Represents descriptive photometric information about a lighting product.
///
/// The `DescriptivePhotometry` struct models various photometric properties of a lighting product.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DescriptivePhotometry {
    /// Luminance of the luminaire, usually expressed in candelas per square meter (cd/m²).
    #[serde(rename = "LuminaireLuminance", skip_serializing_if = "Option::is_none")]
    pub luminaire_luminance: Option<i32>,

    /// Ratio of the total light output of the luminaire to the total input power.
    #[serde(rename = "LightOutputRatio", skip_serializing_if = "Option::is_none")]
    pub light_output_ratio: Option<f64>,

    /// Luminous efficacy of the luminaire, measured in lumens per watt (lm/W).
    #[serde(rename = "LuminousEfficacy", skip_serializing_if = "Option::is_none")]
    pub luminous_efficacy: Option<f64>,

    /// Fraction of light emitted by the luminaire that is directed downwards.
    #[serde(
        rename = "DownwardFluxFraction",
        skip_serializing_if = "Option::is_none"
    )]
    pub downward_flux_fraction: Option<f64>,

    /// Ratio of the downward light output to the total light output of the luminaire.
    #[serde(
        rename = "DownwardLightOutputRatio",
        skip_serializing_if = "Option::is_none"
    )]
    pub downward_light_output_ratio: Option<f64>,

    /// Ratio of the upward light output to the total light output of the luminaire.
    #[serde(
        rename = "UpwardLightOutputRatio",
        skip_serializing_if = "Option::is_none"
    )]
    pub upward_light_output_ratio: Option<f64>,

    /// Divergence angles for the luminaire's light distribution.
    #[serde(
        rename = "TenthPeakDivergence",
        skip_serializing_if = "Option::is_none"
    )]
    pub tenth_peak_divergence: Option<TenthPeakDivergence>,

    /// Divergence angles for the luminaire's light distribution.
    #[serde(rename = "HalfPeakDivergence", skip_serializing_if = "Option::is_none")]
    pub half_peak_divergence: Option<HalfPeakDivergence>,

    /// Code indicating the photometric type of the luminaire.
    #[serde(rename = "PhotometricCode", skip_serializing_if = "Option::is_none")]
    pub photometric_code: Option<String>,

    /// Code indicating the CIE flux group of the luminaire.
    #[serde(rename = "CIE-FluxCode", skip_serializing_if = "Option::is_none")]
    pub cie_flux_code: Option<String>,

    /// Angle at which the luminous intensity is reduced by half.
    #[serde(rename = "CutOffAngle", skip_serializing_if = "Option::is_none")]
    pub cut_off_angle: Option<f64>,

    /// Unified Glare Rating (UGR) values indicating perceived glare for indoor lighting.
    #[serde(
        rename = "UGR-4H8H-70-50-20-LQ",
        skip_serializing_if = "Option::is_none"
    )]
    pub ugr4_h8_h705020_lq: Option<UGR4H8H705020LQ>,

    /// Definition of the light distribution pattern according to IESNA standards.
    #[serde(
        rename = "IESNA-LightDistributionDefinition",
        skip_serializing_if = "Option::is_none"
    )]
    pub iesna_light_distribution_definition: Option<String>,

    /// BUG (Backlight, Uplight, Glare) rating indicating light distribution and potential for discomfort glare.
    #[serde(
        rename = "LightDistributionBUG-Rating",
        skip_serializing_if = "Option::is_none"
    )]
    pub light_distribution_bug_rating: Option<String>,
}

/// Represents photometric information about a lighting product.
///
/// The `Photometry` struct models photometric data related to a lighting product.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Photometry {
    /// The unique identifier for the photometry data.
    #[serde(rename = "@id")]
    pub id: String,

    /// Reference to a photometry file.
    #[serde(
        rename = "PhotometryFileReference",
        skip_serializing_if = "Option::is_none"
    )]
    pub photometry_file_reference: Option<PhotometryFileReference>,

    /// Descriptive photometric information about the lighting product.
    #[serde(
        rename = "DescriptivePhotometry",
        skip_serializing_if = "Option::is_none"
    )]
    pub descriptive_photometry: Option<DescriptivePhotometry>,
}

/// Represents a collection of photometric data.
///
/// The `Photometries` struct models a collection of photometric data entries within the GLDF file.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Photometries {
    /// The list of photometric data entries.
    #[serde(rename = "Photometry", default)]
    pub photometry: Vec<Photometry>,
}

/// Represents a reference to a spectrum file.
///
/// The `SpectrumFileReference` struct models a reference to a spectrum file within the GLDF file.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectrumFileReference {
    /// The ID of the referenced file.
    #[serde(rename = "@fileId")]
    pub file_id: String,
}

/// Represents the intensity of a wavelength in a spectrum.
///
/// The `Intensity` struct models the intensity of a specific wavelength within a spectrum.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Intensity {
    /// The wavelength of the intensity value.
    #[serde(rename = "@wavelength", skip_serializing_if = "Option::is_none")]
    pub wavelength: Option<i32>,

    /// The intensity value at the specified wavelength.
    #[serde(rename = "$text", skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
}

/// Represents spectral data for a light source.
///
/// The `Spectrum` struct models spectral data for a light source within the GLDF file.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spectrum {
    /// The ID of the spectrum.
    #[serde(rename = "@id")]
    pub id: String,

    /// A reference to the spectrum file.
    #[serde(rename = "SpectrumFileReference")]
    pub spectrum_file_reference: SpectrumFileReference,

    /// The list of intensity values at different wavelengths.
    #[serde(rename = "Intensity", default)]
    pub intensity: Vec<Intensity>,
}

/// Represents a collection of spectral data.
///
/// The `Spectrums` struct models a collection of spectral data entries within the GLDF file.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spectrums {
    /// The list of spectral data entries.
    #[serde(rename = "Spectrum", default)]
    pub spectrum: Vec<Spectrum>,
}
