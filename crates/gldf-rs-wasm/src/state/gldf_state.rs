//! GLDF state management for the WASM editor

use gldf_rs::gldf::GldfProduct;
use std::rc::Rc;
use yew::prelude::*;

/// Actions that can be performed on the GLDF state
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum GldfAction {
    /// Load a new GLDF product
    Load(GldfProduct),
    /// Update the header author
    SetAuthor(String),
    /// Update the header manufacturer
    SetManufacturer(String),
    /// Update the creation time code
    SetCreationTimeCode(String),
    /// Update the created with application
    SetCreatedWithApplication(String),
    /// Update the default language
    SetDefaultLanguage(Option<String>),
    /// Update format version
    SetFormatVersion {
        major: i32,
        minor: i32,
        pre_release: i32,
    },
    /// Add a file to the files collection
    AddFile {
        id: String,
        content_type: String,
        type_attr: String,
        file_name: String,
        language: Option<String>,
    },
    /// Remove a file by id
    RemoveFile(String),
    /// Update a file's properties
    UpdateFile {
        id: String,
        content_type: String,
        type_attr: String,
        file_name: String,
    },
    /// Reset state to default
    #[allow(dead_code)]
    Reset,
}

/// State of the GLDF editor
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GldfState {
    /// The current GLDF product being edited
    pub product: GldfProduct,
    /// Whether the product has been modified
    pub is_modified: bool,
}

impl Reducible for GldfState {
    type Action = GldfAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut new_state = (*self).clone();
        new_state.is_modified = true;

        match action {
            GldfAction::Load(product) => {
                new_state.product = product;
                new_state.is_modified = false;
            }
            GldfAction::SetAuthor(author) => {
                new_state.product.header.author = author;
            }
            GldfAction::SetManufacturer(manufacturer) => {
                new_state.product.header.manufacturer = manufacturer;
            }
            GldfAction::SetCreationTimeCode(time_code) => {
                new_state.product.header.creation_time_code = time_code;
            }
            GldfAction::SetCreatedWithApplication(app) => {
                new_state.product.header.created_with_application = app;
            }
            GldfAction::SetDefaultLanguage(lang) => {
                new_state.product.header.default_language = lang;
            }
            GldfAction::SetFormatVersion {
                major,
                minor,
                pre_release,
            } => {
                new_state.product.header.format_version.major = major;
                new_state.product.header.format_version.minor = minor;
                new_state.product.header.format_version.pre_release = pre_release;
            }
            GldfAction::AddFile {
                id,
                content_type,
                type_attr,
                file_name,
                language,
            } => {
                use gldf_rs::gldf::general_definitions::files::File;
                new_state.product.general_definitions.files.file.push(File {
                    id,
                    content_type,
                    type_attr,
                    file_name,
                    language: language.unwrap_or_default(),
                });
            }
            GldfAction::RemoveFile(id) => {
                new_state
                    .product
                    .general_definitions
                    .files
                    .file
                    .retain(|f| f.id != id);
            }
            GldfAction::UpdateFile {
                id,
                content_type,
                type_attr,
                file_name,
            } => {
                if let Some(file) = new_state
                    .product
                    .general_definitions
                    .files
                    .file
                    .iter_mut()
                    .find(|f| f.id == id)
                {
                    file.content_type = content_type;
                    file.type_attr = type_attr;
                    file.file_name = file_name;
                }
            }
            GldfAction::Reset => {
                new_state = GldfState::default();
            }
        }

        Rc::new(new_state)
    }
}

/// Context type for GLDF state
pub type GldfContext = UseReducerHandle<GldfState>;

/// Properties for the GLDF provider component
#[derive(Properties, Clone, PartialEq)]
pub struct GldfProviderProps {
    #[prop_or_default]
    pub children: Children,
}

/// Provider component for GLDF state
#[function_component(GldfProvider)]
pub fn gldf_provider(props: &GldfProviderProps) -> Html {
    let state = use_reducer(GldfState::default);

    html! {
        <ContextProvider<GldfContext> context={state}>
            { for props.children.iter() }
        </ContextProvider<GldfContext>>
    }
}

/// Hook to access the GLDF state
#[hook]
pub fn use_gldf() -> GldfContext {
    use_context::<GldfContext>()
        .expect("GldfContext not found. Did you wrap your component in GldfProvider?")
}
