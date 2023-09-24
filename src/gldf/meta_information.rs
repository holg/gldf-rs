//! # GLDF Meta Information Module
//!
//! This module provides functionality to work with the `MetaInformation` structure in the GLDF format.
//! It allows for serialization and deserialization of `MetaInformation` from and to XML and JSON formats.
//!
//! ## Examples
//!
//! ```rust
//! use gldf_rs::gldf::meta_information::{MetaInformation, Property};
//!
//! // Deserialize from XML
//! let xml_data = r#"<MetaInformation><Property name="example">value</Property></MetaInformation>"#;
//! let meta_info = MetaInformation::from_xml(&xml_data.to_string()).unwrap();
//!
//! // Serialize to JSON
//! let json_data = meta_info.to_json().unwrap();
//! println!("{}", json_data);
//! ```
//!

use serde::{Serialize, Deserialize};
use serde_json::from_str as serde_from_str;
use yaserde_derive::{YaDeserialize, YaSerialize};
use yaserde::de::{from_str};
use anyhow::{Context};

/// Represents the `MetaInformation` structure in the GLDF format.
///
/// This structure can be serialized and deserialized from both XML and JSON formats.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct MetaInformation {
    /// The `Property` structure within the `MetaInformation` structure.
    #[yaserde(rename = "Property")]
    pub property: Vec<Property>
}

/// Represents a property within the `MetaInformation` structure.
///
/// Each property has a name and associated text value.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Property {
    /// The name of the property.
    #[yaserde(attribute)]
    #[yaserde(rename = "name")]
    #[serde(rename = "@name")]
    pub name: String,
    /// The text value of the property.
    #[yaserde(text)]
    #[serde(rename = "$")]
    pub property_text: String,
}
impl MetaInformation {
    /// Load the MetaInformation from a XML String
    #[allow(dead_code)] // it will be used in gldf-sign
    pub fn from_xml(xml_str: &String) -> anyhow::Result<MetaInformation> {
        let result = from_str(&xml_str);
        let loaded = result.map_err(anyhow::Error::msg).context("Failed to parse XML string")?;

        Ok(loaded)
    }
    /// Load the MetaInformation from a JSON String
    #[allow(dead_code)] // it will be used in gldf-sign
    pub fn from_json(json_str: &String) -> anyhow::Result<MetaInformation> {
        let result = serde_from_str(&json_str);
        let loaded = result.map_err(anyhow::Error::msg).context("Failed to parse XML string")?;

        Ok(loaded)
    }
    /// Detach the MetaInformation from the GLDF Product
    #[allow(dead_code)] // it will be used in gldf-sign
    pub fn detach(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
    /// represent the MetaInformation as JSON String
    #[allow(dead_code)] // it will be used in gldf-sign
    pub fn to_json(self: &Self) -> anyhow::Result<String> {
        let json_str = serde_json::to_string(&self)?;
        Ok(json_str)
    }
    /// represent the MetaInformation as XML String
    #[allow(dead_code)] // it will be used in gldf-sign
    pub fn to_xml(self: &Self) -> anyhow::Result<String> {
        let yaserde_cfg = yaserde::ser::Config {
            perform_indent: true,
            ..Default::default()
        };
        let x_serialized = yaserde::ser::to_string_with_config(self, &yaserde_cfg).unwrap();
        Ok(x_serialized)
    }
}

#[cfg(test)]
#[test]
pub fn test_meta_informations() {
    const META_INFORMATION: &str = r#"<MetaInformation>
    <Property name="gldf_rs_file_product.xml">something</Property>
</MetaInformation>"#;
    let mut meta = MetaInformation::from_xml(&META_INFORMATION.to_string()).unwrap();
    let meta_json = meta.to_json();
    let meta_xml = meta.to_xml();
    let meta_from_json = MetaInformation::from_json(&meta_json.unwrap());
    assert_eq!(meta, meta_from_json.unwrap());
    let meta_from_xml = MetaInformation::from_xml(&meta_xml.unwrap());
    assert_eq!(meta, meta_from_xml.unwrap());
    let property = &meta.property[0];
    let new_property = Property {
        name: "test".to_string(),
        property_text: "test".to_string(),
    };
    let new_properties:Vec<Property> = vec![property.clone(), new_property];
    meta.property = new_properties;
    println!("{:?}", meta.to_xml());
    let meta_from_json = MetaInformation::from_json(&meta.clone().to_json().unwrap());
    assert_eq!(&meta, meta_from_json.as_ref().unwrap());
    let meta_from_xml = MetaInformation::from_xml(&meta.clone().to_xml().unwrap());
    assert_eq!(&meta, meta_from_xml.as_ref().unwrap());
    println!("{}", meta_from_json.as_ref().unwrap().to_json().unwrap());
    println!("{}", meta_from_xml.as_ref().unwrap().to_xml().unwrap());
}