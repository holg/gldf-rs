use gldf_rs::version::{BuildVersion, VersionStatus};
use gloo::console::log;

/// Production server URL for version checking
pub const PRODUCTION_URL: &str = "https://gldf.icu";

/// Fetch the version.json from the production server
pub async fn fetch_production_version() -> Result<BuildVersion, String> {
    let url = format!("{}/version.json", PRODUCTION_URL);
    log!("Fetching production version from:", &url);

    let response = gloo::net::http::Request::get(&url)
        .mode(web_sys::RequestMode::Cors)
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

    let json = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    BuildVersion::from_json(&json).map_err(|e| format!("Invalid version JSON: {}", e))
}

/// Fetch the local version.json (same origin)
pub async fn fetch_local_version() -> Result<BuildVersion, String> {
    let response = gloo::net::http::Request::get("/version.json")
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

    let json = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    BuildVersion::from_json(&json).map_err(|e| format!("Invalid version JSON: {}", e))
}

/// Check if this is running on localhost
pub fn is_localhost() -> bool {
    if let Some(window) = web_sys::window() {
        if let Ok(hostname) = window.location().hostname() {
            return hostname == "localhost" || hostname == "127.0.0.1";
        }
    }
    false
}

/// Compare local version against production and return status
pub async fn check_version_status() -> Result<(BuildVersion, VersionStatus), String> {
    let local = fetch_local_version().await?;

    // If not on localhost, we're already on production - version is current
    if !is_localhost() {
        return Ok((local, VersionStatus::Current));
    }

    // On localhost, skip production version check to avoid CORS errors in console.
    // The browser logs CORS errors even when caught, which clutters the console.
    // Local development doesn't need version comparison anyway.
    log!("Version check: Skipping production comparison on localhost");
    Ok((local, VersionStatus::Unknown))
}

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
