#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

use serde::{Deserialize, Serialize};

use super::lightsources::{CircularEmitter, RectangularEmitter};

/// Represents rotation data in the GLDF data structure.
///
/// This struct defines the rotation values around the X, Y, and Z axes, as well as the global
/// rotation value G0. These values describe the orientation of a luminaire or its components.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rotation {
    /// The rotation value around the X-axis.
    #[serde(rename = "X")]
    pub x: i32,

    /// The rotation value around the Y-axis.
    #[serde(rename = "Y")]
    pub y: i32,

    /// The rotation value around the Z-axis.
    #[serde(rename = "Z")]
    pub z: i32,

    /// The global rotation value G0.
    #[serde(rename = "G0")]
    pub g0: i32,
}

/// Represents a cuboid geometry in the GLDF data structure.
///
/// This struct defines the properties of a cuboid geometry, including its width,
/// length, and height dimensions.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cuboid {
    /// The width values for the cuboid.
    #[serde(rename = "Width", default)]
    pub width: Vec<i32>,

    /// The length values for the cuboid.
    #[serde(rename = "Length", default)]
    pub length: Vec<i32>,

    /// The height values for the cuboid.
    #[serde(rename = "Height", default)]
    pub height: Vec<i32>,
}

/// Represents a cylindrical geometry in the GLDF data structure.
///
/// This struct defines the properties of a cylindrical geometry, including its plane,
/// diameter, and height dimensions.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cylinder {
    /// The plane of the cylinder.
    #[serde(rename = "@plane")]
    pub plane: String,

    /// The diameter values for the cylinder.
    #[serde(rename = "Diameter", default)]
    pub diameter: Vec<i32>,

    /// The height values for the cylinder.
    #[serde(rename = "Height", default)]
    pub height: Vec<String>,
}

/// Represents the heights of different orientations in the C plane in the GLDF data structure.
///
/// This struct holds arrays of height values corresponding to different orientations in the C plane.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CHeights {
    /// Heights for orientation 0 degrees in the C plane.
    #[serde(rename = "C0", default)]
    pub c0: Vec<i32>,

    /// Heights for orientation 90 degrees in the C plane.
    #[serde(rename = "C90", default)]
    pub c90: Vec<i32>,

    /// Heights for orientation 180 degrees in the C plane.
    #[serde(rename = "C180", default)]
    pub c180: Vec<i32>,

    /// Heights for orientation 270 degrees in the C plane.
    #[serde(rename = "C270", default)]
    pub c270: Vec<i32>,
}

/// Represents simple geometry properties in the GLDF data structure.
///
/// This struct defines various simple geometric shapes and their dimensions, such as cuboids, cylinders,
/// rectangular emitters, circular emitters, and C-plane heights.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimpleGeometry {
    /// The ID of the simple geometry.
    #[serde(rename = "@id")]
    pub id: String,

    /// Cuboid geometry information.
    #[serde(rename = "Cuboid", default)]
    pub cuboid: Vec<Cuboid>,

    /// Cylinder geometry information.
    #[serde(rename = "Cylinder", default)]
    pub cylinder: Vec<Cylinder>,

    /// Rectangular emitter geometry information.
    #[serde(rename = "RectangularEmitter", default)]
    pub rectangular_emitter: Vec<RectangularEmitter>,

    /// Circular emitter geometry information.
    #[serde(rename = "CircularEmitter", default)]
    pub circular_emitter: Vec<CircularEmitter>,

    /// C-plane heights information.
    #[serde(rename = "C-Heights", default)]
    pub c_heights: Vec<CHeights>,
}

/// Represents a reference to a geometry file in the GLDF data structure.
///
/// This struct holds information about a reference to a geometry file, including the file's ID
/// and an optional level of detail specification.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometryFileReference {
    /// The ID of the geometry file.
    #[serde(rename = "@fileId")]
    pub file_id: String,

    /// The level of detail specification for the geometry file reference.
    #[serde(rename = "@levelOfDetail", skip_serializing_if = "Option::is_none")]
    pub level_of_detail: Option<String>,
}

/// Represents model geometry in the GLDF data structure.
///
/// This struct defines the properties of model geometry, including its ID and references to
/// geometry files that define the shape and details of the model.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelGeometry {
    /// The ID of the model geometry.
    #[serde(rename = "@id")]
    pub id: String,

    /// References to geometry files that define the model's shape and details.
    #[serde(rename = "GeometryFileReference", default)]
    pub geometry_file_reference: Vec<GeometryFileReference>,
}

/// Represents a collection of geometries in the GLDF data structure.
///
/// This struct holds both simple and model geometries that define the shapes and details of luminaires.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Geometries {
    /// Simple geometries that define various shapes, dimensions, and details.
    #[serde(rename = "SimpleGeometry", default)]
    pub simple_geometry: Vec<SimpleGeometry>,

    /// Model geometries that reference external geometry files for shape and details.
    #[serde(rename = "ModelGeometry", default)]
    pub model_geometry: Vec<ModelGeometry>,
}
