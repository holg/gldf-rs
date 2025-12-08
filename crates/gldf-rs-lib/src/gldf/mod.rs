#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
#![allow(non_local_definitions)]

/// The gldf header module (src/gldf/header.rs)
pub mod header;
pub use header::*;

/// The gldf general definitions module (src/gldf/general_definitions.rs)
pub mod general_definitions;

/// The gldf product definitions module (src/gldf/product_definitions.rs)
pub mod product_definitions;

/// The gldf meta information module (src/gldf/meta_information.rs)
/// not strictly needed but it is used in the gldf-sign
pub mod meta_information;

pub use general_definitions::*;
pub use product_definitions::*;
use serde::{Deserialize, Serialize};

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
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Root")]
pub struct GldfProduct {
    /// This field is not serialized or deserialized but can be used to store the path to the
    /// GLDF product file.
    #[serde(skip)]
    pub path: String,

    /// The XML namespace for xsi (XML Schema Instance).
    #[serde(rename = "@xmlns:xsi", default = "get_xmlns_xsi")]
    pub xmlns_xsi: String,

    /// The xsi:noNamespaceSchemaLocation attribute specifying the schema location.
    #[serde(
        rename = "@xsi:noNamespaceSchemaLocation",
        default = "get_xsnonamespaceschemalocation"
    )]
    pub xsnonamespaceschemalocation: String,

    /// The header of the GLDF product.
    #[serde(rename = "Header")]
    pub header: Header,

    /// The general definitions section of the GLDF product.
    #[serde(rename = "GeneralDefinitions")]
    pub general_definitions: GeneralDefinitions,

    /// The product definitions section of the GLDF product.
    #[serde(rename = "ProductDefinitions")]
    pub product_definitions: ProductDefinitions,
}
