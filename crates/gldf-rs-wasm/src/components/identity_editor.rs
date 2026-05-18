//! Product identity editor — document-level ProductMetaData fields.
//!
//! Lives under ProductMetaData → Identity in the editor tab tree. Renders the
//! four `LocaleFoo` fields the XSD places on `ProductMetaData` (Name,
//! ProductNumber, Description, TenderText) plus read-only display of
//! UniqueProductId. Per-variant edits remain in `variant_editor.rs`.

use crate::components::LocaleFooEditor;
use crate::state::{use_gldf, GldfAction, GldfContext};
use gldf_rs::gldf::LocaleFoo;
use yew::prelude::*;

#[function_component(IdentityEditor)]
pub fn identity_editor() -> Html {
    let gldf = use_gldf();
    let meta = gldf.product.product_definitions.product_meta_data.as_ref();

    let upi = meta
        .map(|m| m.unique_product_id.clone())
        .unwrap_or_default();

    html! {
        <div class="editor-section identity-editor">
            <crate::components::LanguageBanner />
            <h2>{ "Product Identity" }</h2>
            <p class="section-description">
                { "Document-level metadata that applies to every variant. Per-SKU overrides live in the Variants tab." }
            </p>

            <div class="metadata-grid">
                <div class="meta-item">
                    <span class="label">{ "Unique Product Id:" }</span>
                    <span class="value code">{ if upi.is_empty() { "(unset)".to_string() } else { upi } }</span>
                </div>

                <div class="meta-item">
                    <LocaleFooEditor
                        label="Product Name"
                        value={meta.and_then(|m| m.name.clone()).unwrap_or_default()}
                        onchange={dispatch_set_meta_name(&gldf)}
                    />
                </div>
                <div class="meta-item">
                    <LocaleFooEditor
                        label="Product Number"
                        value={meta.and_then(|m| m.product_number.clone()).unwrap_or_default()}
                        onchange={dispatch_set_meta_product_number(&gldf)}
                    />
                </div>
                <div class="meta-item wide">
                    <LocaleFooEditor
                        label="Description"
                        value={meta.and_then(|m| m.description.clone()).unwrap_or_default()}
                        onchange={dispatch_set_meta_description(&gldf)}
                        multiline=true
                    />
                </div>
                <div class="meta-item wide">
                    <LocaleFooEditor
                        label="Tender Text"
                        value={meta.and_then(|m| m.tender_text.clone()).unwrap_or_default()}
                        onchange={dispatch_set_meta_tender_text(&gldf)}
                        multiline=true
                    />
                </div>
            </div>
        </div>
    }
}

fn dispatch_set_meta_name(gldf: &GldfContext) -> Callback<LocaleFoo> {
    let gldf = gldf.clone();
    Callback::from(move |v: LocaleFoo| gldf.dispatch(GldfAction::SetProductMetaName(v)))
}
fn dispatch_set_meta_description(gldf: &GldfContext) -> Callback<LocaleFoo> {
    let gldf = gldf.clone();
    Callback::from(move |v: LocaleFoo| gldf.dispatch(GldfAction::SetProductMetaDescription(v)))
}
fn dispatch_set_meta_product_number(gldf: &GldfContext) -> Callback<LocaleFoo> {
    let gldf = gldf.clone();
    Callback::from(move |v: LocaleFoo| gldf.dispatch(GldfAction::SetProductMetaProductNumber(v)))
}
fn dispatch_set_meta_tender_text(gldf: &GldfContext) -> Callback<LocaleFoo> {
    let gldf = gldf.clone();
    Callback::from(move |v: LocaleFoo| gldf.dispatch(GldfAction::SetProductMetaTenderText(v)))
}
