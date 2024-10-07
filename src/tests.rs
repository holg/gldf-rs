#![allow(unused_variables)]
use crate::BufFile;
use gldf::{GldfProduct};
use anyhow::{Result};
use serde::de::StdError;
use crate::{gldf, StdFile};


// const GLDF_FILE_NAME: &str =  "./tests/data/R2MCOBSIK-30.gldf";
const GLDF_FILE_NAME: &str =  "./tests/data/test.gldf";
const GLDF_FILE_NAME_URL: &str =  "./tests/data/R2MCOBSIK-30.gldf";
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


#[tokio::test]
async fn test_get_buf_files() -> Result<()> {
    let loaded: GldfProduct = GldfProduct::load_gldf(GLDF_FILE_NAME_URL).unwrap();
    let buf_files = loaded.get_buf_files_proxied_with_logger_async(None, &None, None).await?;
    for buf_file in buf_files.iter() {
        println!(
            "BufFile {{ name: {:?}, file_id: {:?}, content_type: {:?}, path: {:?}, content_length: {:?} }}",
            buf_file.name,
            buf_file.file_id,
            buf_file.content_type,
            buf_file.path,
            buf_file.content.as_ref().map(|c| c.len())
        );
    }
    // Define expected values for the test
    let expected_files = vec![
        BufFile {
            name: Some("R2MCOBSIK-30_flexi-strip_vegas-cob-2M-IP20-LED-Stripkit_main.jpg".to_string()),
            file_id: Some("file_12071".to_string()),
            content_type: Some("image/jpg".to_string()),
            path: Some("https://asset.eezybridge.com/896646eb-5f86-4ce5-bedd-82bfbbcca160/R2MCOBSIK-30_flexi-strip_vegas-cob-2M-IP20-LED-Stripkit_main.jpg".to_string()),
            content: Some(vec![0; 1712152]),
            size: None,
        },
        BufFile {
            name: Some("Instruction Manual for R2MCOBSIK-30 R2MCOBSIK-40 ISSUE 1 AU.pdf".to_string()),
            file_id: Some("file_12112".to_string()),
            content_type: Some("other".to_string()),
            path: Some("https://asset.eezybridge.com/896646eb-5f86-4ce5-bedd-82bfbbcca160/Instruction Manual for R2MCOBSIK-30 R2MCOBSIK-40 ISSUE 1 AU.pdf".to_string()),
            content: Some(vec![0; 194844]),
            size: None,
        },
        BufFile {
            name: Some("R2MCOBSIK-30-Photometrics.ldt".to_string()),
            file_id: Some("file_12419".to_string()),
            content_type: Some("ldc/eulumdat".to_string()),
            path: Some("https://asset.eezybridge.com/896646eb-5f86-4ce5-bedd-82bfbbcca160/R2MCOBSIK-30-Photometrics.ldt".to_string()),
            content: Some(vec![0; 22164]),
            size: None,
        },
        BufFile {
            name: Some("R2MCOBSIK-30-Photometrics.IES".to_string()),
            file_id: Some("file_12420".to_string()),
            content_type: Some("ldc/eulumdat".to_string()),
            path: Some("https://asset.eezybridge.com/896646eb-5f86-4ce5-bedd-82bfbbcca160/R2MCOBSIK-30-Photometrics.IES".to_string()),
            content: Some(vec![0; 31640]),
            size: None,
        },
        BufFile {
            name: Some("VEGAS COB KITS-AU-RxMCOBSIK-ROBUS Product Information Document.pdf".to_string()),
            file_id: Some("file_12481".to_string()),
            content_type: Some("document/pdf".to_string()),
            path: Some("https://asset.eezybridge.com/896646eb-5f86-4ce5-bedd-82bfbbcca160/VEGAS COB KITS-AU-RxMCOBSIK-ROBUS Product Information Document.pdf".to_string()),
            content: Some(vec![0; 481564]),
            size: None,
        },
    ];

    assert_eq!(buf_files.len(), expected_files.len());

    for (buf_file, expected) in buf_files.iter().zip(expected_files.iter()) {
        assert_eq!(buf_file.name, expected.name);
        assert_eq!(buf_file.file_id, expected.file_id);
        assert_eq!(buf_file.content_type, expected.content_type);
        assert_eq!(buf_file.path, expected.path);
        assert_eq!(
            buf_file.content.as_ref().map(|c| c.len()),
            expected.content.as_ref().map(|c| c.len())
        );
    }

    Ok(())
}
