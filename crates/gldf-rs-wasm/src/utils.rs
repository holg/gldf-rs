use gloo::console::log;
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

#[allow(dead_code)]
pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    // Note: console_error_panic_hook feature not currently enabled
}

/// Get the current origin from the browser window.
fn get_origin() -> Option<String> {
    web_sys::window()?.location().origin().ok()
}

/// Check if a URL is same-origin (allowed for direct fetch without CORS issues)
fn is_same_origin(url: &str) -> bool {
    if let Some(origin) = get_origin() {
        url.starts_with(&origin)
    } else {
        false
    }
}

/// Fetch binary data from a URL. Only allows same-origin requests.
pub async fn fetch_binary_data(url: &str) -> Result<Vec<u8>, String> {
    if !is_same_origin(url) {
        return Err(format!(
            "Cross-origin requests not supported. URL must be on the same domain: {}",
            url
        ));
    }

    log!("fetch_binary_data:", url);

    let response = gloo::net::http::Request::get(url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "HTTP error: {} {}",
            response.status(),
            response.status_text()
        ));
    }

    response
        .binary()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))
}

/// Fetch text data from a URL. Only allows same-origin requests.
#[allow(dead_code)]
pub async fn fetch_text_data(url: &str) -> Result<String, String> {
    if !is_same_origin(url) {
        return Err(format!(
            "Cross-origin requests not supported. URL must be on the same domain: {}",
            url
        ));
    }

    log!("fetch_text_data:", url);

    let response = gloo::net::http::Request::get(url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "HTTP error: {} {}",
            response.status(),
            response.status_text()
        ));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))
}

/// Triggers a browser download for text content (JSON, XML).
///
/// # Arguments
/// * `filename` - The name of the file to download
/// * `content` - The text content to download
/// * `mime_type` - MIME type (e.g., "application/json", "application/xml")
pub fn trigger_text_download(filename: &str, content: &str, mime_type: &str) -> Result<(), String> {
    trigger_binary_download(filename, content.as_bytes(), mime_type)
}

/// Triggers a browser download for binary content (GLDF, images).
///
/// # Arguments
/// * `filename` - The name of the file to download
/// * `data` - The binary content to download
/// * `mime_type` - MIME type (e.g., "application/zip", "application/octet-stream")
pub fn trigger_binary_download(filename: &str, data: &[u8], mime_type: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window object")?;
    let document = window.document().ok_or("No document object")?;

    // Create a Uint8Array from the data
    let uint8arr = js_sys::Uint8Array::new_with_length(data.len() as u32);
    uint8arr.copy_from(data);

    // Create array containing the Uint8Array
    let array = js_sys::Array::new();
    array.push(&uint8arr.buffer());

    // Create blob with proper MIME type
    let opts = BlobPropertyBag::new();
    opts.set_type(mime_type);

    let blob = Blob::new_with_u8_array_sequence_and_options(&array, &opts)
        .map_err(|e| format!("Failed to create Blob: {:?}", e))?;

    // Create object URL for the blob
    let url = Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Failed to create object URL: {:?}", e))?;

    // Create anchor element and trigger download
    let anchor: HtmlAnchorElement = document
        .create_element("a")
        .map_err(|e| format!("Failed to create anchor: {:?}", e))?
        .dyn_into()
        .map_err(|_| "Failed to cast to HtmlAnchorElement")?;

    anchor.set_href(&url);
    anchor.set_download(filename);

    // Append to body, click, and remove
    let body = document.body().ok_or("No body element")?;
    body.append_child(&anchor)
        .map_err(|e| format!("Failed to append anchor: {:?}", e))?;

    anchor.click();

    body.remove_child(&anchor)
        .map_err(|e| format!("Failed to remove anchor: {:?}", e))?;

    // Revoke the object URL to free memory
    Url::revoke_object_url(&url)
        .map_err(|e| format!("Failed to revoke object URL: {:?}", e))?;

    log!("Download triggered:", filename);
    Ok(())
}

/// Helper to download JSON content
pub fn download_json(filename: &str, content: &str) -> Result<(), String> {
    trigger_text_download(filename, content, "application/json")
}

/// Helper to download XML content
pub fn download_xml(filename: &str, content: &str) -> Result<(), String> {
    trigger_text_download(filename, content, "application/xml")
}

/// Helper to download GLDF content (ZIP file)
pub fn download_gldf(filename: &str, data: &[u8]) -> Result<(), String> {
    trigger_binary_download(filename, data, "application/zip")
}
