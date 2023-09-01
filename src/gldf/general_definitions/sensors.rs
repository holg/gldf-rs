#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
use serde::{Serialize};
use serde::Deserialize;
use yaserde_derive::YaDeserialize;
use yaserde_derive::YaSerialize;

/// Example of how to construct a `Files` instance:
///
/// ```
/// use gldf_rs::gldf::{Files, File};
///
/// let file_entries = Files {
///     file: vec![
///         File {
///             id: "file123".to_string(),
///             content_type: "image/jpeg".to_string(),
///             type_attr: "Thumbnail".to_string(),
///             file_name: "thumbnail.jpg".to_string(),
///         },
///         // ... (add more file entries as needed)
///     ],
/// };
/// ```


/// Represents a reference to a sensor file.
///
/// The `SensorFileReference` struct models a reference to a sensor file within the GLDF file.
/// It includes the ID of the referenced file. It supports serialization and deserialization
/// of XML data for working with sensor file references.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SensorFileReference {
    /// The ID of the referenced file.
    #[yaserde(rename = "fileId")]
    #[serde(rename = "fileId")]
    pub file_id: String,
}

/// Represents the characteristics of a detector.
///
/// The `DetectorCharacteristics` struct models the characteristics of a detector within the GLDF file.
/// It includes a list of detector characteristic strings. It supports serialization and deserialization
/// of XML data for working with detector characteristics.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DetectorCharacteristics {
    /// A list of detector characteristic strings.
    #[yaserde(rename = "DetectorCharacteristic")]
    #[serde(rename = "DetectorCharacteristic")]
    pub detector_characteristic: Vec<String>,
}

/// Represents the methods of detection.
///
/// The `DetectionMethods` struct models the methods of detection within the GLDF file.
/// It includes a list of detection method strings. It supports serialization and deserialization
/// of XML data for working with detection methods.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DetectionMethods {
    /// A list of detection method strings.
    #[yaserde(rename = "DetectionMethod")]
    #[serde(rename = "DetectionMethod")]
    pub detection_method: Vec<String>,
}

/// Represents the types of detectors.
///
/// The `DetectorTypes` struct models the types of detectors within the GLDF file.
/// It includes a list of detector type strings. It supports serialization and deserialization
/// of XML data for working with detector types.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct DetectorTypes {
    /// A list of detector type strings.
    #[yaserde(rename = "DetectorType")]
    #[serde(rename = "DetectorType")]
    pub detector_type: Vec<String>,
}

/// Represents a sensor.
///
/// The `Sensor` struct models a sensor within the GLDF file. It includes an ID, sensor file reference,
/// detector characteristics, detection methods, and detector types. It supports serialization and
/// deserialization of XML data for working with sensor definitions.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Sensor {
    /// The ID of the sensor.
    #[yaserde(rename = "id")]
    #[serde(rename = "id")]
    pub id: String,

    /// A reference to the sensor file associated with the sensor.
    #[yaserde(child)]
    #[yaserde(rename = "SensorFileReference")]
    #[serde(rename = "SensorFileReference", skip_serializing_if = "Option::is_none")]
    pub sensor_file_reference: Option<SensorFileReference>,

    /// The characteristics of the detector.
    #[yaserde(child)]
    #[yaserde(rename = "DetectorCharacteristics")]
    #[serde(rename = "DetectorCharacteristics", skip_serializing_if = "Option::is_none")]
    pub detector_characteristics: Option<DetectorCharacteristics>,

    /// The methods of detection.
    #[yaserde(child)]
    #[yaserde(rename = "DetectionMethods")]
    #[serde(rename = "DetectionMethods", skip_serializing_if = "Option::is_none")]
    pub detection_methods: Option<DetectionMethods>,

    /// The types of detectors.
    #[yaserde(child)]
    #[yaserde(rename = "DetectorTypes")]
    #[serde(rename = "DetectorTypes", skip_serializing_if = "Option::is_none")]
    pub detector_types: Option<DetectorTypes>,
}

/// Represents a collection of sensor definitions.
///
/// The `Sensors` struct models a collection of sensor definitions within the GLDF file.
/// It contains a list of individual `Sensor` instances. It supports serialization and deserialization
/// of XML data for working with sensor collections.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Sensors {
    /// The list of sensor definitions.
    #[yaserde(child)]
    #[yaserde(rename = "Sensor")]
    #[serde(rename = "Sensor")]
    pub sensor: Vec<Sensor>,
}
