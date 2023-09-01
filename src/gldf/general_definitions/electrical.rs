#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
pub use super::*;
use serde::{Serialize};
use serde::Deserialize;
use yaserde_derive::YaDeserialize;
use yaserde_derive::YaSerialize;


/// Represents a range of voltage values in the GLDF data structure.
///
/// This struct defines a range of voltage values, including a minimum and maximum value.
/// The voltage range can be used to specify the acceptable voltage levels for a luminaire
/// or its components.
///
/// # Examples
///
/// ```
/// use gldf_rs::gldf::VoltageRange;
///
/// let voltage_range = VoltageRange {
///     min: 100.0,
///     max: 240.0,
/// };
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct VoltageRange {
    /// The minimum voltage value in the range.
    #[yaserde(rename = "Min")]
    #[serde(rename = "Min")]
    pub min: f64,

    /// The maximum voltage value in the range.
    #[yaserde(rename = "Max")]
    #[serde(rename = "Max")]
    pub max: f64,
}

/// Enum representing different frequency options.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
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
/// Represents voltage information in the GLDF data structure.
///
/// This struct defines voltage information, including a voltage range, a fixed voltage value,
/// a voltage type, and a frequency. It provides comprehensive details about the voltage characteristics
/// relevant to a luminaire or its components.
///
/// # Examples
///
/// ```
/// use gldf_rs::gldf::{Voltage, VoltageRange, CurrentType, Frequency};
///
/// let voltage = Voltage {
///     voltage_range: VoltageRange {
///         min: 100.0,
///         max: 240.0,
///     },
///     fixed_voltage: 220.0,
///     type_attr: CurrentType::AC,
///     frequency: Frequency::Hz50
/// };
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Voltage {
    #[yaserde(rename = "VoltageRange")]
    #[serde(rename = "VoltageRange")]
    /// the voltage range
    pub voltage_range: VoltageRange,
    #[yaserde(rename = "FixedVoltage")]
    #[serde(rename = "FixedVoltage")]
    /// the fixed voltage
    pub fixed_voltage: f64,
    #[yaserde(rename = "Type")]
    #[serde(rename = "Type")]
    /// the currnt type (AC, DC, UC)
    pub type_attr: CurrentType,
    #[yaserde(rename = "Frequency")]
    #[serde(rename = "Frequency")]
    /// the frequency of the AC voltage
    pub frequency: Frequency,
}

/// the enum of curennt types
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub enum CurrentType {
    /// Represents an alternating current (AC) type.
    #[default]
    AC,

    /// Represents a direct current (DC) type.
    DC,

    /// Represents an unidirectional current (UC) type.
    UC,
}
/// Represents a range of power values for a light source.
///
/// The `PowerRange` struct models a range of power values for a light source within the GLDF file.
/// It includes the lower and upper bounds of the power range, as well as the default light source
/// power. It supports serialization and deserialization of XML data for working with power ranges.
/// /// Example of how to construct a `PowerRange` instance:
// /// ```
// /// use gldf_rs::gldf::PowerRange;
// ///
// /// let power_range = PowerRange {
// ///     lower: 0.0,
// ///     upper: 100.0,
// ///     default_light_source_power: 50.0,
// /// };
// /// ```
// ///
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct PowerRange {
    /// The lower bound of the power range.
    #[yaserde(rename = "Lower")]
    #[serde(rename = "Lower")]
    pub lower: f64,

    /// The upper bound of the power range.
    #[yaserde(rename = "Upper")]
    #[serde(rename = "Upper")]
    pub upper: f64,

    /// The default light source power.
    #[yaserde(rename = "DefaultLightSourcePower")]
    #[serde(rename = "DefaultLightSourcePower")]
    pub default_light_source_power: f64,
}

/// Definition of energy efficiency classes
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EnergyLabel {
    #[yaserde(rename = "region")]
    #[serde(rename = "region")]
    /// the region of which the energy label is valid (e.g. Germany)
    pub region: String,
    #[yaserde(rename = "$value")]
    /// the value of the energy label (e.g. A++)
    pub value: String,
}

/// Represents a collection of energy labels in the GLDF data structure.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EnergyLabels {
    #[yaserde(rename = "EnergyLabel")]
    #[serde(rename = "EnergyLabel")]
    /// a list of valid energy labels
    pub energy_label: Vec<EnergyLabel>,
}

/// Represents a collection of interfaces in the GLDF data structure.
///
/// Interfaces are defined based on the ISO 7127 standard terminology. This struct
/// holds a list of interface names associated with a luminaire, which allow the
/// luminaire to interact with external systems.
///
/// ISO 7127: This standard defines terminology relating to the functions of luminaires
/// and provides a common basis for communication between manufacturers, designers,
/// and users of luminaires.
///
/// Example:
/// ```
/// use gldf_rs::gldf::Interfaces;
///
/// let interfaces_data = Interfaces {
///     interface: vec!["DALI".to_string(), "DMX".to_string()],
/// };
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Interfaces {
    /// A vector of interface names associated with the luminaire.
    ///
    /// Interface names represent the various connectivity and communication options
    /// available for the luminaire. Examples of interfaces include DALI, Bluetooth,
    /// Wi-Fi, etc.
    #[yaserde(rename = "Interface")]
    #[serde(rename = "Interface")]
    /// the list of interfaces associated with the luminaire
    pub interface: Vec<String>,
}


/// Represents control gear information in the GLDF data structure.
///
/// This struct defines the properties of control gear, which includes components that
/// regulate and control the operation of luminaires. Control gear can affect factors
/// like light output, power consumption, and dimming behavior.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ControlGear {
    /// The identifier for the control gear.
    #[yaserde(attribute, rename = "id")]
    #[serde(rename = "@id")]
    pub id: String,
    /// The localized name of the control gear.
    #[yaserde(rename = "Name")]
    #[serde(rename = "Name")]
    pub name: LocaleFoo,
    /// The localized description of the control gear.
    #[yaserde(child)]
    #[yaserde(rename = "Description")]
    #[serde(rename = "Description")]
    pub description: LocaleFoo,
    /// The nominal voltage of the control gear.
    #[yaserde(child)]
    #[yaserde(rename = "NominalVoltage")]
    #[serde(rename = "NominalVoltage", skip_serializing_if = "Option::is_none")]
    pub nominal_voltage: Option<Voltage>,
    /// The standby power consumption of the control gear.
    #[yaserde(rename = "StandbyPower")]
    #[serde(rename = "StandbyPower", skip_serializing_if = "Option::is_none")]
    pub standby_power: Option<f64>,
    /// The power level at which constant light output starts.
    #[yaserde(rename = "ConstantLightOutputStartPower")]
    #[serde(rename = "ConstantLightOutputStartPower", skip_serializing_if = "Option::is_none")]
    pub constant_light_output_start_power: Option<f64>,
    /// The power level at which constant light output ends.
    #[yaserde(rename = "ConstantLightOutputEndPower")]
    #[serde(rename = "ConstantLightOutputEndPower", skip_serializing_if = "Option::is_none")]
    pub constant_light_output_end_power: Option<f64>,
    /// Power consumption controls associated with the control gear.
    #[yaserde(rename = "PowerConsumptionControls")]
    #[serde(rename = "PowerConsumptionControls", skip_serializing_if = "Option::is_none")]
    pub power_consumption_controls: Option<f64>,
    /// Whether the control gear is dimmable.
    #[yaserde(rename = "Dimmable")]
    #[serde(rename = "Dimmable", skip_serializing_if = "Option::is_none")]
    pub dimmable: Option<bool>,
    /// Whether the control gear is color controllable.
    #[yaserde(rename = "ColorControllable")]
    #[serde(rename = "ColorControllable", skip_serializing_if = "Option::is_none")]
    pub color_controllable: Option<bool>,
    /// Interfaces supported by the control gear.
    #[yaserde(child)]
    #[yaserde(rename = "Interfaces")]
    #[serde(rename = "Interfaces")]
    pub interfaces: Interfaces,
    /// Energy labels associated with the control gear.
    #[yaserde(child)]
    #[yaserde(rename = "EnergyLabels")]
    #[serde(rename = "EnergyLabels", skip_serializing_if = "Option::is_none")]
    pub energy_labels: Option<EnergyLabels>,
}

/// Represents a collection of control gears in the GLDF data structure.
///
/// This struct holds a list of control gears that are part of a luminaire.
/// Control gears are components that regulate and control the operation of luminaires,
/// affecting factors like light output, power consumption, and dimming behavior.
///
/// # Example
///
/// ```rust
/// use gldf_rs::ControlGears;
///
/// let mut control_gears = ControlGears::default();
/// // Add control gears to the collection...
///
/// assert_eq!(control_gears.control_gear.len(), 0);
/// ```
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ControlGears {
    #[yaserde(child)]
    #[yaserde(rename = "ControlGear")]
    #[serde(rename = "ControlGear")]
    /// The Vector of control gears.
    pub control_gear: Vec<ControlGear>,
}
/// Represents a reference to a control gear in the GLDF data structure.
///
/// This struct holds information about the control gears referenced by their IDs
/// and the associated control gear count.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ControlGearReference {
    /// The ID of the referenced control gear.
    #[yaserde(attribute, rename = "controlGearId")]
    #[serde(rename = "@controlGearId")]
    pub control_gear_id: String,

    /// The count of control gears associated with this reference.
    #[yaserde(rename = "controlGearCount")]
    #[serde(rename = "controlGearCount", skip_serializing_if = "Option::is_none")]
    pub control_gear_count: Option<i32>,
}

/// Represents equipment data in the GLDF data structure.
///
/// This struct defines the properties of equipment associated with a luminaire,
/// including light source and control gear references, rated input power,
/// emergency ballast lumen factor, and emergency rated luminous flux.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Equipment {
    /// The unique identifier for the equipment.
    #[yaserde(rename = "id")]
    #[serde(rename = "id")]
    pub id: String,

    /// A reference to the light source associated with the equipment.
    #[yaserde(rename = "LightSourceReference")]
    #[serde(rename = "LightSourceReference")]
    pub light_source_reference: LightSourceReference,

    /// A reference to the control gear associated with the equipment.
    #[yaserde(rename = "ControlGearReference")]
    #[serde(rename = "ControlGearReference")]
    pub control_gear_reference: ControlGearReference,

    /// The rated input power of the equipment.
    #[yaserde(rename = "RatedInputPower")]
    #[serde(rename = "RatedInputPower")]
    pub rated_input_power: f64,

    /// The emergency ballast lumen factor of the equipment.
    #[yaserde(rename = "EmergencyBallastLumenFactor")]
    #[serde(rename = "EmergencyBallastLumenFactor")]
    pub emergency_ballast_lumen_factor: f64,

    /// The emergency rated luminous flux of the equipment.
    #[yaserde(rename = "EmergencyRatedLuminousFlux")]
    #[serde(rename = "EmergencyRatedLuminousFlux")]
    pub emergency_rated_luminous_flux: i32,
}

/// Represents a collection of equipment data in the GLDF data structure.
///
/// This struct holds a list of equipment items associated with a luminaire.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Equipments {
    /// The list of equipment items.
    #[yaserde(rename = "Equipment")]
    #[serde(rename = "Equipment")]
    pub equipment: Vec<Equipment>,
}

/// Represents a reference to an equipment in the GLDF data structure.
///
/// This struct defines a reference to an equipment using its unique identifier.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct EquipmentReference {
    /// The unique identifier of the referenced equipment.
    #[yaserde(rename = "equipmentId")]
    #[serde(rename = "equipmentId")]
    pub equipment_id: String,
}
