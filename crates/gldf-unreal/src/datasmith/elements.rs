//! Typed structs for Datasmith XML elements.
//!
//! Phase 1 stubs in the minimum shapes Phase 2/3 will flesh out. Kept in a
//! separate module from the writer so the schema lives in one place and the
//! serializer in another.

#![allow(dead_code)]

/// Root `<DatasmithUnrealScene>` document.
#[derive(Debug, Clone)]
pub struct DatasmithDocument {
    pub version: String,
    pub sdk_version: String,
    pub host: String,
    pub resource_path: String,
    pub static_meshes: Vec<StaticMesh>,
    pub actors: Vec<Actor>,
}

#[derive(Debug, Clone)]
pub struct StaticMesh {
    pub name: String,
    pub label: String,
    pub file_path: String,
}

#[derive(Debug, Clone, Default)]
pub struct Actor {
    pub name: String,
    pub layer: String,
    pub transform: Transform,
    pub children: Vec<ActorChild>,
}

#[derive(Debug, Clone)]
pub enum ActorChild {
    Mesh(ActorMesh),
    Light(ActorLight),
}

#[derive(Debug, Clone)]
pub struct ActorMesh {
    pub name: String,
    pub mesh_ref: String,
    pub transform: Transform,
}

#[derive(Debug, Clone)]
pub struct ActorLight {
    pub name: String,
    pub light_type: LightType,
    pub transform: Transform,
    pub intensity_lumens: f64,
    pub color_temperature_k: Option<i32>,
    pub ies_file: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum LightType {
    Point,
    Spot,
    Area,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Transform {
    pub tx: f64,
    pub ty: f64,
    pub tz: f64,
    pub rx: f64,
    pub ry: f64,
    pub rz: f64,
    pub sx: f64,
    pub sy: f64,
    pub sz: f64,
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            sx: 1.0,
            sy: 1.0,
            sz: 1.0,
            ..Self::default()
        }
    }
}
