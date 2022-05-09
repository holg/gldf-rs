# gldf-rs
Process GLDF

WIP

For the GLDF new Luminaire / Sensor Container Definition have a look at
Basically -gldfis a zip Container, containing the product.xml for the Definition
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