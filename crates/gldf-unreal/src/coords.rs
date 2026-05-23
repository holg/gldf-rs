//! Coordinate-system conversion from L3D to Unreal Engine.
//!
//! L3D source: Z-up, right-handed, millimetres. Light's default emission
//! direction is −Z. Unreal: Z-up, **left-handed**, centimetres (or metres
//! when [`UnitSystem::M`](crate::options::UnitSystem::M) is selected).
//!
//! The transform is a Y-negating uniform scale: `M = diag(s, -s, s, 1)`
//! where `s` is the mm→target scale factor. Apply it to translation vectors
//! directly. For full part matrices apply the similarity `M * T * M⁻¹`.
//!
//! Because `det(M) < 0`, triangle winding flips — [`mesh`](crate::mesh)
//! handles the reorder.
//!
//! Phase 2 will implement the matrix builder and tests; this file is
//! currently a stub holding the doc contract.

#![allow(dead_code)]

use crate::options::UnitSystem;
use glam::{DMat4, DVec3};

/// Build the L3D→Unreal transform matrix for the chosen unit system.
pub fn l3d_to_ue_matrix(units: UnitSystem) -> DMat4 {
    let s = units.scale_from_mm();
    DMat4::from_cols(
        DVec3::new(s, 0.0, 0.0).extend(0.0),
        DVec3::new(0.0, -s, 0.0).extend(0.0),
        DVec3::new(0.0, 0.0, s).extend(0.0),
        glam::DVec4::new(0.0, 0.0, 0.0, 1.0),
    )
}

/// Apply the L3D→Unreal point transform to a single vector. Inputs are in
/// millimetres; outputs are in the chosen [`UnitSystem`].
pub fn point_l3d_to_ue(v_mm: DVec3, units: UnitSystem) -> DVec3 {
    let s = units.scale_from_mm();
    DVec3::new(v_mm.x * s, -v_mm.y * s, v_mm.z * s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;

    #[test]
    fn point_transform_y_negated() {
        let p = point_l3d_to_ue(DVec3::new(1000.0, 1000.0, 1000.0), UnitSystem::Cm);
        assert!((p.x - 100.0).abs() < 1e-9);
        assert!((p.y - -100.0).abs() < 1e-9);
        assert!((p.z - 100.0).abs() < 1e-9);
    }

    #[test]
    fn matrix_has_negative_determinant() {
        // The Y-flip makes the matrix orientation-reversing; mesh winding
        // must be flipped accordingly. Guard the property here so we notice
        // if anyone "fixes" the negative sign without updating mesh.rs.
        let m = l3d_to_ue_matrix(UnitSystem::Cm);
        assert!(m.determinant() < 0.0, "L3D→UE matrix should reverse orientation");
    }

    #[test]
    fn units_m_scale_factor() {
        let p = point_l3d_to_ue(DVec3::new(1000.0, 0.0, 0.0), UnitSystem::M);
        assert!((p.x - 1.0).abs() < 1e-9);
    }
}
