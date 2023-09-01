#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
use serde::{Serialize};
use serde::Deserialize;
use yaserde_derive::YaDeserialize;
use yaserde_derive::YaSerialize;
/// Represents a reference to a photometry file.
///
/// The `PhotometryFileReference` struct models a reference to a photometry file within the GLDF file.
/// It includes the ID of the referenced file. It supports serialization and deserialization
/// of XML data for working with photometry file references.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PhotometryFileReference {
    /// The ID of the referenced file.
    #[yaserde(rename = "fileId", attribute)]
    #[serde(rename = "@fileId")]
    pub file_id: String,
}

/// Represents the tenth peak divergence of a photometry.
///
/// The `TenthPeakDivergence` struct models the tenth peak divergence of a photometry within the GLDF file.
/// It includes divergence values for angles `C0-C180` and `C90-C270`. It supports serialization and deserialization
/// of XML data for working with tenth peak divergence.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct TenthPeakDivergence {
    /// The divergence value for angles `C0-C180`.
    #[yaserde(rename = "C0-C180")]
    #[serde(rename = "C0-C180", skip_serializing_if = "Option::is_none")]
    pub c0_c180: Option<f64>,

    /// The divergence value for angles `C90-C270`.
    #[yaserde(rename = "C90-C270")]
    #[serde(rename = "C90-C270", skip_serializing_if = "Option::is_none")]
    pub c90_c270: Option<f64>,
}

/// Represents the half peak divergence of a photometry.
///
/// The `HalfPeakDivergence` struct models the half peak divergence of a photometry within the GLDF file.
/// It includes divergence values for angles `C0-C180` and `C90-C270`. It supports serialization and deserialization
/// of XML data for working with half peak divergence.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct HalfPeakDivergence {
    /// The divergence value for angles `C0-C180`.
    #[yaserde(rename = "C0-C180")]
    #[serde(rename = "C0-C180", skip_serializing_if = "Option::is_none")]
    pub c0_c180: Option<f64>,

    /// The divergence value for angles `C90-C270`.
    #[yaserde(rename = "C90-C270")]
    #[serde(rename = "C90-C270", skip_serializing_if = "Option::is_none")]
    pub c90_c270: Option<f64>,
}


/// Represents the Unified Glare Rating (UGR) photometric data.
///
/// The `UGR4H8H705020LQ` struct models the UGR4H8H705020LQ photometric data within the GLDF file.
/// Unified Glare Rating (UGR) is a metric used to assess the discomfort glare experienced by an
/// observer due to luminaires and their arrangement. The `UGR4H8H705020LQ` data provides values
/// for coordinates `X` and `Y`, which are used to calculate UGR levels.
///
/// UGR values are typically calculated using photometric measurements and simulations to ensure
/// optimal lighting conditions in indoor spaces. The UGR values help lighting designers and
/// engineers to create lighting setups that minimize discomfort glare and create visually
/// comfortable environments for occupants.
///
/// The `UGR4H8H705020LQ` photometric data is used in the context of lighting design and evaluation
/// to quantify the level of glare. The `X` and `Y` coordinates are important components of UGR
/// calculations.
///
/// Example usage of `UGR4H8H705020LQ`:
///
/// ```rust
/// use gldf_rs::gldf::UGR4H8H705020LQ;
///
/// let ugr_data = UGR4H8H705020LQ {
///     x: Some(0.5),
///     y: Some(0.3),
/// };
/// ```
///
/// For more information about UGR and its calculation, refer to relevant lighting design
/// standards and guidelines.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct UGR4H8H705020LQ {
    /// The value of the `X` coordinate.
    #[yaserde(rename = "X")]
    #[serde(rename = "X", skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,

    /// The value of the `Y` coordinate.
    #[yaserde(rename = "Y")]
    #[serde(rename = "Y", skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
}

/// Represents descriptive photometric information about a lighting product.
///
/// The `DescriptivePhotometry` struct models various photometric properties of a lighting product.
/// It includes information such as luminaire luminance, light output ratio, luminous efficacy,
/// flux fractions, divergence angles, photometric codes, and more. It supports serialization and
/// deserialization of XML data for working with descriptive photometric data.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DescriptivePhotometry {
    /// Luminance of the luminaire, usually expressed in candelas per square meter (cd/m²).
    #[yaserde(rename = "LuminaireLuminance")]
    #[serde(rename = "LuminaireLuminance", skip_serializing_if = "Option::is_none")]
    pub luminaire_luminance: Option<i32>,

    /// Ratio of the total light output of the luminaire to the total input power.
    #[yaserde(rename = "LightOutputRatio")]
    #[serde(rename = "LightOutputRatio", skip_serializing_if = "Option::is_none")]
    pub light_output_ratio: Option<f64>,

    /// Luminous efficacy of the luminaire, measured in lumens per watt (lm/W).
    #[yaserde(rename = "LuminousEfficacy")]
    #[serde(rename = "LuminousEfficacy", skip_serializing_if = "Option::is_none")]
    pub luminous_efficacy: Option<f64>,

    /// Fraction of light emitted by the luminaire that is directed downwards.
    #[yaserde(rename = "DownwardFluxFraction")]
    #[serde(rename = "DownwardFluxFraction", skip_serializing_if = "Option::is_none")]
    pub downward_flux_fraction: Option<f64>,

    /// Ratio of the downward light output to the total light output of the luminaire.
    #[yaserde(rename = "DownwardLightOutputRatio")]
    #[serde(rename = "DownwardLightOutputRatio", skip_serializing_if = "Option::is_none")]
    pub downward_light_output_ratio: Option<f64>,

    /// Ratio of the upward light output to the total light output of the luminaire.
    #[yaserde(rename = "UpwardLightOutputRatio")]
    #[serde(rename = "UpwardLightOutputRatio", skip_serializing_if = "Option::is_none")]
    pub upward_light_output_ratio: Option<f64>,

    /// Divergence angles for the luminaire's light distribution.
    #[yaserde(rename = "TenthPeakDivergence")]
    #[serde(rename = "TenthPeakDivergence", skip_serializing_if = "Option::is_none")]
    pub tenth_peak_divergence: Option<TenthPeakDivergence>,

    /// Divergence angles for the luminaire's light distribution.
    #[yaserde(rename = "HalfPeakDivergence")]
    #[serde(rename = "HalfPeakDivergence", skip_serializing_if = "Option::is_none")]
    pub half_peak_divergence: Option<HalfPeakDivergence>,

    /// Code indicating the photometric type of the luminaire.
    #[yaserde(rename = "PhotometricCode")]
    #[serde(rename = "PhotometricCode", skip_serializing_if = "Option::is_none")]
    pub photometric_code: Option<String>,

    /// Code indicating the CIE flux group of the luminaire.
    #[yaserde(rename = "CIE-FluxCode")]
    #[serde(rename = "CIE-FluxCode", skip_serializing_if = "Option::is_none")]
    pub cie_flux_code: Option<String>,

    /// Angle at which the luminous intensity is reduced by half.
    #[yaserde(rename = "CutOffAngle")]
    #[serde(rename = "CutOffAngle", skip_serializing_if = "Option::is_none")]
    pub cut_off_angle: Option<f64>,

    /// Unified Glare Rating (UGR) values indicating perceived glare for indoor lighting.
    #[yaserde(rename = "UGR-4H8H-70-50-20-LQ")]
    #[serde(rename = "UGR-4H8H-70-50-20-LQ", skip_serializing_if = "Option::is_none")]
    pub ugr4_h8_h705020_lq: Option<UGR4H8H705020LQ>,

    /// Definition of the light distribution pattern according to IESNA standards.
    #[yaserde(rename = "IESNA-LightDistributionDefinition")]
    #[serde(rename = "IESNA-LightDistributionDefinition", skip_serializing_if = "Option::is_none")]
    pub iesna_light_distribution_definition: Option<String>,

    /// BUG (Backlight, Uplight, Glare) rating indicating light distribution and potential for discomfort glare.
    #[yaserde(rename = "LightDistributionBUG-Rating")]
    #[serde(rename = "LightDistributionBUG-Rating", skip_serializing_if = "Option::is_none")]
    pub light_distribution_bug_rating: Option<String>,
}

/// Represents photometric information about a lighting product.
///
/// The `Photometry` struct models photometric data related to a lighting product. It includes
/// information about the photometry ID, photometry file references, and descriptive photometric
/// details. It supports serialization and deserialization of XML data for working with photometry.
/// Example of how to construct a `Photometry` instance:
/// ```
/// use gldf_rs::gldf::{Photometry, PhotometryFileReference, DescriptivePhotometry, TenthPeakDivergence, HalfPeakDivergence, UGR4H8H705020LQ};
///
/// let photometry_data = Photometry {
///     id: "photometry123".to_string(),
///     photometry_file_reference: Some(PhotometryFileReference {
///         file_id: "file456".to_string(),
///     }),
///     descriptive_photometry: Some(DescriptivePhotometry {
///         luminaire_luminance: Some(500),
///         light_output_ratio: Some(0.8),
///         luminous_efficacy: Some(100.0),
///         downward_flux_fraction: Some(0.7),
///         downward_light_output_ratio: Some(0.6),
///         upward_light_output_ratio: Some(0.4),
///         tenth_peak_divergence: Some(TenthPeakDivergence {
///             c0_c180: Some(10.0),
///             c90_c270: Some(8.0),
///         }),
///         half_peak_divergence: Some(HalfPeakDivergence {
///             c0_c180: Some(15.0),
///             c90_c270: Some(12.0),
///         }),
///         photometric_code: Some("C".to_string()),
///         cie_flux_code: Some("A".to_string()),
///         cut_off_angle: Some(45.0),
///         ugr4_h8_h705020_lq: Some(UGR4H8H705020LQ {
///             x: Some(18.0),
///             y: Some(16.0),
///         }),
///         iesna_light_distribution_definition: Some("Type C".to_string()),
///         light_distribution_bug_rating: Some("B2".to_string()),
///         // Add other fields here...
///     }),
/// };
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Photometry {
    /// The unique identifier for the photometry data.
    #[yaserde(rename = "id", attribute)]
    #[serde(rename = "@id")]
    pub id: String,

    /// Reference to a photometry file.
    ///
    /// This field is optional and may not be present in all photometric data.
    #[yaserde(rename = "PhotometryFileReference")]
    #[serde(rename = "PhotometryFileReference", skip_serializing_if = "Option::is_none")]
    pub photometry_file_reference: Option<PhotometryFileReference>,

    /// Descriptive photometric information about the lighting product.
    ///
    /// This field is optional and may not be present in all photometric data.
    #[yaserde(rename = "DescriptivePhotometry")]
    #[serde(rename = "DescriptivePhotometry", skip_serializing_if = "Option::is_none")]
    pub descriptive_photometry: Option<DescriptivePhotometry>,
}



/// Represents a collection of photometric data.
///
/// The `Photometries` struct models a collection of photometric data entries within the GLDF file.
/// It contains a list of individual `Photometry` instances, each representing photometric data.
/// It supports serialization and deserialization of XML data for working with photometric data.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Photometries {
    /// The list of photometric data entries.
    #[yaserde(rename = "Photometry")]
    #[serde(rename = "Photometry")]
    pub photometry: Vec<Photometry>,
}

/// Represents a reference to a spectrum file.
///
/// The `SpectrumFileReference` struct models a reference to a spectrum file within the GLDF file.
/// It includes attributes such as the ID of the referenced file. It supports serialization and
/// deserialization of XML data for working with spectrum file references.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SpectrumFileReference {
    /// The ID of the referenced file.
    #[yaserde(rename = "fileId")]
    #[serde(rename = "fileId")]
    pub file_id: String,
}

/// Represents the intensity of a wavelength in a spectrum.
///
/// The `Intensity` struct models the intensity of a specific wavelength within a spectrum. It
/// includes attributes such as the wavelength and its corresponding intensity value. It supports
/// serialization and deserialization of XML data for working with intensity values in spectra.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Intensity {
    /// The wavelength of the intensity value.
    #[yaserde(rename = "wavelength")]
    #[serde(rename = "wavelength", skip_serializing_if = "Option::is_none")]
    pub wavelength: Option<i32>,

    /// The intensity value at the specified wavelength.
    #[yaserde(rename = "$value")]
    #[serde(rename = "$value", skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
}

/// Example of how to construct a `Photometries` instance:
///
/// ```
/// use gldf_rs::gldf::{Photometries, Photometry, PhotometryFileReference, Intensity};
///
/// let photometries_data = Photometries {
///     photometry: vec![
///         Photometry {
///             id: "photometry123".to_string(),
///             photometry_file_reference: Some(PhotometryFileReference {
///                 file_id: "file456".to_string(),
///             }),
///             descriptive_photometry: None,
///         },
///         // Add other photometry entries here...
///     ],
/// };
/// ```

/// Represents spectral data for a light source.
///
/// The `Spectrum` struct models spectral data for a light source within the GLDF file. It includes
/// information about the spectrum's ID, a reference to the spectrum file, and a list of intensity
/// values at different wavelengths. It supports serialization and deserialization of XML data for
/// working with spectral data.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Spectrum {
    /// The ID of the spectrum.
    #[yaserde(rename = "id")]
    #[serde(rename = "id")]
    pub id: String,

    /// A reference to the spectrum file.
    #[yaserde(rename = "SpectrumFileReference")]
    #[serde(rename = "SpectrumFileReference")]
    pub spectrum_file_reference: SpectrumFileReference,

    /// The list of intensity values at different wavelengths.
    #[yaserde(rename = "Intensity")]
    #[serde(rename = "Intensity")]
    pub intensity: Vec<Intensity>,
}

/// Represents a collection of spectral data.
///
/// The `Spectrums` struct models a collection of spectral data entries within the GLDF file. It
/// contains a list of individual `Spectrum` instances, each representing spectral data for a
/// light source. It supports serialization and deserialization of XML data for working with
/// spectral data collections.
/// /// Example of how to construct a `Spectrums` instance:
///
/// ```
/// use gldf_rs::gldf::{Spectrums, Spectrum, SpectrumFileReference, Intensity};
///
/// let spectrums_data = Spectrums {
///     spectrum: vec![
///         Spectrum {
///             id: "spectrum123".to_string(),
///             spectrum_file_reference: SpectrumFileReference {
///                 file_id: "file789".to_string(),
///             },
///             intensity: vec![
///                 Intensity {
///                     wavelength: Some(400),
///                     value: Some(0.5),
///                 },
///                 // Add more intensity entries...
///             ],
///         },
///         // Add other spectrum entries here...
///     ],
/// };
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Spectrums {
    /// The list of spectral data entries.
    #[yaserde(rename = "Spectrum")]
    #[serde(rename = "Spectrum")]
    /// the list of spectral data entries
    pub spectrum: Vec<Spectrum>,
}


