//! C ABI surface for the Unreal Engine plugin.
//!
//! Four functions total — kept deliberately tiny because the UE plugin is
//! a thin shim that just shells out to [`gldf_unreal_export`] when the
//! editor user picks a `.gldf` from the import dialog.
//!
//! ## Ownership and lifetimes
//!
//! * All `*const c_char` arguments are **caller-owned**, NUL-terminated
//!   UTF-8. The library copies what it needs before returning.
//! * `out_err` (output) and the report JSON returned by
//!   [`gldf_unreal_last_report_json`] are **Rust-owned** heap strings; the
//!   C caller MUST free them via [`gldf_unreal_string_free`]. Calling
//!   `free()` from the C side instead will corrupt the allocator.
//!
//! ## Error codes
//!
//! [`gldf_unreal_export`] returns 0 on success, otherwise a positive
//! integer matching [`ExportError::code`](crate::error::ExportError::code):
//!
//! | Code | Variant            | Meaning                                |
//! |------|--------------------|----------------------------------------|
//! | 0    | (none)             | Success                                |
//! | 1    | Io                 | Filesystem I/O failure                 |
//! | 2    | GldfLoad           | GLDF parsing failed                    |
//! | 3    | VariantNotFound    | A requested variant id is unknown      |
//! | 4    | NoGeometry         | GLDF has no L3D geometry               |
//! | 5    | Photometry         | LDT/IES processing failed              |
//! | 6    | OutputExists       | Output dir exists and overwrite is off |
//! | 7    | Xml                | XML serialization failed               |
//! | 99   | Internal           | Bug in the exporter                    |

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::error::{ExportError, Result};
use crate::exporter::{ExportReport, Exporter};
use crate::mesh_payload::{build_first_mesh_for_variant, GldfMeshData};
use crate::options::{ExportOptions, UnitSystem, VariantSelector};

/// Layout MUST match `GldfUnrealOpts` in the generated C header. Any field
/// reorder is a breaking ABI change.
#[repr(C)]
pub struct GldfUnrealOpts {
    /// 1 = centimetres (default), 0 = metres.
    pub units_cm: u8,
    /// 1 = copy MTL-referenced textures into the bundle.
    pub embed_textures: u8,
    /// 1 = apply GLDF MountingInfo offsets to the root Actor.
    pub apply_mounting: u8,
    /// 1 = overwrite an existing bundle directory.
    pub overwrite: u8,
    /// Optional comma-separated variant id list. NULL = "all".
    pub variants_csv: *const c_char,
}

/// Thread-local cache for the most recent [`ExportReport`] from the
/// current thread. Wrapped in a `Mutex` for the rare cross-thread access
/// (UE editor + game thread).
static LAST_REPORT: Mutex<Option<String>> = Mutex::new(None);

/// Returns a pointer to a static NUL-terminated string with the crate
/// version. The pointer is valid for the program's lifetime; do NOT free.
#[no_mangle]
pub extern "C" fn gldf_unreal_version() -> *const c_char {
    static VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VERSION.as_ptr() as *const c_char
}

/// Export `gldf_path` to `out_dir` using `opts`.
///
/// On success returns 0 and leaves `*out_err = NULL`. On failure returns
/// the error code (see the table in [`crate::ffi`]) and writes a
/// Rust-owned, heap-allocated NUL-terminated UTF-8 string to `*out_err`;
/// the C caller MUST free it via [`gldf_unreal_string_free`].
///
/// # Safety
/// All pointer arguments must be valid for their stated direction. The
/// caller retains ownership of `gldf_path`, `out_dir`, `opts`, and
/// `opts.variants_csv`.
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_export(
    gldf_path: *const c_char,
    out_dir: *const c_char,
    opts: *const GldfUnrealOpts,
    out_err: *mut *mut c_char,
) -> i32 {
    if !out_err.is_null() {
        *out_err = std::ptr::null_mut();
    }
    match export_impl(gldf_path, out_dir, opts) {
        Ok(report) => {
            // Cache a JSON-ish summary for last_report_json to return.
            *LAST_REPORT.lock().unwrap() = Some(report_to_json(&report));
            0
        }
        Err(e) => {
            let code = e.code();
            if !out_err.is_null() {
                if let Ok(cstr) = CString::new(e.to_string()) {
                    *out_err = cstr.into_raw();
                }
            }
            code
        }
    }
}

/// Return a JSON serialization of the most recent successful
/// [`ExportReport`], or NULL if no export has run yet. Rust-owned string;
/// free via [`gldf_unreal_string_free`].
#[no_mangle]
pub extern "C" fn gldf_unreal_last_report_json() -> *mut c_char {
    let guard = match LAST_REPORT.lock() {
        Ok(g) => g,
        Err(_) => return std::ptr::null_mut(),
    };
    match guard.as_ref() {
        Some(s) => match CString::new(s.as_str()) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Free a string previously returned through `out_err` or
/// [`gldf_unreal_last_report_json`]. Passing NULL is a no-op.
///
/// # Safety
/// `s` must be a pointer previously returned by this library, or NULL.
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_string_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    let _ = CString::from_raw(s);
}

// ─── Interchange translator support (Phase 1) ────────────────────────────
//
// The UE Interchange translator parses GLDF in-process (inside UE), then
// hands per-asset bytes back via Interchange's payload callbacks. The C++
// translator owns the GLDF file path; it asks Rust for individual asset
// blobs.
//
// Phase 1 needs exactly one such call: "give me the IES bytes for the
// first non-emergency emitter of the first variant." Phase 2/3 add the
// mesh + multi-emitter + variant walks.

/// Load `gldf_path` and return the first variant's first non-emergency
/// emitter's IES bytes (variant-resolved lumens/watts already patched).
///
/// On success returns 0 and writes:
/// - `*out_buf` = a Rust-allocated `u8` array; caller MUST free via
///   [`gldf_unreal_bytes_free`] passing both the returned pointer AND
///   the returned length.
/// - `*out_len` = length in bytes.
///
/// On failure returns the [`ExportError::code`] and writes a Rust-owned
/// CString to `*out_err`; caller frees via [`gldf_unreal_string_free`].
///
/// # Safety
/// `gldf_path` must be a valid NUL-terminated UTF-8 C string.
/// `out_buf`, `out_len`, `out_err` may be null individually — the caller
/// loses access to that channel if it passes null.
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_first_ies_bytes(
    gldf_path: *const c_char,
    out_buf: *mut *mut u8,
    out_len: *mut usize,
    out_err: *mut *mut c_char,
) -> i32 {
    if !out_buf.is_null() {
        *out_buf = std::ptr::null_mut();
    }
    if !out_len.is_null() {
        *out_len = 0;
    }
    if !out_err.is_null() {
        *out_err = std::ptr::null_mut();
    }

    let result = (|| -> Result<Vec<u8>> {
        let path = cstr_to_pathbuf(gldf_path, "gldf_path")?;
        let exporter = Exporter::from_path(&path)?;
        // First variant.
        let buf = exporter.file_buf();
        let variant_id = buf
            .gldf
            .product_definitions
            .variants
            .as_ref()
            .and_then(|v| v.variant.first())
            .map(|v| v.id.clone())
            .ok_or_else(|| {
                ExportError::Internal("GLDF has no variants".into())
            })?;
        // First non-emergency emitter (build_ies_outputs_for_variant
        // already filters emergency-only).
        let outs = crate::photometry::build_ies_outputs_for_variant(buf, &variant_id)?;
        let first = outs.into_iter().next().ok_or_else(|| {
            ExportError::Photometry(format!(
                "variant {variant_id:?} has no non-emergency emitters"
            ))
        })?;
        Ok(first.ies_bytes)
    })();

    match result {
        Ok(bytes) => {
            // Hand ownership to the caller via a leaked Box<[u8]>.
            let boxed: Box<[u8]> = bytes.into_boxed_slice();
            let len = boxed.len();
            let ptr = Box::into_raw(boxed) as *mut u8;
            if !out_buf.is_null() {
                *out_buf = ptr;
            }
            if !out_len.is_null() {
                *out_len = len;
            }
            0
        }
        Err(e) => {
            let code = e.code();
            if !out_err.is_null() {
                if let Ok(cstr) = CString::new(e.to_string()) {
                    *out_err = cstr.into_raw();
                }
            }
            code
        }
    }
}

/// Free a byte buffer previously returned by
/// [`gldf_unreal_first_ies_bytes`] (or any future call documented to
/// hand back a Rust-owned `u8*`). The caller MUST pass back exactly the
/// pointer and length they received — using a different length is
/// undefined behavior (we reconstruct a `Box<[u8]>` to drop it).
///
/// # Safety
/// `ptr` must be a pointer previously returned by this library, paired
/// with the `len` it was returned with, or `ptr` may be NULL.
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_bytes_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    let slice: *mut [u8] = std::slice::from_raw_parts_mut(ptr, len);
    let _ = Box::from_raw(slice);
}

// ─── Interchange mesh payload (Phase 2) ──────────────────────────────────
//
// Mesh data is too large + multi-array to hand back the Phase-1 way (one
// owned blob + paired free). Instead we use a HANDLE pattern:
//
//   1. gldf_unreal_first_mesh_open(path) parses the L3D OBJ, stores the
//      result in a process-global registry, returns a u64 handle and
//      fills a header struct (counts).
//   2. gldf_unreal_mesh_borrow(handle) hands back read-only pointers into
//      the registry-owned arrays. Pointers stay valid until close().
//   3. gldf_unreal_mesh_material_group(handle, i) returns the i-th
//      material-group name as a Rust CString (caller frees via
//      gldf_unreal_string_free).
//   4. gldf_unreal_mesh_close(handle) drops the registry entry.
//
// The C++ Interchange translator opens a handle in Translate(), keeps it
// on the translator instance, reuses it in GetMeshPayloadData(), and
// closes it when done — avoiding a re-parse of the OBJ.

/// Layout MUST match `GldfMeshHeader` in the generated C header.
#[repr(C)]
pub struct GldfMeshHeader {
    pub vertex_count: u32,
    /// 0 when the source OBJ has no `vn` lines.
    pub normal_count: u32,
    /// 0 when the source OBJ has no `vt` lines.
    pub uv_count: u32,
    pub polygon_count: u32,
    /// Sum of all polygons' corner counts.
    pub corner_count: u32,
    pub material_group_count: u32,
}

/// One face corner, flattened for C. `normal_idx` / `uv_idx` are -1 when
/// the source face token had no normal / uv. `polygon_idx` says which
/// polygon (0-based) this corner belongs to so the C++ side can rebuild
/// the n-gon grouping without a second indirection array.
#[repr(C)]
pub struct GldfMeshCorner {
    pub position_idx: u32,
    pub normal_idx: i32,
    pub uv_idx: i32,
    pub polygon_idx: u32,
}

/// One polygon: a contiguous run in the corners array + its material
/// group index (into the per-handle material-group name list).
#[repr(C)]
pub struct GldfMeshPolygon {
    pub corner_offset: u32,
    pub corner_count: u32,
    pub material_group_idx: u32,
}

/// Registry-owned, FFI-friendly snapshot of a parsed mesh. Flattened so
/// the borrow call can hand back stable raw pointers. Built once at
/// open(), lives until close().
struct MeshHandleData {
    /// vertex_count * 3 f32, xyz interleaved.
    positions: Vec<f32>,
    /// normal_count * 3 f32 (empty if none).
    normals: Vec<f32>,
    /// uv_count * 2 f32 (empty if none).
    uvs: Vec<f32>,
    corners: Vec<GldfMeshCorner>,
    polygons: Vec<GldfMeshPolygon>,
    material_groups: Vec<String>,
}

impl MeshHandleData {
    fn from_parsed(m: &GldfMeshData) -> Self {
        let groups = m.material_groups();
        let group_index = |name: &str| -> u32 {
            groups.iter().position(|g| g == name).unwrap_or(0) as u32
        };

        let mut positions = Vec::with_capacity(m.positions.len() * 3);
        for p in &m.positions {
            positions.extend_from_slice(p);
        }
        let mut normals = Vec::with_capacity(m.normals.len() * 3);
        for n in &m.normals {
            normals.extend_from_slice(n);
        }
        let mut uvs = Vec::with_capacity(m.uvs.len() * 2);
        for uv in &m.uvs {
            uvs.extend_from_slice(uv);
        }

        let mut corners = Vec::with_capacity(m.corner_count());
        let mut polygons = Vec::with_capacity(m.polygons.len());
        for (poly_idx, poly) in m.polygons.iter().enumerate() {
            let corner_offset = corners.len() as u32;
            for c in &poly.corners {
                corners.push(GldfMeshCorner {
                    position_idx: c.position_idx,
                    normal_idx: c.normal_idx.map(|n| n as i32).unwrap_or(-1),
                    uv_idx: c.uv_idx.map(|u| u as i32).unwrap_or(-1),
                    polygon_idx: poly_idx as u32,
                });
            }
            polygons.push(GldfMeshPolygon {
                corner_offset,
                corner_count: poly.corners.len() as u32,
                material_group_idx: group_index(&poly.material_group),
            });
        }

        Self {
            positions,
            normals,
            uvs,
            corners,
            polygons,
            material_groups: groups,
        }
    }

    fn header(&self) -> GldfMeshHeader {
        GldfMeshHeader {
            vertex_count: (self.positions.len() / 3) as u32,
            normal_count: (self.normals.len() / 3) as u32,
            uv_count: (self.uvs.len() / 2) as u32,
            polygon_count: self.polygons.len() as u32,
            corner_count: self.corners.len() as u32,
            material_group_count: self.material_groups.len() as u32,
        }
    }
}

static MESH_REGISTRY: Mutex<Option<HashMap<u64, Box<MeshHandleData>>>> = Mutex::new(None);
static MESH_NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Open the first mesh of the first variant of the GLDF at `gldf_path`.
///
/// On success returns a non-zero handle and fills `*out_header`. On
/// failure returns 0 and writes a Rust-owned error string to `*out_err`
/// (caller frees via [`gldf_unreal_string_free`]).
///
/// The handle owns the parsed mesh until [`gldf_unreal_mesh_close`].
///
/// # Safety
/// `gldf_path` must be a valid NUL-terminated UTF-8 C string.
/// `out_header` / `out_err` may individually be null.
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_first_mesh_open(
    gldf_path: *const c_char,
    out_header: *mut GldfMeshHeader,
    out_err: *mut *mut c_char,
) -> u64 {
    if !out_err.is_null() {
        *out_err = std::ptr::null_mut();
    }

    let result = (|| -> Result<MeshHandleData> {
        let path = cstr_to_pathbuf(gldf_path, "gldf_path")?;
        let exporter = Exporter::from_path(&path)?;
        let buf = exporter.file_buf();
        let variant_id = buf
            .gldf
            .product_definitions
            .variants
            .as_ref()
            .and_then(|v| v.variant.first())
            .map(|v| v.id.clone())
            .ok_or_else(|| ExportError::Internal("GLDF has no variants".into()))?;
        let parsed = build_first_mesh_for_variant(buf, &variant_id)?;
        Ok(MeshHandleData::from_parsed(&parsed))
    })();

    match result {
        Ok(data) => {
            if !out_header.is_null() {
                *out_header = data.header();
            }
            let id = MESH_NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let mut guard = MESH_REGISTRY.lock().unwrap();
            guard.get_or_insert_with(HashMap::new).insert(id, Box::new(data));
            id
        }
        Err(e) => {
            if !out_err.is_null() {
                if let Ok(cstr) = CString::new(e.to_string()) {
                    *out_err = cstr.into_raw();
                }
            }
            0
        }
    }
}

/// Borrow read-only pointers into a handle's mesh arrays. Pointers stay
/// valid until [`gldf_unreal_mesh_close`] on the same handle.
///
/// `*out_normals` / `*out_uvs` are set to null when the source had no
/// normals / UVs (`normal_count` / `uv_count` == 0 in the header).
///
/// Returns 0 on success, non-zero if the handle is unknown.
///
/// # Safety
/// `handle` must be a live handle from [`gldf_unreal_first_mesh_open`].
/// The out-pointers may individually be null (that channel is skipped).
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_mesh_borrow(
    handle: u64,
    out_positions: *mut *const f32,
    out_normals: *mut *const f32,
    out_uvs: *mut *const f32,
    out_corners: *mut *const GldfMeshCorner,
    out_polygons: *mut *const GldfMeshPolygon,
) -> i32 {
    let guard = MESH_REGISTRY.lock().unwrap();
    let Some(map) = guard.as_ref() else {
        return 1;
    };
    let Some(data) = map.get(&handle) else {
        return 1;
    };

    if !out_positions.is_null() {
        *out_positions = data.positions.as_ptr();
    }
    if !out_normals.is_null() {
        *out_normals = if data.normals.is_empty() {
            std::ptr::null()
        } else {
            data.normals.as_ptr()
        };
    }
    if !out_uvs.is_null() {
        *out_uvs = if data.uvs.is_empty() {
            std::ptr::null()
        } else {
            data.uvs.as_ptr()
        };
    }
    if !out_corners.is_null() {
        *out_corners = data.corners.as_ptr();
    }
    if !out_polygons.is_null() {
        *out_polygons = data.polygons.as_ptr();
    }
    0
}

/// Return the `group_idx`-th material-group name for a handle, as a
/// Rust-owned CString (caller frees via [`gldf_unreal_string_free`]).
/// Returns null if the handle or index is invalid.
///
/// # Safety
/// `handle` must be a live handle from [`gldf_unreal_first_mesh_open`].
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_mesh_material_group(
    handle: u64,
    group_idx: u32,
) -> *mut c_char {
    let guard = MESH_REGISTRY.lock().unwrap();
    let Some(map) = guard.as_ref() else {
        return std::ptr::null_mut();
    };
    let Some(data) = map.get(&handle) else {
        return std::ptr::null_mut();
    };
    match data.material_groups.get(group_idx as usize) {
        Some(name) => match CString::new(name.as_str()) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Drop a mesh handle and free its arrays. No-op if the handle is unknown
/// or zero.
///
/// # Safety
/// `handle` must be a handle from [`gldf_unreal_first_mesh_open`] or 0.
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_mesh_close(handle: u64) {
    if handle == 0 {
        return;
    }
    let mut guard = MESH_REGISTRY.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        map.remove(&handle);
    }
}

// ─── internal helpers ────────────────────────────────────────────────────

unsafe fn export_impl(
    gldf_path: *const c_char,
    out_dir: *const c_char,
    opts: *const GldfUnrealOpts,
) -> Result<ExportReport> {
    let gldf_path = cstr_to_pathbuf(gldf_path, "gldf_path")?;
    let out_dir = cstr_to_pathbuf(out_dir, "out_dir")?;
    let export_opts = build_options(out_dir, opts)?;
    Exporter::from_path(&gldf_path)?.export(&export_opts)
}

unsafe fn cstr_to_pathbuf(p: *const c_char, name: &str) -> Result<PathBuf> {
    if p.is_null() {
        return Err(ExportError::Internal(format!("{name} is null")));
    }
    let s = CStr::from_ptr(p)
        .to_str()
        .map_err(|e| ExportError::Internal(format!("{name} is not valid UTF-8: {e}")))?;
    Ok(PathBuf::from(s))
}

unsafe fn build_options(out_dir: PathBuf, opts: *const GldfUnrealOpts) -> Result<ExportOptions> {
    if opts.is_null() {
        return Err(ExportError::Internal("opts pointer is null".into()));
    }
    let o = &*opts;
    let units = if o.units_cm == 0 {
        UnitSystem::M
    } else {
        UnitSystem::Cm
    };
    let variants = if o.variants_csv.is_null() {
        VariantSelector::All
    } else {
        let s = CStr::from_ptr(o.variants_csv)
            .to_str()
            .map_err(|e| ExportError::Internal(format!("variants_csv not UTF-8: {e}")))?;
        match s.trim() {
            "" | "all" => VariantSelector::All,
            "first" => VariantSelector::First,
            other => VariantSelector::Only(
                other
                    .split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect(),
            ),
        }
    };
    Ok(ExportOptions {
        out_dir,
        bundle_name: "Luminaire".into(),
        units,
        variants,
        embed_textures: o.embed_textures != 0,
        apply_mounting: o.apply_mounting != 0,
        overwrite: o.overwrite != 0,
    })
}

/// Minimal JSON-by-hand for `ExportReport`. Avoids pulling serde_json
/// purely for this — keeps the FFI surface dep-light. Keys match
/// `BundleArtifact` field names; paths are escaped with `escape_json`.
fn report_to_json(r: &ExportReport) -> String {
    let mut out = String::from("{\"bundles\":[");
    for (i, b) in r.bundles.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"variant_id\":\"");
        out.push_str(&escape_json(&b.variant_id));
        out.push_str("\",\"udatasmith_path\":\"");
        out.push_str(&escape_json(&b.udatasmith_path.to_string_lossy()));
        out.push_str("\",\"asset_paths\":[");
        for (j, p) in b.asset_paths.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&escape_json(&p.to_string_lossy()));
            out.push('"');
        }
        out.push_str("]}");
    }
    out.push_str("]}");
    out
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_returns_pkg_version() {
        // SAFETY: gldf_unreal_version returns a static NUL-terminated str.
        let s = unsafe { CStr::from_ptr(gldf_unreal_version()) }
            .to_str()
            .unwrap();
        assert_eq!(s, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn null_paths_return_internal_error() {
        let opts = GldfUnrealOpts {
            units_cm: 1,
            embed_textures: 0,
            apply_mounting: 0,
            overwrite: 1,
            variants_csv: std::ptr::null(),
        };
        let mut err: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            gldf_unreal_export(
                std::ptr::null(),
                std::ptr::null(),
                &opts,
                &mut err as *mut *mut c_char,
            )
        };
        assert_eq!(code, 99, "null path should map to Internal");
        assert!(!err.is_null());
        unsafe { gldf_unreal_string_free(err) };
    }

    #[test]
    fn null_opts_returns_internal_error() {
        let p = CString::new("/nonexistent.gldf").unwrap();
        let mut err: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            gldf_unreal_export(
                p.as_ptr(),
                p.as_ptr(),
                std::ptr::null(),
                &mut err as *mut *mut c_char,
            )
        };
        assert_eq!(code, 99);
        unsafe { gldf_unreal_string_free(err) };
    }

    #[test]
    fn string_free_handles_null() {
        unsafe { gldf_unreal_string_free(std::ptr::null_mut()) };
        // No assertion needed — just shouldn't crash.
    }

    #[test]
    fn bytes_free_handles_null() {
        unsafe { gldf_unreal_bytes_free(std::ptr::null_mut(), 0) };
        unsafe { gldf_unreal_bytes_free(std::ptr::null_mut(), 999) };
        // Both no-ops; mustn't crash on null even with a non-zero len.
    }

    #[test]
    fn first_ies_bytes_null_path_errors() {
        let mut buf: *mut u8 = std::ptr::null_mut();
        let mut len: usize = 0;
        let mut err: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            gldf_unreal_first_ies_bytes(
                std::ptr::null(),
                &mut buf as *mut *mut u8,
                &mut len as *mut usize,
                &mut err as *mut *mut c_char,
            )
        };
        assert_eq!(code, 99, "null path → Internal error code");
        assert!(buf.is_null());
        assert_eq!(len, 0);
        assert!(!err.is_null());
        unsafe { gldf_unreal_string_free(err) };
    }

    #[test]
    fn first_ies_bytes_alurays_round_trip() {
        // The Phase 1 happy path: load alurays-3000mm.gldf, ask for the
        // first emitter's IES bytes, expect a non-empty LM-63 blob.
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("tests/data/alurays-3000mm.gldf");
        if !fixture.exists() {
            eprintln!("skipping: {} not present", fixture.display());
            return;
        }
        let path_c = CString::new(fixture.to_string_lossy().as_bytes()).unwrap();

        let mut buf: *mut u8 = std::ptr::null_mut();
        let mut len: usize = 0;
        let mut err: *mut c_char = std::ptr::null_mut();

        let code = unsafe {
            gldf_unreal_first_ies_bytes(
                path_c.as_ptr(),
                &mut buf as *mut *mut u8,
                &mut len as *mut usize,
                &mut err as *mut *mut c_char,
            )
        };
        assert_eq!(code, 0, "expected success; err = {err:?}");
        assert!(!buf.is_null());
        assert!(len > 0);

        // Copy out before freeing so we can inspect.
        let bytes: Vec<u8> = unsafe { std::slice::from_raw_parts(buf, len).to_vec() };
        unsafe { gldf_unreal_bytes_free(buf, len) };

        // LM-63 IES files start with "IESNA:" or "TILT=" / "IES:" — the
        // gldf-rs photometry pipeline goes through eulumdat's IES
        // exporter, which writes the modern header.
        let head: String = bytes
            .iter()
            .take(64)
            .map(|&b| b as char)
            .collect();
        assert!(
            head.contains("IES") || head.contains("TILT="),
            "expected an IES LM-63 header; got bytes starting with {:?}",
            head
        );
    }

    #[test]
    fn report_json_round_trip_smoke() {
        let r = ExportReport {
            bundles: vec![crate::exporter::BundleArtifact {
                variant_id: "var\"01".into(),
                udatasmith_path: PathBuf::from("/tmp/a/b.udatasmith"),
                asset_paths: vec![PathBuf::from("/tmp/a/Assets/Ies/e1.ies")],
            }],
        };
        let json = report_to_json(&r);
        assert!(json.contains("\"variant_id\":\"var\\\"01\""));
        assert!(json.contains("\"udatasmith_path\":\"/tmp/a/b.udatasmith\""));
        assert!(json.contains("\"asset_paths\":[\"/tmp/a/Assets/Ies/e1.ies\"]"));
    }

    // ─── mesh handle API (Phase 2b) ──────────────────────────────────────

    #[test]
    fn mesh_open_null_path_returns_zero_handle() {
        let mut header = GldfMeshHeader {
            vertex_count: 7,
            normal_count: 7,
            uv_count: 7,
            polygon_count: 7,
            corner_count: 7,
            material_group_count: 7,
        };
        let mut err: *mut c_char = std::ptr::null_mut();
        let handle = unsafe {
            gldf_unreal_first_mesh_open(
                std::ptr::null(),
                &mut header as *mut GldfMeshHeader,
                &mut err as *mut *mut c_char,
            )
        };
        assert_eq!(handle, 0, "null path → handle 0");
        assert!(!err.is_null());
        unsafe { gldf_unreal_string_free(err) };
    }

    #[test]
    fn mesh_borrow_unknown_handle_errors() {
        let mut positions: *const f32 = std::ptr::null();
        let code = unsafe {
            gldf_unreal_mesh_borrow(
                999_999,
                &mut positions as *mut *const f32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(code, 1, "unknown handle → non-zero");
    }

    #[test]
    fn mesh_close_handles_zero_and_unknown() {
        unsafe { gldf_unreal_mesh_close(0) };
        unsafe { gldf_unreal_mesh_close(123_456) };
        // Both no-ops; mustn't crash.
    }

    #[test]
    fn mesh_open_borrow_close_alurays() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("tests/data/alurays-3000mm.gldf");
        if !fixture.exists() {
            eprintln!("skipping: {} not present", fixture.display());
            return;
        }
        let path_c = CString::new(fixture.to_string_lossy().as_bytes()).unwrap();

        let mut header = GldfMeshHeader {
            vertex_count: 0,
            normal_count: 0,
            uv_count: 0,
            polygon_count: 0,
            corner_count: 0,
            material_group_count: 0,
        };
        let mut err: *mut c_char = std::ptr::null_mut();

        let handle = unsafe {
            gldf_unreal_first_mesh_open(
                path_c.as_ptr(),
                &mut header as *mut GldfMeshHeader,
                &mut err as *mut *mut c_char,
            )
        };
        assert_ne!(handle, 0, "expected a live handle; err = {err:?}");
        assert!(err.is_null());

        // alurays: 6468 verts, 6468 normals, 0 uvs, 9998 tris.
        assert_eq!(header.vertex_count, 6468);
        assert_eq!(header.normal_count, 6468);
        assert_eq!(header.uv_count, 0);
        assert_eq!(header.polygon_count, 9998);
        assert_eq!(header.corner_count, 9998 * 3);
        assert_eq!(header.material_group_count, 1);

        // Borrow + sanity-check the array pointers.
        let mut positions: *const f32 = std::ptr::null();
        let mut normals: *const f32 = std::ptr::null();
        let mut uvs: *const f32 = std::ptr::null();
        let mut corners: *const GldfMeshCorner = std::ptr::null();
        let mut polygons: *const GldfMeshPolygon = std::ptr::null();
        let code = unsafe {
            gldf_unreal_mesh_borrow(
                handle,
                &mut positions as *mut *const f32,
                &mut normals as *mut *const f32,
                &mut uvs as *mut *const f32,
                &mut corners as *mut *const GldfMeshCorner,
                &mut polygons as *mut *const GldfMeshPolygon,
            )
        };
        assert_eq!(code, 0);
        assert!(!positions.is_null());
        assert!(!normals.is_null(), "alurays has normals");
        assert!(uvs.is_null(), "alurays has no UVs → null pointer");
        assert!(!corners.is_null());
        assert!(!polygons.is_null());

        // First polygon should start at corner offset 0 with 3 corners.
        let first_poly = unsafe { &*polygons };
        assert_eq!(first_poly.corner_offset, 0);
        assert_eq!(first_poly.corner_count, 3);
        assert_eq!(first_poly.material_group_idx, 0);

        // Material group 0 name round-trips.
        let grp = unsafe { gldf_unreal_mesh_material_group(handle, 0) };
        assert!(!grp.is_null());
        let grp_str = unsafe { CStr::from_ptr(grp) }.to_str().unwrap().to_owned();
        assert_eq!(grp_str, "material_0");
        unsafe { gldf_unreal_string_free(grp) };

        // Out-of-range group → null.
        let oob = unsafe { gldf_unreal_mesh_material_group(handle, 99) };
        assert!(oob.is_null());

        unsafe { gldf_unreal_mesh_close(handle) };

        // After close, borrow should fail.
        let code2 = unsafe {
            gldf_unreal_mesh_borrow(
                handle,
                &mut positions as *mut *const f32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(code2, 1, "borrow after close → error");
    }
}
