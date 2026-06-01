//! Product Identity editor — document-level ProductMetaData fields.
//!
//! Four `LocaleFoo` fields (Name, ProductNumber, Description, TenderText)
//! plus read-only display of UniqueProductId. Per-variant identity overrides
//! live in `VariantEditor` (slice 2 cont.).

use crate::components::LocaleFooEditor;
use crate::state::{use_gldf, GldfAction};
use gldf_rs::gldf::LocaleFoo;
use leptos::prelude::*;

#[component]
pub fn IdentityEditorPage() -> impl IntoView {
    let state = use_gldf();

    let on_name = Callback::new(move |v: LocaleFoo| {
        state.update(|s| s.apply(GldfAction::SetProductMetaName(v)));
    });
    let on_pn = Callback::new(move |v: LocaleFoo| {
        state.update(|s| s.apply(GldfAction::SetProductMetaProductNumber(v)));
    });
    let on_desc = Callback::new(move |v: LocaleFoo| {
        state.update(|s| s.apply(GldfAction::SetProductMetaDescription(v)));
    });
    let on_tender = Callback::new(move |v: LocaleFoo| {
        state.update(|s| s.apply(GldfAction::SetProductMetaTenderText(v)));
    });

    view! {
        <div class="editor-section identity-editor">
            <h2>{ "Product Identity" }</h2>
            <p class="section-description">
                { "Document-level metadata that applies to every variant. Per-SKU overrides live on the Variants page." }
            </p>

            {move || state.with(|s| {
                let meta = s.product.product_definitions.product_meta_data.as_ref();
                let upi = meta.map(|m| m.unique_product_id.clone()).unwrap_or_default();
                let name = meta.and_then(|m| m.name.clone()).unwrap_or_default();
                let pn = meta.and_then(|m| m.product_number.clone()).unwrap_or_default();
                let desc = meta.and_then(|m| m.description.clone()).unwrap_or_default();
                let tender = meta.and_then(|m| m.tender_text.clone()).unwrap_or_default();
                let upi_text = if upi.is_empty() { "(unset)".to_string() } else { upi };
                view! {
                    <div class="metadata-grid">
                        <div class="meta-item">
                            <span class="label">{ "Unique Product Id:" }</span>
                            <span class="value code">{ upi_text }</span>
                        </div>
                        <div class="meta-item">
                            <LocaleFooEditor label="Product Name" value=name on_change=on_name />
                        </div>
                        <div class="meta-item">
                            <LocaleFooEditor label="Product Number" value=pn on_change=on_pn />
                        </div>
                        <div class="meta-item wide">
                            <LocaleFooEditor
                                label="Description"
                                value=desc
                                on_change=on_desc
                                multiline=true
                            />
                        </div>
                        <div class="meta-item wide">
                            <LocaleFooEditor
                                label="Tender Text"
                                value=tender
                                on_change=on_tender
                                multiline=true
                            />
                        </div>
                    </div>
                }
            })}
        </div>
    }
}
