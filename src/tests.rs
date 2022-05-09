use gldf::GldfProduct;
use crate::gldf;


#[test]
fn parsing_gldf_container() {
    use yaserde::de::from_str;
    use serde_json::from_str as serde_from_str;
    let loaded: GldfProduct = GldfProduct::load_gldf("./tests/data/test.gldf").unwrap();
    println!("{:?}", loaded);
    // Display pretty printed XML
    let yaserde_cfg = yaserde::ser::Config {
        perform_indent: true,
        ..Default::default()
    };
    let x_serialized = yaserde::ser::to_string_with_config(&loaded, &yaserde_cfg).unwrap();
    println!("{}", x_serialized);
    let json_str = serde_json::to_string(&loaded).unwrap();
    println!("{}", json_str);
    let j_loaded: GldfProduct = serde_from_str(&json_str).unwrap();
    let x_reserialized = yaserde::ser::to_string_with_config(&j_loaded, &yaserde_cfg).unwrap();
    println!("{}", x_reserialized);
    assert_eq!(x_serialized, x_reserialized);
}

#[test]
fn test_gldf_product_impls() {
    let loaded: GldfProduct = GldfProduct::load_gldf("./tests/data/test.gldf").unwrap();
    println!("{:?}", loaded);
    // Display pretty printed XML
    let x_serialized = loaded.to_xml().unwrap();
    println!("{}", x_serialized);
    let json_str = loaded.to_json().unwrap();
    println!("{}", json_str);
    let j_loaded: GldfProduct = GldfProduct::from_json(&json_str).unwrap();
    let x_reserialized =  j_loaded.to_xml().unwrap();
    println!("{}", x_reserialized);
    assert_eq!(x_serialized, x_reserialized);
}
