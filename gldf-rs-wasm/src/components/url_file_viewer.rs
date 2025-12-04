//! Component for fetching and displaying URL-based GLDF files

use yew::prelude::*;
use gloo::console::log;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use crate::utils::fetch_binary_data;
use super::{LdtViewer, L3dViewer};

/// State of a URL file fetch
#[derive(Clone, PartialEq)]
enum FetchState {
    NotStarted,
    Loading,
    Loaded(Vec<u8>),
    Error(String),
}

/// Properties for URL file viewer
#[derive(Properties, Clone, PartialEq)]
pub struct UrlFileViewerProps {
    /// The URL to fetch
    pub url: String,
    /// Content type (e.g., "ldc/eulumdat", "image/png")
    pub content_type: String,
    /// File ID for display
    #[prop_or_default]
    pub file_id: String,
}

/// Component that fetches and displays a file from a URL
#[function_component(UrlFileViewer)]
pub fn url_file_viewer(props: &UrlFileViewerProps) -> Html {
    let fetch_state = use_state(|| FetchState::NotStarted);

    // Fetch data on mount
    {
        let fetch_state = fetch_state.clone();
        let url = props.url.clone();
        use_effect_with(url.clone(), move |_| {
            if *fetch_state == FetchState::NotStarted {
                fetch_state.set(FetchState::Loading);
                let fetch_state = fetch_state.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match fetch_binary_data(&url).await {
                        Ok(data) => {
                            log!(format!("Fetched {} bytes from URL", data.len()));
                            fetch_state.set(FetchState::Loaded(data));
                        }
                        Err(e) => {
                            log!(format!("Error fetching URL: {}", e));
                            fetch_state.set(FetchState::Error(e));
                        }
                    }
                });
            }
            || ()
        });
    }

    let content_type = &props.content_type;

    match &*fetch_state {
        FetchState::NotStarted | FetchState::Loading => {
            html! {
                <div class="url-file-loading">
                    <span class="spinner"></span>
                    <span>{ format!("Loading {}...", props.url) }</span>
                </div>
            }
        }
        FetchState::Error(e) => {
            html! {
                <div class="url-file-error">
                    <span class="icon">{ "⚠️" }</span>
                    <span>{ format!("Failed to load: {}", e) }</span>
                    <a href={props.url.clone()} target="_blank">{ "Open directly" }</a>
                </div>
            }
        }
        FetchState::Loaded(data) => {
            // Render based on content type
            if content_type.starts_with("ldc") || content_type.contains("eulumdat") || content_type.contains("ies") {
                // Photometric data - use LdtViewer
                html! {
                    <div class="url-file-content">
                        <LdtViewer ldt_data={data.clone()} width={400.0} height={400.0} />
                    </div>
                }
            } else if content_type.starts_with("image") || content_type.contains("png") || content_type.contains("jpg") || content_type.contains("jpeg") {
                // Image
                let mime = if content_type.contains("png") { "image/png" }
                    else if content_type.contains("svg") { "image/svg+xml" }
                    else { "image/jpeg" };
                html! {
                    <div class="url-file-content">
                        <img src={format!("data:{};base64,{}", mime, BASE64_STANDARD.encode(data))} />
                    </div>
                }
            } else if content_type.contains("xml") {
                // XML text
                let text = String::from_utf8_lossy(data);
                html! {
                    <div class="url-file-content">
                        <pre class="xml-content">{ text.to_string() }</pre>
                    </div>
                }
            } else if content_type.contains("geo") || content_type.contains("l3d") {
                // Geometry - use L3dViewer
                html! {
                    <div class="url-file-content">
                        <L3dViewer l3d_data={data.clone()} width={500} height={400} />
                    </div>
                }
            } else {
                // Unknown/binary - show size and download link
                html! {
                    <div class="url-file-content">
                        <p>{ format!("Binary file: {} bytes", data.len()) }</p>
                        <a href={props.url.clone()} target="_blank">{ "Download" }</a>
                    </div>
                }
            }
        }
    }
}
