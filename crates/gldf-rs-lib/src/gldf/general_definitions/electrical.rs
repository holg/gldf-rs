#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

use serde::{Deserialize, Serialize};

use super::lightsources::LightSourceReference;
use super::LocaleFoo;

/// Represents a range of voltage values in the GLDF data structure.
///
/// This struct defines a range of voltage values, including a minimum and maximum value.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoltageRange {
    /// The minimum voltage value in the range.
    #[serde(rename = "Min", default)]
    pub min: f64,

    /// The maximum voltage value in the range.
    #[serde(rename = "Max", default)]
    pub max: f64,
}

/// Enum representing different frequency options.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Frequency {
    /// Represents a frequency of 50 Hertz (Hz).
    #[default]
    #[serde(rename = "50")]
    Hz50,

    /// Represents a frequency of 60 Hertz (Hz).
    #[serde(rename = "60")]
    Hz60,

    /// Represents a frequency of 50/60 Hertz (Hz).
    #[serde(rename = "50/60")]
    Hz50_60,

    /// Represents a frequency of 400 Hertz (Hz).
    #[serde(rename = "400")]
    Hz400,
}

/// The enum of current types
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CurrentType {
    /// Represents an alternating current (AC) type.
    #[default]
    AC,
    /// Represents a direct current (DC) type.
    DC,
    /// Represents an unidirectional current (UC) type.
    UC,
}

/// Represents voltage information in the GLDF data structure.
///
/// This struct defines voltage information, including a voltage range, a fixed voltage value,
/// a voltage type, and a frequency.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Voltage {
    /// The voltage range (optional - either VoltageRange or FixedVoltage is used)
    #[serde(
        rename = "VoltageRange",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub voltage_range: Option<VoltageRange>,

    /// The fixed voltage (optional - either VoltageRange or FixedVoltage is used)
    #[serde(
        rename = "FixedVoltage",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub fixed_voltage: Option<f64>,

    /// The current type (AC, DC, UC)
    #[serde(rename = "Type", default)]
    pub type_attr: CurrentType,

    /// The frequency of the AC voltage
    #[serde(rename = "Frequency", default)]
    pub frequency: Frequency,
}

/// Represents a range of power values for a light source.
///
/// The `PowerRange` struct models a range of power values for a light source within the GLDF file.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerRange {
    /// The lower bound of the power range.
    #[serde(rename = "Lower", default)]
    pub lower: f64,

    /// The upper bound of the power range.
    #[serde(rename = "Upper", default)]
    pub upper: f64,

    /// The default light source power.
    #[serde(rename = "DefaultLightSourcePower", default)]
    pub default_light_source_power: f64,
}

/// Definition of energy efficiency classes
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnergyLabel {
    /// The region of which the energy label is valid (e.g. Germany)
    #[serde(rename = "@region")]
    pub region: String,

    /// The value of the energy label (e.g. A++)
    #[serde(rename = "$text")]
    pub value: String,
}

/// Represents a collection of energy labels in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnergyLabels {
    /// A list of valid energy labels
    #[serde(rename = "EnergyLabel", default)]
    pub energy_label: Vec<EnergyLabel>,
}

/// Represents a collection of interfaces in the GLDF data structure.
///
/// Interfaces are defined based on the ISO 7127 standard terminology.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interfaces {
    /// The list of interfaces associated with the luminaire
    #[serde(rename = "Interface", default)]
    pub interface: Vec<String>,
}

/// Represents control gear information in the GLDF data structure.
///
/// This struct defines the properties of control gear, which includes components that
/// regulate and control the operation of luminaires.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlGear {
    /// The identifier for the control gear.
    #[serde(rename = "@id", default)]
    pub id: String,

    /// The localized name of the control gear.
    #[serde(rename = "Name", default)]
    pub name: LocaleFoo,

    /// The localized description of the control gear.
    #[serde(
        rename = "Description",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<LocaleFoo>,

    /// The nominal voltage of the control gear.
    #[serde(
        rename = "NominalVoltage",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub nominal_voltage: Option<Voltage>,

    /// The standby power consumption of the control gear.
    #[serde(
        rename = "StandbyPower",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub standby_power: Option<f64>,

    /// The power level at which constant light output starts.
    #[serde(
        rename = "ConstantLightOutputStartPower",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub constant_light_output_start_power: Option<f64>,

    /// The power level at which constant light output ends.
    #[serde(
        rename = "ConstantLightOutputEndPower",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub constant_light_output_end_power: Option<f64>,

    /// Power consumption controls associated with the control gear.
    #[serde(
        rename = "PowerConsumptionControls",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub power_consumption_controls: Option<f64>,

    /// Whether the control gear is dimmable.
    #[serde(rename = "Dimmable", default, skip_serializing_if = "Option::is_none")]
    pub dimmable: Option<bool>,

    /// Whether the control gear is color controllable.
    #[serde(
        rename = "ColorControllable",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub color_controllable: Option<bool>,

    /// Interfaces supported by the control gear.
    #[serde(
        rename = "Interfaces",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub interfaces: Option<Interfaces>,

    /// Energy labels associated with the control gear.
    #[serde(
        rename = "EnergyLabels",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub energy_labels: Option<EnergyLabels>,
}

/// Represents a collection of control gears in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlGears {
    /// The Vector of control gears.
    #[serde(rename = "ControlGear", default)]
    pub control_gear: Vec<ControlGear>,
}

/// Represents a reference to a control gear in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlGearReference {
    /// The ID of the referenced control gear.
    #[serde(rename = "@controlGearId")]
    pub control_gear_id: String,

    /// The count of control gears associated with this reference.
    #[serde(rename = "@controlGearCount", skip_serializing_if = "Option::is_none")]
    pub control_gear_count: Option<i32>,
}

/// Represents equipment data in the GLDF data structure.
///
/// This struct defines the properties of equipment associated with a luminaire.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Equipment {
    /// The unique identifier for the equipment.
    #[serde(rename = "@id", default)]
    pub id: String,

    /// A reference to the light source associated with the equipment.
    #[serde(
        rename = "LightSourceReference",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub light_source_reference: Option<LightSourceReference>,

    /// A reference to the control gear associated with the equipment.
    #[serde(
        rename = "ControlGearReference",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub control_gear_reference: Option<ControlGearReference>,

    /// The rated input power of the equipment.
    #[serde(rename = "RatedInputPower", default)]
    pub rated_input_power: f64,

    /// The emergency ballast lumen factor of the equipment.
    #[serde(
        rename = "EmergencyBallastLumenFactor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub emergency_ballast_lumen_factor: Option<f64>,

    /// The emergency rated luminous flux of the equipment.
    #[serde(
        rename = "EmergencyRatedLuminousFlux",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub emergency_rated_luminous_flux: Option<i32>,
}

/// Represents a collection of equipment data in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Equipments {
    /// The list of equipment items.
    #[serde(rename = "Equipment", default)]
    pub equipment: Vec<Equipment>,
}

/// Represents a reference to an equipment in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentReference {
    /// The unique identifier of the referenced equipment.
    #[serde(rename = "@equipmentId")]
    pub equipment_id: String,
}
