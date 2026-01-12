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
