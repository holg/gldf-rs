//! Editor tabs component — top-level navigation that mirrors the GLDF document tree.
//!
//! Top sections: Header, GeneralDefinitions, ProductMetaData, Variants.
//! Sub-sections sit under their natural parent so the UI matches the XSD shape:
//! e.g. Electrical lives under ProductMetaData → DescriptiveAttributes, not at the top level.

use super::{
    ApplicationsEditor, BuilderWidget, CustomPropertiesEditor, ElectricalEditor, EmergencyEditor,
    EmittersBrowser, EquipmentsBrowser, FilesEditor, GeometriesBrowser, HeaderEditor,
    HyperlinksEditor, IdentityEditor, LabelsEditor, LightSourceEditor, MechanicalEditor,
    PhotometriesBrowser, PicturesEditor, SensorsBrowser, VariantEditor,
};
use crate::state::{collect_languages, use_gldf, GldfAction};
use yew::prelude::*;

/// Top-level sections of a GLDF product.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TopSection {
    Header,
    GeneralDefinitions,
    ProductMetaData,
    Variants,
}

impl TopSection {
    fn label(self) -> &'static str {
        match self {
            TopSection::Header => "Header",
            TopSection::GeneralDefinitions => "General Definitions",
            TopSection::ProductMetaData => "Product Metadata",
            TopSection::Variants => "Variants",
        }
    }
}

/// Sub-sections under General Definitions (mirrors GeneralDefinitions XSD children).
/// `Builder` is a UI affordance, not an XSD child — it's a drop-and-link panel
/// that creates Files / Photometries / Spectrums entries via the core-lib
/// `EditableGldf` builder API.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GeneralSection {
    Builder,
    Files,
    Photometries,
    LightSources,
    Equipments,
    Emitters,
    Sensors,
    Geometries,
}

impl GeneralSection {
    fn label(self) -> &'static str {
        match self {
            GeneralSection::Builder => "Builder",
            GeneralSection::Files => "Files",
            GeneralSection::Photometries => "Photometries",
            GeneralSection::LightSources => "Light Sources",
            GeneralSection::Equipments => "Equipments",
            GeneralSection::Emitters => "Emitters",
            GeneralSection::Sensors => "Sensors",
            GeneralSection::Geometries => "Geometries",
        }
    }
}

/// Sub-sections under Product Metadata. `Pictures` sits at this level (XSD
/// places it as a direct child of `ProductMetaData`, not under `Marketing`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ProductMetaSection {
    Identity,
    Pictures,
    Mechanical,
    Electrical,
    Emergency,
    Marketing,
    CustomProperties,
}

impl ProductMetaSection {
    fn label(self) -> &'static str {
        match self {
            ProductMetaSection::Identity => "Identity",
            ProductMetaSection::Pictures => "Pictures",
            ProductMetaSection::Mechanical => "Mechanical",
            ProductMetaSection::Electrical => "Electrical",
            ProductMetaSection::Emergency => "Emergency",
            ProductMetaSection::Marketing => "Marketing",
            ProductMetaSection::CustomProperties => "Custom Properties",
        }
    }
}

/// Sub-sections under Marketing (mirrors Marketing XSD children — note Pictures
/// is *not* a child of Marketing, it lives at ProductMetaData level above).
#[derive(Clone, Copy, PartialEq, Eq)]
enum MarketingSection {
    Applications,
    Hyperlinks,
    ApprovalMarks,
    DesignAwards,
    Labels,
    ListPrices,
}

impl MarketingSection {
    fn label(self) -> &'static str {
        match self {
            MarketingSection::Applications => "Applications",
            MarketingSection::Hyperlinks => "Hyperlinks",
            MarketingSection::ApprovalMarks => "Approval Marks",
            MarketingSection::DesignAwards => "Design Awards",
            MarketingSection::Labels => "Labels",
            MarketingSection::ListPrices => "List Prices",
        }
    }
}

#[function_component(EditorTabs)]
pub fn editor_tabs() -> Html {
    let active_top = use_state(|| TopSection::Header);
    let active_general = use_state(|| GeneralSection::Files);
    let active_meta = use_state(|| ProductMetaSection::Identity);
    let active_marketing = use_state(|| MarketingSection::Applications);

    let top_sections = [
        TopSection::Header,
        TopSection::GeneralDefinitions,
        TopSection::ProductMetaData,
        TopSection::Variants,
    ];

    html! {
        <div class="editor-tabs-container">
            <LanguagePicker />
            <nav class="editor-nav editor-nav-top">
                { for top_sections.iter().map(|section| {
                    let is_active = *active_top == *section;
                    let section = *section;
                    let setter = active_top.clone();
                    html! {
                        <button
                            class={classes!("nav-tab", is_active.then_some("active"))}
                            onclick={Callback::from(move |_| setter.set(section))}
                        >
                            { section.label() }
                        </button>
                    }
                })}
            </nav>

            <div class="editor-content">
                { match *active_top {
                    TopSection::Header => html! { <HeaderEditor /> },
                    TopSection::GeneralDefinitions => render_general(&active_general),
                    TopSection::ProductMetaData => render_meta(&active_meta, &active_marketing),
                    TopSection::Variants => html! { <VariantEditor /> },
                }}
            </div>
        </div>
    }
}

fn render_general(active: &UseStateHandle<GeneralSection>) -> Html {
    let sections = [
        GeneralSection::Builder,
        GeneralSection::Files,
        GeneralSection::Photometries,
        GeneralSection::LightSources,
        GeneralSection::Equipments,
        GeneralSection::Emitters,
        GeneralSection::Sensors,
        GeneralSection::Geometries,
    ];

    html! {
        <div class="editor-subsection">
            <nav class="editor-nav editor-nav-sub">
                { for sections.iter().map(|s| sub_button(*s, **active, *s == **active, active.clone(), |s| s.label())) }
            </nav>
            <div class="editor-subcontent">
                { match **active {
                    GeneralSection::Builder => html! { <BuilderWidget /> },
                    GeneralSection::Files => html! { <FilesEditor /> },
                    GeneralSection::LightSources => html! { <LightSourceEditor /> },
                    GeneralSection::Photometries => html! { <PhotometriesBrowser /> },
                    GeneralSection::Equipments => html! { <EquipmentsBrowser /> },
                    GeneralSection::Emitters => html! { <EmittersBrowser /> },
                    GeneralSection::Sensors => html! { <SensorsBrowser /> },
                    GeneralSection::Geometries => html! { <GeometriesBrowser /> },
                }}
            </div>
        </div>
    }
}

fn render_meta(
    active: &UseStateHandle<ProductMetaSection>,
    marketing: &UseStateHandle<MarketingSection>,
) -> Html {
    let sections = [
        ProductMetaSection::Identity,
        ProductMetaSection::Pictures,
        ProductMetaSection::Mechanical,
        ProductMetaSection::Electrical,
        ProductMetaSection::Emergency,
        ProductMetaSection::Marketing,
        ProductMetaSection::CustomProperties,
    ];

    html! {
        <div class="editor-subsection">
            <nav class="editor-nav editor-nav-sub">
                { for sections.iter().map(|s| sub_button(*s, **active, *s == **active, active.clone(), |s| s.label())) }
            </nav>
            <div class="editor-subcontent">
                { match **active {
                    ProductMetaSection::Identity => html! { <IdentityEditor /> },
                    ProductMetaSection::Pictures => html! { <PicturesEditor /> },
                    ProductMetaSection::Mechanical => html! { <MechanicalEditor /> },
                    ProductMetaSection::Electrical => html! { <ElectricalEditor /> },
                    ProductMetaSection::Emergency => html! { <EmergencyEditor /> },
                    ProductMetaSection::Marketing => render_marketing(marketing),
                    ProductMetaSection::CustomProperties => html! { <CustomPropertiesEditor /> },
                }}
            </div>
        </div>
    }
}

fn render_marketing(active: &UseStateHandle<MarketingSection>) -> Html {
    let sections = [
        MarketingSection::Applications,
        MarketingSection::Hyperlinks,
        MarketingSection::ApprovalMarks,
        MarketingSection::DesignAwards,
        MarketingSection::Labels,
        MarketingSection::ListPrices,
    ];

    html! {
        <div class="editor-subsection editor-subsection-nested">
            <nav class="editor-nav editor-nav-sub">
                { for sections.iter().map(|s| sub_button(*s, **active, *s == **active, active.clone(), |s| s.label())) }
            </nav>
            <div class="editor-subcontent">
                { match **active {
                    MarketingSection::Applications => html! { <ApplicationsEditor /> },
                    MarketingSection::Hyperlinks => html! { <HyperlinksEditor /> },
                    MarketingSection::ApprovalMarks => placeholder("Approval Marks", "CE, GS, ENEC, UL, etc. (free string per XSD; language-neutral)."),
                    MarketingSection::DesignAwards => placeholder("Design Awards", "Free strings — e.g. \"Red Dot Award 2024\"."),
                    MarketingSection::Labels => html! { <LabelsEditor /> },
                    MarketingSection::ListPrices => placeholder("List Prices", "Per-currency prices (ISO 4217)."),
                }}
            </div>
        </div>
    }
}

fn sub_button<S: Copy + PartialEq + 'static>(
    section: S,
    _active: S,
    is_active: bool,
    setter: UseStateHandle<S>,
    label_of: fn(S) -> &'static str,
) -> Html {
    html! {
        <button
            class={classes!("nav-tab", "nav-tab-sub", is_active.then_some("active"))}
            onclick={Callback::from(move |_| setter.set(section))}
        >
            { label_of(section) }
        </button>
    }
}

fn placeholder(title: &str, desc: &str) -> Html {
    html! {
        <div class="editor-section editor-placeholder">
            <h2>{ title }</h2>
            <p class="placeholder-hint">{ desc }</p>
            <p class="placeholder-status">{ "Not yet implemented." }</p>
        </div>
    }
}

/// Top-of-editor language picker.
///
/// Renders the union of every language present in the document plus the
/// currently-active language (so newly typed codes stick around even before
/// the user creates a corresponding Locale entry). A small text field next to
/// the dropdown lets the user introduce a new language code.
#[function_component(LanguagePicker)]
pub fn language_picker() -> Html {
    let gldf = use_gldf();
    let active = gldf.active_language.clone();
    let pending_new = use_state(String::new);

    let mut langs = collect_languages(&gldf.product);
    if !active.is_empty() && !langs.iter().any(|l| l == &active) {
        langs.insert(0, active.clone());
        langs.sort();
    }

    let on_select = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            gldf.dispatch(GldfAction::SetActiveLanguage(select.value()));
        })
    };

    let on_new_change = {
        let pending_new = pending_new.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            pending_new.set(input.value());
        })
    };

    let on_add_new = {
        let gldf = gldf.clone();
        let pending_new = pending_new.clone();
        Callback::from(move |_| {
            let lang = pending_new.trim().to_string();
            if !lang.is_empty() {
                gldf.dispatch(GldfAction::SetActiveLanguage(lang));
                pending_new.set(String::new());
            }
        })
    };

    html! {
        <div class="language-picker">
            <label class="language-picker-label">{ "Active language:" }</label>
            <select class="language-picker-select" onchange={on_select} value={active.clone()}>
                { for langs.iter().map(|l| {
                    let is_selected = l == &active;
                    html! {
                        <option value={l.clone()} selected={is_selected}>{ l }</option>
                    }
                }) }
            </select>
            <input
                type="text"
                class="language-picker-new"
                placeholder="add lang (e.g., de)"
                value={(*pending_new).clone()}
                oninput={on_new_change}
                maxlength="5"
            />
            <button type="button" class="language-picker-add" onclick={on_add_new}>
                { "+" }
            </button>
        </div>
    }
}

/// Viewer-mode language banner. Appears once at the top of the workspace
/// when the loaded GLDF carries more than one locale. The user picks a
/// display language; the choice is remembered in `localStorage` and
/// applied on subsequent loads (when the same locale is available).
///
/// Auto-hides when the document has 0 or 1 locales — there's nothing to
/// switch.
#[function_component(LanguageBanner)]
pub fn language_banner() -> Html {
    let gldf = use_gldf();
    let langs = collect_languages(&gldf.product);

    if langs.len() <= 1 {
        return html! {};
    }

    let active = gldf.active_language.clone();
    let dismissed = use_state(|| false);

    // First render: reconcile active_language against localStorage and the
    // languages actually present in this document. We do this once in an
    // effect so it only fires when `langs` changes (i.e. on file load).
    {
        let gldf = gldf.clone();
        let langs = langs.clone();
        let active = active.clone();
        use_effect_with(langs.clone(), move |_| {
            let want = preferred_language(&active, &langs);
            if want != active {
                gldf.dispatch(GldfAction::SetActiveLanguage(want));
            }
            || ()
        });
    }

    if *dismissed {
        return html! {};
    }

    let on_select = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let lang = select.value();
            store_preferred_language(&lang);
            gldf.dispatch(GldfAction::SetActiveLanguage(lang));
        })
    };

    let on_dismiss = {
        let dismissed = dismissed.clone();
        Callback::from(move |_| dismissed.set(true))
    };

    html! {
        <div class="language-banner">
            <span class="language-banner-icon">{ "🌐" }</span>
            <span class="language-banner-label">
                { format!("This file has {} languages. Display in:", langs.len()) }
            </span>
            <select class="language-banner-select" onchange={on_select} value={active.clone()}>
                { for langs.iter().map(|l| {
                    let is_selected = l == &active;
                    html! { <option value={l.clone()} selected={is_selected}>{ l }</option> }
                }) }
            </select>
            <button type="button" class="language-banner-dismiss" onclick={on_dismiss} title="Dismiss">
                { "✕" }
            </button>
        </div>
    }
}

/// UI translation language picker. Always visible (small bar above the
/// content body) so users can immediately verify the effect of switching
/// — even on pages where labels haven't been translated yet, the value
/// columns (e.g. LightDistribution) react to it.
///
/// Independent of [`LanguageBanner`]: that one switches the document's
/// own locales (driven by the file's `LocaleFoo` content), this one
/// switches the viewer's translation tables (driven by what the viewer
/// itself ships).
#[function_component(UiLanguagePicker)]
pub fn ui_language_picker() -> Html {
    let gldf = use_gldf();
    let active = gldf.ui_language.clone();

    let on_select = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            gldf.dispatch(GldfAction::SetUiLanguage(select.value()));
        })
    };

    html! {
        <div class="ui-language-bar" title="Viewer translation language (XSD enum labels). Independent of the document's own locales.">
            <span class="ui-language-bar-icon">{ "🖥" }</span>
            <span class="ui-language-bar-label">{ "UI:" }</span>
            <select class="ui-language-bar-select" onchange={on_select} value={active.clone()}>
                { for crate::state::SUPPORTED_UI_LANGUAGES.iter().map(|l| {
                    let is_selected = *l == active;
                    html! { <option value={(*l).to_string()} selected={is_selected}>{ *l }</option> }
                }) }
            </select>
        </div>
    }
}

const STORAGE_KEY: &str = "gldf.display_language";

/// Resolve the language to display for the freshly-loaded document.
///
/// Order of preference:
/// 1. The current `active_language` if it's already valid for this doc.
/// 2. The user's last choice from `localStorage`, if available here.
/// 3. `"en"` if available.
/// 4. The first locale in the document.
fn preferred_language(current: &str, available: &[String]) -> String {
    if available.iter().any(|l| l == current) && !current.is_empty() {
        return current.to_string();
    }
    if let Some(stored) = load_preferred_language() {
        if available.iter().any(|l| l == &stored) {
            return stored;
        }
    }
    if available.iter().any(|l| l == "en") {
        return "en".to_string();
    }
    available
        .first()
        .cloned()
        .unwrap_or_else(|| "en".to_string())
}

fn load_preferred_language() -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()
        .flatten()?
        .get_item(STORAGE_KEY)
        .ok()
        .flatten()
}

fn store_preferred_language(lang: &str) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(STORAGE_KEY, lang);
        }
    }
}
