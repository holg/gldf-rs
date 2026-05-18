#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

use serde::{Deserialize, Serialize};

/// Represents a file entry.
///
/// The `File` struct models a file entry within the GLDF file, including attributes such as ID,
/// content type, and file name. It supports serialization and deserialization of XML data for
/// working with file entries.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct File {
    /// The ID of the file.
    #[serde(rename = "@id")]
    pub id: String,

    /// The content type of the file.
    #[serde(rename = "@contentType")]
    pub content_type: String,

    /// The type of the file (localFileName or url).
    #[serde(rename = "@type", default)]
    pub type_attr: String,

    /// The language of the file (optional).
    #[serde(
        rename = "@language",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub language: String,

    /// The name of the file.
    #[serde(rename = "$text")]
    pub file_name: String,
}

/// Represents a collection of file entries.
///
/// The `Files` struct models a collection of file entries within the GLDF file. It contains a list
/// of individual `File` instances. It supports serialization and deserialization of XML data for
/// working with file collections.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Files {
    /// The list of file entries.
    #[serde(rename = "File", default)]
    pub file: Vec<File>,
}

impl Files {
    /// Whether the collection has zero file entries. Used to skip
    /// serialization of an empty `<Files/>` element (XSD requires at
    /// least one `<File>` child if Files is present).
    pub fn is_empty(&self) -> bool {
        self.file.is_empty()
    }
}

/// Free function used by `serde(skip_serializing_if = ...)` because the
/// macro takes a path-string and resolves it at expansion time without
/// `&self`. Forwards to [`Files::is_empty`].
pub(crate) fn files_is_empty(f: &Files) -> bool {
    f.is_empty()
}
