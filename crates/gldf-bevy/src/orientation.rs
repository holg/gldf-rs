//! Pure orientation helpers — extracted from `l3d_loader.rs` so the
//! L3D → Bevy direction pipeline is unit-testable without spinning
//! up a full Bevy `App`.
//!
//! ## Convention reminder
//!
//! - **L3D world**: right-handed, **Z-up**. Geometry is authored such
//!   that an emitter's natural emission axis points along **−Z**.
//!   The mount-plate is at **+Z**.
//! - **Bevy world**: right-handed, **Y-up**. "Down" is **−Y**.
//!
//! ## Canonical reference
//!
//! l3d-egui (`../l3d-rs/crates/l3d-egui/src/viewer.rs:882-910`) is the
//! reference Rust viewer for L3D. Its transform for every model part is
//! literally:
//!
//! ```text
//!   mat = Rot_X(-90°) * part_mat
//! ```
//!
//! Nothing else. No ceiling-flip, no force-y-negative. The single
//! `Rot_X(-90°)` (= [`z_to_y_rotation`]) is the whole pipeline. Anyone
//! tempted to add extra rotations should re-read that file first.
//!
//! Cross-referenced by ROLF (`../light-other-rs/crates/rolf/src/convert/l3d.rs`):
//! r3d is Z-up, light emits in −Z; L3D inherits the Z-up storage; the
//! viewer applies `Rot_X(-90°)` to render Y-up.
//!
//! ## Pipeline (this module)
//!
//! For an emitter with Euler XYZ rotation `(rx, ry, rz)` in **degrees**
//! within its L3D parent joint:
//!
//! ```text
//!   l3d_dir   = Rot_XYZ(rx,ry,rz) * (-Z)         // in L3D space
//!   bevy_dir  = Rot_X(-90°) * l3d_dir            // L3D Z-up → Bevy Y-up
//! ```
//!
//! For a "neutral" emitter (rotation = 0), `l3d_dir == -Z`, and after
//! the coordinate flip `bevy_dir == -Y` — the spotlight points straight
//! down, the physically correct default for a ceiling fixture.

use bevy::math::{EulerRot, Quat, Vec3};

/// L3D's Z-up to Bevy's Y-up coordinate rotation: rotate the world
/// −90° around the X axis. Used everywhere in `l3d_loader.rs`.
pub fn z_to_y_rotation() -> Quat {
    Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
}

/// Compute the Bevy-space emission direction of an L3D emitter from
/// its Euler XYZ rotation (degrees).
///
/// `(0, 0, 0)` → `(0, -1, 0)` (straight down — the spec-default for a
/// ceiling-mounted L3D fixture).
pub fn l3d_emitter_bevy_direction(rot_deg: Vec3) -> Vec3 {
    let emitter_rot = Quat::from_euler(
        EulerRot::XYZ,
        rot_deg.x.to_radians(),
        rot_deg.y.to_radians(),
        rot_deg.z.to_radians(),
    );
    // L3D default emission axis is -Z.
    let l3d_dir = emitter_rot * Vec3::NEG_Z;
    z_to_y_rotation() * l3d_dir
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    fn approx_eq_vec(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < EPSILON
    }

    /// Spec-default: a freshly-authored LEO with zero rotation must
    /// emit straight down in Bevy world space. This is the load-bearing
    /// invariant — every other emitter direction is derived from this
    /// one by composing the LEO's own rotation on top.
    #[test]
    fn vertical_emitter_points_down() {
        let dir = l3d_emitter_bevy_direction(Vec3::ZERO);
        assert!(
            approx_eq_vec(dir, Vec3::NEG_Y),
            "expected (0,-1,0), got {:?}",
            dir
        );
    }

    /// 180° around X in L3D flips -Z to +Z. After Z-to-Y conversion,
    /// +Z (L3D) becomes -Y...wait, no: Rot_X(-90°) maps +Z → +Y. So
    /// the result should be +Y (pointing UP) — the emitter has been
    /// rotated to point "backward" along its mount axis.
    #[test]
    fn upside_down_emitter_points_up() {
        let dir = l3d_emitter_bevy_direction(Vec3::new(180.0, 0.0, 0.0));
        assert!(
            approx_eq_vec(dir, Vec3::Y),
            "expected (0,1,0), got {:?}",
            dir
        );
    }

    /// 90° around Y in L3D rotates -Z to -X. After Z-to-Y conversion,
    /// -X (L3D) stays -X (Bevy) because the conversion only touches Y/Z.
    /// Spotlight points sideways. Used as a regression guard against
    /// accidentally swapping the axis-order in the Euler decomposition.
    #[test]
    fn ninety_y_emitter_points_sideways() {
        let dir = l3d_emitter_bevy_direction(Vec3::new(0.0, 90.0, 0.0));
        assert!(
            approx_eq_vec(dir, Vec3::NEG_X),
            "expected (-1,0,0), got {:?}",
            dir
        );
    }

    /// The L3D mount-plate sits at +Z in L3D space (housing top). After
    /// the single Z-up→Y-up rotation it must land at Bevy's +Y. This is
    /// the load-bearing geometric invariant that proves a ceiling-mounted
    /// housing renders right-side-up WITHOUT any extra ceiling_flip.
    #[test]
    fn mount_plate_lands_at_bevy_up() {
        let mount_plate_l3d = Vec3::Z;
        let mount_plate_bevy = z_to_y_rotation() * mount_plate_l3d;
        assert!(
            approx_eq_vec(mount_plate_bevy, Vec3::Y),
            "expected mount-plate (+Z L3D) to map to +Y Bevy, got {:?}",
            mount_plate_bevy
        );
    }

    /// Inverse of the above: the L3D emission axis (-Z) must map to
    /// Bevy's "down" (-Y). Together these two tests pin both ends of the
    /// L3D-Z-up ↔ Bevy-Y-up convention.
    #[test]
    fn emission_axis_lands_at_bevy_down() {
        let emission_l3d = Vec3::NEG_Z;
        let emission_bevy = z_to_y_rotation() * emission_l3d;
        assert!(
            approx_eq_vec(emission_bevy, Vec3::NEG_Y),
            "expected emission (-Z L3D) to map to -Y Bevy, got {:?}",
            emission_bevy
        );
    }

    /// L3D structure positions are in **meters** per the spec and
    /// per l3d_rs's `fbx.rs` comment ("L3D coordinates are in meters
    /// after unit scaling"). The viewer must NOT divide by 1000 — that
    /// was a long-standing bug that collapsed multi-LEO luminaires
    /// like SLV Tria 2 (LEO0 at ±0.4 m) to a sub-millimeter blob at
    /// the origin, looking like a single dot. A 0.4 m offset in L3D
    /// must remain 0.4 m in Bevy world space after the coordinate
    /// conversion.
    #[test]
    fn l3d_position_preserves_meter_scale() {
        // A 0.4 m offset along L3D X stays a 0.4 m offset along Bevy X
        // (the Z-to-Y rotation only swaps Y and Z).
        let l3d_pos = Vec3::new(0.4, 0.0, 0.0);
        let bevy_pos = z_to_y_rotation() * l3d_pos;
        assert!(
            (bevy_pos.length() - 0.4).abs() < EPSILON,
            "expected a 0.4 m L3D offset to remain 0.4 m in Bevy, got {} m",
            bevy_pos.length()
        );
    }
}
