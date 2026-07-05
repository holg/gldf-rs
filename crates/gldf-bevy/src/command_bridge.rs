//! Native command bridge: turn editor-dispatched `SceneCommand`s into the
//! same request-cell writes the WASM `*_from_js` setters perform, and
//! drain them into Bevy each frame.
//!
//! On WASM the editor's Lisp `SceneBridge` forwards each command to a
//! `window.gldf_bevy_*` JS global (the `#[wasm_bindgen]` setters in
//! `lib.rs`). Natively there is no JS: the editor spawns this viewer with
//! a piped stdin and writes one JSON `SceneCommand` per line. A reader
//! thread parses that NDJSON and pushes commands into an mpsc channel; the
//! `drain_command_channel` Bevy system pulls them each frame and applies
//! them via the shared request cells (`super::*_request_cell`, un-gated in
//! Phase 2 so both targets use one apply path).

use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;

use bevy::prelude::*;
use gldf_rs_lisp::{RoomSurface, SceneCommand};

/// Bevy resource holding the receiving end of the command channel.
/// The `Receiver` is not `Sync`, and Bevy resources must be — so it lives
/// behind a `Mutex` (single consumer, uncontended).
#[derive(Resource)]
pub struct CommandRx(pub Mutex<Receiver<SceneCommand>>);

/// Spawn the stdin reader thread and return the channel receiver to hand
/// to Bevy as [`CommandRx`]. Each stdin line is one JSON `SceneCommand`.
pub fn spawn_stdin_reader() -> Receiver<SceneCommand> {
    let (tx, rx): (Sender<SceneCommand>, Receiver<SceneCommand>) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<SceneCommand>(line) {
                Ok(cmd) => {
                    if tx.send(cmd).is_err() {
                        break; // Bevy side gone
                    }
                }
                Err(e) => eprintln!("[gldf-bevy] bad command line: {e}: {line}"),
            }
        }
    });
    rx
}

/// Milliseconds clock for animation `started_at_ms` — must share the same
/// epoch as the apply systems, so it delegates to `super::anim_now_ms`.
pub fn now_ms() -> f64 {
    super::anim_now_ms()
}

/// Bevy system: drain queued `SceneCommand`s and apply each via the shared
/// request cells. Runs before the appliers registered after it in
/// `run_native_with_scene` (`js_camera_anim_apply` etc.).
pub fn drain_command_channel(rx: Option<Res<CommandRx>>) {
    let Some(rx) = rx else { return };
    let Ok(rx) = rx.0.lock() else { return };
    while let Ok(cmd) = rx.try_recv() {
        apply(cmd);
    }
}

fn apply(cmd: SceneCommand) {
    match cmd {
        SceneCommand::CameraSet { eye, target } => {
            super::request_camera_look_at(eye, target);
        }
        SceneCommand::CameraAnimate {
            eye,
            target,
            duration_ms,
        } => {
            super::request_camera_animate(eye, target, duration_ms, now_ms());
        }
        SceneCommand::LuminaireTilt {
            joint_path,
            euler_deg,
        } => {
            super::request_joint_tilt(&strip_l3d(&joint_path), euler_deg);
        }
        SceneCommand::LuminaireAnimate {
            joint_path,
            euler_deg,
            duration_ms,
        } => {
            super::request_joint_animate(&strip_l3d(&joint_path), euler_deg, duration_ms, now_ms());
        }
        SceneCommand::SetSurfaceColor { surface, r, g, b } => {
            super::request_surface_color(surface_name(surface), r, g, b);
        }
        SceneCommand::SetSurfaceReflectance { surface, value } => {
            super::request_surface_reflectance(surface_name(surface), value);
        }
        SceneCommand::SetAmbient { brightness } => {
            super::request_ambient(brightness);
        }
        SceneCommand::ResetTilts => {
            super::request_reset_tilts();
        }
        // All other variants are editor-side concerns (variant switch,
        // sleep pacing, show-view, wizard/needfiles, room-size,
        // surface-emissive, editor actions) with no native Bevy action
        // in this slice. A catch-all keeps the viewer forward-compatible
        // with new SceneCommand variants added editor-side.
        _ => {}
    }
}

/// The Lisp engine addresses joints as `"l3d://<partName>"`; the Bevy
/// side keys on the bare `partName`.
fn strip_l3d(path: &str) -> String {
    path.strip_prefix("l3d://").unwrap_or(path).to_string()
}

fn surface_name(s: RoomSurface) -> &'static str {
    match s {
        RoomSurface::Walls => "walls",
        RoomSurface::Floor => "floor",
        RoomSurface::Ceiling => "ceiling",
    }
}
