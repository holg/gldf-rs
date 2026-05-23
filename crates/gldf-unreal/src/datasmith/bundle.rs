//! Filesystem layout for a Datasmith bundle.
//!
//! ```text
//! <out_dir>/<bundle_name>__<variant_id>/
//!     <bundle_name>__<variant_id>.udatasmith
//!     Assets/
//!         Geometry/
//!             luminaire_body.obj
//!             luminaire_body.mtl
//!             textures/...
//!         Ies/
//!             emitter_<photometry_id>.ies
//!         Manifest.json
//! ```
//!
//! Phase 2 implements directory creation + file writes; Phase 1 declares
//! the layout contract.

#![allow(dead_code)]
