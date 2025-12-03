//! Editor tabs component for navigation between GLDF sections

use yew::prelude::*;
use super::{HeaderEditor, FilesEditor, LightSourceEditor, VariantEditor};

/// Available editor sections
#[derive(Clone, PartialEq)]
pub enum EditorSection {
    Header,
    Files,
    LightSources,
    Variants,
}

impl EditorSection {
    fn label(&self) -> &'static str {
        match self {
            EditorSection::Header => "Header",
            EditorSection::Files => "Files",
            EditorSection::LightSources => "Light Sources",
            EditorSection::Variants => "Variants",
        }
    }
}

/// Editor tabs component
#[function_component(EditorTabs)]
pub fn editor_tabs() -> Html {
    let active_section = use_state(|| EditorSection::Header);

    let sections = vec![
        EditorSection::Header,
        EditorSection::Files,
        EditorSection::LightSources,
        EditorSection::Variants,
    ];

    html! {
        <div class="editor-tabs-container">
            <nav class="editor-nav">
                { for sections.iter().map(|section| {
                    let is_active = *active_section == *section;
                    let section_clone = section.clone();
                    let active_section = active_section.clone();

                    html! {
                        <button
                            class={classes!("nav-tab", is_active.then(|| "active"))}
                            onclick={Callback::from(move |_| {
                                active_section.set(section_clone.clone());
                            })}
                        >
                            { section.label() }
                        </button>
                    }
                })}
            </nav>

            <div class="editor-content">
                {
                    match &*active_section {
                        EditorSection::Header => html! { <HeaderEditor /> },
                        EditorSection::Files => html! { <FilesEditor /> },
                        EditorSection::LightSources => html! { <LightSourceEditor /> },
                        EditorSection::Variants => html! { <VariantEditor /> },
                    }
                }
            </div>
        </div>
    }
}
