#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

// Re-export header types from parent module
pub use super::header::*;

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

/// the module electrical describes the electrical details of the GLDF file
pub mod electrical;
pub use electrical::*;

/// the module geometries describes the geometries of the GLDF file
pub mod geometries;
pub use geometries::*;

use serde::{Deserialize, Serialize};

fn is_emitters_empty(emitters: &Option<Emitters>) -> bool {
    match emitters {
        None => true,
        Some(e) => e.is_empty(),
    }
}

/// The GeneralDefinitions struct describes the general definitions of the GLDF file
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneralDefinitions {
    /// A collection of files referenced in the GLDF file.
    #[serde(rename = "Files")]
    pub files: Files,

    /// A collection of sensor definitions.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[serde(rename = "Sensors", skip_serializing_if = "Option::is_none")]
    pub sensors: Option<Sensors>,

    /// A collection of photometry definitions.
    #[serde(rename = "Photometries", skip_serializing_if = "Option::is_none")]
    pub photometries: Option<Photometries>,

    /// A collection of spectrum definitions.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[serde(rename = "Spectrums", skip_serializing_if = "Option::is_none")]
    pub spectrums: Option<Spectrums>,

    /// A collection of light source definitions.
    #[serde(rename = "LightSources", skip_serializing_if = "Option::is_none")]
    pub light_sources: Option<LightSources>,

    /// A collection of control gear definitions.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[serde(rename = "ControlGears", skip_serializing_if = "Option::is_none")]
    pub control_gears: Option<ControlGears>,

    /// A collection of equipment definitions.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[serde(rename = "Equipments", skip_serializing_if = "Option::is_none")]
    pub equipments: Option<Equipments>,

    /// A collection of emitter definitions.
    ///
    /// This field is optional and may not be present in all GLDF files.
    /// Skipped if empty or contains only empty emitters.
    #[serde(rename = "Emitters", skip_serializing_if = "is_emitters_empty")]
    pub emitters: Option<Emitters>,

    /// A collection of geometry definitions.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[serde(rename = "Geometries", skip_serializing_if = "Option::is_none")]
    pub geometries: Option<Geometries>,
}

/// The ProductSerie struct describes a product serie of the GLDF file.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductSerie {
    /// The unique identifier for the product series (required in rc.3).
    #[serde(rename = "@id", default)]
    pub id: String,

    /// The name of the product series.
    #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
    pub name: Option<LocaleFoo>,

    /// A description of the product series.
    #[serde(rename = "Description", skip_serializing_if = "Option::is_none")]
    pub description: Option<LocaleFoo>,

    /// Pictures associated with the product series.
    #[serde(rename = "Pictures", skip_serializing_if = "Option::is_none")]
    pub pictures: Option<Images>,

    /// Hyperlinks related to the product series.
    #[serde(rename = "Hyperlinks", skip_serializing_if = "Option::is_none")]
    pub hyperlinks: Option<Hyperlinks>,
}

/// The ProductSeries struct represents a collection of product series in the GLDF file.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductSeries {
    /// A list of product series.
    #[serde(rename = "ProductSerie", default)]
    pub product_serie: Vec<ProductSerie>,
}

/// The Image struct describes an image in the GLDF file.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
    /// The type of the image (e.g., "png", "jpg").
    #[serde(rename = "@imageType")]
    pub image_type: String,

    /// The file ID of the image.
    #[serde(rename = "@fileId")]
    pub file_id: String,
}

/// The Images struct describes a collection of images in the GLDF file.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Images {
    /// A vector of `Image` instances.
    #[serde(rename = "Image", default)]
    pub image: Vec<Image>,
}

/// The Hyperlink struct describes a hyperlink in the GLDF file.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hyperlink {
    /// The hyperlink's href (URL or URI).
    #[serde(rename = "@href")]
    pub href: String,

    /// The language of the hyperlink.
    #[serde(rename = "@language", skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// The region of the hyperlink.
    #[serde(rename = "@region", skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    /// The country code of the hyperlink.
    #[serde(rename = "@countryCode", skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,

    /// The value of the hyperlink.
    #[serde(rename = "$text")]
    pub value: String,
}

/// The Hyperlinks struct describes a collection of hyperlinks in the GLDF file.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hyperlinks {
    /// A vector of `Hyperlink` instances.
    #[serde(rename = "Hyperlink", default)]
    pub hyperlink: Vec<Hyperlink>,
}
