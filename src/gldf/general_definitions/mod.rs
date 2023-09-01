#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
use super::header;
/// the module sensors describes the sensors of the GLDF file
pub mod sensors;
pub use sensors::*;
/// the module files describes the files of the GLDF file
pub mod files;
pub use files::*;
/// the module photometries describes the photometries of the GLDF file
pub mod photometries;

pub use photometries::*;
/// the module lightsources describes the light sources of the GLDF file
pub mod lightsources;
pub use lightsources::*;
/// the module electical describes the electrical details of the GLDF file
pub mod electrical;
/// the module geeometries describes the geometries of the GLDF file
pub mod geometries;
pub use geometries::*;


use serde::{Serialize};
use serde::Deserialize;
use yaserde_derive::YaDeserialize;
use yaserde_derive::YaSerialize;

#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
/// the GeneralDefinitions struct describes the general definitions of the GLDF file
pub struct GeneralDefinitions {
    /// A collection of files referenced in the GLDF file.
    #[yaserde(child)]
    #[yaserde(rename = "Files")]
    #[serde(rename = "Files")]
    pub files: Files,

    /// A collection of sensor definitions.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[yaserde(child)]
    #[yaserde(rename = "Sensors")]
    #[serde(rename = "Sensors", skip_serializing_if = "Option::is_none")]
    pub sensors: Option<Sensors>,

    /// A collection of photometry definitions.
    #[yaserde(child)]
    #[yaserde(rename = "Photometries")]
    #[serde(rename = "Photometries")]
    pub photometries: Option<Photometries>,

    /// A collection of spectrum definitions.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[yaserde(child)]
    #[yaserde(rename = "Spectrums")]
    #[serde(rename = "Spectrums", skip_serializing_if = "Option::is_none")]
    pub spectrums: Option<Spectrums>,

    /// A collection of light source definitions.
    #[yaserde(child)]
    #[yaserde(rename = "LightSources")]
    #[serde(rename = "LightSources")]
    pub light_sources: Option<LightSources>,

    /// A collection of control gear definitions.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[yaserde(child)]
    #[yaserde(rename = "ControlGears")]
    #[serde(rename = "ControlGears", skip_serializing_if = "Option::is_none")]
    pub control_gears: Option<ControlGears>,

    /// A collection of equipment definitions.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[yaserde(child)]
    #[yaserde(rename = "Equipments")]
    #[serde(rename = "Equipments", skip_serializing_if = "Option::is_none")]
    pub equipments: Option<Equipments>,

    /// A collection of emitter definitions.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[yaserde(child)]
    #[yaserde(rename = "Emitters")]
    #[serde(rename = "Emitters", skip_serializing_if = "Option::is_none")]
    pub emitters: Option<Emitters>,

    /// A collection of geometry definitions.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[yaserde(rename = "Geometries")]
    #[serde(rename = "Geometries", skip_serializing_if = "Option::is_none")]
    #[yaserde(child)]
    pub geometries: Option<Geometries>,
}
/// The ProductSerie struct describes a product serie of the GLDF file.
///
/// This struct provides information about a product series, including its name, description,
/// pictures, and hyperlinks. It allows you to define characteristics and attributes that apply
/// to a specific series of products within the GLDF data structure.
///
/// # Examples
///
/// ```rust
/// use gldf_rs::gldf::{ProductSerie, Locale, LocaleFoo, Image, Images, Hyperlink, Hyperlinks};
///
/// let product_serie = ProductSerie {
///     name: Some(LocaleFoo {
///         locale: vec![Locale {
///             language: String::from("en"),
///             value: String::from("Product Series A")
///         }],
///     }),
///     description: Some(LocaleFoo {
///         locale: vec![Locale {
///             language: String::from("en"),
///             value: String::from("This is a description of Product Series A"),
///         }]}),
///     pictures: Some(Images {
///         image: vec![Image{ image_type: String::from("png"), file_id: String::from("image_01") }],
///     }),
///     hyperlinks: Some(Hyperlinks {
///         hyperlink: vec![Hyperlink{
///             href: String::from("https://example.com"),
///             language: Some(String::from("en")),
///             region: None,
///             country_code: None,
///             value: String::from("Example Link"),
///         }],
///     }),
/// };
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductSerie {
    /// The name of the product series.
    #[yaserde(rename = "Name")]
    #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
    pub name: Option<LocaleFoo>,

    /// A description of the product series.
    #[yaserde(rename = "Description")]
    #[serde(rename = "Description", skip_serializing_if = "Option::is_none")]
    pub description: Option<LocaleFoo>,

    /// Pictures associated with the product series.
    #[yaserde(rename = "Pictures")]
    #[serde(rename = "Pictures", skip_serializing_if = "Option::is_none")]
    pub pictures: Option<Images>,

    /// Hyperlinks related to the product series.
    #[yaserde(rename = "Hyperlinks")]
    #[serde(rename = "Hyperlinks", skip_serializing_if = "Option::is_none")]
    pub hyperlinks: Option<Hyperlinks>,
}

/// The ProductSeries struct represents a collection of product series in the GLDF file.
///
/// This struct holds a list of product series, each represented by the `ProductSerie` struct.
/// It allows you to organize and categorize multiple product series within the GLDF data structure.
///
/// # Examples
///
/// ```rust
/// use gldf_rs::gldf::{ProductSeries, ProductSerie};
///
/// let product_series = ProductSeries {
///     product_serie: vec![
///         ProductSerie {
///         name: None,description: None,pictures: None,hyperlinks: None,},
///         // ... more product series
///     ],
/// };
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductSeries {
    /// A list of product series.
    #[yaserde(rename = "ProductSerie")]
    #[serde(rename = "ProductSerie")]
    pub product_serie: Vec<ProductSerie>,
}


/// The Image struct describes an image in the GLDF file.
///
/// This struct provides information about an image, including its type and file ID.
/// It allows you to reference and associate images with other elements in the GLDF data structure.
///
/// # Examples
///
/// ```rust
/// use gldf_rs::gldf::{Image};
///
/// let image = Image {
///     image_type: String::from("png"),
///     file_id: String::from("image_01"),
/// };
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Image {
    /// The type of the image (e.g., "png", "jpg").
    #[yaserde(attribute, rename = "imageType")]
    #[serde(rename = "@imageType")]
    pub image_type: String,

    /// The file ID of the image.
    #[yaserde(attribute, rename = "fileId")]
    #[serde(rename = "@fileId")]
    pub file_id: String,
}


/// The Images struct describes a collection of images in the GLDF file.
///
/// This struct holds multiple `Image` instances and allows you to group and associate
/// multiple images with other elements in the GLDF data structure.
///
/// # Examples
///
/// ```rust
/// use gldf_rs::gldf::{Images, Image};
///
/// let image1 = Image {
///     image_type: String::from("png"),
///     file_id: String::from("image_01"),
/// };
///
/// let image2 = Image {
///     image_type: String::from("jpg"),
///     file_id: String::from("image_02"),
/// };
///
/// let images = Images {
///     image: vec![image1, image2],
/// };
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Images {
    /// A vector of `Image` instances.
    #[yaserde(rename = "Image")]
    #[serde(rename = "Image")]
    pub image: Vec<Image>,
}

/// The Hyperlink struct describes a hyperlink in the GLDF file.
///
/// This struct provides information about a hyperlink, including its href, language,
/// region, country code, and value. It allows you to associate hyperlinks with other
/// elements in the GLDF data structure.
///
/// # Examples
///
/// ```rust
/// use gldf_rs::gldf::{Hyperlink};
///
/// let hyperlink = Hyperlink {
///     href: String::from("https://example.com"),
///     language: Some(String::from("en")),
///     region: None,
///     country_code: None,
///     value: String::from("Example Link"),
/// };
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Hyperlink {
    /// The hyperlink's href (URL or URI).
    #[yaserde(attribute, rename = "href")]
    #[serde(rename = "@href")]
    pub href: String,

    /// The language of the hyperlink.
    #[yaserde(rename = "language")]
    #[serde(rename = "language", skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// The region of the hyperlink.
    #[yaserde(rename = "region")]
    #[serde(rename = "region", skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    /// The country code of the hyperlink.
    #[yaserde(rename = "countryCode")]
    #[serde(rename = "countryCode", skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,

    /// The value of the hyperlink.
    #[yaserde(text)]
    #[yaserde(rename = "$value")]
    #[serde(rename = "$")]
    pub value: String,
}

/// The Hyperlinks struct describes a collection of hyperlinks in the GLDF file.
///
/// This struct holds multiple `Hyperlink` instances and allows you to group and associate
/// multiple hyperlinks with other elements in the GLDF data structure.
///
/// # Examples
///
/// ```rust
/// use gldf_rs::gldf::{Hyperlinks, Hyperlink};
///
/// let hyperlink1 = Hyperlink {
///     href: String::from("https://example1.com"),
///     language: Some(String::from("en")),
///     region: None,
///     country_code: None,
///     value: String::from("Example Link 1"),
/// };
///
/// let hyperlink2 = Hyperlink {
///     href: String::from("https://example2.com"),
///     language: Some(String::from("fr")),
///     region: None,
///     country_code: None,
///     value: String::from("Example Link 2"),
/// };
///
/// let hyperlinks = Hyperlinks {
///     hyperlink: vec![hyperlink1, hyperlink2],
/// };
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Hyperlinks {
    /// A vector of `Hyperlink` instances.
    #[yaserde(rename = "Hyperlink")]
    #[serde(rename = "Hyperlink")]
    pub hyperlink: Vec<Hyperlink>,
}
