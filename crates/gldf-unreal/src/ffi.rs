//! C ABI surface for the Unreal Engine plugin.
//!
//! Four functions total — kept deliberately tiny because the UE plugin is
//! a thin shim that just shells out to [`gldf_unreal_export`] when the
//! editor user picks a `.gldf` from the import dialog.
//!
//! All `*const c_char` arguments are caller-owned, NUL-terminated UTF-8.
//! `out_err` and the report JSON returned by [`gldf_unreal_last_report_json`]
//! are Rust-owned heap strings; the C caller MUST free them via
//! [`gldf_unreal_string_free`].
//!
//! Error codes returned by [`gldf_unreal_export`] map 1:1 to
//! [`ExportError::code`](crate::error::ExportError::code) — see that
//! function for the table.
//!
//! Phase 4 implements the real bodies; Phase 1 stubs them out so the
//! C header generator (`bin/gen-header.rs`) has the surface to scan.

#![allow(dead_code)]

use std::os::raw::c_char;

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

/// Returns a pointer to a static NUL-terminated string with the crate
/// version. The pointer is valid for the program's lifetime; do NOT free.
#[no_mangle]
pub extern "C" fn gldf_unreal_version() -> *const c_char {
    // The `\0` keeps the byte literal NUL-terminated for the C consumer.
    static VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VERSION.as_ptr() as *const c_char
}

/// Export `gldf_path` to `out_dir` using `opts`.
///
/// On success returns 0 and leaves `*out_err = NULL`. On failure returns
/// the [`ExportError::code`] and writes a Rust-owned, heap-allocated
/// NUL-terminated UTF-8 string to `*out_err`; the C caller MUST free it
/// via [`gldf_unreal_string_free`].
///
/// Phase 1: stub that always returns an Internal error (code 99). Phase 4
/// wires this to [`crate::export_gldf_to_datasmith`].
///
/// # Safety
/// All pointer arguments must be valid for their stated direction. The
/// caller retains ownership of `gldf_path`, `out_dir`, `opts`, and
/// `opts.variants_csv`. If `out_err` is non-null, the caller must free the
/// resulting string via [`gldf_unreal_string_free`].
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_export(
    _gldf_path: *const c_char,
    _out_dir: *const c_char,
    _opts: *const GldfUnrealOpts,
    _out_err: *mut *mut c_char,
) -> i32 {
    // TODO(phase-4): implement.
    99
}

/// Return a JSON serialization of the most recent successful
/// [`ExportReport`](crate::exporter::ExportReport), or NULL if no export
/// has run on this thread. Rust-owned string; free via
/// [`gldf_unreal_string_free`].
///
/// Phase 1: always NULL.
#[no_mangle]
pub extern "C" fn gldf_unreal_last_report_json() -> *mut c_char {
    std::ptr::null_mut()
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
    // SAFETY: contract above — the pointer came from `CString::into_raw`
    // in Phase 4. Phase 1 never hands one out, so this branch is currently
    // dead; the body is correct for when it goes live.
    let _ = std::ffi::CString::from_raw(s);
}
