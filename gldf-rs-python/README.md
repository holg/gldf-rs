[![CI](https://github.com/holg/gldf-rs-python/actions/workflows/CI.yml/badge.svg)](https://github.com/holg/gldf-rs-python/actions/workflows/CI.yml)
# gldf-rs-python
Some more descriptive info in the Cargo.toml file
Added the build.rs mostly bcs of macOS

v0.2.0
New:
support for file types of url
support for BOM encoded UTF8 product.xml

WIP python module and binding for the gldf-rs rust library  
the github workflow shall make sure all the wheels are build  
and published to pypi  

install via pip:  

```
pip install gldf-rs-python
```


create venv and maturin

maturin develop


```
import gldf_rs_python

xml = gldf_rs_python.gldf_to_xml('tests/data/test.gldf')

json = gldf_rs_python.gldf_to_json('tests/data/test.gldf')

xml2 = gldf_rs_python.xml_from_json(json)

xml == xml2
```
=> True
