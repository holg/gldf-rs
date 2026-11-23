//! End-to-end smoke tests for the Lisp engine.
//!
//! The "do the foreign functions actually wire up to gldf-rs?"
//! integration assertion. Uses `SLV - Tria 2.gldf` from `tests/data/`
//! because it has multiple variants — a 0-variant file would let bugs
//! in the indexing path slip through.

use gldf_rs_lisp::{create_gldf_interpreter, GldfState, SceneCommand};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../tests/data");
    p.push(name);
    p
}

#[test]
fn load_and_variant_query_round_trip() {
    let path = fixture("SLV - Tria 2.gldf");
    let path_str = path
        .to_str()
        .expect("test fixture path must be UTF-8")
        .replace('\\', "/");

    let mut interp = create_gldf_interpreter();
    let script = format!(
        r#"
(GLDF-LOAD "{path_str}")
(GLDF-VARIANT-COUNT)
(GLDF-VARIANT-NAMES)
(GLDF-MANUFACTURER)
"#
    );
    let results = interp.run(&script);

    // (GLDF-LOAD ...) -> T, count -> Integer >= 1, names -> non-empty list,
    // manufacturer -> non-empty string.
    let load = &results[0];
    assert!(
        matches!(load, acadlisp::Expr::T),
        "load failed: {load:?}; output: {:?}",
        interp.output
    );
    let count = match &results[1] {
        acadlisp::Expr::Integer(n) => *n,
        other => panic!("count not an Integer: {other:?}"),
    };
    assert!(count >= 1, "variant count should be >=1, got {count}");
    let names = match &results[2] {
        acadlisp::Expr::List(xs) => xs.clone(),
        other => panic!("names not a List: {other:?}"),
    };
    assert_eq!(
        names.len(),
        count as usize,
        "variant-names length should match variant-count"
    );
    let mfr = match &results[3] {
        acadlisp::Expr::String(s) => s.clone(),
        other => panic!("manufacturer not a String: {other:?}"),
    };
    assert!(!mfr.is_empty(), "manufacturer should be populated");
}

#[test]
fn camera_set_and_luminaire_tilt_log_into_state() {
    // No product needed — the camera/tilt primitives only mutate the
    // dispatch log. This is the SLV-hero-shot proof: a script can
    // express the two primitives and we can introspect them.
    let mut interp = create_gldf_interpreter();
    let script = r#"
(CAMERA-SET 0.5 0.7 1.2 0.0 0.0 0.0)
(LUMINAIRE-TILT "l3d://leo0" 20.0 0.0 0.0)
"#;
    let results = interp.run(script);
    assert!(matches!(results[0], acadlisp::Expr::T));
    assert!(matches!(results[1], acadlisp::Expr::T));

    let state = interp
        .user_data
        .downcast_ref::<GldfState>()
        .expect("user_data should be GldfState");
    assert_eq!(
        state.log,
        vec![
            SceneCommand::CameraSet {
                eye: [0.5, 0.7, 1.2],
                target: [0.0, 0.0, 0.0],
            },
            SceneCommand::LuminaireTilt {
                joint_path: "l3d://leo0".to_string(),
                euler_deg: [20.0, 0.0, 0.0],
            },
        ]
    );
}

#[test]
fn animate_primitives_dispatch_chainable_commands() {
    // The "impressive demo" depends on chaining: a script with N
    // animation calls should produce N CameraAnimate / LuminaireAnimate
    // commands in order. The Bevy side captures the current state as
    // each one's "from" pose, so even though all are dispatched
    // synchronously, the timeline plays out as a smooth multi-leg
    // trajectory.
    let mut interp = create_gldf_interpreter();
    let script = r#"
(ANIMATE-CAMERA 0.5 0.3 1.5  0.0 0.0 0.0  800.0)
(ANIMATE-TILT "l3d://LEO0" 0.0 30.0 0.0  600.0)
(ANIMATE-CAMERA 0.0 1.2 0.8  0.0 0.0 0.0  900.0)
(ANIMATE-TILT "l3d://LEO0" 0.0 -30.0 0.0  600.0)
"#;
    let results = interp.run(script);
    for r in &results {
        assert!(matches!(r, acadlisp::Expr::T), "got {r:?}");
    }
    let state = interp
        .user_data
        .downcast_ref::<GldfState>()
        .expect("user_data should be GldfState");
    assert_eq!(state.log.len(), 4, "got log: {:?}", state.log);
    assert!(matches!(
        state.log[0],
        SceneCommand::CameraAnimate { duration_ms, .. } if (duration_ms - 800.0).abs() < 1e-3
    ));
    assert!(matches!(
        state.log[1],
        SceneCommand::LuminaireAnimate { duration_ms, .. } if (duration_ms - 600.0).abs() < 1e-3
    ));
    assert!(matches!(state.log[2], SceneCommand::CameraAnimate { .. }));
    assert!(matches!(
        state.log[3],
        SceneCommand::LuminaireAnimate { .. }
    ));
}

#[test]
fn variant_set_dispatches_scene_command() {
    let path = fixture("SLV - Tria 2.gldf");
    let path_str = path
        .to_str()
        .expect("test fixture path must be UTF-8")
        .replace('\\', "/");

    let mut interp = create_gldf_interpreter();
    let script = format!(
        r#"
(GLDF-LOAD "{path_str}")
(GLDF-VARIANT-SET 0)
(GLDF-VARIANT-SET 1)
"#
    );
    let _ = interp.run(&script);
    let state = interp
        .user_data
        .downcast_ref::<gldf_rs_lisp::GldfState>()
        .expect("user_data must be GldfState");
    let variant_cmds: Vec<_> = state
        .log
        .iter()
        .filter_map(|c| match c {
            SceneCommand::SetActiveVariant { index } => Some(*index),
            _ => None,
        })
        .collect();
    assert_eq!(
        variant_cmds,
        vec![0, 1],
        "GLDF-VARIANT-SET should emit one SetActiveVariant per call, in order"
    );
}

#[test]
fn variant_set_clamps_to_range() {
    let path = fixture("SLV - Tria 2.gldf");
    let path_str = path
        .to_str()
        .expect("test fixture path must be UTF-8")
        .replace('\\', "/");

    let mut interp = create_gldf_interpreter();
    let script = format!(
        r#"
(GLDF-LOAD "{path_str}")
(GLDF-VARIANT-SET 9999)
(GLDF-VARIANT-ACTIVE)
"#
    );
    let results = interp.run(&script);
    // out-of-range -> Nil (logged), active stays at 0
    assert!(matches!(results[1], acadlisp::Expr::Nil));
    match &results[2] {
        acadlisp::Expr::Integer(0) => {}
        other => panic!("active variant should stay at 0, got {other:?}"),
    }
}
