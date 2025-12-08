use base64::encode;
use gloo::file as gloo_file;
use leptos::*;
use leptos::html::Img;
use web_sys::{Blob, Url};
use gldf_rs::{gldf::GldfProduct, FileBufGldf, BufFile};
use console_log;
use wasm_bindgen::JsCast;


#[derive(Debug, Clone, Copy)] // Derive traits for debugging, copying, and comparisons
enum FileType {
    Jpg,
    Png,
    Ldt,
    Ies,
    Xml,
    Unknown, // Optionally handle other types
}

fn get_file_type(file_name: &str) -> FileType {
    let lowercase_name = file_name.to_lowercase();
    if lowercase_name.ends_with(".jpg") {
        FileType::Jpg
    } else if lowercase_name.ends_with(".png") {
        FileType::Png
    } else if lowercase_name.ends_with(".ldt") {
        FileType::Ldt
    } else if lowercase_name.ends_with(".ies") {
        FileType::Ies
    } else if lowercase_name.ends_with(".xml") {
        FileType::Xml
    } else {
        FileType::Unknown
    }
}
// Utility functions
#[allow(dead_code)]
struct WasmGldfProduct(GldfProduct);
impl WasmGldfProduct {
    pub fn load_gldf_from_buf_all(buf: Vec<u8>) -> anyhow::Result<FileBufGldf> {
        let file_buf = GldfProduct::load_gldf_from_buf_all(buf)?;
        Ok(file_buf)
    }
}

#[component]
pub fn App() -> impl IntoView {

    let (file, set_file) = create_signal::<Option<web_sys::File>>(None);
    let (files, set_files) = create_signal(Vec::<FileBufGldf>::new());

    // File Input
    let on_file_input = move |event: web_sys::Event| {
        let files = event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
            .and_then(|input| input.files());

        if let Some(file_list) = files {
            if let Some(file) = file_list.get(0) {
                set_file.set(Some(file));
            }
        }
    };

    // GLDF Processing
    create_effect(move |_| {
        if let Some(file) = file() {
            gloo_file::callbacks::read_as_bytes(&file, move |res| {
                if let Ok(data) = res {
                    match WasmGldfProduct::load_gldf_from_buf_all(data) {
                        Ok(gldf) => {
                            set_files.set(gldf.files);
                        },
                        Err(e) => {
                            // Handle error here (e.g., display an error message to the user)
                            log!("Error loading GLDF: {:?}", e);
                        }
                    }
                } else {
                    // Handle file reading error
                    log!("Error reading file.");
                }
            });
        }
    });

    // View

    view! {
        <div id="wrapper">
            <Input type="file" on:change=on_file_input />
            <div>
                <For
                    // iterate over the signal, which is a `Vec<FileBufGldf>`
                    each=move || files.get()
                    // each item should be shown as `view_file`
                    view=view_gldf_file
                    // provide a unique key for each item
                    key=|file| file.name.clone().unwrap_or_default()
                />
            </div>
        </div>
    }
}

// Helper component for viewing GLDF files



#[component]
fn view_file(file:BufFile) -> impl IntoView {
    view! {
        <div id="file">
            <p>{ file.name.clone().unwrap_or("".to_string()) }</p>
            <p>
                <img src={format!("data:image/jpg;base64,{}", encode(file.clone().content.unwrap_or(Vec::new()))  )} />
            </p>
        </div>
    }
}
#[component]
fn view_gldf_file(file: BufFile) -> impl IntoView {
    let file_type = get_file_type(file.name.as_ref().unwrap_or(&"".to_string())); // Get file type
    view! {
        <div id="gldf_file">
            <p>{ file.name.clone().unwrap_or("".to_string()) }</p>
            <p>
                {match file_type {
                    FileType::Jpg => view_jpg(file),
                    FileType::Png => view_png(file),
                    FileType::Ldt => view_ldt(file),
                    FileType::Ies => view_ies(file),
                    FileType::Xml => view_xml(file),
                    FileType::Unknown => view_unknown(file),
                }}
            </p>
        </div>
    }
    view! {
        <div id="buf_file"><p>{ format!("{}", file.name.clone().unwrap_or("".to_string())) }</p>
            // if file.name.clone().expect("REASON").to_lowercase().ends_with(".jpg"){
            //     <Img
            // }
            // else if file.name.clone().expect("REASON").to_lowercase().ends_with(".png"){
            //     <img src={format!("data:image/jpg;base64,{}", encode(file.clone().content.unwrap_or(Vec::new()))  )} />
            // }
            // else if file.name.clone().expect("REASON").to_lowercase().ends_with(".ldt"){
            //     <a href={format!(r"/QLumEdit/QLumEdit.html?ldc_name=trahe.ldt&ldc_blob_url={}", get_blob(&file))}>{"Open in QLumEdit"}</a>
            //     <br/><textarea value={format!(r"{}", String::from_utf8_lossy(file.content.clone().unwrap().as_slice()))}></textarea>
            // //     <textarea value={format!(r"{}", string_test("test"))}></textarea>
            // }
            // else if file.name.clone().expect("REASON").to_lowercase().ends_with(".xml"){
            //     <textarea value={format!(r"{}", String::from_utf8_lossy(file.content.clone().unwrap().as_slice()))}></textarea>
            // }
        </div>
    }
}

pub fn get_blob(buf_file: &BufFile) -> String {
    let uint8arr = js_sys::Uint8Array::new(&unsafe { js_sys::Uint8Array::view(&buf_file.content.clone().unwrap()) }.into());
    let array = js_sys::Array::new();
    array.push(&uint8arr.buffer());
    let blob = Blob::new_with_str_sequence_and_options(
        &array,
        web_sys::BlobPropertyBag::new().type_("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
    ).unwrap();
    let download_url = Url::create_object_url_with_blob(&blob).unwrap();
    return download_url
}


// #[cfg(target_arch = "wasm32")] 
fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
