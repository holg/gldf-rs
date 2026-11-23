//! eulumdat core over the FFI: intensity sampling + illuminance grid.
use std::ffi::CString;
use std::path::PathBuf;

use gldf_unreal::ffi::{
    gldf_unreal_variant_emitter_ies, gldf_unreal_variants_close, gldf_unreal_variants_open,
};
use gldf_unreal::ffi_photometry::{
    gldf_unreal_illuminance_grid, gldf_unreal_ldc_close, gldf_unreal_ldc_flux_lm,
    gldf_unreal_ldc_intensity_cd, gldf_unreal_ldc_open, GldfEmitterPose, GldfGridSpec,
};

fn data(name: &str) -> CString {
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
    CString::new(p.to_string_lossy().into_owned()).unwrap()
}

fn first_emitter_ldc(name: &str) -> (u64, i32) {
    let path = data(name);
    let mut count = 0u32;
    let mut err = std::ptr::null_mut();
    let h = unsafe { gldf_unreal_variants_open(path.as_ptr(), &mut count, &mut err) };
    assert!(h != 0 && count > 0);
    let mut ptr: *const u8 = std::ptr::null();
    let mut len: u32 = 0;
    let rc = unsafe { gldf_unreal_variant_emitter_ies(h, 0, 0, &mut ptr, &mut len) };
    assert_eq!(rc, 0);
    assert!(len > 100);
    let ldc = unsafe { gldf_unreal_ldc_open(ptr, len as usize) };
    unsafe { gldf_unreal_variants_close(h) };
    assert!(
        ldc != 0,
        "IES bytes from the importer must parse back through eulumdat"
    );
    let mut info = gldf_unreal::ffi::GldfVariantInfo {
        lumens: 0,
        lumens_present: 0,
        watts: 0.0,
        watts_present: 0,
        cct: 0,
        cct_present: 0,
        ies_len: 0,
    };
    let path2 = data(name);
    let h2 = unsafe { gldf_unreal_variants_open(path2.as_ptr(), &mut count, &mut err) };
    unsafe { gldf_unreal::ffi::gldf_unreal_variant_emitter_info(h2, 0, 0, &mut info) };
    unsafe { gldf_unreal_variants_close(h2) };
    (ldc, info.lumens)
}

#[test]
fn alurays_intensity_and_flux_round_trip() {
    let (ldc, lumens) = first_emitter_ldc("alurays-3000mm.gldf");
    let flux = gldf_unreal_ldc_flux_lm(ldc);
    assert!(flux > 0.0);
    if lumens > 0 {
        let rel = (flux - lumens as f64).abs() / (lumens as f64);
        assert!(rel < 0.05, "flux {flux} vs variant {lumens} lm");
    }
    let nadir = gldf_unreal_ldc_intensity_cd(ldc, 0.0, 0.0);
    let zenith = gldf_unreal_ldc_intensity_cd(ldc, 0.0, 180.0);
    assert!(nadir > 0.0, "a pendant must emit downward");
    assert!(
        nadir >= zenith,
        "downlight: nadir {nadir} cd ≥ zenith {zenith} cd"
    );
    // sanity: a Lambertian-ish source has I(0) ≈ flux/π; real optics vary ±3×
    assert!(nadir < flux, "{nadir} cd for {flux} lm is implausible");
    gldf_unreal_ldc_close(ldc);
    assert_eq!(gldf_unreal_ldc_intensity_cd(ldc, 0.0, 0.0), 0.0);
}

#[test]
fn illuminance_grid_inverse_square_straight_below() {
    let (ldc, _) = first_emitter_ldc("alurays-3000mm.gldf");
    let i0 = gldf_unreal_ldc_intensity_cd(ldc, 0.0, 0.0);
    // Emitter 3 m above a 1×1 cell centred under it (z-up frame).
    let pose = GldfEmitterPose {
        ldc,
        pos_x: 0.5,
        pos_y: 0.5,
        pos_z: 3.0,
        nadir_x: 0.0,
        nadir_y: 0.0,
        nadir_z: -1.0,
        c0_x: 1.0,
        c0_y: 0.0,
        c0_z: 0.0,
        c90_x: 0.0,
        c90_y: 1.0,
        c90_z: 0.0,
        factor: 1.0,
    };
    let grid = GldfGridSpec {
        origin_x: 0.0,
        origin_y: 0.0,
        origin_z: 0.0,
        u_x: 1.0,
        u_y: 0.0,
        u_z: 0.0,
        v_x: 0.0,
        v_y: 1.0,
        v_z: 0.0,
        normal_x: 0.0,
        normal_y: 0.0,
        normal_z: 1.0,
        nx: 1,
        ny: 1,
    };
    let mut out = [0.0f64; 1];
    let rc = unsafe {
        gldf_unreal_illuminance_grid(&pose, 1, &grid, std::ptr::null(), out.as_mut_ptr())
    };
    assert_eq!(rc, 0);
    let expect = i0 / 9.0;
    assert!(
        (out[0] - expect).abs() < 1e-6 * expect.max(1.0),
        "E={} expected I0/h² = {}",
        out[0],
        expect
    );

    // Receiver facing away gets nothing; a cell above the emitter gets nothing.
    let grid_down = GldfGridSpec {
        normal_z: -1.0,
        ..grid
    };
    unsafe {
        gldf_unreal_illuminance_grid(&pose, 1, &grid_down, std::ptr::null(), out.as_mut_ptr())
    };
    assert_eq!(out[0], 0.0);
    let offsets = [4.0f64]; // raise the cell 4 m → above the luminaire
    unsafe { gldf_unreal_illuminance_grid(&pose, 1, &grid, offsets.as_ptr(), out.as_mut_ptr()) };
    assert_eq!(out[0], 0.0);
    gldf_unreal_ldc_close(ldc);
}

#[test]
fn illuminance_grid_two_emitters_layered() {
    let (a, _) = first_emitter_ldc("alurays-3000mm.gldf");
    let (b, _) = first_emitter_ldc("SLV - Tria 2.gldf");
    let mk = |ldc, x| GldfEmitterPose {
        ldc,
        pos_x: x,
        pos_y: 1.0,
        pos_z: 2.5,
        nadir_x: 0.0,
        nadir_y: 0.0,
        nadir_z: -1.0,
        c0_x: 1.0,
        c0_y: 0.0,
        c0_z: 0.0,
        c90_x: 0.0,
        c90_y: 1.0,
        c90_z: 0.0,
        factor: 1.0,
    };
    let poses = [mk(a, 1.25), mk(b, 3.25)]; // cell centres (0.5 m cells)
    let grid = GldfGridSpec {
        origin_x: 0.0,
        origin_y: 0.0,
        origin_z: 0.0,
        u_x: 0.5,
        u_y: 0.0,
        u_z: 0.0,
        v_x: 0.0,
        v_y: 0.5,
        v_z: 0.0,
        normal_x: 0.0,
        normal_y: 0.0,
        normal_z: 1.0,
        nx: 8,
        ny: 4,
    };
    let mut out = vec![0.0f64; 2 * 32];
    let rc = unsafe {
        gldf_unreal_illuminance_grid(poses.as_ptr(), 2, &grid, std::ptr::null(), out.as_mut_ptr())
    };
    assert_eq!(rc, 0);
    let layer_a = &out[..32];
    let layer_b = &out[32..];
    assert!(layer_a.iter().all(|e| e.is_finite() && *e >= 0.0));
    // Each layer peaks under its own luminaire.
    let argmax = |l: &[f64]| {
        l.iter()
            .enumerate()
            .max_by(|x, y| x.1.partial_cmp(y.1).unwrap())
            .unwrap()
            .0
    };
    let (ia, ib) = (argmax(layer_a), argmax(layer_b));
    assert_eq!(ia % 8, 2, "alurays peak under x=1.25 m (cell 2)");
    assert_eq!(ib % 8, 6, "Tria peak under x=3.25 m (cell 6)");
    gldf_unreal_ldc_close(a);
    gldf_unreal_ldc_close(b);
}
