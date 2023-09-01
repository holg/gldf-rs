#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
use serde::{Serialize};
use serde::Deserialize;
use yaserde_derive::YaDeserialize;
use yaserde_derive::YaSerialize;
pub use super::*;

/// Represents rotation data in the GLDF data structure.
///
/// This struct defines the rotation values around the X, Y, and Z axes, as well as the global
/// rotation value G0. These values describe the orientation of a luminaire or its components.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Rotation {
    /// The rotation value around the X-axis.
    #[yaserde(rename = "X")]
    #[serde(rename = "X")]
    pub x: i32,

    /// The rotation value around the Y-axis.
    #[yaserde(rename = "Y")]
    #[serde(rename = "Y")]
    pub y: i32,

    /// The rotation value around the Z-axis.
    #[yaserde(rename = "Z")]
    #[serde(rename = "Z")]
    pub z: i32,

    /// The global rotation value G0.
    #[yaserde(rename = "G0")]
    #[serde(rename = "G0")]
    pub g0: i32,
}

/// Represents a cuboid geometry in the GLDF data structure.
///
/// This struct defines the properties of a cuboid geometry, including its width,
/// length, and height dimensions.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Cuboid {
    /// The width values for the cuboid.
    #[yaserde(rename = "Width")]
    #[serde(rename = "Width")]
    pub width: Vec<i32>,

    /// The length values for the cuboid.
    #[yaserde(rename = "Length")]
    #[serde(rename = "Length")]
    pub length: Vec<i32>,

    /// The height values for the cuboid.
    #[yaserde(rename = "Height")]
    #[serde(rename = "Height")]
    pub height: Vec<i32>,
}

/// Represents a cylindrical geometry in the GLDF data structure.
///
/// This struct defines the properties of a cylindrical geometry, including its plane,
/// diameter, and height dimensions.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Cylinder {
    /// The plane of the cylinder.
    #[yaserde(rename = "plane")]
    #[serde(rename = "plane")]
    pub plane: String,

    /// The diameter values for the cylinder.
    #[yaserde(rename = "Diameter")]
    #[serde(rename = "Diameter")]
    pub diameter: Vec<i32>,

    /// The height values for the cylinder.
    #[yaserde(rename = "Height")]
    #[serde(rename = "Height")]
    pub height: Vec<String>,
}

/// Represents the heights of different orientations in the C plane in the GLDF data structure.
///
/// This struct holds arrays of height values corresponding to different orientations in the C plane.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct CHeights {
    /// Heights for orientation 0 degrees in the C plane.
    #[yaserde(rename = "C0")]
    #[serde(rename = "C0")]
    pub c0: Vec<i32>,

    /// Heights for orientation 90 degrees in the C plane.
    #[yaserde(rename = "C90")]
    #[serde(rename = "C90")]
    pub c90: Vec<i32>,

    /// Heights for orientation 180 degrees in the C plane.
    #[yaserde(rename = "C180")]
    #[serde(rename = "C180")]
    pub c180: Vec<i32>,

    /// Heights for orientation 270 degrees in the C plane.
    #[yaserde(rename = "C270")]
    #[serde(rename = "C270")]
    pub c270: Vec<i32>,
}

/// Represents simple geometry properties in the GLDF data structure.
///
/// This struct defines various simple geometric shapes and their dimensions, such as cuboids, cylinders,
/// rectangular emitters, circular emitters, and C-plane heights.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct SimpleGeometry {
    /// The ID of the simple geometry.
    #[yaserde(attribute, rename = "id")]
    #[serde(rename = "@id")]
    pub id: String,

    /// Cuboid geometry information.
    #[yaserde(rename = "Cuboid")]
    #[serde(rename = "Cuboid")]
    pub cuboid: Vec<Cuboid>,

    /// Cylinder geometry information.
    #[yaserde(rename = "Cylinder")]
    #[serde(rename = "Cylinder")]
    pub cylinder: Vec<Cylinder>,

    /// Rectangular emitter geometry information.
    #[yaserde(rename = "RectangularEmitter")]
    #[serde(rename = "RectangularEmitter")]
    pub rectangular_emitter: Vec<RectangularEmitter>,

    /// Circular emitter geometry information.
    #[yaserde(rename = "CircularEmitter")]
    #[serde(rename = "CircularEmitter")]
    pub circular_emitter: Vec<CircularEmitter>,

    /// C-plane heights information.
    #[yaserde(rename = "C-Heights")]
    pub c_heights: Vec<CHeights>,
}

/// Represents a reference to a geometry file in the GLDF data structure.
///
/// This struct holds information about a reference to a geometry file, including the file's ID
/// and an optional level of detail specification.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct GeometryFileReference {
    /// The ID of the geometry file.
    #[yaserde(attribute, rename = "fileId")]
    #[serde(rename = "@fileId")]
    pub file_id: String,

    /// The level of detail specification for the geometry file reference.
    #[yaserde(rename = "levelOfDetail")]
    #[serde(rename = "levelOfDetail", skip_serializing_if = "Option::is_none")]
    pub level_of_detail: Option<String>,
}

/// Represents model geometry in the GLDF data structure.
///
/// This struct defines the properties of model geometry, including its ID and references to
/// geometry files that define the shape and details of the model.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct ModelGeometry {
    /// The ID of the model geometry.
    #[yaserde(attribute, rename = "id")]
    #[serde(rename = "@id")]
    pub id: String,

    /// References to geometry files that define the model's shape and details.
    #[yaserde(rename = "GeometryFileReference")]
    #[serde(rename = "GeometryFileReference")]
    pub geometry_file_reference: Vec<GeometryFileReference>,
}

/// Represents a collection of geometries in the GLDF data structure.
///
/// This struct holds both simple and model geometries that define the shapes and details of luminaires.
#[derive(Default, Debug, Clone, PartialEq, YaDeserialize, YaSerialize, Serialize, Deserialize)]
pub struct Geometries {
    /// Simple geometries that define various shapes, dimensions, and details.
    #[yaserde(rename = "SimpleGeometry")]
    #[serde(rename = "SimpleGeometry")]
    pub simple_geometry: Vec<SimpleGeometry>,

    /// Model geometries that reference external geometry files for shape and details.
    #[yaserde(rename = "ModelGeometry")]
    #[serde(rename = "ModelGeometry")]
    pub model_geometry: Vec<ModelGeometry>,
}
