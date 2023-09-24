[![Rust](https://github.com/holg/gldf-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/holg/gldf-rs/actions/workflows/rust.yml)


# gldf-rs
Process GLDF

Release notes:
0.2.2
- added support meta-information.xml



0.2.1
- added better documentation fo the main page
- for wasm support some refactoring was needed, to use reqwest::blocking 


0.2.0 
- refactored gldf.rs into submodules
- added support for BOM encoded UTF8 product.xml
- added support for url file_types
- added better documentation

A cross platform GLDF processing library.

For the GLDF new Luminaire / Sensor Container Definition.

Basically .gldf is a zip Container, containing the product.xml for the Definition

and as well the needed binaries, e.g. images and soem Eulumdat / or IES files,
as well as 3D Models characterising the luminaire.

More:

https://gldf.io

This rust lib for now can read the product.xml definition directly from the .gldf Container
and is able to represent the content as well as JSON, which is the preferred way for feeding some content of it
into Search Engines or in General as JSON Storage now is quite common, last but not least Postgres.


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

Passes OK, so we are able to read xml, convert to JSON and again into the same XML.

For processing for now there are some implemantations already:

    let phot_files = loaded.get_phot_files().unwrap();
    let mut ldc_contents: Vec<String> = Vec::new();
    for f in phot_files.iter(){
        let mut ldc_content = "".to_owned();
        let file_id = f.id.to_string();
        ldc_content.push_str(&loaded.get_ldc_by_id(file_id).unwrap().to_owned());
        ldc_contents.push(ldc_content);
        println!("{}", f.file_name)
    }

    Here it is shown how to read the ldc files from the GLDF Container.
