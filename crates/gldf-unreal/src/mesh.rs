//! OBJ rewriting from L3D coordinate frame to Unreal Engine frame.
//!
//! Reads an L3D-bundled Wavefront OBJ (mm, right-handed Z-up), applies the
//! [`coords::l3d_to_ue_matrix`](crate::coords::l3d_to_ue_matrix) transform
//! to all positions, negates the Y component of vertex normals, and
//! reverses triangle face winding so the mesh renders correctly under
//! Unreal's left-handed convention.
//!
//! Phase 2 will implement the parser + rewriter; this file is currently a
//! stub holding the contract.

#![allow(dead_code)]

use crate::error::Result;
use crate::options::UnitSystem;

/// Rewrite OBJ bytes from L3D frame into Unreal frame. Returns the new
/// OBJ as a UTF-8 string.
///
/// Phase 1: returns the input unchanged (so the public API surface is
/// stable). Phase 2 implements the actual transform.
pub fn rewrite_obj_to_ue(obj_bytes: &[u8], _units: UnitSystem) -> Result<String> {
    // TODO(phase-2): parse v/vn/f lines; transform v with point_l3d_to_ue;
    // negate normal Y; reverse triangle index order on f lines.
    Ok(String::from_utf8_lossy(obj_bytes).into_owned())
}
