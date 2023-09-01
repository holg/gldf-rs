#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]use serde::{Serialize};
use serde::Deserialize;
use yaserde_derive::YaDeserialize;
use yaserde_derive::YaSerialize;

/// Represents a license key.
/// Optionally, a license key can be associated with an application.
/// LicenseKey is a Rust struct that models a license key. It provides serialization and
/// deserialization methods for working with license key XML data.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
#[yaserde(rename = "LicenseKey")]
pub struct LicenseKey {
    /// The application associated with the license key.
    #[yaserde(attribute)]
    #[yaserde(rename = "application")]
    #[serde(rename = "@application")]
    pub application: String,

    /// The actual license key value.
    #[yaserde(text)]
    #[serde(rename = "$")]
    pub license_key: String,
}

/// Represents a collection of license keys.
/// Optionally, a license key can be associated with an application.
/// The `LicenseKeys` struct models a collection of license keys. It supports serialization
/// and deserialization of XML data for working with multiple license keys.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LicenseKeys {
    /// A vector of individual license keys.
    /// Optionally a list of license keys
    /// This field holds a list of `LicenseKey` instances, each representing a distinct license key
    /// associated with a specific application. The list may be empty if no license keys are present.
    #[yaserde(child)]
    #[yaserde(rename = "LicenseKey")]
    #[serde(rename = "LicenseKey")]
    pub license_key: Vec<LicenseKey>,
}

/// Represents an email address.
///
/// The `EMail` struct models an email address. It supports serialization and deserialization
/// of XML data for working with email addresses.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EMail {
    /// The mailto attribute of the email address.
    ///
    /// This field represents the email address in the "mailto" format as an XML attribute.
    #[yaserde(attribute)]
    #[yaserde(rename = "mailto")]
    #[serde(rename = "@mailto")]
    pub mailto: String,

    /// The value of the email address.
    ///
    /// This field holds the actual email address as part of the XML content.
    #[yaserde(text)]
    #[serde(rename = "$")]
    pub value: String,
}


/// Represents a collection of email addresses.
///
/// The `EMailAddresses` struct models a collection of email addresses. It supports serialization
/// and deserialization of XML data for working with multiple email addresses.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EMailAddresses {
    /// A vector of individual email addresses.
    ///
    /// This field holds a list of `EMail` instances, each representing an email address. The list
    /// may be empty if no email addresses are present.
    #[yaserde(child)]
    #[yaserde(rename = "EMail")]
    #[serde(rename = "EMail")]
    pub e_mail: Vec<EMail>,
}


/// Represents an address entry.
///
/// The `Address` struct models an address entry, including details such as first name, last name,
/// street, city, and more. It supports serialization and deserialization of XML data for working
/// with address information.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Address {
    /// The first name associated with the address.
    #[yaserde(rename = "FirstName")]
    #[serde(rename = "FirstName")]
    pub first_name: String,

    /// The last name or name associated with the address.
    #[yaserde(rename = "Name")]
    #[serde(rename = "Name")]
    pub name: String,

    /// The street information of the address.
    #[yaserde(rename = "Street")]
    #[serde(rename = "Street")]
    pub street: String,

    /// The ZIP code of the address.
    #[yaserde(rename = "ZIPCode")]
    #[serde(rename = "ZIPCode")]
    pub zip_code: String,

    /// The city associated with the address.
    #[yaserde(rename = "City")]
    #[serde(rename = "City")]
    pub city: String,

    /// The country associated with the address.
    #[yaserde(rename = "Country")]
    #[serde(rename = "Country")]
    pub country: String,

    /// The phone number associated with the address.
    #[yaserde(rename = "Phone")]
    #[serde(rename = "Phone")]
    pub phone: String,

    /// A collection of email addresses associated with the address.
    #[yaserde(child)]
    #[yaserde(rename = "EMailAddresses")]
    #[serde(rename = "EMailAddresses")]
    pub e_mail_addresses: EMailAddresses,
}


/// Represents a contact information entry.
///
/// The `Contact` struct models contact information, including addresses. It supports serialization
/// and deserialization of XML data for working with contact details.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Contact {
    /// A vector of addresses associated with the contact.
    ///
    /// This field holds a list of `Address` instances, each representing a distinct address
    /// associated with the contact. The list may be empty if no addresses are present.
    #[yaserde(child)]
    #[yaserde(rename = "Address")]
    #[serde(rename = "Address")]
    pub address: Vec<Address>,
}


/// Represents the header information of a GLDF file.
///
/// The `Header` struct models the header section of a GLDF (Global Lighting Data Format) file. It
/// includes information about the software used to create the file, author details, creation time,
/// format version, and more. It supports serialization and deserialization of XML data for working
/// with GLDF header information.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Header {
    /// The author of the GLDF file.
    #[yaserde(rename = "Author")]
    #[serde(rename = "Author")]
    pub author: String,

    /// The manufacturer associated with the GLDF file.
    #[yaserde(rename = "Manufacturer")]
    #[serde(rename = "Manufacturer")]
    pub manufacturer: String,

    /// The creation time code of the GLDF file.
    #[yaserde(rename = "CreationTimeCode")]
    #[serde(rename = "CreationTimeCode")]
    pub creation_time_code: String,

    /// The application used to create the GLDF file.
    #[yaserde(rename = "CreatedWithApplication")]
    #[serde(rename = "CreatedWithApplication")]
    pub created_with_application: String,

    /// The format version of the GLDF file.
    #[yaserde(rename = "FormatVersion")]
    #[serde(rename = "FormatVersion")]
    pub format_version: String,

    /// The default language for the GLDF content.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[yaserde(rename = "DefaultLanguage")]
    #[serde(rename = "DefaultLanguage")]
    pub default_language: Option<String>,

    /// A collection of license keys associated with the GLDF file.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[yaserde(rename = "LicenseKeys")]
    #[serde(rename = "LicenseKeys")]
    #[yaserde(child)]
    pub license_keys: Option<LicenseKeys>,

    /// The Relux member ID.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[yaserde(rename = "ReluxMemberId")]
    #[serde(rename = "ReluxMemberId")]
    pub relux_member_id: Option<String>,

    /// The DIALux member ID.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[yaserde(rename = "DIALuxMemberId")]
    #[serde(rename = "DIALuxMemberId")]
    pub dia_lux_member_id: Option<String>,

    /// The contact information associated with the GLDF file.
    #[yaserde(child)]
    #[yaserde(rename = "Contact")]
    #[serde(rename = "Contact")]
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
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Locale {
    /// Specifies the language of the text value.
    #[serde(rename = "@language")]
    #[yaserde(attribute)]
    pub language: String,

    /// The localized text content.
    #[yaserde(text)]
    #[serde(rename = "$")]
    pub value: String,
}

/// A helper struct for serializing and deserializing a collection of localized text strings.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LocaleFoo {
    #[yaserde(rename = "Locale")]
    #[serde(rename = "Locale")]
    /// a collection of locales
    pub locale: Vec<Locale>,
}