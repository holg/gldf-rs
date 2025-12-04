use gloo::console::log;

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Get the current origin from the browser window.
fn get_origin() -> Option<String> {
    web_sys::window()?
        .location()
        .origin()
        .ok()
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
        return Err(format!("Cross-origin requests not supported. URL must be on the same domain: {}", url));
    }

    log!("fetch_binary_data:", url);

    let response = gloo::net::http::Request::get(url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.ok() {
        return Err(format!("HTTP error: {} {}", response.status(), response.status_text()));
    }

    response
        .binary()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))
}

/// Fetch text data from a URL. Only allows same-origin requests.
pub async fn fetch_text_data(url: &str) -> Result<String, String> {
    if !is_same_origin(url) {
        return Err(format!("Cross-origin requests not supported. URL must be on the same domain: {}", url));
    }

    log!("fetch_text_data:", url);

    let response = gloo::net::http::Request::get(url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.ok() {
        return Err(format!("HTTP error: {} {}", response.status(), response.status_text()));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))
}
