//! Datasmith `.udatasmith` XML emission and bundle filesystem layout.
//!
//! Phase 2 introduces a minimal document with one `<StaticMesh>` referencing
//! an external OBJ; Phase 3 adds `<ActorLight type="Spot">` with `<Ies>`
//! children and per-variant intensity.
//!
//! The exact element names / attribute spellings reflect Datasmith schema
//! 0.24 (UE 5.4). The reference fixture lives at
//! `tests/data/datasmith/reference.udatasmith` (committed alongside the
//! first real Phase 2 milestone).

#![allow(dead_code)]

pub mod bundle;
pub mod elements;
pub mod xml;
