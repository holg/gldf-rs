//! # GLDF Meta Information Module
//!
//! This module provides functionality to work with the `MetaInformation` structure in the GLDF format.
//! It allows for serialization and deserialization of `MetaInformation` from and to XML and JSON formats.
//!
//! ## Examples
//!
//! ```rust
//! use gldf_rs::gldf::meta_information::{MetaInformation, MetaProperty};
//!
//! // Deserialize from XML
//! let xml_data = r#"<MetaInformation><Property name="example">value</Property></MetaInformation>"#;
//! let meta_info = MetaInformation::from_xml(xml_data).unwrap();
//!
//! // Serialize to JSON
//! let json_data = meta_info.to_json().unwrap();
//! println!("{}", json_data);
//! ```
//!

use serde::{Deserialize, Serialize};
use anyhow::Context;

/// Represents the `MetaInformation` structure in the GLDF format.
///
/// This structure can be serialized and deserialized from both XML and JSON formats.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "MetaInformation")]
pub struct MetaInformation {
    /// The `Property` structure within the `MetaInformation` structure.
    #[serde(rename = "Property", default)]
    pub property: Vec<MetaProperty>,
}

/// Represents a property within the `MetaInformation` structure.
///
/// Each property has a name and associated text value.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaProperty {
    /// The name of the property.
    #[serde(rename = "@name")]
    pub name: String,
    /// The text value of the property.
    #[serde(rename = "$text")]
    pub property_text: String,
}

impl MetaInformation {
    /// Load the MetaInformation from a XML String
    #[allow(dead_code)]
    pub fn from_xml(xml_str: &str) -> anyhow::Result<MetaInformation> {
        let result: MetaInformation = quick_xml::de::from_str(xml_str)
            .map_err(anyhow::Error::msg)
            .context("Failed to parse XML string")?;
        Ok(result)
    }

    /// Load the MetaInformation from a JSON String
    #[allow(dead_code)]
    pub fn from_json(json_str: &str) -> anyhow::Result<MetaInformation> {
        let result: MetaInformation = serde_json::from_str(json_str)
            .map_err(anyhow::Error::msg)
            .context("Failed to parse JSON string")?;
        Ok(result)
    }

    /// Detach the MetaInformation from the GLDF Product
    #[allow(dead_code)]
    pub fn detach(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Represent the MetaInformation as JSON String
    #[allow(dead_code)]
    pub fn to_json(&self) -> anyhow::Result<String> {
        let json_str = serde_json::to_string(&self)?;
        Ok(json_str)
    }

    /// Represent the MetaInformation as XML String
    #[allow(dead_code)]
    pub fn to_xml(&self) -> anyhow::Result<String> {
        let xml_str = quick_xml::se::to_string(&self)?;
        Ok(xml_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_meta_informations() {
        const META_INFORMATION: &str = r#"<MetaInformation>
    <Property name="gldf_rs_file_product.xml">something</Property>
</MetaInformation>"#;
        let mut meta = MetaInformation::from_xml(META_INFORMATION).unwrap();
        let meta_json = meta.to_json();
        let meta_xml = meta.to_xml();
        let meta_from_json = MetaInformation::from_json(&meta_json.unwrap());
        assert_eq!(meta, meta_from_json.unwrap());
        let meta_from_xml = MetaInformation::from_xml(&meta_xml.unwrap());
        assert_eq!(meta, meta_from_xml.unwrap());
        let property = &meta.property[0];
        let new_property = MetaProperty {
            name: "test".to_string(),
            property_text: "test".to_string(),
        };
        let new_properties: Vec<MetaProperty> = vec![property.clone(), new_property];
        meta.property = new_properties;
        println!("{:?}", meta.to_xml());
        let meta_from_json = MetaInformation::from_json(&meta.clone().to_json().unwrap());
        assert_eq!(&meta, meta_from_json.as_ref().unwrap());
        let meta_from_xml = MetaInformation::from_xml(&meta.clone().to_xml().unwrap());
        assert_eq!(&meta, meta_from_xml.as_ref().unwrap());
        println!("{}", meta_from_json.as_ref().unwrap().to_json().unwrap());
        println!("{}", meta_from_xml.as_ref().unwrap().to_xml().unwrap());
    }
}
