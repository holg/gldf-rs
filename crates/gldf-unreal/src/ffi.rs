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

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::error::{ExportError, Result};
use crate::exporter::{ExportReport, Exporter};
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
}
