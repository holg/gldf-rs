#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
use serde::{Serialize};
use serde::Deserialize;
use yaserde_derive::YaDeserialize;
use yaserde_derive::YaSerialize;

/// Represents a file entry.
///
/// The `File` struct models a file entry within the GLDF file, including attributes such as ID,
/// content type, and file name. It supports serialization and deserialization of XML data for
/// working with file entries.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct File {
    /// The ID of the file.
    #[yaserde(attribute)]
    #[yaserde(rename = "id")]
    #[serde(rename = "@id")]
    pub id: String,

    /// The content type of the file.
    #[yaserde(attribute)]
    #[yaserde(rename = "contentType")]
    #[serde(rename = "@contentType")]
    pub content_type: String,

    /// The type of the file.
    #[yaserde(attribute)]
    #[yaserde(rename = "type")]
    #[serde(rename = "@type")]
    pub type_attr: String,

    /// The name of the file.
    #[yaserde(text)]
    #[serde(rename = "$")]
    pub file_name: String,
}

/// Represents a collection of file entries.
///
/// The `Files` struct models a collection of file entries within the GLDF file. It contains a list
/// of individual `File` instances. It supports serialization and deserialization of XML data for
/// working with file collections.
/// /// Example of how to construct a `Files` instance:
// ///
// /// ```
// /// use gldf_rs::gldf::{Files, File};
// ///
// /// let file_entries = Files {
// ///     file: vec![
// ///         File {
// ///             id: "file123".to_string(),
// ///             content_type: "image/jpeg".to_string(),
// ///             type_attr: "Thumbnail".to_string(),
// ///             file_name: "thumbnail.jpg".to_string(),
// ///         },
// ///         // ... (add more file entries as needed)
// ///     ],
// /// };
// /// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Files {
    /// The list of file entries.
    #[yaserde(child)]
    #[yaserde(rename = "File")]
    #[serde(rename = "File")]
    // a collection of files referenced in the GLDF file
    pub file: Vec<File>,
}


