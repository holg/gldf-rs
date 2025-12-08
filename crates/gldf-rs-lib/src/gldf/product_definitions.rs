#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

use super::*;
use serde::{Deserialize, Serialize};

/// Represents the product definitions section of a GLDF file.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductDefinitions {
    /// Product metadata information.
    #[serde(rename = "ProductMetaData", skip_serializing_if = "Option::is_none")]
    pub product_meta_data: Option<ProductMetaData>,

    /// A collection of product variants.
    #[serde(rename = "Variants", skip_serializing_if = "Option::is_none")]
    pub variants: Option<Variants>,
}

/// Represents the maintenance factor of a luminaire.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LuminaireMaintenanceFactor {
    /// The number of years for which the maintenance factor is specified.
    #[serde(rename = "@years", default, skip_serializing_if = "Option::is_none")]
    pub years: Option<i32>,

    /// The room condition under which the maintenance factor is specified.
    #[serde(
        rename = "@roomCondition",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub room_condition: Option<String>,

    /// The value maintenance factor.
    #[serde(rename = "$text", default)]
    pub value: String,
}

/// Represents CIE-specific luminaire maintenance factors.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CieLuminaireMaintenanceFactors {
    /// Luminaire maintenance factors defined according to CIE standards.
    #[serde(rename = "LuminaireMaintenanceFactor", default)]
    pub luminaire_maintenance_factor: Vec<LuminaireMaintenanceFactor>,
}

/// Represents the dirt depreciation factor of a luminaire.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LuminaireDirtDepreciation {
    /// The number of years for which the dirt depreciation factor is specified.
    #[serde(rename = "@years", default)]
    pub years: i32,

    /// The room condition under which the dirt depreciation factor is specified.
    #[serde(rename = "@roomCondition", default)]
    pub room_condition: String,

    /// The value dirt depreciation factor.
    #[serde(rename = "$text", default)]
    pub value: f64,
}

/// Represents IES-specific luminaire light loss factors.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IesLuminaireLightLossFactors {
    /// Luminaire dirt depreciation factors defined according to IES standards.
    #[serde(rename = "LuminaireDirtDepreciation", default)]
    pub luminaire_dirt_depreciation: Vec<LuminaireDirtDepreciation>,
}

/// Represents JIEG-specific luminaire maintenance factors.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JiegMaintenanceFactors {
    /// Luminaire maintenance factors defined according to JIEG standards.
    #[serde(rename = "LuminaireMaintenanceFactor", default)]
    pub luminaire_maintenance_factor: Vec<LuminaireMaintenanceFactor>,
}

/// Represents maintenance-related information for a luminaire.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LuminaireMaintenance {
    /// The CIE97 luminaire type classification.
    #[serde(
        rename = "Cie97LuminaireType",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub cie97_luminaire_type: Option<String>,

    /// CIE-specific luminaire maintenance factors.
    #[serde(
        rename = "CieLuminaireMaintenanceFactors",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub cie_luminaire_maintenance_factors: Option<CieLuminaireMaintenanceFactors>,

    /// IES-specific luminaire light loss factors, if available.
    #[serde(
        rename = "IesLuminaireLightLossFactors",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ies_luminaire_light_loss_factors: Option<IesLuminaireLightLossFactors>,

    /// JIEG-specific luminaire maintenance factors, if available.
    #[serde(
        rename = "JiegMaintenanceFactors",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub jieg_maintenance_factors: Option<JiegMaintenanceFactors>,
}

/// Represents metadata for a luminaire product.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductMetaData {
    /// The product number, if available.
    #[serde(rename = "ProductNumber", skip_serializing_if = "Option::is_none")]
    pub product_number: Option<LocaleFoo>,

    /// The name of the product, if available.
    #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
    pub name: Option<LocaleFoo>,

    /// The description of the product, if available.
    #[serde(rename = "Description", skip_serializing_if = "Option::is_none")]
    pub description: Option<LocaleFoo>,

    /// The tender text for the product.
    #[serde(rename = "TenderText", skip_serializing_if = "Option::is_none")]
    pub tender_text: Option<LocaleFoo>,

    /// The product series to which the product belongs, if available.
    #[serde(rename = "ProductSeries", skip_serializing_if = "Option::is_none")]
    pub product_series: Option<ProductSeries>,

    /// Pictures of the product, if available.
    #[serde(rename = "Pictures", skip_serializing_if = "Option::is_none")]
    pub pictures: Option<Images>,

    /// Luminaire maintenance information for the product, if available.
    #[serde(
        rename = "LuminaireMaintenance",
        skip_serializing_if = "Option::is_none"
    )]
    pub luminaire_maintenance: Option<LuminaireMaintenance>,

    /// Descriptive attributes of the product, if available.
    #[serde(
        rename = "DescriptiveAttributes",
        skip_serializing_if = "Option::is_none"
    )]
    pub descriptive_attributes: Option<DescriptiveAttributes>,
}

/// Represents a rectangular cutout.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RectangularCutout {
    /// The width of the rectangular cutout.
    #[serde(rename = "@Width", default)]
    pub width: i32,

    /// The length of the rectangular cutout.
    #[serde(rename = "@Length", default)]
    pub length: i32,

    /// The depth of the rectangular cutout.
    #[serde(rename = "@Depth", default)]
    pub depth: i32,
}

/// Represents a circular cutout.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircularCutout {
    /// The diameter of the circular cutout.
    #[serde(rename = "@Diameter", default)]
    pub diameter: i32,

    /// The depth of the circular cutout.
    #[serde(rename = "@Depth", default)]
    pub depth: i32,
}

/// Represents a recessed luminaire.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recessed {
    /// The recessed depth in mm.
    #[serde(rename = "@recessedDepth", default)]
    pub recessed_depth: i32,

    /// The rectangular cutout details for the recessed luminaire.
    #[serde(
        rename = "RectangularCutout",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub rectangular_cutout: Option<RectangularCutout>,

    /// The circular cutout details for the recessed luminaire.
    #[serde(
        rename = "CircularCutout",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub circular_cutout: Option<CircularCutout>,

    /// The overall depth of the recessed luminaire in mm.
    #[serde(rename = "@Depth", default)]
    pub depth: i32,
}

/// Represents a surface-mounted luminaire.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceMounted {}

/// The `Pendant` struct describes pendant-related properties.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pendant {
    /// The length of the pendant.
    #[serde(rename = "pendantLength", default)]
    pub pendant_length: f64,
}

/// Represents a luminaire with ceiling mount.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ceiling {
    /// The recessed type
    #[serde(rename = "Recessed", skip_serializing_if = "Option::is_none")]
    pub recessed: Option<Recessed>,
    /// The surface mounted type
    #[serde(rename = "SurfaceMounted", skip_serializing_if = "Option::is_none")]
    pub surface_mounted: Option<SurfaceMounted>,
    /// The pendant type
    #[serde(rename = "Pendant", skip_serializing_if = "Option::is_none")]
    pub pendant: Option<Pendant>,
}

/// Luminaire on the Wall
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Wall {
    /// The mounting height in mm
    #[serde(rename = "@mountingHeight", default)]
    pub mounting_height: i32,
    /// The recessed type
    #[serde(rename = "Recessed", default, skip_serializing_if = "Option::is_none")]
    pub recessed: Option<Recessed>,
    /// The surface mounted type
    #[serde(
        rename = "SurfaceMounted",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub surface_mounted: Option<SurfaceMounted>,
    /// The depth in mm
    #[serde(rename = "Depth", default)]
    pub depth: i32,
}

/// FreeStanding Luminaire
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FreeStanding {}

/// WorkingPlane
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkingPlane {
    /// The free standing type
    #[serde(rename = "FreeStanding", skip_serializing_if = "Option::is_none")]
    pub free_standing: Option<FreeStanding>,
}

/// PoleTop
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoleTop {
    /// The pole height in mm
    #[serde(rename = "poleHeight")]
    pub pole_height: i32,
}

/// PoleIntegrated
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoleIntegrated {
    /// The pole height in mm
    #[serde(rename = "poleHeight")]
    pub pole_height: i32,
}

/// Ground
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ground {
    /// The pole top type
    #[serde(rename = "PoleTop", skip_serializing_if = "Option::is_none")]
    pub pole_top: Option<PoleTop>,
    /// The pole integrated type
    #[serde(rename = "PoleIntegrated", skip_serializing_if = "Option::is_none")]
    pub pole_integrated: Option<PoleIntegrated>,
    /// The free standing type
    #[serde(rename = "FreeStanding", skip_serializing_if = "Option::is_none")]
    pub free_standing: Option<FreeStanding>,
    /// The surface mounted type
    #[serde(rename = "SurfaceMounted", skip_serializing_if = "Option::is_none")]
    pub surface_mounted: Option<SurfaceMounted>,
    /// The recessed type
    #[serde(rename = "Recessed", skip_serializing_if = "Option::is_none")]
    pub recessed: Option<Recessed>,
}

/// Mountings
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mountings {
    /// The ceiling type
    #[serde(rename = "Ceiling", skip_serializing_if = "Option::is_none")]
    pub ceiling: Option<Ceiling>,
    /// The wall type
    #[serde(rename = "Wall", skip_serializing_if = "Option::is_none")]
    pub wall: Option<Wall>,
    /// The working plane type
    #[serde(rename = "WorkingPlane", skip_serializing_if = "Option::is_none")]
    pub working_plane: Option<WorkingPlane>,
    /// The ground type
    #[serde(rename = "Ground", skip_serializing_if = "Option::is_none")]
    pub ground: Option<Ground>,
}

/// EmitterReference
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmitterReference {
    /// The emitter id
    #[serde(rename = "@emitterId")]
    pub emitter_id: String,
    /// The external name of the emitter
    #[serde(rename = "EmitterObjectExternalName")]
    pub emitter_object_external_name: String,
}

/// SimpleGeometryReference
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimpleGeometryReference {
    /// The geometry id
    #[serde(rename = "@geometryId")]
    pub geometry_id: String,
    /// The emitter id
    #[serde(rename = "@emitterId")]
    pub emitter_id: String,
}

/// ModelGeometryReference
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelGeometryReference {
    /// The geometry id
    #[serde(rename = "@geometryId")]
    pub geometry_id: String,
    /// The list of emitter references
    #[serde(rename = "EmitterReference", default)]
    pub emitter_reference: Vec<EmitterReference>,
}

/// GeometryReferences
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometryReferences {
    /// The simple geometry reference
    #[serde(
        rename = "SimpleGeometryReference",
        skip_serializing_if = "Option::is_none"
    )]
    pub simple_geometry_reference: Option<SimpleGeometryReference>,
    /// The model geometry reference
    #[serde(
        rename = "ModelGeometryReference",
        skip_serializing_if = "Option::is_none"
    )]
    pub model_geometry_reference: Option<ModelGeometryReference>,
}

/// Geometry
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Geometry {
    /// The simple geometry reference
    #[serde(
        rename = "SimpleGeometryReference",
        skip_serializing_if = "Option::is_none"
    )]
    pub simple_geometry_reference: Option<SimpleGeometryReference>,
    /// The model geometry reference
    #[serde(
        rename = "ModelGeometryReference",
        skip_serializing_if = "Option::is_none"
    )]
    pub model_geometry_reference: Option<ModelGeometryReference>,
}

/// Symbol
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Symbol {
    /// The file id of the symbol
    #[serde(rename = "@fileId")]
    pub file_id: String,
}

/// Variant
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    /// The id of the variant
    #[serde(rename = "@id")]
    pub id: String,
    /// The sort order of the variant
    #[serde(rename = "sortOrder", skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    /// The product number of the variant as Locale string
    #[serde(rename = "ProductNumber", skip_serializing_if = "Option::is_none")]
    pub product_number: Option<LocaleFoo>,
    /// The name of the variant as Locale string
    #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
    pub name: Option<LocaleFoo>,
    /// The description of the variant as Locale string
    #[serde(rename = "Description", skip_serializing_if = "Option::is_none")]
    pub description: Option<LocaleFoo>,
    /// The tender text of the variant as Locale string
    #[serde(rename = "TenderText", skip_serializing_if = "Option::is_none")]
    pub tender_text: Option<LocaleFoo>,
    /// The gtin of the variant
    #[serde(rename = "GTIN", skip_serializing_if = "Option::is_none")]
    pub gtin: Option<String>,
    /// The mountings of the variant
    #[serde(rename = "Mountings", skip_serializing_if = "Option::is_none")]
    pub mountings: Option<Mountings>,
    /// The geometry of the variant
    #[serde(rename = "Geometry", skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Geometry>,
    /// The product series of the variant
    #[serde(rename = "ProductSeries", skip_serializing_if = "Option::is_none")]
    pub product_series: Option<ProductSeries>,
    /// The images of the variant
    #[serde(rename = "Pictures", skip_serializing_if = "Option::is_none")]
    pub pictures: Option<Images>,
    /// The symbol of the variant
    #[serde(rename = "Symbol", skip_serializing_if = "Option::is_none")]
    pub symbol: Option<Symbol>,
    /// The descriptive attributes of the variant
    #[serde(
        rename = "DescriptiveAttributes",
        skip_serializing_if = "Option::is_none"
    )]
    pub descriptive_attributes: Option<DescriptiveAttributes>,
}

/// Variants
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variants {
    /// The variant
    #[serde(rename = "Variant", default)]
    pub variant: Vec<Variant>,
}

/// ProductSize
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductSize {
    /// The length of the product in mm
    #[serde(rename = "Length")]
    pub length: i32,
    /// The width of the product in mm
    #[serde(rename = "Width")]
    pub width: i32,
    /// The height of the product in mm
    #[serde(rename = "Height")]
    pub height: i32,
}

/// Adjustabilities
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Adjustabilities {
    /// The adjustability as a list of strings
    #[serde(rename = "Adjustability", default)]
    pub adjustability: Vec<String>,
}

/// ProtectiveAreas
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtectiveAreas {
    /// The protective areas as a list of strings
    #[serde(rename = "Area", default)]
    pub area: Vec<String>,
}

/// Mechanical attributes
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mechanical {
    /// The product size
    #[serde(rename = "ProductSize", skip_serializing_if = "Option::is_none")]
    pub product_size: Option<ProductSize>,
    /// The product form
    #[serde(rename = "ProductForm", skip_serializing_if = "Option::is_none")]
    pub product_form: Option<String>,
    /// The sealing material as a Locale string
    #[serde(rename = "SealingMaterial", skip_serializing_if = "Option::is_none")]
    pub sealing_material: Option<LocaleFoo>,
    /// The adjustabilities
    #[serde(rename = "Adjustabilities", skip_serializing_if = "Option::is_none")]
    pub adjustabilities: Option<Adjustabilities>,
    /// The ik rating
    #[serde(rename = "IKRating", skip_serializing_if = "Option::is_none")]
    pub ik_rating: Option<String>,
    /// The protective areas
    #[serde(rename = "ProtectiveAreas", skip_serializing_if = "Option::is_none")]
    pub protective_areas: Option<ProtectiveAreas>,
    /// The weight in kg
    #[serde(rename = "Weight", skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
}

/// ClampingRange
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClampingRange {
    /// The lower value of the clamping range
    #[serde(rename = "Lower")]
    pub lower: f64,
    /// The upper value of the clamping range
    #[serde(rename = "Upper")]
    pub upper: f64,
}

/// Electrical
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Electrical {
    /// The clamping range
    #[serde(rename = "ClampingRange", skip_serializing_if = "Option::is_none")]
    pub clamping_range: Option<ClampingRange>,
    /// The switching capacity
    #[serde(rename = "SwitchingCapacity", skip_serializing_if = "Option::is_none")]
    pub switching_capacity: Option<String>,
    /// The electrical safety class
    #[serde(
        rename = "ElectricalSafetyClass",
        skip_serializing_if = "Option::is_none"
    )]
    pub electrical_safety_class: Option<String>,
    /// The ingress protection ip code
    #[serde(
        rename = "IngressProtectionIPCode",
        skip_serializing_if = "Option::is_none"
    )]
    pub ingress_protection_ip_code: Option<String>,
    /// The power factor
    #[serde(rename = "PowerFactor", skip_serializing_if = "Option::is_none")]
    pub power_factor: Option<f64>,
    /// Bool constant light output
    #[serde(
        rename = "ConstantLightOutput",
        skip_serializing_if = "Option::is_none"
    )]
    pub constant_light_output: Option<bool>,
    /// The light distribution
    #[serde(rename = "LightDistribution", skip_serializing_if = "Option::is_none")]
    pub light_distribution: Option<String>,
}

/// Flux
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flux {
    /// The hours of the flux
    #[serde(rename = "@hours")]
    pub hours: i32,
    /// The value of the flux
    #[serde(rename = "$text")]
    pub value: i32,
}

/// DurationTimeAndFlux
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DurationTimeAndFlux {
    /// The list of flux
    #[serde(rename = "Flux", default)]
    pub flux: Vec<Flux>,
}

/// Emergency
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Emergency {
    /// The duration time and flux
    #[serde(
        rename = "DurationTimeAndFlux",
        skip_serializing_if = "Option::is_none"
    )]
    pub duration_time_and_flux: Option<DurationTimeAndFlux>,
    /// The dedicated emergency lighting type
    #[serde(
        rename = "DedicatedEmergencyLightingType",
        skip_serializing_if = "Option::is_none"
    )]
    pub dedicated_emergency_lighting_type: Option<String>,
}

/// ListPrice
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListPrice {
    /// The currency of the list price
    #[serde(rename = "@currency", skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// The value of the list price
    #[serde(rename = "$text")]
    pub value: f64,
}

/// ListPrices
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListPrices {
    /// The list of prices
    #[serde(rename = "ListPrice", default)]
    pub list_price: Vec<ListPrice>,
}

/// HousingColor
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HousingColor {
    /// The ral of the housing color
    #[serde(rename = "@ral", skip_serializing_if = "Option::is_none")]
    pub ral: Option<i32>,
    /// The locale of the housing color
    #[serde(flatten)]
    pub locale: Locale,
}

/// HousingColors
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HousingColors {
    /// The list of housing colors
    #[serde(rename = "HousingColor", default)]
    pub housing_color: Vec<HousingColor>,
}

/// Markets
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Markets {
    /// The list of market regions
    #[serde(rename = "Region", default)]
    pub region: Vec<Locale>,
}

/// ApprovalMarks
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalMarks {
    /// The list of approval marks
    #[serde(rename = "ApprovalMark", default)]
    pub approval_mark: Vec<String>,
}

/// DesignAwards
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignAwards {
    /// The list of design awards
    #[serde(rename = "DesignAward", default)]
    pub design_award: Vec<String>,
}

/// Labels
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Labels {
    /// The list of labels
    #[serde(rename = "Label", default)]
    pub label: Vec<String>,
}

/// Applications
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Applications {
    /// The list of applications
    #[serde(rename = "Application", default)]
    pub application: Vec<String>,
}

/// Marketing
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Marketing {
    /// The list prices
    #[serde(rename = "ListPrices", skip_serializing_if = "Option::is_none")]
    pub list_prices: Option<ListPrices>,
    /// The housing colors
    #[serde(rename = "HousingColors", skip_serializing_if = "Option::is_none")]
    pub housing_colors: Option<HousingColors>,
    /// The markets
    #[serde(rename = "Markets", skip_serializing_if = "Option::is_none")]
    pub markets: Option<Markets>,
    /// The hyperlinks
    #[serde(rename = "Hyperlinks", skip_serializing_if = "Option::is_none")]
    pub hyperlinks: Option<Hyperlinks>,
    /// The designer
    #[serde(rename = "Designer", skip_serializing_if = "Option::is_none")]
    pub designer: Option<String>,
    /// The approval marks
    #[serde(rename = "ApprovalMarks", skip_serializing_if = "Option::is_none")]
    pub approval_marks: Option<ApprovalMarks>,
    /// The design awards
    #[serde(rename = "DesignAwards", skip_serializing_if = "Option::is_none")]
    pub design_awards: Option<DesignAwards>,
    /// The labels
    #[serde(rename = "Labels", skip_serializing_if = "Option::is_none")]
    pub labels: Option<Labels>,
    /// The applications
    #[serde(rename = "Applications", skip_serializing_if = "Option::is_none")]
    pub applications: Option<Applications>,
}

/// UsefulLifeTimes
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsefulLifeTimes {
    /// The list of useful life times
    #[serde(rename = "UsefulLife", default)]
    pub useful_life: Vec<String>,
}

/// MedianUsefulLifeTimes
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MedianUsefulLifeTimes {
    /// The list of median useful life times
    #[serde(rename = "MedianUsefulLife", default)]
    pub median_useful_life: Vec<String>,
}

/// Directives for ATEX
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Directives {
    /// The list of directive names.
    #[serde(rename = "Directive", default)]
    pub directive: Vec<String>,
}

/// Classes for ATEX
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Classes {
    /// The list of classification class names.
    #[serde(rename = "Class", default)]
    pub class: Vec<String>,
}

/// Divisions for ATEX
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Divisions {
    /// The list of division names.
    #[serde(rename = "Division", default)]
    pub division: Vec<String>,
}

/// Gas groups
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gas {
    /// The list of gas group names.
    #[serde(rename = "Group", default)]
    pub group: Vec<String>,
}

/// Dust groups
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dust {
    /// The list of dust group names.
    #[serde(rename = "Group", default)]
    pub group: Vec<String>,
}

/// DivisionGroups
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DivisionGroups {
    /// The gas classification group.
    #[serde(rename = "Gas")]
    pub gas: Gas,
    /// The dust classification group.
    #[serde(rename = "Dust")]
    pub dust: Dust,
}

/// Zones
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zones {
    /// The gas protection zone.
    #[serde(rename = "Gas")]
    pub gas: Gas,
    /// The dust protection zone.
    #[serde(rename = "Dust")]
    pub dust: Dust,
}

/// ZoneGroups
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoneGroups {
    /// The gas protection zone group.
    #[serde(rename = "Gas")]
    pub gas: Gas,
    /// The dust protection zone group.
    #[serde(rename = "Dust")]
    pub dust: Dust,
}

/// TemperatureClasses
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemperatureClasses {
    /// The list of temperature classes
    #[serde(rename = "TemperatureClass", default)]
    pub temperature_class: Vec<String>,
}

/// ExCodes
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExCodes {
    /// The list of explosion protection (Ex) codes.
    #[serde(rename = "ExCode", default)]
    pub ex_code: Vec<String>,
}

/// EquipmentProtectionLevels
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentProtectionLevels {
    /// The list of equipment protection levels.
    #[serde(rename = "EquipmentProtectionLevel", default)]
    pub equipment_protection_level: Vec<String>,
}

/// EquipmentGroups
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentGroups {
    /// The list of equipment groups.
    #[serde(rename = "EquipmentGroup", default)]
    pub equipment_group: Vec<String>,
}

/// EquipmentCategories
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentCategories {
    /// The list of equipment categories.
    #[serde(rename = "EquipmentCategory", default)]
    pub equipment_category: Vec<String>,
}

/// Atmospheres
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Atmospheres {
    /// The list of atmospheres.
    #[serde(rename = "Atmosphere", default)]
    pub atmosphere: Vec<String>,
}

/// Groups
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Groups {
    /// The list of groups.
    #[serde(rename = "Group", default)]
    pub group: Vec<String>,
}

/// ATEX classification
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ATEX {
    /// The ATEX directives information.
    #[serde(rename = "Directives")]
    pub directives: Directives,
    /// The ATEX classes information.
    #[serde(rename = "Classes")]
    pub classes: Classes,
    /// The ATEX divisions information.
    #[serde(rename = "Divisions")]
    pub divisions: Divisions,
    /// The ATEX division groups information.
    #[serde(rename = "DivisionGroups")]
    pub division_groups: DivisionGroups,
    /// The ATEX zones information.
    #[serde(rename = "Zones")]
    pub zones: Zones,
    /// The ATEX zone groups information.
    #[serde(rename = "ZoneGroups")]
    pub zone_groups: ZoneGroups,
    /// The maximum surface temperature allowed by ATEX.
    #[serde(rename = "MaximumSurfaceTemperature")]
    pub maximum_surface_temperature: String,
    /// The ATEX groups information.
    #[serde(rename = "Groups")]
    pub groups: Groups,
}

/// AbsorptionRate
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbsorptionRate {
    /// The rate of absorption in hertz (Hz).
    #[serde(rename = "@hertz")]
    pub hertz: i32,
    /// The corresponding value of the absorption rate.
    #[serde(rename = "$text")]
    pub value: f64,
}

/// AcousticAbsorptionRates
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcousticAbsorptionRates {
    /// The collection of absorption rates at different frequencies.
    #[serde(rename = "AbsorptionRate", default)]
    pub absorption_rate: Vec<AbsorptionRate>,
}

/// TemperatureRange
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemperatureRange {
    /// The lower value of the temperature range
    #[serde(rename = "Lower")]
    pub lower: i32,
    /// The upper value of the temperature range
    #[serde(rename = "Upper")]
    pub upper: i32,
}

/// OperationsAndMaintenance
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationsAndMaintenance {
    /// The list of useful life times for the luminaire.
    #[serde(rename = "UsefulLifeTimes", skip_serializing_if = "Option::is_none")]
    pub useful_life_times: Option<UsefulLifeTimes>,
    /// The list of median useful life times for the luminaire.
    #[serde(
        rename = "MedianUsefulLifeTimes",
        skip_serializing_if = "Option::is_none"
    )]
    pub median_useful_life_times: Option<MedianUsefulLifeTimes>,
    /// The operating temperature range for the luminaire.
    #[serde(
        rename = "OperatingTemperature",
        skip_serializing_if = "Option::is_none"
    )]
    pub operating_temperature: Option<TemperatureRange>,
    /// The ambient temperature range for the luminaire.
    #[serde(rename = "AmbientTemperature", skip_serializing_if = "Option::is_none")]
    pub ambient_temperature: Option<TemperatureRange>,
    /// The rated ambient temperature for the luminaire.
    #[serde(
        rename = "RatedAmbientTemperature",
        skip_serializing_if = "Option::is_none"
    )]
    pub rated_ambient_temperature: Option<i32>,
    /// The ATEX classification details for the luminaire.
    #[serde(rename = "ATEX", skip_serializing_if = "Option::is_none")]
    pub atex: Option<ATEX>,
    /// The list of acoustic absorption rates for the luminaire.
    #[serde(
        rename = "AcousticAbsorptionRates",
        skip_serializing_if = "Option::is_none"
    )]
    pub acoustic_absorption_rates: Option<AcousticAbsorptionRates>,
}

/// FileReference
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileReference {
    /// The file ID for the file reference.
    #[serde(rename = "@fileId")]
    pub file_id: String,
}

/// Property
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    /// The ID of the property.
    #[serde(rename = "@id", default)]
    pub id: String,
    /// The locale information for the property.
    #[serde(rename = "Name", default)]
    pub name: Locale,
    /// The source of the property.
    #[serde(rename = "PropertySource", default)]
    pub property_source: String,
    /// The value of the property.
    #[serde(rename = "Value", default)]
    pub value: String,
    /// The file reference for the property, if applicable.
    #[serde(
        rename = "FileReference",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub file_reference: Option<FileReference>,
}

/// CustomProperties
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomProperties {
    /// The list of custom properties for the luminaire.
    #[serde(rename = "Property", default)]
    pub property: Vec<Property>,
}

/// DescriptiveAttributes
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DescriptiveAttributes {
    /// The mechanical attributes for the luminaire.
    #[serde(rename = "Mechanical", skip_serializing_if = "Option::is_none")]
    pub mechanical: Option<Mechanical>,
    /// The electrical attributes for the luminaire.
    #[serde(rename = "Electrical", skip_serializing_if = "Option::is_none")]
    pub electrical: Option<Electrical>,
    /// The emergency attributes for the luminaire.
    #[serde(rename = "Emergency", skip_serializing_if = "Option::is_none")]
    pub emergency: Option<Emergency>,
    /// The marketing attributes for the luminaire.
    #[serde(rename = "Marketing", skip_serializing_if = "Option::is_none")]
    pub marketing: Option<Marketing>,
    /// The operations and maintenance attributes for the luminaire.
    #[serde(
        rename = "OperationsAndMaintenance",
        skip_serializing_if = "Option::is_none"
    )]
    pub operations_and_maintenance: Option<OperationsAndMaintenance>,
    /// The custom properties for the luminaire, if applicable.
    #[serde(rename = "CustomProperties", skip_serializing_if = "Option::is_none")]
    pub custom_properties: Option<CustomProperties>,
}
