//! GLDF state management for the WASM editor

use gldf_rs::gldf::{
    GldfProduct,
    general_definitions::photometries::{DescriptivePhotometry, Photometries, Photometry, UGR4H8H705020LQ},
    product_definitions::{
        Applications, DescriptiveAttributes, Electrical, Marketing, ProductMetaData,
    },
};
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
    // --- Electrical attributes ---
    /// Update electrical safety class
    SetElectricalSafetyClass(Option<String>),
    /// Update ingress protection IP code
    SetIngressProtectionIPCode(Option<String>),
    /// Update power factor
    SetPowerFactor(Option<f64>),
    /// Update constant light output
    SetConstantLightOutput(Option<bool>),
    /// Update light distribution
    SetLightDistribution(Option<String>),
    /// Update switching capacity
    SetSwitchingCapacity(Option<String>),
    // --- Applications ---
    /// Set applications list
    SetApplications(Vec<String>),
    /// Add an application
    AddApplication(String),
    /// Remove an application by index
    RemoveApplication(usize),
    // --- Photometry (DescriptivePhotometry) ---
    /// Update photometry by index
    SetPhotometryCieFluxCode { index: usize, value: Option<String> },
    SetPhotometryLightOutputRatio { index: usize, value: Option<f64> },
    SetPhotometryLuminousEfficacy { index: usize, value: Option<f64> },
    SetPhotometryDownwardFluxFraction { index: usize, value: Option<f64> },
    SetPhotometryDownwardLOR { index: usize, value: Option<f64> },
    SetPhotometryUpwardLOR { index: usize, value: Option<f64> },
    SetPhotometryCutOffAngle { index: usize, value: Option<f64> },
    SetPhotometryLuminaireLuminance { index: usize, value: Option<i32> },
    SetPhotometryUgrX { index: usize, value: Option<f64> },
    SetPhotometryUgrY { index: usize, value: Option<f64> },
    SetPhotometryPhotometricCode { index: usize, value: Option<String> },
    SetPhotometryBugRating { index: usize, value: Option<String> },
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
            // --- Electrical attributes ---
            GldfAction::SetElectricalSafetyClass(value) => {
                ensure_electrical(&mut new_state).electrical_safety_class = value;
            }
            GldfAction::SetIngressProtectionIPCode(value) => {
                ensure_electrical(&mut new_state).ingress_protection_ip_code = value;
            }
            GldfAction::SetPowerFactor(value) => {
                ensure_electrical(&mut new_state).power_factor = value;
            }
            GldfAction::SetConstantLightOutput(value) => {
                ensure_electrical(&mut new_state).constant_light_output = value;
            }
            GldfAction::SetLightDistribution(value) => {
                ensure_electrical(&mut new_state).light_distribution = value;
            }
            GldfAction::SetSwitchingCapacity(value) => {
                ensure_electrical(&mut new_state).switching_capacity = value;
            }
            // --- Applications ---
            GldfAction::SetApplications(apps) => {
                ensure_applications(&mut new_state).application = apps;
            }
            GldfAction::AddApplication(app) => {
                ensure_applications(&mut new_state).application.push(app);
            }
            GldfAction::RemoveApplication(index) => {
                let apps = &mut ensure_applications(&mut new_state).application;
                if index < apps.len() {
                    apps.remove(index);
                }
            }
            // --- Photometry ---
            GldfAction::SetPhotometryCieFluxCode { index, value } => {
                ensure_descriptive_photometry(&mut new_state, index).cie_flux_code = value;
            }
            GldfAction::SetPhotometryLightOutputRatio { index, value } => {
                ensure_descriptive_photometry(&mut new_state, index).light_output_ratio = value;
            }
            GldfAction::SetPhotometryLuminousEfficacy { index, value } => {
                ensure_descriptive_photometry(&mut new_state, index).luminous_efficacy = value;
            }
            GldfAction::SetPhotometryDownwardFluxFraction { index, value } => {
                ensure_descriptive_photometry(&mut new_state, index).downward_flux_fraction = value;
            }
            GldfAction::SetPhotometryDownwardLOR { index, value } => {
                ensure_descriptive_photometry(&mut new_state, index).downward_light_output_ratio = value;
            }
            GldfAction::SetPhotometryUpwardLOR { index, value } => {
                ensure_descriptive_photometry(&mut new_state, index).upward_light_output_ratio = value;
            }
            GldfAction::SetPhotometryCutOffAngle { index, value } => {
                ensure_descriptive_photometry(&mut new_state, index).cut_off_angle = value;
            }
            GldfAction::SetPhotometryLuminaireLuminance { index, value } => {
                ensure_descriptive_photometry(&mut new_state, index).luminaire_luminance = value;
            }
            GldfAction::SetPhotometryUgrX { index, value } => {
                ensure_ugr(&mut new_state, index).x = value;
            }
            GldfAction::SetPhotometryUgrY { index, value } => {
                ensure_ugr(&mut new_state, index).y = value;
            }
            GldfAction::SetPhotometryPhotometricCode { index, value } => {
                ensure_descriptive_photometry(&mut new_state, index).photometric_code = value;
            }
            GldfAction::SetPhotometryBugRating { index, value } => {
                ensure_descriptive_photometry(&mut new_state, index).light_distribution_bug_rating = value;
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

/// Helper to ensure ProductMetaData exists and return mutable reference
fn ensure_product_meta_data(state: &mut GldfState) -> &mut ProductMetaData {
    if state.product.product_definitions.product_meta_data.is_none() {
        state.product.product_definitions.product_meta_data = Some(ProductMetaData::default());
    }
    state
        .product
        .product_definitions
        .product_meta_data
        .as_mut()
        .unwrap()
}

/// Helper to ensure DescriptiveAttributes exists and return mutable reference
fn ensure_descriptive_attributes(state: &mut GldfState) -> &mut DescriptiveAttributes {
    let meta = ensure_product_meta_data(state);
    if meta.descriptive_attributes.is_none() {
        meta.descriptive_attributes = Some(DescriptiveAttributes::default());
    }
    meta.descriptive_attributes.as_mut().unwrap()
}

/// Helper to ensure Electrical exists and return mutable reference
fn ensure_electrical(state: &mut GldfState) -> &mut Electrical {
    let attrs = ensure_descriptive_attributes(state);
    if attrs.electrical.is_none() {
        attrs.electrical = Some(Electrical::default());
    }
    attrs.electrical.as_mut().unwrap()
}

/// Helper to ensure Marketing exists and return mutable reference
fn ensure_marketing(state: &mut GldfState) -> &mut Marketing {
    let attrs = ensure_descriptive_attributes(state);
    if attrs.marketing.is_none() {
        attrs.marketing = Some(Marketing::default());
    }
    attrs.marketing.as_mut().unwrap()
}

/// Helper to ensure Applications exists and return mutable reference
fn ensure_applications(state: &mut GldfState) -> &mut Applications {
    let marketing = ensure_marketing(state);
    if marketing.applications.is_none() {
        marketing.applications = Some(Applications::default());
    }
    marketing.applications.as_mut().unwrap()
}

/// Helper to ensure Photometries exists and return mutable reference
fn ensure_photometries(state: &mut GldfState) -> &mut Photometries {
    if state.product.general_definitions.photometries.is_none() {
        state.product.general_definitions.photometries = Some(Photometries::default());
    }
    state.product.general_definitions.photometries.as_mut().unwrap()
}

/// Helper to ensure a specific Photometry exists at index
fn ensure_photometry(state: &mut GldfState, index: usize) -> &mut Photometry {
    let photometries = ensure_photometries(state);
    // Extend the vector if needed
    while photometries.photometry.len() <= index {
        photometries.photometry.push(Photometry {
            id: format!("photometry_{}", photometries.photometry.len()),
            ..Default::default()
        });
    }
    &mut photometries.photometry[index]
}

/// Helper to ensure DescriptivePhotometry exists for a specific Photometry
fn ensure_descriptive_photometry(state: &mut GldfState, index: usize) -> &mut DescriptivePhotometry {
    let photometry = ensure_photometry(state, index);
    if photometry.descriptive_photometry.is_none() {
        photometry.descriptive_photometry = Some(DescriptivePhotometry::default());
    }
    photometry.descriptive_photometry.as_mut().unwrap()
}

/// Helper to ensure UGR exists for a specific Photometry
fn ensure_ugr(state: &mut GldfState, index: usize) -> &mut UGR4H8H705020LQ {
    let desc = ensure_descriptive_photometry(state, index);
    if desc.ugr4_h8_h705020_lq.is_none() {
        desc.ugr4_h8_h705020_lq = Some(UGR4H8H705020LQ::default());
    }
    desc.ugr4_h8_h705020_lq.as_mut().unwrap()
}
