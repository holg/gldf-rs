//! Wire-contract test: `SceneCommand` must survive a JSON round trip.
//!
//! The native 3D viewer receives Lisp-dispatched `SceneCommand`s from the
//! editor as newline-delimited JSON over the child process's stdin (there
//! is no `window` bridge natively). If serialization drifts, the whole
//! native scripting path silently breaks — so pin it with a round trip
//! over a representative command of each shape (nested enum field, floats,
//! arrays, strings, unit-ish variants).

use gldf_rs_lisp::{RoomSurface, SceneCommand};

fn roundtrip(cmd: &SceneCommand) {
    let line = serde_json::to_string(cmd).expect("serialize");
    assert!(!line.contains('\n'), "NDJSON line must not contain newlines");
    let back: SceneCommand = serde_json::from_str(&line).expect("deserialize");
    assert_eq!(cmd, &back, "round trip changed the command\njson: {line}");
}

#[test]
fn scene_command_json_round_trip() {
    let cmds = [
        SceneCommand::CameraSet {
            eye: [1.0, 2.5, -3.0],
            target: [0.0, 0.0, 0.0],
        },
        SceneCommand::CameraAnimate {
            eye: [4.0, 1.0, 2.0],
            target: [0.0, 1.0, 0.0],
            duration_ms: 1500.0,
        },
        SceneCommand::LuminaireTilt {
            joint_path: "l3d://InnerRing".to_string(),
            euler_deg: [0.0, 0.0, 30.0],
        },
        SceneCommand::LuminaireAnimate {
            joint_path: "l3d://Gimbal".to_string(),
            euler_deg: [15.0, 0.0, 0.0],
            duration_ms: 800.0,
        },
        SceneCommand::SetActiveVariant { index: 2 },
        SceneCommand::Sleep { duration_ms: 250.0 },
        SceneCommand::SetSurfaceColor {
            surface: RoomSurface::Walls,
            r: 0.8,
            g: 0.8,
            b: 0.82,
        },
    ];
    for cmd in &cmds {
        roundtrip(cmd);
    }
}
