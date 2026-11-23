//! Photometric core for the UE runtime: the eulumdat engine behind the
//! demo's illuminance heatmap.
//!
//! * `gldf_unreal_ldc_open(bytes, len)` parses an IES (LM-63) or EULUMDAT
//!   (LDT) text blob — the per-emitter bytes the importer already stores on
//!   the luminaire actor — into an `Eulumdat` photometry and returns a handle.
//! * `gldf_unreal_ldc_intensity_cd(h, c, gamma)` is the symmetry-aware,
//!   interpolated luminous intensity in absolute candela (eulumdat stores
//!   cd/klm; scaled by the file's flux).
//! * `gldf_unreal_illuminance_grid(...)` computes, for every emitter pose and
//!   every cell of a planar grid, the direct illuminance E = I(C,γ)·cos i / r²
//!   in lux — one layer per emitter so the caller can apply its own
//!   visibility (shadow) mask before summing. Pure photometry, no occlusion.
//!
//! Frames: everything in METRES in whatever right- or left-handed frame the
//! caller uses, as long as positions and axes agree. An emitter pose carries
//! its nadir axis (photometric γ=0, the direction a downlight points), the
//! C=0° axis and the C=90° axis. The grid is origin + u·(ix+0.5) + v·(iy+0.5)
//! with a receiver normal.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use eulumdat::{Eulumdat, IesParser};

struct Ldc {
    ldt: Eulumdat,
    /// Absolute flux of the file (lm); eulumdat intensities are per klm.
    flux_lm: f64,
}

static LDC_REGISTRY: Mutex<Option<HashMap<u64, Box<Ldc>>>> = Mutex::new(None);
static LDC_NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn parse_ldc(text: &str) -> Option<Eulumdat> {
    let head = text.trim_start();
    let looks_ies = head.starts_with("IESNA") || head.starts_with("IES") || head.contains("TILT=");
    if looks_ies {
        if let Ok(l) = IesParser::parse(text) {
            return Some(l);
        }
    }
    if let Ok(l) = Eulumdat::parse(text) {
        return Some(l);
    }
    if !looks_ies {
        IesParser::parse(text).ok()
    } else {
        None
    }
}

/// Parse IES/LDT text bytes into a photometry handle (0 on failure).
///
/// # Safety
/// `bytes` points to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_ldc_open(bytes: *const u8, len: usize) -> u64 {
    if bytes.is_null() || len == 0 {
        return 0;
    }
    let slice = std::slice::from_raw_parts(bytes, len);
    let text = String::from_utf8_lossy(slice);
    let Some(ldt) = parse_ldc(&text) else {
        return 0;
    };
    let mut flux_lm = ldt.luminous_flux();
    if !(flux_lm.is_finite() && flux_lm > 0.0) {
        flux_lm = 1000.0; // unknown flux: treat the table as absolute cd
    }
    let id = LDC_NEXT_ID.fetch_add(1, Ordering::Relaxed);
    LDC_REGISTRY
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(id, Box::new(Ldc { ldt, flux_lm }));
    id
}

/// Luminous flux (lm) of the photometry behind `handle`; 0 if unknown handle.
#[no_mangle]
pub extern "C" fn gldf_unreal_ldc_flux_lm(handle: u64) -> f64 {
    let guard = LDC_REGISTRY.lock().unwrap();
    guard
        .as_ref()
        .and_then(|m| m.get(&handle))
        .map(|l| l.flux_lm)
        .unwrap_or(0.0)
}

/// Absolute luminous intensity (cd) toward photometric angles `c_deg`
/// (0..360) and `gamma_deg` (0 = nadir, 180 = zenith). Symmetry-aware and
/// interpolated by the eulumdat core. 0 for an unknown handle.
#[no_mangle]
pub extern "C" fn gldf_unreal_ldc_intensity_cd(handle: u64, c_deg: f64, gamma_deg: f64) -> f64 {
    let guard = LDC_REGISTRY.lock().unwrap();
    let Some(l) = guard.as_ref().and_then(|m| m.get(&handle)) else {
        return 0.0;
    };
    let c = c_deg.rem_euclid(360.0);
    let g = gamma_deg.clamp(0.0, 180.0);
    l.ldt.sample(c, g) * l.flux_lm / 1000.0
}

/// Release a photometry handle.
#[no_mangle]
pub extern "C" fn gldf_unreal_ldc_close(handle: u64) {
    if let Some(m) = LDC_REGISTRY.lock().unwrap().as_mut() {
        m.remove(&handle);
    }
}

/// One emitter's pose for [`gldf_unreal_illuminance_grid`], metres.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GldfEmitterPose {
    pub ldc: u64,
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    /// Photometric nadir (γ = 0) direction — where a downlight points.
    pub nadir_x: f64,
    pub nadir_y: f64,
    pub nadir_z: f64,
    /// C = 0° half-plane direction (perpendicular to nadir).
    pub c0_x: f64,
    pub c0_y: f64,
    pub c0_z: f64,
    /// C = 90° half-plane direction (perpendicular to both).
    pub c90_x: f64,
    pub c90_y: f64,
    pub c90_z: f64,
    /// Dimming / maintenance factor applied to this emitter (1 = nominal).
    pub factor: f64,
}

/// A planar receiver grid, metres. Cell (ix, iy) centre =
/// origin + u·(ix + 0.5) + v·(iy + 0.5); `u`/`v` are full cell vectors.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GldfGridSpec {
    pub origin_x: f64,
    pub origin_y: f64,
    pub origin_z: f64,
    pub u_x: f64,
    pub u_y: f64,
    pub u_z: f64,
    pub v_x: f64,
    pub v_y: f64,
    pub v_z: f64,
    pub normal_x: f64,
    pub normal_y: f64,
    pub normal_z: f64,
    pub nx: u32,
    pub ny: u32,
}

#[inline]
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
#[inline]
fn norm(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt();
    if l > 1e-12 {
        [a[0] / l, a[1] / l, a[2] / l]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// Direct illuminance (lux) of every emitter on every grid cell.
///
/// `out_lux` must hold `n_emitters * nx * ny` doubles, laid out
/// `[emitter][iy][ix]`. Optional `cell_z` (nx*ny doubles, may be null)
/// overrides each cell's height along the grid normal — lets the caller
/// drape the grid over uneven ground it measured itself. Returns 0 on
/// success, 1 on bad arguments.
///
/// # Safety
/// Pointers must be valid for the stated counts.
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_illuminance_grid(
    emitters: *const GldfEmitterPose,
    n_emitters: u32,
    grid: *const GldfGridSpec,
    cell_offsets: *const f64,
    out_lux: *mut f64,
) -> i32 {
    if emitters.is_null() || grid.is_null() || out_lux.is_null() || n_emitters == 0 {
        return 1;
    }
    let g = *grid;
    let (nx, ny) = (g.nx as usize, g.ny as usize);
    if nx == 0 || ny == 0 {
        return 1;
    }
    let poses = std::slice::from_raw_parts(emitters, n_emitters as usize);
    let out = std::slice::from_raw_parts_mut(out_lux, n_emitters as usize * nx * ny);
    let offsets = if cell_offsets.is_null() {
        None
    } else {
        Some(std::slice::from_raw_parts(cell_offsets, nx * ny))
    };
    let n = norm([g.normal_x, g.normal_y, g.normal_z]);

    let guard = LDC_REGISTRY.lock().unwrap();
    let registry = guard.as_ref();

    for (ei, pose) in poses.iter().enumerate() {
        let ldc = registry.and_then(|m| m.get(&pose.ldc));
        let nadir = norm([pose.nadir_x, pose.nadir_y, pose.nadir_z]);
        let c0 = norm([pose.c0_x, pose.c0_y, pose.c0_z]);
        let c90 = norm([pose.c90_x, pose.c90_y, pose.c90_z]);
        let base = ei * nx * ny;
        for iy in 0..ny {
            for ix in 0..nx {
                let fx = ix as f64 + 0.5;
                let fy = iy as f64 + 0.5;
                let off = offsets.map(|o| o[iy * nx + ix]).unwrap_or(0.0);
                let p = [
                    g.origin_x + g.u_x * fx + g.v_x * fy + n[0] * off,
                    g.origin_y + g.u_y * fx + g.v_y * fy + n[1] * off,
                    g.origin_z + g.u_z * fx + g.v_z * fy + n[2] * off,
                ];
                let d = [p[0] - pose.pos_x, p[1] - pose.pos_y, p[2] - pose.pos_z];
                let r2 = dot(d, d);
                let mut e = 0.0;
                if let (Some(l), true) = (ldc, r2 > 1e-6) {
                    let r = r2.sqrt();
                    let dir = [d[0] / r, d[1] / r, d[2] / r];
                    let cos_inc = -dot(dir, n); // light arriving onto the receiver
                    if cos_inc > 0.0 {
                        let cg = dot(dir, nadir).clamp(-1.0, 1.0);
                        let gamma = cg.acos().to_degrees();
                        let c = dot(dir, c90)
                            .atan2(dot(dir, c0))
                            .to_degrees()
                            .rem_euclid(360.0);
                        let cd = l.ldt.sample(c, gamma) * l.flux_lm / 1000.0 * pose.factor;
                        e = cd * cos_inc / r2;
                    }
                }
                out[base + iy * nx + ix] = e;
            }
        }
    }
    0
}
