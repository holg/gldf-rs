[![Rust](https://github.com/holg/gldf-rs-wasm/actions/workflows/rust.yml/badge.svg)](https://github.com/holg/gldf-rs-wasm/actions/workflows/rust.yml)
[![Build and Test WASM](https://github.com/holg/gldf-rs-wasm/actions/workflows/wasm-pack.yml/badge.svg)](https://github.com/holg/gldf-rs-wasm/actions/workflows/wasm-pack.yml)
# GLDF-RS-WASM

#### The wasm version of [gldr-rs](https://crates.io/gldr-rs)

GLDF-RS-WASM is a WebAssembly (Wasm) version of the Global Lighting Data Format (GLDF) library. It allows you to work with GLDF data directly in the browser by leveraging WebAssembly technology. GLDF is a standardized format for describing lighting products and their technical details.
### Release Notes
- 0.2.1 usage of new gldf-rs 0.2.1
- Inheritance and overwriting of properties (needed bcs of reqwest)


## Features

- Deserialize GLDF files on the client-side using WebAssembly.
- Interact with GLDF data directly within web applications.
- Use GLDF data for dynamic visualization and analysis in the browser.

## Usage

1. Include the GLDF-RS-WASM JavaScript module in your HTML:

```html
<!DOCTYPE html><html lang="en"><head>
    <meta charset="utf-8">
    <title>GLDF • File Upload</title>

    <script type="module">import init from '/gldf-rs-wasm-e633e9b682fa57a5.js';init('/gldf-rs-wasm-e633e9b682fa57a5_bg.wasm');</script>
    <link rel="stylesheet" href="/styles-a8c36e60fd065d7c.css">
    <link rel="stylesheet" href="/font-awesome.min.css">
  
<link rel="preload" href="/gldf-rs-wasm-e633e9b682fa57a5_bg.wasm" as="fetch" type="application/wasm" crossorigin="">
<link rel="modulepreload" href="/gldf-rs-wasm-e633e9b682fa57a5.js"></head>

  <body>

</body></html>
```

That's it! You can now use the `gldf-rs-wasm` module in your JavaScript code.
The working example can be found here:   

### https://gldf.icu

There you have the advantage, that included eulumdat files, can be directly opened into  
### https://eulumdat.icu,  

which is an WASM Eulumdat Editor and Viewer, which can as well export to IESNA Format.
