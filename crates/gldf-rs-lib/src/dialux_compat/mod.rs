//! DIALux GLDF-import compatibility helpers.
//!
//! DIALux is the dominant GLDF consumer but its importer enforces
//! several constraints **beyond** the rc-3 XSD. A schema-valid GLDF
//! can still be silently rejected — no error dialog, no log line,
//! just "Luminaire Data could not be imported". The findings here
//! mirror the gldf.net `SpectrumDialuxCompatibilityRule` and the
//! manual bisection captured in `docs/_dialux_compat_findings.md`.
//!
//! Two surfaces:
//!
//! * [`check`] — pure inspection. Walks a `GldfProduct` and its
//!   embedded asset bytes and returns a `Vec<DialuxCompatHint>` for
//!   anything that would trip DIALux. Use this on the read side
//!   (validation banner, CLI `dialux-check`).
//!
//! * [`sanitize`] — opt-in transforms. Pure functions that take raw
//!   bytes / strings and return a DIALux-safe version. Use these on
//!   the write side, gated by [`crate::editable::ExportOptions`].
//!
//! Naming convention: every public item is prefixed `dialux_compat_*`
//! or lives inside this module so it never gets confused with
//! genuine GLDF validity (see [`crate::validation`]).

pub mod check;
pub mod sanitize;

pub use check::{dialux_compat_check, DialuxCompatHint};
pub use sanitize::sanitize_spd;
