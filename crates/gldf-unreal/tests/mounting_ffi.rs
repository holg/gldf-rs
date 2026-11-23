//! `gldf_unreal_variant_mounting` — the GLDF `<Mountings>` of a variant,
//! flattened for the UE placement code (surface snapping).

use std::ffi::CString;
use std::path::PathBuf;

use gldf_unreal::ffi::{
    gldf_unreal_variant_mounting, gldf_unreal_variants_close, gldf_unreal_variants_open,
    GldfMountingInfo, GLDF_MOUNTING_CEILING, GLDF_MOUNTING_GROUND, GLDF_MOUNTING_NONE,
    GLDF_MOUNTING_WALL,
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
    assert!(p.exists(), "missing fixture {}", p.display());
    CString::new(p.to_string_lossy().into_owned()).unwrap()
}

fn mounting_of(name: &str, idx: u32) -> GldfMountingInfo {
    let path = data(name);
    let mut count = 0u32;
    let mut err = std::ptr::null_mut();
    let handle = unsafe { gldf_unreal_variants_open(path.as_ptr(), &mut count, &mut err) };
    assert!(handle != 0, "variants_open failed for {name}");
    assert!(idx < count, "{name}: variant {idx} out of {count}");
    let mut out = GldfMountingInfo {
        kind: 0xFF,
        surface_mounted: 0,
        free_standing: 0,
        pole_top: 0,
        pole_integrated: 0,
        recessed_depth_present: 0,
        pendant_length_present: 0,
        mounting_height_present: 0,
        pole_height_present: 0,
        recessed_depth_mm: 0,
        pendant_length_mm: 0,
        mounting_height_mm: 0,
        pole_height_mm: 0,
    };
    let rc = unsafe { gldf_unreal_variant_mounting(handle, idx, &mut out) };
    assert_eq!(rc, 0);
    // Out-of-range index is a clean error, not a crash.
    assert_ne!(
        unsafe { gldf_unreal_variant_mounting(handle, count, std::ptr::null_mut()) },
        0
    );
    unsafe { gldf_unreal_variants_close(handle) };
    out
}

#[test]
fn slv_tria2_is_ceiling_recessed_11mm() {
    let m = mounting_of("SLV - Tria 2.gldf", 0);
    assert_eq!(m.kind, GLDF_MOUNTING_CEILING);
    assert_eq!(m.recessed_depth_present, 1);
    assert_eq!(m.recessed_depth_mm, 11);
    assert_eq!(m.pendant_length_present, 0);
    assert_eq!(m.surface_mounted, 0);
}

#[test]
fn philips_sp542p_is_ceiling_pendant_500mm_surface() {
    let m = mounting_of("Philips SP542P 0.9 DEMO.gldf", 0);
    assert_eq!(m.kind, GLDF_MOUNTING_CEILING);
    assert_eq!(m.surface_mounted, 1);
    assert_eq!(m.pendant_length_present, 1);
    assert_eq!(m.pendant_length_mm, 500);
}

#[test]
fn floodlight_wall_wins_over_ground_pole() {
    // <Wall mountingHeight="20000"><SurfaceMounted/></Wall><Ground><PoleTop/></Ground>
    let m = mounting_of("FLOODLIGHT_MAX_1200W_757_SYM_10_WAL_enriched.gldf", 0);
    assert_eq!(m.kind, GLDF_MOUNTING_WALL);
    assert_eq!(m.mounting_height_present, 1);
    assert_eq!(m.mounting_height_mm, 20000);
    assert_eq!(m.surface_mounted, 1);
    // PoleTop without a poleHeight → no height, but the flag survives so the
    // consumer can still build a default pole; the kind stays Wall.
    assert_eq!(m.pole_height_present, 0);
    assert_eq!(m.pole_top, 1);
    assert_eq!(m.pole_integrated, 0);
}

#[test]
fn belviso_is_ground_free_standing() {
    let m = mounting_of("Freestand_Belviso-2l3d.gldf", 0);
    assert_eq!(m.kind, GLDF_MOUNTING_GROUND);
    assert_eq!(m.free_standing, 1);
    assert_eq!(m.pole_height_present, 0);
    assert_eq!(m.pole_top, 0);
}

#[test]
fn alurays_has_no_mountings() {
    let m = mounting_of("alurays-3000mm.gldf", 0);
    assert_eq!(m.kind, GLDF_MOUNTING_NONE);
    assert_eq!(m.recessed_depth_present, 0);
    assert_eq!(m.pendant_length_present, 0);
    assert_eq!(m.mounting_height_present, 0);
    assert_eq!(m.pole_height_present, 0);
}

#[test]
fn product_strings_round_trip() {
    use gldf_unreal::ffi::{
        gldf_unreal_product_string, gldf_unreal_string_free, gldf_unreal_variant_string,
        GLDF_PRODUCT_FIELD_MANUFACTURER, GLDF_PRODUCT_FIELD_NAME,
        GLDF_VARIANT_FIELD_PRODUCT_NUMBER,
    };
    let path = data("SLV - Tria 2.gldf");
    let mut count = 0u32;
    let mut err = std::ptr::null_mut();
    let handle = unsafe { gldf_unreal_variants_open(path.as_ptr(), &mut count, &mut err) };
    assert!(handle != 0);
    let get = |p: *mut std::os::raw::c_char| {
        assert!(!p.is_null());
        let s = unsafe { std::ffi::CStr::from_ptr(p) }
            .to_string_lossy()
            .into_owned();
        unsafe { gldf_unreal_string_free(p) };
        s
    };
    let mfr = get(unsafe { gldf_unreal_product_string(handle, GLDF_PRODUCT_FIELD_MANUFACTURER) });
    let name = get(unsafe { gldf_unreal_product_string(handle, GLDF_PRODUCT_FIELD_NAME) });
    let pnum =
        get(unsafe { gldf_unreal_variant_string(handle, 0, GLDF_VARIANT_FIELD_PRODUCT_NUMBER) });
    assert!(mfr.to_lowercase().contains("slv"), "manufacturer {mfr}");
    assert!(!name.is_empty(), "product name");
    println!("mfr={mfr} name={name} variant0 number={pnum}");
    assert!(unsafe { gldf_unreal_product_string(handle, 99) }.is_null());
    unsafe { gldf_unreal_variants_close(handle) };
}
