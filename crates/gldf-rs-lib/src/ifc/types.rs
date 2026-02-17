//! IFC type definitions for lighting entities

use serde::{Deserialize, Serialize};

/// Light fixture type enum matching IFC IfcLightFixtureTypeEnum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LightFixtureTypeEnum {
    /// Point light source
    PointSource,
    /// Directional light source
    DirectionSource,
    /// Security/emergency lighting
    SecurityLighting,
    /// User-defined type
    UserDefined,
    /// Not defined
    #[default]
    NotDefined,
}

impl LightFixtureTypeEnum {
    /// Convert to IFC STEP enum string
    pub fn to_step(&self) -> &'static str {
        match self {
            Self::PointSource => ".POINTSOURCE.",
            Self::DirectionSource => ".DIRECTIONSOURCE.",
            Self::SecurityLighting => ".SECURITYLIGHTING.",
            Self::UserDefined => ".USERDEFINED.",
            Self::NotDefined => ".NOTDEFINED.",
        }
    }
}

/// Light emission source enum matching IFC IfcLightEmissionSourceEnum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LightEmissionSourceEnum {
    CompactFluorescent,
    Fluorescent,
    HighPressureMercury,
    HighPressureSodium,
    Led,
    LightEmittingDiode,
    LowPressureSodium,
    LowVoltageHalogen,
    MainVoltageHalogen,
    MetalHalide,
    TungstenFilament,
    #[default]
    NotDefined,
}

impl LightEmissionSourceEnum {
    /// Convert to IFC STEP enum string
    pub fn to_step(&self) -> &'static str {
        match self {
            Self::CompactFluorescent => ".COMPACTFLUORESCENT.",
            Self::Fluorescent => ".FLUORESCENT.",
            Self::HighPressureMercury => ".HIGHPRESSUREMERCURY.",
            Self::HighPressureSodium => ".HIGHPRESSURESODIUM.",
            Self::Led => ".LED.",
            Self::LightEmittingDiode => ".LIGHTEMITTINGDIODE.",
            Self::LowPressureSodium => ".LOWPRESSURESODIUM.",
            Self::LowVoltageHalogen => ".LOWVOLTAGEHALOGEN.",
            Self::MainVoltageHalogen => ".MAINVOLTAGEHALOGEN.",
            Self::MetalHalide => ".METALHALIDE.",
            Self::TungstenFilament => ".TUNGSTENFILAMENT.",
            Self::NotDefined => ".NOTDEFINED.",
        }
    }
}

/// Entity reference in IFC STEP format (e.g., #123)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityRef(pub u64);

impl EntityRef {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for EntityRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Optional entity reference ($ for null)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionalRef {
    Some(EntityRef),
    None,
}

impl std::fmt::Display for OptionalRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Some(r) => write!(f, "{}", r),
            Self::None => write!(f, "$"),
        }
    }
}

impl From<EntityRef> for OptionalRef {
    fn from(r: EntityRef) -> Self {
        Self::Some(r)
    }
}

impl From<Option<EntityRef>> for OptionalRef {
    fn from(opt: Option<EntityRef>) -> Self {
        match opt {
            Some(r) => Self::Some(r),
            None => Self::None,
        }
    }
}

// ============================================================================
// Electrical MVD Types (SPARKIE / Electrical Information Exchange)
// ============================================================================

/// Conductor function enum for electrical phase identification (IEC 60446)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[allow(non_camel_case_types)]
pub enum ConductorFunctionEnum {
    /// Phase L1
    PHASE_L1,
    /// Phase L2
    PHASE_L2,
    /// Phase L3
    PHASE_L3,
    /// Neutral
    Neutral,
    /// Protective Earth
    ProtectiveEarth,
    /// Protective Earth and Neutral combined
    ProtectiveEarthNeutral,
    #[default]
    NotDefined,
}

impl ConductorFunctionEnum {
    pub fn to_step(&self) -> &'static str {
        match self {
            Self::PHASE_L1 => ".PHASE_L1.",
            Self::PHASE_L2 => ".PHASE_L2.",
            Self::PHASE_L3 => ".PHASE_L3.",
            Self::Neutral => ".NEUTRAL.",
            Self::ProtectiveEarth => ".PROTECTIVEEARTH.",
            Self::ProtectiveEarthNeutral => ".PROTECTIVEEARTHNEUTRAL.",
            Self::NotDefined => ".NOTDEFINED.",
        }
    }
}

/// Insulation standard class for electrical safety (IEC 61140)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InsulationStandardClass {
    /// Class 0 - Basic insulation, no earth
    Class0,
    /// Class I - Basic insulation + protective earth
    ClassI,
    /// Class II - Double/reinforced insulation
    ClassII,
    /// Class III - Safety extra-low voltage (SELV)
    ClassIII,
    #[default]
    NotDefined,
}

impl InsulationStandardClass {
    pub fn to_step(&self) -> &'static str {
        match self {
            Self::Class0 => ".CLASS0.",
            Self::ClassI => ".CLASSI.",
            Self::ClassII => ".CLASSII.",
            Self::ClassIII => ".CLASSIII.",
            Self::NotDefined => ".NOTDEFINED.",
        }
    }

    /// Convert from GLDF electrical safety class string
    pub fn from_gldf_safety_class(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "I" | "CLASS I" | "CLASSI" | "1" => Self::ClassI,
            "II" | "CLASS II" | "CLASSII" | "2" => Self::ClassII,
            "III" | "CLASS III" | "CLASSIII" | "3" => Self::ClassIII,
            "0" | "CLASS 0" | "CLASS0" => Self::Class0,
            _ => Self::NotDefined,
        }
    }
}

/// Comprehensive electrical device properties for Pset_ElectricalDeviceCommon
///
/// Based on IFC4.3 specification and SPARKIE MVD requirements.
/// See: https://ifc43-docs.standards.buildingsmart.org/IFC/RELEASE/IFC4x3/HTML/lexical/Pset_ElectricalDeviceCommon.htm
#[derive(Debug, Clone, Default)]
pub struct ElectricalDeviceProperties {
    /// Rated current in Amperes (A)
    pub rated_current: Option<f64>,
    /// Rated voltage in Volts (V) - can be range (min, max)
    pub rated_voltage: Option<f64>,
    /// Maximum rated voltage (for bounded value)
    pub rated_voltage_max: Option<f64>,
    /// Nominal frequency range in Hz (min, max)
    pub nominal_frequency_min: Option<f64>,
    pub nominal_frequency_max: Option<f64>,
    /// Power factor (cos φ), normalized ratio 0.0-1.0
    pub power_factor: Option<f64>,
    /// Conductor function (phase line identification)
    pub conductor_function: Option<ConductorFunctionEnum>,
    /// Number of poles (live lines)
    pub number_of_poles: Option<i32>,
    /// Has protective earth connection
    pub has_protective_earth: Option<bool>,
    /// Insulation standard class (IEC 61140)
    pub insulation_standard_class: Option<InsulationStandardClass>,
    /// IP Code (IEC 60529) - e.g., "IP65"
    pub ip_code: Option<String>,
    /// IK Code (IEC 62262) - mechanical impact protection, e.g., "IK08"
    pub ik_code: Option<String>,
    /// Earthing style description
    pub earthing_style: Option<String>,
    /// Heat dissipation in Watts
    pub heat_dissipation: Option<f64>,
    /// Actual power output in Watts
    pub power: Option<f64>,
    /// Nominal power consumption in Watts
    pub nominal_power_consumption: Option<f64>,
    /// Number of power supply ports
    pub number_of_power_supply_ports: Option<i32>,
}

/// Light fixture common properties for Pset_LightFixtureTypeCommon
#[derive(Debug, Clone, Default)]
pub struct LightFixtureCommonProperties {
    /// Reference identifier
    pub reference: Option<String>,
    /// Status (NEW, EXISTING, DEMOLISH, TEMPORARY)
    pub status: Option<String>,
    /// Number of light sources
    pub number_of_sources: Option<i32>,
    /// Total wattage in Watts
    pub total_wattage: Option<f64>,
    /// Mounting type (SURFACE, RECESSED, SUSPENDED, POLE, etc.)
    pub mounting_type: Option<String>,
    /// Placing type (CEILING, FLOOR, WALL, etc.)
    pub placing_type: Option<String>,
    /// Maintenance factor (0.0-1.0)
    pub maintenance_factor: Option<f64>,
    /// Maximum plenum sensible load in Watts
    pub max_plenum_sensible_load: Option<f64>,
    /// Maximum space sensible load in Watts
    pub max_space_sensible_load: Option<f64>,
    /// Ratio of sensible load to radiant (0.0-1.0)
    pub sensible_load_to_radiant: Option<f64>,
}

/// COBie manufacturer type information for Pset_ManufacturerTypeInformation
#[derive(Debug, Clone, Default)]
pub struct ManufacturerTypeInfo {
    /// Global trade item number (GTIN/EAN/UPC)
    pub global_trade_item_number: Option<String>,
    /// Article number / product code
    pub article_number: Option<String>,
    /// Model reference
    pub model_reference: Option<String>,
    /// Model label
    pub model_label: Option<String>,
    /// Manufacturer name
    pub manufacturer: Option<String>,
    /// Production year
    pub production_year: Option<i32>,
    /// Assembly place
    pub assembly_place: Option<String>,
}

/// COBie warranty information for Pset_Warranty
#[derive(Debug, Clone, Default)]
pub struct WarrantyInfo {
    /// Warranty identifier
    pub warranty_identifier: Option<String>,
    /// Warranty start date (ISO 8601)
    pub warranty_start_date: Option<String>,
    /// Warranty end date (ISO 8601)
    pub warranty_end_date: Option<String>,
    /// Warranty period in years
    pub warranty_period: Option<f64>,
    /// Point of contact
    pub point_of_contact: Option<String>,
    /// Terms and conditions
    pub terms_and_conditions: Option<String>,
    /// Exclusions
    pub exclusions: Option<String>,
}

/// COBie service life information for Pset_ServiceLife
#[derive(Debug, Clone, Default)]
pub struct ServiceLifeInfo {
    /// Service life duration in years
    pub service_life_duration: Option<f64>,
    /// Service life type (ACTUALSERVICELIFE, EXPECTEDSERVICELIFE, etc.)
    pub service_life_type: Option<String>,
    /// Mean time between failures in hours
    pub mean_time_between_failure: Option<f64>,
}

/// Distribution port flow direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FlowDirectionEnum {
    /// Incoming flow (power input)
    Source,
    /// Outgoing flow (power output)
    Sink,
    /// Bidirectional flow
    SourceAndSink,
    #[default]
    NotDefined,
}

impl FlowDirectionEnum {
    pub fn to_step(&self) -> &'static str {
        match self {
            Self::Source => ".SOURCE.",
            Self::Sink => ".SINK.",
            Self::SourceAndSink => ".SOURCEANDSINK.",
            Self::NotDefined => ".NOTDEFINED.",
        }
    }
}

/// Distribution system type for electrical systems
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DistributionSystemEnum {
    Electrical,
    Lighting,
    LightningProtection,
    PowerGeneration,
    #[default]
    NotDefined,
}

impl DistributionSystemEnum {
    pub fn to_step(&self) -> &'static str {
        match self {
            Self::Electrical => ".ELECTRICAL.",
            Self::Lighting => ".LIGHTING.",
            Self::LightningProtection => ".LIGHTNINGPROTECTION.",
            Self::PowerGeneration => ".POWERGENERATION.",
            Self::NotDefined => ".NOTDEFINED.",
        }
    }
}
