use gldf::GldfProduct;
use crate::gldf;

#[test]
fn parsing_gldf_namespace() {
    use std::fs;
    use yaserde::de::from_str;
    use serde_json::from_str as serde_from_str;

    let filename = "./tests/data/product.xml";
    let content = fs::read_to_string(filename).expect("something went wrong reading the file");

    let loaded: GldfProduct = from_str(&content).unwrap();
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
    // assert_eq!(loaded.tabpage[0].control.len(), 13);
    // assert_eq!(loaded.tabpage[1].control.len(), 16);
    // assert_eq!(loaded.tabpage[2].control.len(), 65);
    // assert_eq!(loaded.tabpage[3].control.len(), 40);
}
