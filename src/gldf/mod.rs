#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

/// The gldf header module (src/gldf/header.rs)
pub mod header;
pub use header::*;
/// The gldf general definitions module (src/gldf/general_definitions.rs)
pub mod general_definitions;
/// The gldf product definitions module (src/gldf/product_definitions.rs)
pub mod product_definitions;
/// The gldf meta information module (src/gldf/meta_information.rs)
/// not strictly neded but it is used in the gldf-sign
pub mod meta_information;

use serde::{Serialize};
use serde::Deserialize;
use yaserde_derive::YaDeserialize;
use yaserde_derive::YaSerialize;
pub use general_definitions::*;
pub use product_definitions::*;
fn get_xsnonamespaceschemalocation() -> String {
    "https://gldf.io/xsd/gldf/1.0.0-rc.1/gldf.xsd".to_string()
}

fn get_xmlns_xsi() -> String {
    "http://www.w3.org/2001/XMLSchema-instance".to_string()
}

/// Represents a GLDF (Global Lighting Data Format) product.
///
/// GLDFProduct is a Rust struct that models a product in the Global Lighting Data Format (GLDF).
/// It provides serialization and deserialization methods for working with GLDF XML data.
///
/// This struct is intended to represent GLDF products conforming to the schema specified at
/// `<https://gldf.io/xsd/gldf/1.0.0-rc.1/gldf.xsd.>`
///
/// The path to the GLDF product.
///
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
#[yaserde(
rename = "Root",
root = "Root",
namespace = "xsi: http://www.w3.org/2001/XMLSchema-instance",
namespace = "xsi:noNamespaceSchemaLocation: https://gldf.io/xsd/gldf/1.0.0-rc.1/gldf.xsd",
xsi: noNamespaceSchemaLocation = "https://gldf.io/xsd/gldf/1.0.0-rc.1/gldf.xsd"
)]
pub struct GldfProduct {
    /// This field is not serialized or deserialized but can be used to store the path to the
    /// GLDF product file.
    #[serde(skip_serializing, skip_deserializing)]
    #[yaserde(skip_serializing, skip_deserializing)]
    pub path: String,

    /// The XML namespace for xsi (XML Schema Instance).
    #[yaserde(attribute, rename = "xmlns:xsi", default = "get_xmlns_xsi", skip_deserializing)]
    #[serde(rename = "@xmlns:xsi")]
    pub xmlns_xsi: String,

    /// The xsi:noNamespaceSchemaLocation attribute specifying the schema location.
    #[serde(rename = "@xsi:noNamespaceSchemaLocation")]
    #[yaserde(
    attribute,
    rename = "xsi:noNamespaceSchemaLocation",
    default = "get_xsnonamespaceschemalocation",
    prefix = xsi,
    text
    )]
    pub xsnonamespaceschemalocation: String,

    /// The header of the GLDF product.
    #[yaserde(rename = "Header")]
    #[serde(rename = "Header")]
    pub header: Header,

    /// The general definitions section of the GLDF product.
    #[yaserde(child)]
    #[yaserde(rename = "GeneralDefinitions")]
    #[serde(rename = "GeneralDefinitions")]
    pub general_definitions: GeneralDefinitions,

    /// The product definitions section of the GLDF product.
    #[yaserde(child)]
    #[yaserde(rename = "ProductDefinitions")]
    #[serde(rename = "ProductDefinitions")]
    pub product_definitions: ProductDefinitions,
}


