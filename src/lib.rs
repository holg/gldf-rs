extern crate yaserde_derive;
pub mod gldf;
pub use gldf::*;
mod tests;
use std::fs::File as StdFile;
use std::path::PathBuf;
use std::io::Read;
use std::error::Error as StdError;
use std::path::Path;
use yaserde::de::from_str;
use serde_json::from_str as serde_from_str;
use zip::ZipArchive;

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


impl GldfProduct {
    pub fn load_gldf_file_str(self: &Self, path: String) -> Result<String, Box<dyn StdError>> {
        let zipfile = StdFile::open(Path::new(&self.path))?;
        let mut zip = ZipArchive::new(zipfile)?;
        let mut some_str = String::new();
        let mut some_file = zip.by_name(&path)?;
        some_file.read_to_string(&mut some_str)?;
        Ok(some_str)
    }
    pub fn load_gldf_file(self: &Self, path: String) -> Result<Vec<u8>, Box<dyn StdError>> {
        let zipfile = StdFile::open(Path::new(&self.path))?;
        let mut zip = ZipArchive::new(zipfile)?;
        let mut file_buf = Vec::new();
        let mut some_file = zip.by_name(&path)?;
        some_file.read_to_end(&mut file_buf)?;
        Ok(file_buf)
    }
    pub fn get_xml_str_from_gldf(path: PathBuf) -> Result<String, Box<dyn StdError>> {
        let zipfile = StdFile::open(path)?;
        let mut zip = ZipArchive::new(zipfile)?;
        let mut xmlfile = zip.by_name("product.xml")?;
        let mut xml_str = String::new();
        xmlfile.read_to_string(&mut xml_str)?;
        Ok(xml_str)
    }
    pub fn from_xml(xml_str: &str) -> Result<GldfProduct, Box<dyn StdError>> {
        let loaded = from_str(xml_str).unwrap();
        Ok(loaded)
    }
    pub fn load_gldf(path: &str) -> Result<GldfProduct, Box<dyn StdError>> {
        let path_buf = Path::new(path).to_path_buf();
        let mut loaded: GldfProduct = GldfProduct::from_xml(&GldfProduct::get_xml_str_from_gldf(path_buf).unwrap()).unwrap();
        loaded.path = path.to_string();
        Ok(loaded)
    }
    pub fn load_gldf_from_buf_all(gldf_buf: Vec<u8>) -> Result<FileBufGldf, Box<dyn StdError>> {
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
                file.read_to_end(&mut buf);
                let buf_file = BufFile {
                    name: Some(file.name().to_string()),
                    content: Some(buf),
                    file_id: None,
                    path: Some(file.name().to_string()),
                };
                file_bufs.push(buf_file);
            }
        }
        let mut file_buf = FileBufGldf{files: file_bufs, gldf: loaded};

        Ok(file_buf)
    }
    pub fn load_gldf_from_buf(file_buf: Vec<u8>) -> Result<GldfProduct, Box<dyn StdError>> {
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
    pub fn to_json(self: &Self) -> Result<String, Box<dyn StdError>> {
        let json_str = serde_json::to_string(&self).unwrap();
        Ok(json_str)
    }
    pub fn to_pretty_json(self: &Self) -> Result<String, Box<dyn StdError>> {
        let json_str = serde_json::to_string_pretty(&self).unwrap();
        Ok(json_str)
    }
    pub fn from_json(json_str: &str) -> Result<GldfProduct, Box<dyn StdError>> {
        let j_loaded: GldfProduct = serde_from_str(&json_str).unwrap();
        Ok(j_loaded)
    }
    pub fn from_json_file(path: PathBuf) -> Result<GldfProduct, Box<dyn StdError>> {
        let mut json_file = StdFile::open(path)?;
        let mut json_str = String::new();
        json_file.read_to_string(& mut json_str);
        Ok(GldfProduct::from_json(&json_str).unwrap())
    }
    pub fn to_xml(self: &Self) -> Result<String, Box<dyn StdError>> {
        let yaserde_cfg = yaserde::ser::Config {
            perform_indent: true,
            ..Default::default()
        };
        let x_serialized = yaserde::ser::to_string_with_config(self, &yaserde_cfg).unwrap();
        Ok(x_serialized)
    }
    pub fn get_phot_files(self: &Self) -> Result<Vec<&File>, Box<dyn StdError>> {
        let mut result: Vec<&File> = Vec::new();
        for f in self.general_definitions.files.file.iter() {
            let content_type = &f.content_type;
            if content_type.starts_with("ldc") {
                result.push(f)
            }
        }
        Ok(result.to_owned())
    }
    pub fn get_image_def_files(self: &Self) -> Result<Vec<&File>, Box<dyn StdError>> {
        let mut result: Vec<&File> = Vec::new();
        for f in self.general_definitions.files.file.iter() {
            let content_type = &f.content_type;
            if content_type.starts_with("image") {
                result.push(f)
            }
        }
        Ok(result.to_owned())
    }

    pub fn get_image_zip_files(self: &Self) -> Result<Vec<&File>, Box<dyn StdError>> {
        let mut result: Vec<&File> = Vec::new();
        for f in self.general_definitions.files.file.iter() {
            let content_type = &f.content_type;
            if content_type.starts_with("image") {
                result.push(f)
            }
        }
        Ok(result.to_owned())
    }

    pub fn get_ldc_by_id(self: &Self, file_id: String) -> Result<String, Box<dyn StdError>> {
        let mut result: String = "".to_owned();
        for f in self.general_definitions.files.file.iter() {
            if f.id == file_id{
                let mut ldc_path = "ldc/".to_owned();
                let file_name = f.file_name.to_owned();
                ldc_path.push_str(&file_name);
                result.push_str(&self.load_gldf_file_str(ldc_path).unwrap());
            }
        }
        Ok(result)
    }
}