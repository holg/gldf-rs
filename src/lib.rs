//! # gldf_rs
//! GLDF (Global Lighting Data Format) Library
//!
//! The GLDF crate provides a set of structures and tools for working with the Global Lighting Data Format (GLDF),
//! a standardized format for describing lighting products, their characteristics, and technical details.
//!
//! GLDF is used in the lighting industry to exchange product information between manufacturers, designers,
//! and other stakeholders, ensuring consistent representation and interoperability across various software tools.
//!
//! This crate offers utilities for serializing and deserializing GLDF data, enabling you to read and write GLDF files
//! while adhering to the ISO 7127 standard. It also provides helper macros for working with GLDF-specific attributes.
//!
//! For more information about GLDF and its specifications, <https::gldf.io> and refer to the ISO 7127 standard.
//!
//! # Features
//!
//! - Serialize and deserialize GLDF files in compliance with ISO 7127 standard.
//! - From XML into JSON and vice versa.
//! - Define GLDF-specific attributes using custom procedural macros.
//! - Easily work with GLDF data structures and their components.
//!
//! For more usage examples and detailed documentation, please refer to the documentation of individual modules and structs.
//! Most functions are implemented as methods on the struct GldfProduct, which shall represent the Root of the XML structure.
//! **For more information see : gldf_rs::gldf::GldfProduct**
//!
//! [`GldfProduct`]
//! # Example
//! ```rust
//! use gldf_rs::gldf::GldfProduct;
//! let loaded: GldfProduct = GldfProduct::load_gldf("./tests/data/test.gldf").unwrap();
//! println!("{:?}", loaded);
//! // Display pretty printed XML
//! let x_serialized = loaded.to_xml().unwrap();
//! println!("{}", x_serialized);
//! let json_str = loaded.to_json().unwrap();
//! println!("{}", json_str);
//! let j_loaded: GldfProduct = GldfProduct::from_json(&json_str).unwrap();
//! let x_reserialized =  j_loaded.to_xml().unwrap();
//! println!("{}", x_reserialized);
//! assert_eq!(x_serialized, x_reserialized);
//! ```
//!
//!
//! For more information about GLDF and its specifications, refer to the ISO 7127 standard.
//!
//! # License
//!
//! This project is licensed under the terms of the MIT license.
/// the gldf module (src/gldf/mod.rs)
pub mod gldf;
pub use gldf::*;
extern crate yaserde_derive;
#[cfg(test)]
mod tests;
use std::fs::File as StdFile;
use std::path::PathBuf;
use std::io::Read;
//use std::error::Error as StdError;
use std::path::Path;
use yaserde::de::from_str;
use serde_json::from_str as serde_from_str;
use zip::ZipArchive;
use anyhow::{Context, Result};
use regex::Regex;

impl GldfProduct {
    pub fn detach(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct BufFile{
    pub name: Option<String>,
    pub content: Option<Vec<u8>>,
    pub file_id: Option<String>,
    pub path: Option<String>,
}
pub struct FileBufGldf{
    pub files: Vec<BufFile>,
    pub gldf: GldfProduct,
}
//
// impl TryFrom<T as TryFrom<_>>::Error: Into<anyhow::Error> for GldfProduct {
//     type Error = &'static str;
//
//     fn try_from(value: String) -> Result<Self, Self::Error> {
//         if value.find("<?xml version=\"1.0\" encoding=\"UTF-8\"?>").is_none() {
//             Err("GreaterThanZero only accepts values greater than zero!")
//         } else {
//             Ok(GldfProduct::from_xml(&value).unwrap())
//         }
//     }
// }

/// Implementations for the per se informational GldfProduct struct
impl GldfProduct {
    /// Loads a GLDF file from a given path as String and return the XML String of the product.xml file
    /// //! # Example
    /// ```rust
    /// use gldf_rs::gldf::GldfProduct;
    /// let loaded: GldfProduct = GldfProduct::load_gldf("./tests/data/test.gldf").unwrap();
    /// println!("{:?}", loaded);
    /// // Display pretty printed XML
    /// let x_serialized = loaded.to_xml().unwrap();
    /// println!("{}", x_serialized);
    /// let json_str = loaded.to_json().unwrap();
    /// println!("{}", json_str);
    /// let j_loaded: GldfProduct = GldfProduct::from_json(&json_str).unwrap();
    /// let x_reserialized =  j_loaded.to_xml().unwrap();
    /// println!("{}", x_reserialized);
    /// assert_eq!(x_serialized, x_reserialized);
    /// ```
    pub fn load_gldf_file_str(self: &Self, path: String) -> anyhow::Result<String> {
        let zipfile = StdFile::open(Path::new(&self.path))?;
        let mut zip = ZipArchive::new(zipfile)?;
        let mut some_str = String::new();
        let mut some_file = zip.by_name(&path)?;
        some_file.read_to_string(&mut some_str)?;
        Ok(some_str)
    }
    /// Loads a GLDF file from a given path as Vec<u8> and return the Vec<u8> of the product.xml file
    /// last but not least for WASM usage.
    pub fn load_gldf_file(self: &Self, path: String) -> anyhow::Result<Vec<u8>> {
        let zipfile = StdFile::open(Path::new(&self.path))?;
        let mut zip = ZipArchive::new(zipfile)?;
        let mut file_buf = Vec::new();
        let mut some_file = zip.by_name(&path)?;
        some_file.read_to_end(&mut file_buf)?;
        Ok(file_buf)
    }
    /// a helper function to used by the load_gldf function
    /// takes a PathBuf and returns the XML String of the product.xml file
    pub fn get_xml_str_from_gldf(path: PathBuf) -> anyhow::Result<String> {
        let zipfile = StdFile::open(path)?;
        let mut zip = ZipArchive::new(zipfile)?;
        let mut xmlfile = zip.by_name("product.xml")?;
        let mut xml_str = String::new();
        xmlfile.read_to_string(&mut xml_str)?;
        Ok(xml_str)
    }
    /// a helper function to remove the UTF8 Bom, if present from a given String
    /// takes a String and returns a String
    /// needed for some GLDF files, which have BOM in the XML file
    pub fn remove_bom(s: &str) -> String {
        if s.starts_with("\u{FEFF}") {
            (&s[3..]).to_string()
        } else {
            s.to_string()
        }
    }
    /// a helper function to saintize the XML String
    /// takes a String and returns a String
    /// GldfPRoduct does not really care about the XSD version, so we remove it
    /// and add our own later
    pub fn sanitize_xml_str(xml_str: &str) -> String {
        let cleaned_str = Self::remove_bom(xml_str);
        let re = Regex::new(r"<Root .*?>").unwrap();
        // well we are lazy for now and simple replace the root element with a generic one
        re.replace_all(&cleaned_str, "<Root>").to_string()
    }
    /// a helper function to load a XML String and return the GldfProduct struct
    pub fn from_xml(xml_str: &String) -> anyhow::Result<GldfProduct> {
        let my_xml_str = Self::sanitize_xml_str(&xml_str);
        let result = from_str(&my_xml_str);
        let loaded = result.map_err(anyhow::Error::msg).context("Failed to parse XML string")?;

        Ok(loaded)
    }
    /// Argument the &str path to the GLDF file and return the GldfProduct struct
    pub fn load_gldf(path: &str) -> anyhow::Result<GldfProduct> {
        let path_buf = Path::new(path).to_path_buf();
        let xml_str = GldfProduct::get_xml_str_from_gldf(path_buf).map_err(anyhow::Error::msg).context("Failed to parse XML string")?;
        let mut loaded: GldfProduct = GldfProduct::from_xml(&xml_str)?;
        loaded.path = path.to_string();
        Ok(loaded)
    }
    /// a helper for the WASM, which has the GLDF file as Vec<u8> and returns all the files as Vec<BufFile>
    /// which can be later rendered into HTML, e.g. for some GLDF Viewer
    pub fn load_gldf_from_buf_all(gldf_buf: Vec<u8>) -> anyhow::Result<FileBufGldf> {
        let zip_buf = std::io::Cursor::new(gldf_buf);
        let mut zip = ZipArchive::new(zip_buf)?;
        let mut file_bufs:Vec<BufFile>= Vec::new();
        let mut xmlfile = zip.by_name("product.xml")?;
        let mut xml_str = String::new();
        xmlfile.read_to_string(&mut xml_str)?;
        let loaded: GldfProduct = GldfProduct::from_xml(&xml_str).unwrap();
        drop(xmlfile);

        for i in 0..zip.len() {
            let mut file = zip.by_index(i).unwrap();
            println!("Filename: {}", file.name());
            // println!("{}", file.bytes().next().unwrap()?);
            if file.is_file() {
                let mut buf: Vec<u8> = Vec::new();
                file.read_to_end(&mut buf)?;
                let buf_file = BufFile {
                    name: Some(file.name().to_string()),
                    content: Some(buf),
                    file_id: None,
                    path: Some(file.name().to_string()),
                };
                file_bufs.push(buf_file);
            }
        }
        let file_buf = FileBufGldf{files: file_bufs, gldf: loaded};

        Ok(file_buf)
    }
    /// a helper function Argument is the Vec<u8> of the GLDF file and returns the GldfProduct struct
    /// WASM usage e.g.
    pub fn load_gldf_from_buf(file_buf: Vec<u8>) -> anyhow::Result<GldfProduct> {
        let zip_buf = std::io::Cursor::new(file_buf);
        let mut zip = ZipArchive::new(zip_buf)?;
        for i in 0..zip.len()
        {
            let  file = zip.by_index(i).unwrap();
            println!("Filename: {}", file.name());
            let first_byte = file.bytes().next().unwrap()?;
            println!("{}", first_byte);
        }
        let mut xmlfile = zip.by_name("product.xml")?;
        let mut xml_str = String::new();
        xmlfile.read_to_string(&mut xml_str)?;
        let loaded: GldfProduct = GldfProduct::from_xml(&xml_str).unwrap();
        Ok(loaded)
    }
    /// represent the GldfProduct as JSON String
    pub fn to_json(self: &Self) -> anyhow::Result<String> {
        let json_str = serde_json::to_string(&self)?;
        Ok(json_str)
    }
    /// represent the GldfProduct as pretty pretty JSON String
    pub fn to_pretty_json(self: &Self) -> anyhow::Result<String> {
        let json_str = serde_json::to_string_pretty(&self).unwrap();
        Ok(json_str)
    }
    /// loads a given JSON &str and returns the GldfProduct struct
    pub fn from_json(json_str: &str) -> anyhow::Result<GldfProduct> {
        let j_loaded: GldfProduct = serde_from_str(&json_str)?;
        Ok(j_loaded)
    }
    /// loads a given JSON file from a PathBuf and returns the GldfProduct struct
    /// last but not least for WASM usage.
    pub fn from_json_file(path: PathBuf) -> anyhow::Result<GldfProduct> {
        let mut json_file = StdFile::open(path)?;
        let mut json_str = String::new();
        json_file.read_to_string(& mut json_str)?;
        Ok(GldfProduct::from_json(&json_str).unwrap())
    }
    /// represent the GldfProduct as XML String
    pub fn to_xml(self: &Self) -> anyhow::Result<String> {
        let yaserde_cfg = yaserde::ser::Config {
            perform_indent: true,
            ..Default::default()
        };
        let x_serialized = yaserde::ser::to_string_with_config(self, &yaserde_cfg).unwrap();
        Ok(x_serialized)
    }
    /// returns the photometric files as Vec<&File>
    pub fn get_phot_files(self: &Self) -> anyhow::Result<Vec<&File>> {
        let mut result: Vec<&File> = Vec::new();
        for f in self.general_definitions.files.file.iter() {
            let content_type = &f.content_type;
            if content_type.starts_with("ldc") {
                result.push(f)
            }
        }
        Ok(result.to_owned())
    }
    /// returns the iamge files as Vec<&File>
    pub fn get_image_def_files(self: &Self) -> Result<Vec<&File>> {
        let mut result: Vec<&File> = Vec::new();
        for f in self.general_definitions.files.file.iter() {
            let content_type = &f.content_type;
            if content_type.starts_with("image") {
                result.push(f)
            }
        }
        Ok(result.to_owned())
    }

    /// returns the image files as Vec<&File>
    pub fn get_image_zip_files(self: &Self) -> anyhow::Result<Vec<&File>> {
        let mut result: Vec<&File> = Vec::new();
        for f in self.general_definitions.files.file.iter() {
            let content_type = &f.content_type;
            if content_type.starts_with("image") {
                result.push(f)
            }
        }
        Ok(result.to_owned())
    }
    /// from the given file_id of the ldc file reference, return the ldc file as String
    /// it could as well be the type_attr "url", which wil be fetched from the web first
    /// overriden for the WASM portage, so not used for WASM portage
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_ldc_by_id(self: &Self, file_id: String) -> anyhow::Result<String> {
        let mut result: String = "".to_owned();
        for f in self.general_definitions.files.file.iter() {
            if f.id == file_id{
                let mut ldc_path = "ldc/".to_owned();
                let file_name = f.file_name.to_owned();
                if f.type_attr == "url" {
                    result.push_str(fetch_text_from_url(&file_name)?.as_str());
                } else {
                    ldc_path.push_str(&file_name);
                    result.push_str(&self.load_gldf_file_str(ldc_path).unwrap());
                }
            }
        }
        Ok(result)
    }

    /// gets all the file definitions as Vec<File>
    pub fn get_all_file_definitions(self: &Self) -> anyhow::Result<Vec<File>> {
        let mut result:Vec<File> = Vec::new();
        for f in self.general_definitions.files.file.iter() {
            result.push(f.to_owned());
        }
        Ok(result)
    }
    /// gets all the file definitions, which are of content_type url as Vec<File>
    pub fn get_url_file_definitions(self: &Self) -> anyhow::Result<Vec<File>> {
        let mut result:Vec<File> = Vec::new();
        for f in self.general_definitions.files.file.iter() {
            if f.content_type == "url" {
                result.push(f.to_owned());
            }
        }
        Ok(result)
    }
}
/// helper function to get the content of the url as File from the given url
/// not used for the wasm portage, which overrides this function
#[cfg(not(target_arch = "wasm32"))]
pub  fn fetch_text_from_url(url: &str) -> Result<String, reqwest_wasm::Error> {
    let response = reqwest_wasm::blocking::get(url)?;
    response.text()
}
