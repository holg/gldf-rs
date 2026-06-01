//! Raw JSON dump of the current product. Mirrors `view_raw_data` from Yew.

use crate::state::use_gldf;
use leptos::prelude::*;

#[component]
pub fn RawDataPage() -> impl IntoView {
    let state = use_gldf();

    view! {
        {move || {
            let result = state.with(|s| s.product.to_pretty_json());
            match result {
                Ok(json) => view! {
                    <div class="code-block">
                        <pre>{ json }</pre>
                    </div>
                }.into_any(),
                Err(_) => view! { <p>{ "Failed to serialize GLDF data" }</p> }.into_any(),
            }
        }}
    }
}
