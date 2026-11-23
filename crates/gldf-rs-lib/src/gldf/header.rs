#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

use serde::{Deserialize, Serialize};

fn default_major() -> i32 {
    1
}

fn default_minor() -> i32 {
    0
}

/// Represents the format version with major, minor, and pre-release attributes.
///
/// GLDF FormatVersion is an element with attributes for the version components.
/// Supports both the old simple text format and the new rc.3 attribute format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "FormatVersion")]
pub struct FormatVersion {
    /// The major version number (defaults to 1 for backward compatibility)
    #[serde(rename = "@major", default = "default_major")]
    pub major: i32,

    /// The minor version number (defaults to 0 for backward compatibility)
    #[serde(rename = "@minor", default = "default_minor")]
    pub minor: i32,

    /// The pre-release version number (optional, but typically "3" for rc.3)
    #[serde(
        rename = "@pre-release",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub pre_release: Option<i32>,

    /// Legacy text value for backward compatibility with old format
    /// This is only used during deserialization of old GLDF files
    #[serde(rename = "$text", skip_serializing, default)]
    legacy_value: Option<String>,
}

impl Default for FormatVersion {
    fn default() -> Self {
        Self {
            major: 1,
            minor: 0,
            pre_release: Some(3),
            legacy_value: None,
        }
    }
}

impl FormatVersion {
    /// Create a new FormatVersion with the given components
    pub fn new(major: i32, minor: i32, pre_release: Option<i32>) -> Self {
        Self {
            major,
            minor,
            pre_release,
            legacy_value: None,
        }
    }

    /// Create FormatVersion from a version string like "1.0.0-rc.3"
    pub fn from_string(value: &str) -> Self {
        let parts: Vec<&str> = value.split('-').collect();
        let version_parts: Vec<&str> = parts.first().unwrap_or(&"1.0.0").split('.').collect();
        let major = version_parts
            .first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let minor = version_parts
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let pre_release = if parts.len() > 1 {
            // Extract number from "rc.3" -> 3
            parts[1].split('.').next_back().and_then(|s| s.parse().ok())
        } else {
            None
        };
        Self {
            major,
            minor,
            pre_release,
            legacy_value: None,
        }
    }

    /// Convert to version string like "1.0.0-rc.3"
    /// If legacy_value is present (from old GLDF files), return that instead
    pub fn to_version_string(&self) -> String {
        if let Some(ref legacy) = self.legacy_value {
            return legacy.clone();
        }
        match self.pre_release {
            Some(pr) => format!("{}.{}.0-rc.{}", self.major, self.minor, pr),
            None => format!("{}.{}.0", self.major, self.minor),
        }
    }
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
/// Note: According to GLDF schema, Contact must have at least one Address child.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    /// A vector of addresses associated with the contact.
    ///
    /// This field holds a list of `Address` instances, each representing a distinct address
    /// associated with the contact. The list may be empty if no addresses are present.
    #[serde(rename = "Address", default)]
    pub address: Vec<Address>,
}

impl Contact {
    /// Returns true if the contact has no addresses
    pub fn is_empty(&self) -> bool {
        self.address.is_empty()
    }
}

fn is_contact_empty(contact: &Contact) -> bool {
    contact.is_empty()
}

fn default_author() -> String {
    "__empty__".to_string()
}

fn is_default_author(s: &str) -> bool {
    s.is_empty()
}

/// Represents the header information of a GLDF file.
///
/// The `Header` struct models the header section of a GLDF (Global Lighting Data Format) file. It
/// includes information about the software used to create the file, author details, creation time,
/// format version, and more. It supports serialization and deserialization of XML data for working
/// with GLDF header information.
///
/// Field order follows GLDF 1.0.0-rc.3 schema requirements.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Header {
    /// The manufacturer associated with the GLDF file (required, must come first).
    #[serde(rename = "Manufacturer", default)]
    pub manufacturer: String,

    /// The format version of the GLDF file (required, must come second).
    #[serde(rename = "FormatVersion", default)]
    pub format_version: FormatVersion,

    /// The application used to create the GLDF file.
    #[serde(rename = "CreatedWithApplication", default)]
    pub created_with_application: String,

    /// The creation time code of the GLDF file.
    #[serde(rename = "GldfCreationTimeCode", alias = "CreationTimeCode", default)]
    pub creation_time_code: String,

    /// The unique GLDF ID (optional).
    #[serde(
        rename = "UniqueGldfId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub unique_gldf_id: Option<String>,

    /// The default language for the GLDF content.
    #[serde(
        rename = "DefaultLanguage",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_language: Option<String>,

    /// A collection of license keys associated with the GLDF file.
    #[serde(
        rename = "LicenseKeys",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub license_keys: Option<LicenseKeys>,

    /// The Relux member ID.
    #[serde(
        rename = "ReluxMemberId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub relux_member_id: Option<String>,

    /// The DIALux member ID.
    #[serde(
        rename = "DIALuxMemberId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub dia_lux_member_id: Option<String>,

    /// The author of the GLDF file (comes after member IDs in rc.3).
    /// Defaults to "__empty__" if not set.
    #[serde(
        rename = "Author",
        default = "default_author",
        skip_serializing_if = "is_default_author"
    )]
    pub author: String,

    /// The contact information associated with the GLDF file.
    /// Only serialized if it contains at least one address.
    #[serde(rename = "Contact", default, skip_serializing_if = "is_contact_empty")]
    pub contact: Contact,
}

impl Header {
    /// Populate the two XSD-required header fields that the editor/wizard
    /// leaves empty by default: `GldfCreationTimeCode` (xs:dateTime — empty
    /// string is invalid) and `UniqueGldfId` (required by rc-3 XSD, but
    /// modelled as Option in this struct for backward compatibility with
    /// older fixtures that simply omit it).
    ///
    /// Call this on a clone right before serialization so the on-disk XML
    /// satisfies the rc-3 schema even when neither the wizard nor the user
    /// touched the Header page.
    pub fn ensure_xsd_required(&mut self) {
        if self.creation_time_code.trim().is_empty() {
            self.creation_time_code = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        }
        if self
            .unique_gldf_id
            .as_deref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            // UUIDv4 is what every other GLDF tool uses for this field.
            // We don't pull the `uuid` crate just for this — derive a v4-shaped
            // string from a timestamp + random bytes via getrandom (already in
            // the dep tree via other crates).
            self.unique_gldf_id = Some(make_uuid_v4());
        }
    }
}

/// Public re-export so sibling modules (e.g. `product_definitions`) can
/// generate UUIDs with the same fallback semantics for their own
/// `ensure_xsd_required` hooks.
#[doc(hidden)]
pub fn make_uuid_v4_public() -> String {
    make_uuid_v4()
}

/// Best-effort "now" in nanoseconds, used to seed the UUID PRNG.
///
/// On native: `std::time::SystemTime::now()`. On `wasm32-unknown-unknown`
/// that panics because std has no clock; instead we use the JS `Date.now()`
/// epoch milliseconds via web_sys. Failing both, returns 0 — the PRNG
/// still de-duplicates within a process via the atomic counter, just
/// not across processes.
// The `return` is the tail only on native; on wasm32 the other block follows.
#[allow(clippy::needless_return)]
fn time_seed_ns() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        return SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
    }
    #[cfg(target_arch = "wasm32")]
    {
        // `js_sys::Date::now()` returns epoch milliseconds as f64.
        // The web_sys::window() guard ensures we don't try this in a
        // pre-DOM context (web workers without `self`, off-thread tests,
        // etc.) — outside a browser window we'd panic on the JS shim.
        if web_sys::window().is_some() {
            let ms = js_sys::Date::now();
            return (ms * 1_000_000.0) as u64;
        }
        0
    }
}

/// Build a UUIDv4-shaped string without pulling the `uuid` crate.
/// Format: 8-4-4-4-12 lowercase hex with the standard v4/variant bits.
///
/// The XSD only requires `UniqueGldfId` to be an `xs:string`; downstream
/// tools just expect *something* uniquely identifying. Cryptographic
/// strength isn't needed — a nanosecond timestamp + SplitMix64 PRNG
/// keyed by a per-process atomic counter gives plenty of entropy in
/// practice without adding a dependency.
fn make_uuid_v4() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seed_now = time_seed_ns();
    let mut state = seed_now
        ^ COUNTER
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut bytes = [0u8; 16];
    for chunk in bytes.chunks_mut(8) {
        // SplitMix64 step
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        chunk.copy_from_slice(&z.to_le_bytes()[..chunk.len()]);
    }
    // Set version (4) and variant (10xx) bits per RFC 4122.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    )
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
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct Locale {
    /// Specifies the language of the text value.
    #[serde(rename = "@language", default)]
    pub language: String,

    /// The localized text content.
    #[serde(rename = "$text", default)]
    pub value: String,
}

// Custom Serialize because the derived impl emits self-closing
// `<Locale language="…"/>` when `value` is empty, and quick-xml then
// fails to RE-deserialize that ("missing field `$text`"). We emit a
// non-self-closing form `<Locale language="…"></Locale>` by writing
// a single space then trimming on the deserialize side via the
// `String::trim_end` happening on every consumer. The space is
// invisible in the rendered text and round-trips through `quick-xml`
// without the placeholder-field error.
impl serde::Serialize for Locale {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Locale", 2)?;
        // XSD requires `@language` to be a valid xs:language (BCP-47)
        // tag — empty string fails validation. Default to "en" when the
        // editor / wizard left it blank so exports remain XSD-clean.
        // Users who want a different default can set it explicitly via
        // the Header's `DefaultLanguage` field; this is only a guard
        // against fully-empty `language=""` on disk.
        let lang = if self.language.trim().is_empty() {
            "en"
        } else {
            self.language.as_str()
        };
        s.serialize_field("@language", lang)?;
        let text = if self.value.is_empty() {
            " "
        } else {
            self.value.as_str()
        };
        s.serialize_field("$text", text)?;
        s.end()
    }
}

/// A helper struct for serializing and deserializing a collection of localized text strings.
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct LocaleFoo {
    /// a collection of locales
    #[serde(rename = "Locale", default)]
    pub locale: Vec<Locale>,
}

// Custom Serialize emits at least one Locale child even when the
// `locale` Vec is empty. quick-xml's struct serializer self-closes a
// struct with no children (`<Name/>`), and its deserializer then
// chokes on that with "missing field `$text`" when re-parsing — so
// empty LocaleFoo round-trips were broken. The synthetic child has an
// `en` language and a single space character; when the file is
// reloaded the deserializer rehydrates the Locale and the space looks
// like a blank name to every downstream consumer.
impl serde::Serialize for LocaleFoo {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("LocaleFoo", 1)?;
        if self.locale.is_empty() {
            // Synthetic placeholder. The matching Locale Serialize impl
            // emits a single space for an empty value, so the wire
            // form ends up as `<Name><Locale language=""> </Locale></Name>`.
            // Round-trips through quick-xml and looks like a blank
            // name to downstream consumers (they trim).
            let placeholder = [Locale {
                language: String::new(),
                value: String::new(),
            }];
            s.serialize_field("Locale", &placeholder[..])?;
        } else {
            s.serialize_field("Locale", &self.locale)?;
        }
        s.end()
    }
}
