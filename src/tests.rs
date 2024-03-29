#![allow(unused_variables)]
use gldf::{GldfProduct};
use anyhow::{Result};
use serde::de::StdError;
use crate::{gldf, StdFile};


// const GLDF_FILE_NAME: &str =  "./tests/data/R2MCOBSIK-30.gldf";
const GLDF_FILE_NAME: &str =  "./tests/data/test.gldf";
#[test]
fn test_default() {
    let gldf = GldfProduct::default();
    println!("{:?}", gldf);
    println!("{:?}", gldf.to_json());
}

#[test]
fn parsing_gldf_container() -> Result<(), Box<dyn StdError>> {
    use serde_json::from_str as serde_from_str;
    let loaded: GldfProduct = GldfProduct::load_gldf(GLDF_FILE_NAME).unwrap();
    let general_files = loaded.get_all_file_definitions().unwrap();
    println!("{:?}", loaded);
    // Display pretty printed XML
    let yaserde_cfg = yaserde::ser::Config {
        perform_indent: true,
        ..Default::default()
    };
    let gldf_to_json = loaded.to_json()?;
    let gldf_to_xml = loaded.to_xml()?;
    let json_to_xml = GldfProduct::from_json(&gldf_to_json)?.to_xml()?;
    assert_eq!(gldf_to_xml, json_to_xml);
    let result = GldfProduct::from_xml(&gldf_to_xml)?;
    let xml_to_json = result.to_json().unwrap();
    let x_serialized = yaserde::ser::to_string_with_config(&loaded, &yaserde_cfg).unwrap();
    println!("{}", x_serialized);
    let json_str = serde_json::to_string(&loaded).unwrap();
    println!("{}", json_str);
    let j_loaded: GldfProduct = serde_from_str(&json_str).unwrap();
    let x_reserialized = yaserde::ser::to_string_with_config(&j_loaded, &yaserde_cfg).unwrap();
    println!("{}", x_reserialized);
    assert_eq!(x_serialized, x_reserialized);
    Ok(())
}

#[test]
fn test_gldf_product_impls() {
    let loaded: GldfProduct = GldfProduct::load_gldf(GLDF_FILE_NAME).unwrap();
    println!("{:?}", loaded);
    // Display pretty printed XML
    let x_serialized = loaded.to_xml().unwrap();
    println!("{}", x_serialized);
    let json_str = loaded.to_json().unwrap();
    let j_loaded: GldfProduct = GldfProduct::from_json(&json_str).unwrap();
    let x_reserialized =  j_loaded.to_xml().unwrap();
    println!("{}", x_reserialized);
    println!{r#"{{"product":"#}
    println!("{}", loaded.to_json().unwrap());
    //println!("{}", loaded.to_pretty_json().unwrap());
    println!("}}");


    assert_eq!(x_serialized, x_reserialized);
}

fn read_test_gldf() -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut gldf_file = StdFile::open(GLDF_FILE_NAME).unwrap();
    let mut file_buf = Vec::new();
    gldf_file.read_to_end(&mut file_buf)?;
    return Ok(file_buf);
}

#[test]
fn test_gldf_from_buf() {
    let file_buf = read_test_gldf().unwrap();
    let loaded= GldfProduct::load_gldf_from_buf_all(file_buf).unwrap();
    println!("{:?}", loaded.gldf);
    // Display pretty printed XML
    let x_serialized = loaded.gldf.to_xml().unwrap();
    println!("{}", x_serialized);
    let json_str = loaded.gldf.to_json().unwrap();
    println!("{}", json_str);
    let j_loaded: GldfProduct = GldfProduct::from_json(&json_str).unwrap();
    let j_reserialized =  j_loaded.to_xml().unwrap();
    println!("{}", j_reserialized);
    assert_eq!(x_serialized, j_reserialized);
}

#[test]
fn test_gldf_get_phot_files() {
    use std::string::String;
    let loaded: GldfProduct = GldfProduct::load_gldf(GLDF_FILE_NAME).unwrap();
    let phot_files = loaded.get_phot_files().unwrap();
    let mut ldc_contents: Vec<String> = Vec::new();
    for f in phot_files.iter(){
        let mut ldc_content = "".to_owned();
        let file_id = f.id.to_string();
        ldc_content.push_str(&loaded.get_ldc_by_id(file_id).unwrap().to_owned());
        ldc_contents.push(ldc_content);
        println!("{}", f.file_name)
    }
}

#[test]
fn test_gldf_get_pic_files() {
    let loaded: GldfProduct = GldfProduct::load_gldf(GLDF_FILE_NAME).unwrap();
    let image_files = loaded.get_image_def_files().unwrap();
    //let mut image_contents = Vec::new();
    for f in image_files.iter(){
        let image_content = "".to_owned();
    }
}
