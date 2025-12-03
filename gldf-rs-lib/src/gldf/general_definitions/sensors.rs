#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

use serde::{Deserialize, Serialize};

/// Represents a reference to a sensor file.
///
/// The `SensorFileReference` struct models a reference to a sensor file within the GLDF file.
/// It includes the ID of the referenced file. It supports serialization and deserialization
/// of XML data for working with sensor file references.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorFileReference {
    /// The ID of the referenced file.
    #[serde(rename = "@fileId")]
    pub file_id: String,
}

/// Represents the characteristics of a detector.
///
/// The `DetectorCharacteristics` struct models the characteristics of a detector within the GLDF file.
/// It includes a list of detector characteristic strings. It supports serialization and deserialization
/// of XML data for working with detector characteristics.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectorCharacteristics {
    /// A list of detector characteristic strings.
    #[serde(rename = "DetectorCharacteristic", default)]
    pub detector_characteristic: Vec<String>,
}

/// Represents the methods of detection.
///
/// The `DetectionMethods` struct models the methods of detection within the GLDF file.
/// It includes a list of detection method strings. It supports serialization and deserialization
/// of XML data for working with detection methods.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectionMethods {
    /// A list of detection method strings.
    #[serde(rename = "DetectionMethod", default)]
    pub detection_method: Vec<String>,
}

/// Represents the types of detectors.
///
/// The `DetectorTypes` struct models the types of detectors within the GLDF file.
/// It includes a list of detector type strings. It supports serialization and deserialization
/// of XML data for working with detector types.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectorTypes {
    /// A list of detector type strings.
    #[serde(rename = "DetectorType", default)]
    pub detector_type: Vec<String>,
}

/// Represents a sensor.
///
/// The `Sensor` struct models a sensor within the GLDF file. It includes an ID, sensor file reference,
/// detector characteristics, detection methods, and detector types. It supports serialization and
/// deserialization of XML data for working with sensor definitions.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sensor {
    /// The ID of the sensor.
    #[serde(rename = "@id")]
    pub id: String,

    /// A reference to the sensor file associated with the sensor.
    #[serde(rename = "SensorFileReference", skip_serializing_if = "Option::is_none")]
    pub sensor_file_reference: Option<SensorFileReference>,

    /// The characteristics of the detector.
    #[serde(rename = "DetectorCharacteristics", skip_serializing_if = "Option::is_none")]
    pub detector_characteristics: Option<DetectorCharacteristics>,

    /// The methods of detection.
    #[serde(rename = "DetectionMethods", skip_serializing_if = "Option::is_none")]
    pub detection_methods: Option<DetectionMethods>,

    /// The types of detectors.
    #[serde(rename = "DetectorTypes", skip_serializing_if = "Option::is_none")]
    pub detector_types: Option<DetectorTypes>,
}

/// Represents a collection of sensor definitions.
///
/// The `Sensors` struct models a collection of sensor definitions within the GLDF file.
/// It contains a list of individual `Sensor` instances. It supports serialization and deserialization
/// of XML data for working with sensor collections.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sensors {
    /// The list of sensor definitions.
    #[serde(rename = "Sensor", default)]
    pub sensor: Vec<Sensor>,
}
