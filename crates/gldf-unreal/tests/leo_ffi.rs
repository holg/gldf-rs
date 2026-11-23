//! `gldf_unreal_leo_count` / `gldf_unreal_leo_at` — every Light Emitting
//! Object of the fixture GLDFs must land at a sane position: the emitter
//! centre (cm, L3D frame) lies on or near the luminaire body, never hundreds
//! of metres away. A runaway position puts the UE light far from the actor.
//!
//! The LEDVANCE floodlight fixture is the documented counter-example: its
//! L3D writes the LEO position in millimetres (`pos=(0,-503.9,184.8)` for a
//! 60 cm body) where L3D wants metres, so the FFI reports it 500 m away.
//! The FFI stays spec-correct (no unit guessing); the UE actor pulls such a
//! runaway emitter back onto the body with a warning
//! (`AGldfLuminaireActor::AdjustLightsOutsideHousing`).

use std::ffi::CString;
use std::path::PathBuf;

use gldf_unreal::ffi::{
    gldf_unreal_first_mesh_open, gldf_unreal_leo_at, gldf_unreal_leo_count,
    gldf_unreal_mesh_borrow, gldf_unreal_mesh_close, GldfLeoShape, GldfMeshHeader,
};

fn data(name: &str) -> (PathBuf, CString) {
    let p: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        "..",
        "tests",
        "data",
        name,
    ]
    .iter()
    .collect();
    assert!(p.exists(), "missing fixture {}", p.display());
    let c = CString::new(p.to_string_lossy().into_owned()).unwrap();
    (p, c)
}

/// Axis-aligned bounds of the first mesh (cm, L3D frame), if the GLDF has one.
fn body_bounds(path: &CString) -> Option<([f32; 3], [f32; 3])> {
    let mut header = GldfMeshHeader::default();
    let mut err = std::ptr::null_mut();
    let handle = unsafe { gldf_unreal_first_mesh_open(path.as_ptr(), &mut header, &mut err) };
    if handle == 0 {
        return None;
    }
    let mut positions: *const f32 = std::ptr::null();
    let rc = unsafe {
        gldf_unreal_mesh_borrow(
            handle,
            &mut positions,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(rc, 0);
    let n = header.vertex_count as usize;
    let slice = unsafe { std::slice::from_raw_parts(positions, n * 3) };
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for v in slice.as_chunks::<3>().0 {
        for k in 0..3 {
            min[k] = min[k].min(v[k]);
            max[k] = max[k].max(v[k]);
        }
    }
    unsafe { gldf_unreal_mesh_close(handle) };
    Some((min, max))
}

fn leos(path: &CString) -> Vec<GldfLeoShape> {
    let count = unsafe { gldf_unreal_leo_count(path.as_ptr()) };
    assert!(count >= 0, "leo_count failed");
    (0..count as u32)
        .map(|i| {
            let mut s = GldfLeoShape::default();
            let rc = unsafe { gldf_unreal_leo_at(path.as_ptr(), i, &mut s) };
            assert_eq!(rc, 0, "leo_at({i}) failed");
            s
        })
        .collect()
}

#[test]
fn every_fixture_leo_sits_on_its_body() {
    // (file, true when the fixture is known to carry mm LEO positions)
    let fixtures = [
        ("SLV - Tria 2.gldf", false),
        ("Philips SP542P 0.9 DEMO.gldf", false),
        ("FLOODLIGHT_MAX_1200W_757_SYM_10_WAL_enriched.gldf", true),
        ("Freestand_Belviso-2l3d.gldf", false),
        ("alurays-3500mm.gldf", false),
    ];
    let slack_cm = 50.0f32;
    let mut failures = Vec::new();
    for (name, known_mm_bug) in fixtures {
        let (_p, path) = data(name);
        let Some((min, max)) = body_bounds(&path) else {
            eprintln!("[{name}] no mesh — skipped");
            continue;
        };
        for (i, s) in leos(&path).iter().enumerate() {
            let pos = [s.pos_x, s.pos_y, s.pos_z];
            let mut dist = 0.0f32;
            for k in 0..3 {
                let d = if pos[k] < min[k] {
                    min[k] - pos[k]
                } else if pos[k] > max[k] {
                    pos[k] - max[k]
                } else {
                    0.0
                };
                dist = dist.max(d);
            }
            eprintln!(
                "[{name}] LEO {i}: kind {} {}x{} m @ ({:.1},{:.1},{:.1}) cm, body ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1}), outside by {dist:.1} cm",
                s.kind, s.dim_a, s.dim_b, pos[0], pos[1], pos[2], min[0], min[1], min[2], max[0], max[1], max[2]
            );
            let finite = pos[0].is_finite() && pos[1].is_finite() && pos[2].is_finite();
            if known_mm_bug {
                // Reported as-is (1000× too far): the consumer's runaway guard
                // must see the raw value, not a silently "fixed" one.
                if !finite || dist < 10_000.0 {
                    failures.push(format!("{name} LEO {i}: expected the raw mm position far off the body, got {dist:.0} cm"));
                }
            } else if !finite || dist > slack_cm {
                failures.push(format!(
                    "{name} LEO {i} at {pos:?} is {dist:.0} cm off the body"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "LEO positions off the body:\n{}",
        failures.join("\n")
    );
}
