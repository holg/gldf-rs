//! Hyperlinks editor (ProductMetaData → DescriptiveAttributes → Marketing → Hyperlinks).
//!
//! Each `<Hyperlink>` has `@href`, optional `@language` / `@region` / `@countryCode`,
//! and a free-text label as the element body. GLDF uses one element per locale
//! rather than the multi-`Locale` envelope (XSD lines 5594–5614). Adding the same
//! URL in eight languages → eight `<Hyperlink>` rows distinguished by `@language`.

use crate::state::{use_gldf, GldfAction};
use yew::prelude::*;

#[function_component(HyperlinksEditor)]
pub fn hyperlinks_editor() -> Html {
    let gldf = use_gldf();

    let hyperlinks = gldf
        .product
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|m| m.descriptive_attributes.as_ref())
        .and_then(|a| a.marketing.as_ref())
        .and_then(|m| m.hyperlinks.as_ref());

    let on_add = {
        let gldf = gldf.clone();
        Callback::from(move |_| gldf.dispatch(GldfAction::AddHyperlink))
    };

    html! {
        <div class="editor-section hyperlinks-editor">
            <h2>{ "Hyperlinks" }</h2>
            <p class="section-description">
                { "Per-locale URLs (datasheets, product pages, IES files…). The href is required; language/region/country are optional. Add one row per language for multilingual links." }
            </p>

            { render_rows(hyperlinks, &gldf) }

            <div class="add-row">
                <button type="button" onclick={on_add}>{ "+ Add hyperlink" }</button>
            </div>
        </div>
    }
}

fn render_rows(
    hyperlinks: Option<&gldf_rs::gldf::Hyperlinks>,
    gldf: &crate::state::GldfContext,
) -> Html {
    let entries = hyperlinks.map(|h| h.hyperlink.as_slice()).unwrap_or(&[]);

    if entries.is_empty() {
        return html! {
            <p class="empty-message">{ "No hyperlinks." }</p>
        };
    }

    html! {
        <div class="hyperlinks-list">
            { for entries.iter().enumerate().map(|(idx, link)| render_row(idx, link, gldf)) }
        </div>
    }
}

fn render_row(
    idx: usize,
    link: &gldf_rs::gldf::Hyperlink,
    gldf: &crate::state::GldfContext,
) -> Html {
    let snapshot = link.clone();

    // Each onchange handler reads the current input value, copies the rest of
    // the hyperlink from the snapshot, and dispatches a whole-record update.
    let on_href = {
        let gldf = gldf.clone();
        let snap = snapshot.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            gldf.dispatch(GldfAction::UpdateHyperlink {
                index: idx,
                href: input.value(),
                language: snap.language.clone(),
                region: snap.region.clone(),
                country_code: snap.country_code.clone(),
                value: snap.value.clone(),
            });
        })
    };
    let on_lang = {
        let gldf = gldf.clone();
        let snap = snapshot.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let v = input.value();
            gldf.dispatch(GldfAction::UpdateHyperlink {
                index: idx,
                href: snap.href.clone(),
                language: if v.is_empty() { None } else { Some(v) },
                region: snap.region.clone(),
                country_code: snap.country_code.clone(),
                value: snap.value.clone(),
            });
        })
    };
    let on_region = {
        let gldf = gldf.clone();
        let snap = snapshot.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let v = input.value();
            gldf.dispatch(GldfAction::UpdateHyperlink {
                index: idx,
                href: snap.href.clone(),
                language: snap.language.clone(),
                region: if v.is_empty() { None } else { Some(v) },
                country_code: snap.country_code.clone(),
                value: snap.value.clone(),
            });
        })
    };
    let on_country = {
        let gldf = gldf.clone();
        let snap = snapshot.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let v = input.value();
            gldf.dispatch(GldfAction::UpdateHyperlink {
                index: idx,
                href: snap.href.clone(),
                language: snap.language.clone(),
                region: snap.region.clone(),
                country_code: if v.is_empty() { None } else { Some(v) },
                value: snap.value.clone(),
            });
        })
    };
    let on_label = {
        let gldf = gldf.clone();
        let snap = snapshot.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            gldf.dispatch(GldfAction::UpdateHyperlink {
                index: idx,
                href: snap.href.clone(),
                language: snap.language.clone(),
                region: snap.region.clone(),
                country_code: snap.country_code.clone(),
                value: input.value(),
            });
        })
    };
    let on_remove = {
        let gldf = gldf.clone();
        Callback::from(move |_| gldf.dispatch(GldfAction::RemoveHyperlink(idx)))
    };

    html! {
        <div class="hyperlink-row">
            <div class="hyperlink-row-main">
                <input
                    type="url"
                    class="hyperlink-href"
                    placeholder="https://…"
                    value={link.href.clone()}
                    onchange={on_href}
                />
                <input
                    type="text"
                    class="hyperlink-label"
                    placeholder="Label"
                    value={link.value.clone()}
                    onchange={on_label}
                />
                <button type="button" class="hyperlink-remove" title="Remove" onclick={on_remove}>
                    { "✕" }
                </button>
            </div>
            <div class="hyperlink-row-meta">
                <label>
                    { "lang:" }
                    <input
                        type="text"
                        class="hyperlink-lang"
                        maxlength="5"
                        placeholder="en"
                        value={link.language.clone().unwrap_or_default()}
                        onchange={on_lang}
                    />
                </label>
                <label>
                    { "region:" }
                    <input
                        type="text"
                        class="hyperlink-region"
                        maxlength="5"
                        placeholder="EU"
                        value={link.region.clone().unwrap_or_default()}
                        onchange={on_region}
                    />
                </label>
                <label>
                    { "country:" }
                    <input
                        type="text"
                        class="hyperlink-country"
                        maxlength="3"
                        placeholder="DE"
                        value={link.country_code.clone().unwrap_or_default()}
                        onchange={on_country}
                    />
                </label>
            </div>
        </div>
    }
}
