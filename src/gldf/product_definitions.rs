#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
use serde::{Serialize};
use serde::Deserialize;
use yaserde_derive::YaDeserialize;
use yaserde_derive::YaSerialize;
use super::*;

/// Represents the product definitions section of a GLDF file.
///
/// The `ProductDefinitions` struct models the product definitions section of a GLDF (Global Lighting
/// Data Format) file. It includes information about product metadata and variants. It supports
/// serialization and deserialization of XML data for working with product definitions.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductDefinitions {
    /// Product metadata information.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[yaserde(child)]
    #[yaserde(rename = "ProductMetaData")]
    #[serde(rename = "ProductMetaData")]
    pub product_meta_data: Option<ProductMetaData>,

    /// A collection of product variants.
    ///
    /// This field is optional and may not be present in all GLDF files.
    #[yaserde(child)]
    #[yaserde(rename = "Variants")]
    #[serde(rename = "Variants", skip_serializing_if = "Option::is_none")]
    pub variants: Option<Variants>,
}


/// Represents the maintenance factor of a luminaire in the GLDF data structure.
///
/// This struct holds information about the maintenance factor of a luminaire over a certain number of years
/// and under specific room conditions.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LuminaireMaintenanceFactor {
    /// The number of years for which the maintenance factor is specified.
    #[yaserde(attribute, rename = "years")]
    #[serde(rename = "@years", skip_serializing_if = "Option::is_none")]
    pub years: Option<i32>,

    /// The room condition under which the maintenance factor is specified.
    #[yaserde(attribute, rename = "roomCondition")]
    #[serde(rename = "@roomCondition", skip_serializing_if = "Option::is_none")]
    pub room_condition: Option<String>,

    /// The value  maintenance factor.
    #[yaserde(text, rename = "$value")]
    #[serde(rename = "$")]
    pub value: String,
}

/// Represents CIE-specific luminaire maintenance factors in the GLDF data structure.
///
/// This struct holds a collection of luminaire maintenance factors defined according to CIE standards.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CieLuminaireMaintenanceFactors {
    /// Luminaire maintenance factors defined according to CIE standards.
    #[yaserde(rename = "LuminaireMaintenanceFactor")]
    #[serde(rename = "LuminaireMaintenanceFactor")]
    pub luminaire_maintenance_factor: Vec<LuminaireMaintenanceFactor>,
}

/// Represents the dirt depreciation factor of a luminaire in the GLDF data structure.
///
/// This struct holds information about the dirt depreciation factor of a luminaire over a certain number of years
/// and under specific room conditions.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LuminaireDirtDepreciation {
    /// The number of years for which the dirt depreciation factor is specified.
    #[yaserde(rename = "years")]
    #[serde(rename = "years")]
    pub years: i32,

    /// The room condition under which the dirt depreciation factor is specified.
    #[yaserde(rename = "roomCondition")]
    #[serde(rename = "roomCondition")]
    pub room_condition: String,

    /// The value  dirt depreciation factor.
    #[yaserde(rename = "$value")]
    pub value: f64,
}

/// Represents IES-specific luminaire light loss factors in the GLDF data structure.
///
/// This struct holds a collection of luminaire dirt depreciation factors defined according to IES standards.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct IesLuminaireLightLossFactors {
    /// Luminaire dirt depreciation factors defined according to IES standards.
    #[yaserde(rename = "LuminaireDirtDepreciation")]
    #[serde(rename = "LuminaireDirtDepreciation")]
    pub luminaire_dirt_depreciation: Vec<LuminaireDirtDepreciation>,
}

/// Represents JIEG-specific luminaire maintenance factors in the GLDF data structure.
///
/// This struct holds a collection of luminaire maintenance factors defined according to JIEG standards.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct JiegMaintenanceFactors {
    /// Luminaire maintenance factors defined according to JIEG standards.
    #[yaserde(rename = "LuminaireMaintenanceFactor")]
    #[serde(rename = "LuminaireMaintenanceFactor")]
    pub luminaire_maintenance_factor: Vec<LuminaireMaintenanceFactor>,
}

/// Represents maintenance-related information for a luminaire in the GLDF data structure.
///
/// This struct holds information about the luminaire type, maintenance factors, and light loss factors.
///
/// ISO 7127:2017 - Section 4.19
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct LuminaireMaintenance {
    /// The CIE97 luminaire type classification.
    ///
    /// ISO 7127:2017 - Section 4.19.2
    #[yaserde(rename = "Cie97LuminaireType")]
    #[serde(rename = "Cie97LuminaireType")]
    pub cie97_luminaire_type: String,

    /// CIE-specific luminaire maintenance factors.
    ///
    /// ISO 7127:2017 - Section 4.19.3
    #[yaserde(rename = "CieLuminaireMaintenanceFactors")]
    #[serde(rename = "CieLuminaireMaintenanceFactors")]
    pub cie_luminaire_maintenance_factors: CieLuminaireMaintenanceFactors,

    /// IES-specific luminaire light loss factors, if available.
    ///
    /// ISO 7127:2017 - Section 4.19.4
    #[yaserde(rename = "IesLuminaireLightLossFactors")]
    #[serde(rename = "IesLuminaireLightLossFactors", skip_serializing_if = "Option::is_none")]
    pub ies_luminaire_light_loss_factors: Option<IesLuminaireLightLossFactors>,

    /// JIEG-specific luminaire maintenance factors, if available.
    ///
    /// ISO 7127:2017 - Section 4.19.5
    #[yaserde(rename = "JiegMaintenanceFactors")]
    #[serde(rename = "JiegMaintenanceFactors", skip_serializing_if = "Option::is_none")]
    pub jieg_maintenance_factors: Option<JiegMaintenanceFactors>,
}

/// Represents metadata for a luminaire product in the GLDF data structure.
///
/// This struct holds descriptive information about the product, including its name, description,
/// tender text, product series, pictures, maintenance details, and descriptive attributes.
///
/// ISO 7127:2017 - Section 4.21
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductMetaData {
    /// The product number, if available.
    ///
    /// ISO 7127:2017 - Section 4.21.2
    #[yaserde(rename = "ProductNumber")]
    #[serde(rename = "ProductNumber", skip_serializing_if = "Option::is_none")]
    pub product_number: Option<LocaleFoo>,

    /// The name of the product, if available.
    ///
    /// ISO 7127:2017 - Section 4.21.3
    #[yaserde(rename = "Name")]
    #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
    pub name: Option<LocaleFoo>,

    /// The description of the product, if available.
    ///
    /// ISO 7127:2017 - Section 4.21.4
    #[yaserde(rename = "Description")]
    #[serde(rename = "Description", skip_serializing_if = "Option::is_none")]
    pub description: Option<LocaleFoo>,

    /// The tender text for the product.
    ///
    /// ISO 7127:2017 - Section 4.21.5
    #[yaserde(rename = "TenderText")]
    #[serde(rename = "TenderText")]
    pub tender_text: Option<LocaleFoo>,

    /// The product series to which the product belongs, if available.
    ///
    /// ISO 7127:2017 - Section 4.21.6
    #[yaserde(rename = "ProductSeries")]
    #[serde(rename = "ProductSeries", skip_serializing_if = "Option::is_none")]
    pub product_series: Option<ProductSeries>,

    /// Pictures of the product, if available.
    ///
    /// ISO 7127:2017 - Section 4.21.7
    #[yaserde(rename = "Pictures")]
    #[serde(rename = "Pictures", skip_serializing_if = "Option::is_none")]
    pub pictures: Option<Images>,

    /// Luminaire maintenance information for the product, if available.
    ///
    /// ISO 7127:2017 - Section 4.21.8
    #[yaserde(rename = "LuminaireMaintenance")]
    #[serde(rename = "LuminaireMaintenance", skip_serializing_if = "Option::is_none")]
    pub luminaire_maintenance: Option<LuminaireMaintenance>,

    /// Descriptive attributes of the product, if available.
    ///
    /// ISO 7127:2017 - Section 4.21.9
    #[yaserde(rename = "DescriptiveAttributes")]
    #[serde(rename = "DescriptiveAttributes", skip_serializing_if = "Option::is_none")]
    pub descriptive_attributes: Option<DescriptiveAttributes>,
}
/// Represents a rectangular cutout in the GLDF data structure.
///
/// This struct holds information about the width, length, and depth of a rectangular cutout.
///
/// ISO 7127:2017 - Section 4.27.2
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct RectangularCutout {
    /// The width of the rectangular cutout.
    ///
    /// ISO 7127:2017 - Section 4.27.2.1
    #[yaserde(attribute, rename = "Width")]
    #[serde(rename = "@Width")]
    pub width: i32,

    /// The length of the rectangular cutout.
    ///
    /// ISO 7127:2017 - Section 4.27.2.2
    #[yaserde(attribute, rename = "Length")]
    #[serde(rename = "@Length")]
    pub length: i32,

    /// The depth of the rectangular cutout.
    ///
    /// ISO 7127:2017 - Section 4.27.2.3
    #[yaserde(attribute, rename = "Depth")]
    #[serde(rename = "@Depth")]
    pub depth: i32,
}

/// Represents a circular cutout in the GLDF data structure.
///
/// This struct holds information about the diameter and depth of a circular cutout.
///
/// ISO 7127:2017 - Section 4.27.3
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CircularCutout {
    /// The diameter of the circular cutout.
    ///
    /// ISO 7127:2017 - Section 4.27.3.1
    #[yaserde(attribute, rename = "Diameter")]
    #[serde(rename = "@Diameter")]
    pub diameter: i32,

    /// The depth of the circular cutout.
    ///
    /// ISO 7127:2017 - Section 4.27.3.2
    #[yaserde(attribute, rename = "Depth")]
    #[serde(rename = "@Depth")]
    pub depth: i32,
}

/// Represents a recessed luminaire in the GLDF data structure.
///
/// This struct holds information about the recessed depth, rectangular cutout, and depth of the recessed luminaire.
///
/// ISO 7127:2017 - Section 4.27.4
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Recessed {
    /// The recessed depth .
    ///
    /// ISO 7127:2017 - Section 4.27.4.1
    #[yaserde(attribute, rename = "recessedDepth")]
    #[serde(rename = "@recessedDepth")]
    // the recessed depth in mm (millimeter)
    pub recessed_depth: i32,

    /// The rectangular cutout details for the recessed luminaire.
    ///
    /// ISO 7127:2017 - Section 4.27.4.2
    #[yaserde(attribute, rename = "RectangularCutout")]
    #[serde(rename = "@RectangularCutout")]
    /// the rectangular cutout details for the recessed luminaire.
    pub rectangular_cutout: RectangularCutout,

    /// The overall depth of the recessed luminaire.
    ///
    /// ISO 7127:2017 - Section 4.27.4.3
    #[yaserde(attribute, rename = "Depth")]
    #[serde(rename = "@Depth")]
    /// the depth of the recessed luminaire in mm (millimeter)
    pub depth: i32,
}

/// Represents a surface-mounted luminaire in the GLDF data structure.
///
/// This struct represents a surface-mounted luminaire and does not hold any additional attributes.
///  Mounting type: surface (of the ceiling)
/// ISO 7127:2017 - Section 4.27.5
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SurfaceMounted {}

/// The `Pendant` struct describes pendant-related properties in the GLDF file.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Pendant {
    /// The length of the pendant.
    #[yaserde(rename = "pendantLength")]
    #[serde(rename = "pendantLength")]
    pub pendant_length: f64,
}

/// Represents a luminaire with a pole top mount in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Ceiling {
    #[yaserde(rename = "Recessed")]
    #[serde(rename = "Recessed")]
    /// the recessed Type
    pub recessed: Recessed,
    /// the surface mounted type
    #[yaserde(rename = "SurfaceMounted")]
    #[serde(rename = "SurfaceMounted")]
    pub surface_mounted: SurfaceMounted,
    #[yaserde(rename = "Pendant")]
    #[serde(rename = "Pendant")]
    /// the pendant type
    pub pendant: Pendant,
}

/// Luminaire on the  Wall ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Wall {
    #[yaserde(attribute, rename = "mountingHeight")]
    #[serde(rename = "@mountingHeight")]
    /// the mounting height in mm (millimeter)
    pub mounting_height: i32,
    #[yaserde(rename = "Recessed")]
    #[serde(rename = "Recessed")]
    /// the recessed height in mm (millimeter)
    pub recessed: Option<Recessed>,
    #[yaserde(rename = "Depth")]
    #[serde(rename = "Depth")]
    /// the depth in mm (millimeter)
    pub depth: i32,
}

/// FreeStanding Luminaire ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FreeStanding {}

/// WorkingPlane 
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct WorkingPlane {
    #[yaserde(rename = "FreeStanding")]
    #[serde(rename = "FreeStanding")]
    /// the free standing type
    pub free_standing: FreeStanding,
}

/// PoleTop ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PoleTop {
    #[yaserde(rename = "poleHeight")]
    #[serde(rename = "poleHeight")]
    /// the pole height in mm (millimeter)
    pub pole_height: i32,
}

/// PoleIntegrated ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PoleIntegrated {
    #[yaserde(rename = "poleHeight")]
    #[serde(rename = "poleHeight")]
    /// the pole height in mm (millimeter)
    pub pole_height: i32,
}

/// Ground ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Ground {
    #[yaserde(rename = "PoleTop")]
    #[serde(rename = "PoleTop")]
    /// the pole top type
    pub pole_top: PoleTop,
    #[yaserde(rename = "PoleIntegrated")]
    #[serde(rename = "PoleIntegrated")]
    /// the pole integrated type
    pub pole_integrated: PoleIntegrated,
    #[yaserde(rename = "FreeStanding")]
    #[serde(rename = "FreeStanding")]
    /// the free standing type
    pub free_standing: FreeStanding,
    #[yaserde(rename = "SurfaceMounted")]
    #[serde(rename = "SurfaceMounted")]
    /// the surface mounted type
    pub surface_mounted: SurfaceMounted,
    #[yaserde(rename = "Recessed")]
    #[serde(rename = "Recessed")]
    /// the recessed type
    pub recessed: Recessed,
}

/// Mountings ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Mountings {
    #[yaserde(rename = "Ceiling")]
    #[serde(rename = "Ceiling")]
    /// the ceiling type
    pub ceiling: Option<Ceiling>,
    #[yaserde(rename = "Wall")]
    #[serde(rename = "Wall")]
    /// the wall type
    pub wall: Option<Wall>,
    #[yaserde(rename = "WorkingPlane")]
    #[serde(rename = "WorkingPlane")]
    /// the working plane type
    pub working_plane: Option<WorkingPlane>,
    #[yaserde(rename = "Ground")]
    #[serde(rename = "Ground")]
    /// the ground type
    pub ground: Option<Ground>,
}

/// EmitterReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EmitterReference {
    #[yaserde(attribute, rename = "emitterId")]
    #[serde(rename = "@emitterId")]
    /// the emitter id
    pub emitter_id: String,
    #[yaserde(rename = "EmitterObjectExternalName")]
    #[serde(rename = "EmitterObjectExternalName")]
    /// the external name of the emitter
    pub emitter_object_external_name: String,
}

/// SimpleGeometryReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SimpleGeometryReference {
    #[yaserde(rename = "geometryId")]
    #[serde(rename = "geometryId")]
    /// the geometry id 
    pub geometry_id: String,
    #[yaserde(rename = "emitterId")]
    #[serde(rename = "emitterId")]
    /// the emitter id 
    pub emitter_id: String,
}

/// ModelGeometryReference ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ModelGeometryReference {
    #[yaserde(attribute, rename = "geometryId")]
    #[serde(rename = "@geometryId")]
    /// the geometry id 
    pub geometry_id: String,
    #[yaserde(rename = "EmitterReference")]
    #[serde(rename = "EmitterReference")]
    /// the list of emitter references 
    pub emitter_reference: Vec<EmitterReference>,
}

/// GeometryReferences ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct GeometryReferences {
    #[yaserde(rename = "SimpleGeometryReference")]
    #[serde(rename = "SimpleGeometryReference")]
    /// the simple geometry reference 
    pub simple_geometry_reference: SimpleGeometryReference,
    #[yaserde(rename = "ModelGeometryReference")]
    #[serde(rename = "ModelGeometryReference")]
    /// the model geometry reference 
    pub model_geometry_reference: ModelGeometryReference,
}

/// Geometry ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Geometry {
    #[yaserde(rename = "SimpleGeometryReference")]
    #[serde(rename = "SimpleGeometryReference", skip_serializing_if = "Option::is_none")]
    /// the simple geometry reference 
    pub simple_geometry_reference: Option<SimpleGeometryReference>,
    #[yaserde(rename = "ModelGeometryReference")]
    #[serde(rename = "ModelGeometryReference", skip_serializing_if = "Option::is_none")]
    /// the model geometry reference
    pub model_geometry_reference: Option<ModelGeometryReference>,

}

/// Symbol ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Symbol {
    #[yaserde(rename = "fileId")]
    #[serde(rename = "fileId")]
    /// the file id  symbol
    pub file_id: String,
}

/// Variant ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Variant {
    #[yaserde(attribute, rename = "id")]
    #[serde(rename = "@id")]
    /// the id of the variant
    pub id: String,
    #[yaserde(rename = "sortOrder")]
    #[serde(rename = "sortOrder", skip_serializing_if = "Option::is_none")]
    /// the sort order of the variant
    pub sort_order: Option<i32>,
    #[yaserde(rename = "ProductNumber")]
    #[serde(rename = "ProductNumber", skip_serializing_if = "Option::is_none")]
    /// the product number of the variant as Locale string
    pub product_number: Option<LocaleFoo>,
    #[yaserde(rename = "Name")]
    #[serde(rename = "Name", skip_serializing_if = "Option::is_none")]
    /// the name of the variant as Locale string
    pub name: Option<LocaleFoo>,
    #[yaserde(rename = "Description")]
    #[serde(rename = "Description", skip_serializing_if = "Option::is_none")]
    /// The description of the variant as Locale string
    pub description: Option<LocaleFoo>,
    #[yaserde(rename = "TenderText")]
    #[serde(rename = "TenderText", skip_serializing_if = "Option::is_none")]
    /// the tender text of the variant as Locale string
    pub tender_text: Option<LocaleFoo>,
    #[yaserde(rename = "GTIN")]
    #[serde(rename = "GTIN", skip_serializing_if = "Option::is_none")]
    /// the gtin of the variant
    pub gtin: Option<String>,
    #[yaserde(rename = "Mountings")]
    #[serde(rename = "Mountings", skip_serializing_if = "Option::is_none")]
    /// the mountings of the variant
    pub mountings: Option<Mountings>,
    #[yaserde(rename = "Geometry")]
    #[serde(rename = "Geometry", skip_serializing_if = "Option::is_none")]
    /// the geometry of the variant
    pub geometry: Option<Geometry>,
    #[yaserde(rename = "ProductSeries")]
    #[serde(rename = "ProductSeries", skip_serializing_if = "Option::is_none")]
    /// the product series of the variant
    pub product_series: Option<ProductSeries>,
    #[yaserde(rename = "Pictures")]
    #[serde(rename = "Pictures", skip_serializing_if = "Option::is_none")]
    /// the images of the variant
    pub pictures: Option<Images>,
    #[yaserde(rename = "Symbol")]
    #[serde(rename = "Symbol", skip_serializing_if = "Option::is_none")]
    /// the symbol of the variant
    pub symbol: Option<Symbol>,
    #[yaserde(rename = "DescriptiveAttributes")]
    #[serde(rename = "DescriptiveAttributes", skip_serializing_if = "Option::is_none")]
    /// the descriptive attributes of the variant
    pub descriptive_attributes: Option<DescriptiveAttributes>,
}

///Variants ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Variants {
    #[yaserde(rename = "Variant")]
    #[serde(rename = "Variant")]
    /// the variant 
    pub variant: Vec<Variant>,
}

/// ProductSize ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProductSize {
    #[yaserde(rename = "Length")]
    #[serde(rename = "Length")]
    /// the length of the product in mm (millimeter)
    pub length: i32,
    #[yaserde(rename = "Width")]
    #[serde(rename = "Width")]
    /// the width of the product in mm (millimeter)
    pub width: i32,
    #[yaserde(rename = "Height")]
    #[serde(rename = "Height")]
    /// the height of the product in mm (millimeter)
    pub height: i32,
}

/// Adjustabilities ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Adjustabilities {
    #[yaserde(rename = "Adjustability")]
    #[serde(rename = "Adjustability")]
    /// the adjustability  as a list of strings
    pub adjustability: Vec<String>,
}

/// ProtectiveAreas ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ProtectiveAreas {
    #[yaserde(rename = "Area")]
    #[serde(rename = "Area")]
    /// the protective areas  as a list of strings
    pub area: Vec<String>,
}

/// Mechanical attributes 
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Mechanical {
    #[yaserde(rename = "ProductSize")]
    #[serde(rename = "ProductSize", skip_serializing_if = "Option::is_none")]
    /// the product size 
    pub product_size: Option<ProductSize>,
    #[yaserde(rename = "ProductForm")]
    #[serde(rename = "ProductForm", skip_serializing_if = "Option::is_none")]
    /// the product form 
    pub product_form: Option<String>,
    #[yaserde(rename = "SealingMaterial")]
    #[serde(rename = "SealingMaterial", skip_serializing_if = "Option::is_none")]
    /// the sealing material  a Locale string
    pub sealing_material: Option<LocaleFoo>,
    #[yaserde(rename = "Adjustabilities")]
    #[serde(rename = "Adjustabilities", skip_serializing_if = "Option::is_none")]
    /// the adjustabilities 
    pub adjustabilities: Option<Adjustabilities>,
    #[yaserde(rename = "IKRating")]
    #[serde(rename = "IKRating", skip_serializing_if = "Option::is_none")]
    /// the ik rating 
    pub ik_rating: Option<String>,
    #[yaserde(rename = "ProtectiveAreas")]
    #[serde(rename = "ProtectiveAreas", skip_serializing_if = "Option::is_none")]
    /// the protective areas 
    pub protective_areas: Option<ProtectiveAreas>,
    #[yaserde(rename = "Weight")]
    #[serde(rename = "Weight", skip_serializing_if = "Option::is_none")]
    /// the weight  in kg (kilogram)
    pub weight: Option<f64>,
}

/// ClampingRange ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ClampingRange {
    #[yaserde(rename = "Lower")]
    #[serde(rename = "Lower")]
    /// the lower value of the clamping range
    pub lower: f64,
    #[yaserde(rename = "Upper")]
    #[serde(rename = "Upper")]
    /// the upper value of the clamping range
    pub upper: f64,
}

/// Electrical ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Electrical {
    #[yaserde(rename = "ClampingRange")]
    #[serde(rename = "ClampingRange")]
    /// the clamping range 
    pub clamping_range: ClampingRange,
    #[yaserde(rename = "SwitchingCapacity")]
    #[serde(rename = "SwitchingCapacity")]
    /// the switching capacity 
    pub switching_capacity: String,
    #[yaserde(rename = "ElectricalSafetyClass")]
    #[serde(rename = "ElectricalSafetyClass")]
    /// the electrical safety class 
    pub electrical_safety_class: String,
    #[yaserde(rename = "IngressProtectionIPCode")]
    #[serde(rename = "IngressProtectionIPCode")]
    /// the ingress protection ip code 
    pub ingress_protection_ip_code: String,
    #[yaserde(rename = "PowerFactor")]
    #[serde(rename = "PowerFactor")]
    /// the power factor 
    pub power_factor: f64,
    #[yaserde(rename = "ConstantLightOutput")]
    #[serde(rename = "ConstantLightOutput")]
    /// bool constant light output , has constant light output or not
    pub constant_light_output: bool,
    #[yaserde(rename = "LightDistribution")]
    #[serde(rename = "LightDistribution", skip_serializing_if = "Option::is_none")]
    /// the light distribution
    pub light_distribution: Option<String>,
}

/// Flux ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Flux {
    #[yaserde(rename = "hours")]
    #[serde(rename = "hours")]
    /// the hours of the flux
    pub hours: i32,
    #[yaserde(rename = "$value")]
    /// the value of the flux
    pub value: i32,
}

/// DurationTimeAndFlux ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DurationTimeAndFlux {
    #[yaserde(rename = "Flux")]
    #[serde(rename = "Flux")]
    /// the list of flux 
    pub flux: Vec<Flux>,
}

/// Emergency ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Emergency {
    #[yaserde(rename = "DurationTimeAndFlux")]
    #[serde(rename = "DurationTimeAndFlux", skip_serializing_if = "Option::is_none")]
    /// the duration time and flux 
    pub duration_time_and_flux: Option<DurationTimeAndFlux>,
    #[yaserde(rename = "DedicatedEmergencyLightingType")]
    #[serde(rename = "DedicatedEmergencyLightingType", skip_serializing_if = "Option::is_none")]
    /// the dedicated emergency lighting type 
    pub dedicated_emergency_lighting_type: Option<String>,
}

/// ListPrice ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ListPrice {
    #[yaserde(rename = "currency")]
    #[serde(rename = "currency", skip_serializing_if = "Option::is_none")]
    /// the currency of the list price
    pub currency: Option<String>,
    #[yaserde(rename = "$value")]
    #[serde(rename = "$")]
    /// the value of the list price
    pub value: f64,
}

/// ListPrices ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ListPrices {
    #[yaserde(rename = "ListPrice")]
    #[serde(rename = "ListPrice")]
    /// the list of prices 
    pub list_price: Vec<ListPrice>,
}

/// HousingColor ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct HousingColor {
    #[yaserde(rename = "ral")]
    #[serde(rename = "ral")]
    /// the ral of the housing color
    pub ral: Option<i32>,
    #[yaserde(flatten)]
    /// the locale of the housing color
    pub locale: Locale,
}

/// HousingColors ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct HousingColors {
    #[yaserde(rename = "HousingColor")]
    #[serde(rename = "HousingColor")]
    /// the list of housing colors 
    pub housing_color: Vec<HousingColor>,
}

/// Markets ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Markets {
    #[yaserde(rename = "Region")]
    #[serde(rename = "Region")]
    /// the list of  market regions 
    pub region: Vec<Locale>,
}

/// ApprovalMarks ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ApprovalMarks {
    #[yaserde(rename = "ApprovalMark")]
    #[serde(rename = "ApprovalMark")]
    /// the list of approval marks
    pub approval_mark: Vec<String>,
}

/// DesignAwards ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DesignAwards {
    #[yaserde(rename = "DesignAward")]
    #[serde(rename = "DesignAward")]
    /// the list of design awards
    pub design_award: Vec<String>,
}

/// Labels ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Labels {
    #[yaserde(rename = "Label")]
    #[serde(rename = "Label")]
    /// the list of labels
    pub label: Vec<String>,
}

/// Applications ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Applications {
    #[yaserde(rename = "Application")]
    #[serde(rename = "Application")]
    /// the list of applications
    pub application: Vec<String>,
}

/// Marketing ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Marketing {
    #[yaserde(rename = "ListPrices")]
    #[serde(rename = "ListPrices", skip_serializing_if = "Option::is_none")]
    /// the list prices 
    pub list_prices: Option<ListPrices>,
    #[yaserde(rename = "HousingColors")]
    #[serde(rename = "HousingColors", skip_serializing_if = "Option::is_none")]
    /// the housing colors 
    pub housing_colors: Option<HousingColors>,
    #[yaserde(rename = "Markets")]
    #[serde(rename = "Markets", skip_serializing_if = "Option::is_none")]
    /// the markets 
    pub markets: Option<Markets>,
    #[yaserde(rename = "Hyperlinks")]
    #[serde(rename = "Hyperlinks", skip_serializing_if = "Option::is_none")]
    /// the hyperlinks 
    pub hyperlinks: Option<Hyperlinks>,
    #[yaserde(rename = "Designer")]
    #[serde(rename = "Designer", skip_serializing_if = "Option::is_none")]
    /// the designer 
    pub designer: Option<String>,
    #[yaserde(rename = "ApprovalMarks")]
    #[serde(rename = "ApprovalMarks", skip_serializing_if = "Option::is_none")]
    /// the approval marks 
    pub approval_marks: Option<ApprovalMarks>,
    #[yaserde(rename = "DesignAwards")]
    #[serde(rename = "DesignAwards", skip_serializing_if = "Option::is_none")]
    /// the design awards 
    pub design_awards: Option<DesignAwards>,
    #[yaserde(rename = "Labels")]
    #[serde(rename = "Labels", skip_serializing_if = "Option::is_none")]
    /// the labels 
    pub labels: Option<Labels>,
    #[yaserde(rename = "Applications")]
    #[serde(rename = "Applications", skip_serializing_if = "Option::is_none")]
    /// the applications 
    pub applications: Option<Applications>,
}

/// UsefulLifeTimes ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct UsefulLifeTimes {
    #[yaserde(rename = "UsefulLife")]
    #[serde(rename = "UsefulLife")]
    /// the list of useful life times 
    pub useful_life: Vec<String>,
}

/// MedianUsefulLifeTimes ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct MedianUsefulLifeTimes {
    #[yaserde(rename = "MedianUsefulLife")]
    #[serde(rename = "MedianUsefulLife")] //, skip_serializing_if = "Option::is_none")]
    /// teh list of median useful life times 
    pub median_useful_life: Vec<String>,
}
/// The `Directives` struct represents a list of directives related to explosion protection according to the ATEX directive.
///
/// Directives are regulatory guidelines issued by authorities to ensure compliance with safety and quality standards.
/// In the context of ATEX, directives specify requirements and procedures for the design, production, and use of equipment
/// intended for use in explosive atmospheres. The `Directives` struct holds a list of directive names.
///
/// For more information, refer to the ISO 7127 standard and relevant ATEX documentation.
/// Example:
///
/// ```
/// use gldf_rs::gldf::{Directives};
///
/// let directives = Directives {
///     directive: vec![
///         "2014/34/EU".to_string(),
///         "2014/35/EU".to_string(),
///         "2014/30/EU".to_string(),
///     ],
/// };
/// ```
///
/// In this example, the `Directives` struct is populated with a list of directive names,
/// including "2014/34/EU", "2014/35/EU", and "2014/30/EU", which correspond to specific ATEX directives.
///
/// For more information about ATEX directives and their implications, consult the ISO 7127 standard and official ATEX documentation.
///
/// Note: This example code is for illustrative purposes and may not reflect actual ATEX directive numbers or names.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Directives {
    /// The list of directive names.
    #[yaserde(rename = "Directive")]
    #[serde(rename = "Directive")]
    pub directive: Vec<String>,

}


/// The `Classes` struct represents a list of classification classes related to explosion protection.
///
/// Classification classes categorize equipment and devices based on their suitability for use in hazardous environments.
/// The `Classes` struct holds a list of class names representing different categories of equipment protection.
///
/// For more information about classification classes and their implications, consult the ISO 7127 standard and relevant documentation.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Classes {
    /// The list of classification class names.
    #[yaserde(rename = "Class")]
    #[serde(rename = "Class")]
    /// The list of classification class names.
    pub class: Vec<String>,
}

/// The `Divisions` struct represents a list of division divisions related to explosion protection.
///
/// Division divisions further categorize equipment and devices based on specific hazardous environment criteria.
/// The `Divisions` struct holds a list of division names representing different divisions of equipment protection.
///
/// For more information about division divisions and their implications, consult the ISO 7127 standard and relevant documentation.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Divisions {
    /// The list of division names.
    #[yaserde(rename = "Division")]
    #[serde(rename = "Division")]
    /// The list of division names.
    pub division: Vec<String>,
}

/// The `Gas` struct represents a list of gas groups related to explosion protection.
///
/// Gas groups classify hazardous gases based on their characteristics and potential risks in industrial environments.
/// The `Gas` struct holds a list of gas group names representing different gas classifications.
///
/// For more information about gas groups and their implications, consult the ISO 7127 standard and relevant documentation.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Gas {
    /// The list of gas group names.
    #[yaserde(rename = "Group")]
    #[serde(rename = "Group")]
    /// The list of gas group names.
    pub group: Vec<String>,
}

/// The `Dust` struct represents a list of dust groups related to explosion protection.
///
/// Dust groups classify hazardous dusts based on their properties and potential risks in industrial environments.
/// The `Dust` struct holds a list of dust group names representing different dust classifications.
///
/// For more information about dust groups and their implications, consult the ISO 7127 standard and relevant documentation.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Dust {
    /// The list of dust group names.
    #[yaserde(rename = "Group")]
    #[serde(rename = "Group")]
    /// The list of dust group names.
    pub group: Vec<String>,
}

/// The `DivisionGroups` struct represents a collection of division groups for explosion protection.
///
/// Division groups provide a combination of division divisions related to both gas and dust classification.
/// The `DivisionGroups` struct holds instances of `Gas` and `Dust` structs representing the respective groupings.
///
/// For more information about division groups and their implications, consult the ISO 7127 standard and relevant documentation.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DivisionGroups {
    /// The gas classification group.
    #[yaserde(rename = "Gas")]
    #[serde(rename = "Gas")]
    /// The gas classification group.
    pub gas: Gas,

    /// The dust classification group.
    #[yaserde(rename = "Dust")]
    #[serde(rename = "Dust")]
    /// The dust classification group.
    pub dust: Dust,
}

/// The `Zones` struct represents possible explosion protection zones for a product.
///
/// Zones define specific hazardous environments where equipment can be safely used.
/// The `Zones` struct holds instances of `Gas` and `Dust` structs representing the respective zones.
///
/// For more information about explosion protection zones and their implications, consult the ISO 7127 standard and relevant documentation.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Zones {
    /// The gas protection zone.
    #[yaserde(rename = "Gas")]
    #[serde(rename = "Gas")]
    /// The gas protection zone.
    pub gas: Gas,

    /// The dust protection zone.
    #[yaserde(rename = "Dust")]
    #[serde(rename = "Dust")]
    /// The dust protection zone.
    pub dust: Dust,
}

/// The `ZoneGroups` struct represents collection of zone groups for explosion protection.
///
/// Zone groups provide a combination of protection zones related to both gas and dust classification.
/// The `ZoneGroups` struct holds instances of `Gas` and `Dust` structs representing the respective groupings.
///
/// For more information about zone groups and their implications, consult the ISO 7127 standard and relevant documentation.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ZoneGroups {
    /// The gas protection zone group.
    #[yaserde(rename = "Gas")]
    #[serde(rename = "Gas")]
    /// The gas protection zone group.
    pub gas: Gas,

    /// The dust protection zone group.
    #[yaserde(rename = "Dust")]
    #[serde(rename = "Dust")]
    /// The dust protection zone group.
    pub dust: Dust,
}

/// TemperatureClasses ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct TemperatureClasses {
    #[yaserde(rename = "TemperatureClass")]
    #[serde(rename = "TemperatureClass")]
    /// the list of temperature classes 
    pub temperature_class: Vec<String>,
}

/// The `ExCodes` struct represents the list of explosion protection (Ex) codes in the GLDF file.
///
/// Example:
///
/// ```rust
/// use gldf_rs::gldf::ExCodes;
///
/// // Create a sample ExCodes struct
/// let mut ex_codes = ExCodes::default();
/// ex_codes.ex_code.push("Exd".to_string());
/// ex_codes.ex_code.push("Exe".to_string());
/// // Add more Ex codes if needed
///
/// // Serialize and deserialize the ExCodes struct
/// let serialized = yaserde::ser::to_string(&ex_codes).unwrap();
/// let deserialized: ExCodes = yaserde::de::from_str(&serialized).unwrap();
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ExCodes {
    /// The list of explosion protection (Ex) codes.
    #[yaserde(rename = "ExCode")]
    #[serde(rename = "ExCode")]
    pub ex_code: Vec<String>,
}

/// The `EquipmentProtectionLevels` struct represents the list of equipment protection levels in the GLDF file.
///
/// Example:
///
/// ```rust
/// use gldf_rs::gldf::EquipmentProtectionLevels;
///
/// // Create a sample EquipmentProtectionLevels struct
/// let mut equip_prot_levels = EquipmentProtectionLevels::default();
/// equip_prot_levels.equipment_protection_level.push("IP20".to_string());
/// equip_prot_levels.equipment_protection_level.push("IP54".to_string());
/// // Add more equipment protection levels if needed
///
/// // Serialize and deserialize the EquipmentProtectionLevels struct
/// let serialized = yaserde::ser::to_string(&equip_prot_levels).unwrap();
/// let deserialized: EquipmentProtectionLevels = yaserde::de::from_str(&serialized).unwrap();
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EquipmentProtectionLevels {
    /// The list of equipment protection levels.
    #[yaserde(rename = "EquipmentProtectionLevel")]
    #[serde(rename = "EquipmentProtectionLevel")]
    pub equipment_protection_level: Vec<String>,
}

/// The `EquipmentGroups` struct represents the list of equipment groups in the GLDF file.
///
/// Example:
///
/// ```rust
/// use gldf_rs::gldf::EquipmentGroups;
///
/// // Create a sample EquipmentGroups struct
/// let mut equip_groups = EquipmentGroups::default();
/// equip_groups.equipment_group.push("Group I".to_string());
/// equip_groups.equipment_group.push("Group II".to_string());
/// // Add more equipment groups if needed
///
/// // Serialize and deserialize the EquipmentGroups struct
/// let serialized = yaserde::ser::to_string(&equip_groups).unwrap();
/// let deserialized: EquipmentGroups = yaserde::de::from_str(&serialized).unwrap();
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EquipmentGroups {
    /// The list of equipment groups.
    #[yaserde(rename = "EquipmentGroup")]
    #[serde(rename = "EquipmentGroup")]
    pub equipment_group: Vec<String>,
}

/// The `EquipmentCategories` struct represents the list of equipment categories in the GLDF file.
///
/// Example:
///
/// ```rust
/// use gldf_rs::gldf::EquipmentCategories;
///
/// // Create a sample EquipmentCategories struct
/// let mut equip_categories = EquipmentCategories::default();
/// equip_categories.equipment_category.push("Category A".to_string());
/// equip_categories.equipment_category.push("Category B".to_string());
/// // Add more equipment categories if needed
///
/// // Serialize and deserialize the EquipmentCategories struct
/// let serialized = yaserde::ser::to_string(&equip_categories).unwrap();
/// let deserialized: EquipmentCategories = yaserde::de::from_str(&serialized).unwrap();
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EquipmentCategories {
    /// The list of equipment categories.
    #[yaserde(rename = "EquipmentCategory")]
    #[serde(rename = "EquipmentCategory")]
    pub equipment_category: Vec<String>,
}

/// The `Atmospheres` struct represents the list of atmospheres in the GLDF file.
///
/// Example:
///
/// ```rust
/// use gldf_rs::gldf::Atmospheres;
///
/// // Create a sample Atmospheres struct
/// let mut atmospheres = Atmospheres::default();
/// atmospheres.atmosphere.push("Zone 0".to_string());
/// atmospheres.atmosphere.push("Zone 1".to_string());
/// // Add more atmospheres if needed
///
/// // Serialize and deserialize the Atmospheres struct
/// let serialized = yaserde::ser::to_string(&atmospheres).unwrap();
/// let deserialized: Atmospheres = yaserde::de::from_str(&serialized).unwrap();
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Atmospheres {
    /// The list of atmospheres.
    #[yaserde(rename = "Atmosphere")]
    #[serde(rename = "Atmosphere")]
    pub atmosphere: Vec<String>,
}

/// The `Groups` struct represents the list of groups in the GLDF file.
///
/// Example:
///
/// ```rust
/// use gldf_rs::gldf::Groups;
///
/// // Create a sample Groups struct
/// let mut groups = Groups::default();
/// groups.group.push("Group 1".to_string());
/// groups.group.push("Group 2".to_string());
/// // Add more groups if needed
///
/// // Serialize and deserialize the Groups struct
/// let serialized = yaserde::ser::to_string(&groups).unwrap();
/// let deserialized: Groups = yaserde::de::from_str(&serialized).unwrap();
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Groups {
    /// The list of groups.
    #[yaserde(rename = "Group")]
    #[serde(rename = "Group")]
    pub group: Vec<String>,
}

/// The `ATEX` struct represents the information related to explosion protection according to the ATEX directive.
///
/// ATEX (ATmosphères EXplosibles) is a European Union directive that outlines the essential requirements for equipment
/// and protective systems intended for use in potentially explosive atmospheres. It classifies equipment into various
/// categories based on the level of protection they provide against the potential ignition of explosive gases or dust.
/// The directive defines equipment groups, temperature classes, and protection levels, among other parameters.
///
/// For more information, refer to the ISO 7127 standard.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ATEX {
    /// The ATEX directives information.
    #[yaserde(rename = "Directives")]
    #[serde(rename = "Directives")]
    pub directives: Directives,
    /// The ATEX classes information.
    #[yaserde(rename = "Classes")]
    #[serde(rename = "Classes")]
    pub classes: Classes,
    /// The ATEX divisions information.
    #[yaserde(rename = "Divisions")]
    #[serde(rename = "Divisions")]
    pub divisions: Divisions,
    /// The ATEX division groups information.
    #[yaserde(rename = "DivisionGroups")]
    #[serde(rename = "DivisionGroups")]
    pub division_groups: DivisionGroups,
    /// The ATEX zones information.
    #[yaserde(rename = "Zones")]
    #[serde(rename = "Zones")]
    pub zones: Zones,
    /// The ATEX zone groups information.
    #[yaserde(rename = "ZoneGroups")]
    #[serde(rename = "ZoneGroups")]
    pub zone_groups: ZoneGroups,
    /// The maximum surface temperature allowed by ATEX.
    #[yaserde(rename = "MaximumSurfaceTemperature")]
    #[serde(rename = "MaximumSurfaceTemperature")]
    pub maximum_surface_temperature: String,
    // Add more ATEX information if needed
    // ...
    /// The ATEX groups information.
    #[yaserde(rename = "Groups")]
    #[serde(rename = "Groups")]
    pub groups: Groups,
}
/// The `AbsorptionRate` struct represents the rate of absorption of sound waves in hertz (Hz) and its corresponding value.
///
/// Absorption rate refers to the rate at which sound waves are absorbed by a material or surface. In this context, the
/// `AbsorptionRate` struct holds information about the absorption rate measured in hertz (Hz) and its corresponding value.
///
/// For more information about sound absorption and its measurement, consult relevant acoustic standards and documentation.
/// Example:
///
/// ```
/// use gldf_rs::gldf::{AbsorptionRate};
///
/// let absorption_rate = AbsorptionRate {
///     hertz: 1000,
///     value: 0.25,
/// };
/// ```
///
/// In this example, the `AbsorptionRate` struct is populated with an absorption rate of 1000 Hz and a value of 0.25.
/// This indicates that the material or surface absorbs sound waves at a rate of 1000 Hz with a coefficient of 0.25.
///
/// For accurate sound absorption measurements and interpretations, refer to relevant acoustic standards and guidelines.
///
/// Note: This example code is for illustrative purposes and may not reflect actual absorption rate values or units.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct AbsorptionRate {
    /// The rate of absorption in hertz (Hz).
    #[yaserde(rename = "hertz")]
    #[serde(rename = "hertz")]
    pub hertz: i32,

    /// The corresponding value of the absorption rate.
    #[yaserde(rename = "$value")]
    pub value: f64,
}

/// The `AcousticAbsorptionRates` struct represents a collection of acoustic absorption rates at different frequencies.
///
/// Acoustic absorption rates provide information about the amount of sound energy absorbed by a material or surface
/// across different frequencies. The `AcousticAbsorptionRates` struct holds a collection of `AbsorptionRate` instances,
/// each specifying the absorption rate at a specific frequency.
///
/// For accurate sound absorption measurements and interpretations, refer to relevant acoustic standards and guidelines.
/// Example:
///
/// ```
/// use gldf_rs::gldf::{AcousticAbsorptionRates, AbsorptionRate};
///
/// let absorption_rates = AcousticAbsorptionRates {
///     absorption_rate: vec![
///         AbsorptionRate { hertz: 125, value: 0.15 },
///         AbsorptionRate { hertz: 250, value: 0.25 },
///         AbsorptionRate { hertz: 500, value: 0.35 },
///     ],
/// };
/// ```
///
/// In this example, the `AcousticAbsorptionRates` struct is populated with a collection of `AbsorptionRate` instances,
/// each specifying the absorption rate at a specific frequency (125 Hz, 250 Hz, and 500 Hz).
///
/// For accurate interpretation of acoustic absorption rates and their implications, consult relevant acoustic standards
/// and documentation.
///
/// Note: This example code is for illustrative purposes and may not reflect actual absorption rate values or frequencies.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct AcousticAbsorptionRates {
    /// The collection of absorption rates at different frequencies.
    #[yaserde(rename = "AbsorptionRate")]
    #[serde(rename = "AbsorptionRate")]
    pub absorption_rate: Vec<AbsorptionRate>,
}

/// The `OperationsAndMaintenance` struct represents information related to the operations and maintenance of a luminaire.
///
/// Operations and maintenance details provide insights into how the luminaire should be used, maintained, and monitored
/// to ensure optimal performance and longevity. The `OperationsAndMaintenance` struct holds various attributes such as
/// useful life times, operating temperatures, ATEX classification, acoustic absorption rates, and more.
///
/// Accurate understanding and adherence to these details are crucial for effective luminaire usage and upkeep.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct OperationsAndMaintenance {
    /// The useful life times of the luminaire.
    #[yaserde(rename = "UsefulLifeTimes")]
    #[serde(rename = "UsefulLifeTimes", skip_serializing_if = "Option::is_none")]
    /// The list of useful life times for the luminaire.
    pub useful_life_times: Option<UsefulLifeTimes>,

    /// The median useful life times of the luminaire.
    #[yaserde(rename = "MedianUsefulLifeTimes")]
    #[serde(rename = "MedianUsefulLifeTimes", skip_serializing_if = "Option::is_none")]
    /// The list of median useful life times for the luminaire.
    pub median_useful_life_times: Option<MedianUsefulLifeTimes>,

    /// The operating temperature range of the luminaire.
    #[yaserde(rename = "OperatingTemperature")]
    #[serde(rename = "OperatingTemperature", skip_serializing_if = "Option::is_none")]
    /// The operating temperature range for the luminaire.
    pub operating_temperature: Option<TemperatureRange>,

    /// The ambient temperature range of the luminaire.
    #[yaserde(rename = "AmbientTemperature")]
    #[serde(rename = "AmbientTemperature", skip_serializing_if = "Option::is_none")]
    /// The ambient temperature range for the luminaire.
    pub ambient_temperature: Option<TemperatureRange>,

    /// The rated ambient temperature of the luminaire.
    #[yaserde(rename = "RatedAmbientTemperature")]
    #[serde(rename = "RatedAmbientTemperature", skip_serializing_if = "Option::is_none")]
    /// The rated ambient temperature for the luminaire.
    pub rated_ambient_temperature: Option<i32>,

    /// The ATEX classification of the luminaire.
    #[yaserde(rename = "ATEX")]
    #[serde(rename = "ATEX", skip_serializing_if = "Option::is_none")]
    /// The ATEX classification details for the luminaire.
    pub atex: Option<ATEX>,

    /// The acoustic absorption rates of the luminaire.
    #[yaserde(rename = "AcousticAbsorptionRates")]
    #[serde(rename = "AcousticAbsorptionRates", skip_serializing_if = "Option::is_none")]
    /// The list of acoustic absorption rates for the luminaire.
    pub acoustic_absorption_rates: Option<AcousticAbsorptionRates>,
}

/// The `FileReference` struct represents a reference to a file associated with a property.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct FileReference {
    /// The file ID of the referenced file.
    #[yaserde(rename = "fileId")]
    #[serde(rename = "fileId")]
    /// The file ID for the file reference.
    pub file_id: String,
}

/// The `Property` struct represents a property associated with a luminaire.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Property {
    /// The ID of the property.
    #[yaserde(rename = "id")]
    #[serde(rename = "id")]
    /// The ID of the property.
    pub id: String,

    /// The locale of the property.
    #[yaserde(rename = "Name")]
    #[serde(rename = "Name")]
    /// The locale information for the property.
    pub name: Locale,

    /// The property source.
    #[yaserde(rename = "PropertySource")]
    #[serde(rename = "PropertySource")]
    /// The source of the property.
    pub property_source: String,

    /// The value of the property.
    #[yaserde(rename = "Value")]
    #[serde(rename = "Value")]
    /// The value of the property.
    pub value: String,

    /// The file reference associated with the property.
    #[yaserde(rename = "FileReference")]
    #[serde(rename = "FileReference")]
    /// The file reference for the property, if applicable.
    pub file_reference: FileReference,
}

/// The `CustomProperties` struct represents a collection of custom properties associated with a luminaire.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CustomProperties {
    /// The list of custom properties associated with the luminaire.
    #[yaserde(rename = "Property")]
    #[serde(rename = "Property")]
    /// The list of custom properties for the luminaire.
    pub property: Vec<Property>,
}

/// The `DescriptiveAttributes` struct represents various descriptive attributes of a luminaire.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DescriptiveAttributes {
    /// The mechanical attributes of the luminaire.
    #[yaserde(rename = "Mechanical")]
    #[serde(rename = "Mechanical", skip_serializing_if = "Option::is_none")]
    /// The mechanical attributes for the luminaire.
    pub mechanical: Option<Mechanical>,

    /// The electrical attributes of the luminaire.
    #[yaserde(rename = "Electrical")]
    #[serde(rename = "Electrical", skip_serializing_if = "Option::is_none")]
    /// The electrical attributes for the luminaire.
    pub electrical: Option<Electrical>,

    /// The emergency attributes of the luminaire.
    #[yaserde(rename = "Emergency")]
    #[serde(rename = "Emergency", skip_serializing_if = "Option::is_none")]
    /// The emergency attributes for the luminaire.
    pub emergency: Option<Emergency>,

    /// The marketing attributes of the luminaire.
    #[yaserde(rename = "Marketing")]
    #[serde(rename = "Marketing", skip_serializing_if = "Option::is_none")]
    /// The marketing attributes for the luminaire.
    pub marketing: Option<Marketing>,

    /// The operations and maintenance attributes of the luminaire.
    #[yaserde(rename = "OperationsAndMaintenance")]
    #[serde(rename = "OperationsAndMaintenance", skip_serializing_if = "Option::is_none")]
    /// The operations and maintenance attributes for the luminaire.
    pub operations_and_maintenance: Option<OperationsAndMaintenance>,

    /// The custom properties of the luminaire.
    #[yaserde(rename = "CustomProperties")]
    #[serde(rename = "CustomProperties", skip_serializing_if = "Option::is_none")]
    /// The custom properties for the luminaire, if applicable.
    pub custom_properties: Option<CustomProperties>,
}

/// TemperatureRange ...
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct TemperatureRange {
    #[yaserde(rename = "Lower")]
    #[serde(rename = "Lower")]
    /// the lower value of the temperature range
    pub lower: i32,
    #[yaserde(rename = "Upper")]
    #[serde(rename = "Upper")]
    /// the upper value of the temperature range
    pub upper: i32,
}






