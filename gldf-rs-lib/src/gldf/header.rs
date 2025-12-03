#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

use serde::{Deserialize, Serialize};

/// Represents the format version
/// some more needed
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "FormatVersion")]
pub struct FormatVersion {
    /// The major version number.
    #[serde(rename = "@major", default)]
    pub major: i32,

    /// The minor version number.
    #[serde(rename = "@minor", default)]
    pub minor: i32,

    /// The pre-release version number.
    #[serde(rename = "@pre-release", default, skip_serializing_if = "is_zero")]
    pub pre_release: i32,
}

fn is_zero(val: &i32) -> bool {
    *val == 0
}

/// Represents the format version of the GLDF file.
///
/// The `FormatVersion` struct models the version information of the GLDF file format.
/// It includes major, minor, and pre-release version numbers.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
#[yaserde(rename = "FormatVersion")]
pub struct FormatVersion {
    /// The application associated with the license key.
    #[yaserde(attribute)]
    #[yaserde(rename = "major")]
    #[serde(rename = "major")]
    pub major: i32,

    /// The application associated with the license key.
    #[yaserde(attribute)]
    #[yaserde(rename = "minor")]
    #[serde(rename = "minor")]
    pub minor: i32,

    /// The application associated with the license key.
    #[yaserde(attribute)]
    #[yaserde(rename = "pre-release")]
    #[serde(rename = "pre-release")]
    pub pre_release: i32,
}

/// Represents a license key.
/// Optionally, a license key can be associated with an application.
/// LicenseKey is a Rust struct that models a license key. It provides serialization and
/// deserialization methods for working with license key XML data.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "LicenseKey")]
pub struct LicenseKey {
    /// The application associated with the license key.
    #[serde(rename = "@application", default)]
    pub application: String,

    /// The actual license key value.
    #[serde(rename = "$text", default)]
    pub license_key: String,
}

/// Represents a collection of license keys.
/// Optionally, a license key can be associated with an application.
/// The `LicenseKeys` struct models a collection of license keys. It supports serialization
/// and deserialization of XML data for working with multiple license keys.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicenseKeys {
    /// A vector of individual license keys.
    /// Optionally a list of license keys
    /// This field holds a list of `LicenseKey` instances, each representing a distinct license key
    /// associated with a specific application. The list may be empty if no license keys are present.
    #[serde(rename = "LicenseKey", default)]
    pub license_key: Vec<LicenseKey>,
}

/// Represents an email address.
///
/// The `EMail` struct models an email address. It supports serialization and deserialization
/// of XML data for working with email addresses.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EMail {
    /// The mailto attribute of the email address.
    ///
    /// This field represents the email address in the "mailto" format as an XML attribute.
    #[serde(rename = "@mailto", default)]
    pub mailto: String,

    /// The value of the email address.
    ///
    /// This field holds the actual email address as part of the XML content.
    #[serde(rename = "$text", default)]
    pub value: String,
}

/// Represents a collection of email addresses.
///
/// The `EMailAddresses` struct models a collection of email addresses. It supports serialization
/// and deserialization of XML data for working with multiple email addresses.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EMailAddresses {
    /// A vector of individual email addresses.
    ///
    /// This field holds a list of `EMail` instances, each representing an email address. The list
    /// may be empty if no email addresses are present.
    #[serde(rename = "EMail", default)]
    pub e_mail: Vec<EMail>,
}

/// Represents an address entry.
///
/// The `Address` struct models an address entry, including details such as first name, last name,
/// street, city, and more. It supports serialization and deserialization of XML data for working
/// with address information.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Address {
    /// The first name associated with the address.
    #[serde(rename = "FirstName")]
    pub first_name: String,

    /// The last name or name associated with the address.
    #[serde(rename = "Name")]
    pub name: String,

    /// The street information of the address.
    #[serde(rename = "Street")]
    pub street: String,

    /// The ZIP code of the address.
    #[serde(rename = "ZIPCode")]
    pub zip_code: String,

    /// The city associated with the address.
    #[serde(rename = "City")]
    pub city: String,

    /// The country associated with the address.
    #[serde(rename = "Country")]
    pub country: String,

    /// The phone number associated with the address.
    #[serde(rename = "Phone")]
    pub phone: String,

    /// A collection of email addresses associated with the address.
    #[serde(rename = "EMailAddresses")]
    pub e_mail_addresses: EMailAddresses,
}

/// Represents a contact information entry.
///
/// The `Contact` struct models contact information, including addresses. It supports serialization
/// and deserialization of XML data for working with contact details.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    /// A vector of addresses associated with the contact.
    ///
    /// This field holds a list of `Address` instances, each representing a distinct address
    /// associated with the contact. The list may be empty if no addresses are present.
    #[serde(rename = "Address", default)]
    pub address: Vec<Address>,
}

/// Represents the header information of a GLDF file.
///
/// The `Header` struct models the header section of a GLDF (Global Lighting Data Format) file. It
/// includes information about the software used to create the file, author details, creation time,
/// format version, and more. It supports serialization and deserialization of XML data for working
/// with GLDF header information.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Header {
    /// The author of the GLDF file.
    #[serde(rename = "Author", default)]
    pub author: String,

    /// The manufacturer associated with the GLDF file.
    #[serde(rename = "Manufacturer", default)]
    pub manufacturer: String,

    /// The creation time code of the GLDF file.
    #[serde(rename = "GldfCreationTimeCode", alias = "CreationTimeCode", default)]
    pub creation_time_code: String,

    /// The application used to create the GLDF file.
    #[serde(rename = "CreatedWithApplication", default)]
    pub created_with_application: String,

    /// The format version of the GLDF file.
    #[serde(rename = "FormatVersion", default)]
    pub format_version: FormatVersion,

    /// The default language for the GLDF content.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[serde(rename = "DefaultLanguage", default, skip_serializing_if = "Option::is_none")]
    pub default_language: Option<String>,

    /// A collection of license keys associated with the GLDF file.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[serde(rename = "LicenseKeys", default, skip_serializing_if = "Option::is_none")]
    pub license_keys: Option<LicenseKeys>,

    /// The Relux member ID.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[serde(rename = "ReluxMemberId", default, skip_serializing_if = "Option::is_none")]
    pub relux_member_id: Option<String>,

    /// The DIALux member ID.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[serde(rename = "DIALuxMemberId", default, skip_serializing_if = "Option::is_none")]
    pub dia_lux_member_id: Option<String>,

    /// The contact information associated with the GLDF file.
    #[serde(rename = "Contact", default)]
    pub contact: Contact,
}

/// Represents a localized text string in the GLDF data structure.
///
/// This struct defines a text string with its associated language for localization purposes.
/// The text value is provided as the content of the element, and the language is specified
/// using the `@language` attribute.
///
/// # Examples
///
/// ```
/// use gldf_rs::gldf::Locale;
///
/// let locale = Locale {
///     language: "en".to_string(),
///     value: "English".to_string(),
/// };
/// ```
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Locale {
    /// Specifies the language of the text value.
    #[serde(rename = "@language", default)]
    pub language: String,

    /// The localized text content.
    #[serde(rename = "$text", default)]
    pub value: String,
}

/// A helper struct for serializing and deserializing a collection of localized text strings.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocaleFoo {
    /// a collection of locales
    #[serde(rename = "Locale", default)]
    pub locale: Vec<Locale>,
}
