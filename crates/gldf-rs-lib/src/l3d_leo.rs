//! Flatten the Light Emitting Objects (LEOs / "Lichtaustritte") out of
//! an L3D archive into a simple, frontend-agnostic list of footprints.
//!
//! Available only when the `l3d` feature is enabled (which pulls in
//! `l3d_rs`). The WASM editor's graphical LEO→photometry assignment
//! editor uses [`extract_leos`] to draw a 2D plan view of every LEO so
//! the user can click one and bind a photometry (LVK) to it.
//!
//! # The chain
//!
//! ```text
//! .l3d ZIP bytes
//!   └─ l3d_rs::from_buffer → L3dFile.structure  (= raw structure.xml)
//!        └─ Luminaire::from_xml → typed tree
//!             └─ Structure.Geometry  (recurse: Geometry.Joints.Joint[].Geometries.Geometry[])
//!                  └─ Geometry.LightEmittingObjects.LightEmittingObject[]
//!                       └─ { partName, Position, Rotation, Rectangle | Circle }
//! ```
//!
//! # v1 limitation — raw positions
//!
//! LEO positions are taken verbatim from each `LightEmittingObject`'s
//! `Position`; joint offsets along the structure tree are **not**
//! composed. For the overwhelmingly common case (LEOs sharing a plane on
//! a single head) this is exactly right; for articulated multi-head
//! luminaires the footprints may overlap. Good enough for clickable
//! picking — exact world transforms are a later phase.
//!
//! # Soft failure
//!
//! `l3d_rs::from_buffer` swallows IO/zip errors (returning a default with
//! an empty `structure`), so a non-L3D or corrupt input yields
//! `Ok(vec![])` rather than an error. Only an XML that parses but is
//! structurally surprising would error.

use l3d_rs::{Geometry, Luminaire};

/// One Light Emitting Object's 2D footprint, ready for plan-view
/// rendering. Coordinates and sizes are in metres (L3D units).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LeoFootprint {
    /// L3D `@partName`, e.g. `"LEO0"`. This is the value a GLDF
    /// `EmitterReference.emitterObjectExternalName` binds to.
    pub part_name: String,
    /// Position relative to the parent geometry (raw, no joint compose).
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Rotation about Z in degrees — used to orient a rectangular
    /// footprint in the plan view.
    pub rot_z_deg: f32,
    /// The LEO's emitting surface shape.
    pub shape: LeoShape,
}

/// The shape of a LEO's emitting surface. A LEO carries either a
/// `Rectangle` or a `Circle`; `Point` covers the (rare) case where the
/// L3D declares neither, so the plan view can still draw a marker.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LeoShape {
    Rectangle { size_x: f64, size_y: f64 },
    Circle { diameter: f64 },
    Point,
}

impl LeoFootprint {
    /// Half-extent in X / Y used for bounding-box and hit-area
    /// calculations (metres). Circles use radius for both axes; points
    /// fall back to a small default so they remain clickable.
    pub fn half_extent(&self) -> (f64, f64) {
        match self.shape {
            LeoShape::Rectangle { size_x, size_y } => (size_x / 2.0, size_y / 2.0),
            LeoShape::Circle { diameter } => (diameter / 2.0, diameter / 2.0),
            LeoShape::Point => (0.01, 0.01),
        }
    }
}

/// Parse an L3D ZIP and flatten every Light Emitting Object into a list
/// of [`LeoFootprint`]s, recursing through articulated-joint subtrees.
///
/// Returns `Ok(vec![])` for inputs that don't yield a parseable
/// structure (non-L3D bytes, corrupt archive) — see the module docs.
pub fn extract_leos(l3d_bytes: &[u8]) -> anyhow::Result<Vec<LeoFootprint>> {
    let l3d = l3d_rs::from_buffer(l3d_bytes);
    let xml = l3d.file.structure.trim();
    if xml.is_empty() {
        return Ok(Vec::new());
    }
    let luminaire = Luminaire::from_xml(xml)?;
    let mut out = Vec::new();
    walk(&luminaire.structure.geometry, &mut out);
    Ok(out)
}

/// Recursively collect LEOs from a geometry node and its joint subtrees.
fn walk(geometry: &Geometry, out: &mut Vec<LeoFootprint>) {
    if let Some(leos) = geometry.light_emitting_objects.as_ref() {
        for leo in &leos.light_emitting_object {
            let (px, py, pz) = leo.position();
            let (_, _, rz) = leo.rotation();
            let shape = if let Some(r) = leo.rectangle() {
                LeoShape::Rectangle {
                    size_x: r.size_x,
                    size_y: r.size_y,
                }
            } else if let Some(c) = leo.circle() {
                LeoShape::Circle {
                    diameter: c.diameter,
                }
            } else {
                LeoShape::Point
            };
            out.push(LeoFootprint {
                part_name: leo.part_name().to_string(),
                x: px,
                y: py,
                z: pz,
                rot_z_deg: rz,
                shape,
            });
        }
    }
    // Articulated luminaires nest child geometries under joints.
    if let Some(joints) = geometry.joints.as_ref() {
        for joint in &joints.joint {
            for child in &joint.geometries.geometry {
                walk(child, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `example_007.l3d` (the demo downlight) has two circular LEOs,
    /// `LEO0` and `LEO1`, nested inside a joint subtree — so this also
    /// proves the joint recursion in `walk`.
    #[test]
    fn extracts_leo0_and_leo1_from_example_007() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/test-create/example_007.l3d"
        );
        let bytes = std::fs::read(path).expect("read example_007.l3d");
        let leos = extract_leos(&bytes).expect("extract leos");

        let names: Vec<&str> = leos.iter().map(|l| l.part_name.as_str()).collect();
        assert!(names.contains(&"LEO0"), "LEO0 present, got {names:?}");
        assert!(names.contains(&"LEO1"), "LEO1 present, got {names:?}");

        let leo0 = leos.iter().find(|l| l.part_name == "LEO0").unwrap();
        assert!(
            matches!(leo0.shape, LeoShape::Circle { .. }),
            "LEO0 is circular, got {:?}",
            leo0.shape
        );
    }

    #[test]
    fn non_l3d_bytes_yield_empty_list() {
        let leos = extract_leos(b"not a zip").expect("soft-fails to empty");
        assert!(leos.is_empty());
    }
}
