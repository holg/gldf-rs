//! C ABI for the ARTICULATED L3D structure: nodes (geometry parts + joints
//! with axis limits), per-part meshes in node space, and LEO → node
//! ownership. Lets a consumer build a real part hierarchy and animate the
//! joints (pan / tilt a spot head within the manufacturer's limits) instead
//! of one baked mesh. The flat API (`gldf_unreal_first_mesh_open`,
//! `gldf_unreal_leo_at`) stays for consumers that don't care.
//!
//! Frames: everything is in the L3D frame (right-handed Z-up); the consumer
//! applies its own basis change (UE: negate Y, reverse winding). Matrices are
//! column-major `[f32; 16]` with translations already in CENTIMETRES.

use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::error::Result;
use crate::exporter::Exporter;
use crate::ffi::{
    cstr_to_pathbuf, leo_shape_from_placement, GldfLeoShape, GldfMeshHeader, MeshHandleData,
    MESH_NEXT_ID, MESH_REGISTRY,
};
use crate::mesh_payload::{collect_l3d_structure, L3dStructure};

/// One L3D node. `parent` is -1 for the root. `axis_mask` bit 0/1/2 =
/// the joint rotates about its local X/Y/Z (limits in `axis_min/max/step`,
/// degrees); 0 for geometry nodes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GldfL3dNode {
    pub parent: i32,
    pub is_joint: u8,
    pub has_mesh: u8,
    pub axis_mask: u8,
    pub has_default_rotation: u8,
    /// parent_world · local, column-major, translation in cm.
    pub world: [f32; 16],
    /// Rest transform relative to the parent node, column-major, translation in cm.
    pub local: [f32; 16],
    /// The authored rest pose: position in cm and rotation in degrees
    /// (X, then Y, then Z) relative to the parent node.
    pub local_pos_cm: [f32; 3],
    pub local_rot_deg: [f32; 3],
    pub axis_min: [f32; 3],
    pub axis_max: [f32; 3],
    pub axis_step: [f32; 3],
    /// Joint `DefaultRotation` in degrees (valid when `has_default_rotation`).
    pub default_rotation_deg: [f32; 3],
}

/// A LEO with the index of the geometry node that carries it. `shape` is
/// the same world pose `gldf_unreal_leo_at` reports (cm, L3D frame, rest
/// pose of all joints).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GldfL3dLeo {
    pub node: i32,
    pub shape: GldfLeoShape,
}

struct L3dHandleData {
    structure: L3dStructure,
}

static L3D_REGISTRY: Mutex<Option<HashMap<u64, Box<L3dHandleData>>>> = Mutex::new(None);
static L3D_NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn cm_matrix(m: &[f32; 16]) -> [f32; 16] {
    let mut out = *m;
    out[12] *= 100.0;
    out[13] *= 100.0;
    out[14] *= 100.0;
    out
}

/// Open the articulated structure of the GLDF's first L3D. Returns a
/// non-zero handle and writes the node / LEO counts; 0 and `*out_err` on
/// failure (caller frees via `gldf_unreal_string_free`).
///
/// # Safety
/// `gldf_path` is a NUL-terminated UTF-8 C string; out-pointers may be null.
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_l3d_open(
    gldf_path: *const c_char,
    out_node_count: *mut u32,
    out_leo_count: *mut u32,
    out_err: *mut *mut c_char,
) -> u64 {
    if !out_err.is_null() {
        *out_err = std::ptr::null_mut();
    }
    let result = (|| -> Result<L3dStructure> {
        let path = unsafe { cstr_to_pathbuf(gldf_path, "gldf_path") }?;
        let exporter = Exporter::from_path(&path)?;
        collect_l3d_structure(exporter.file_buf())
    })();
    match result {
        Ok(structure) => {
            if !out_node_count.is_null() {
                *out_node_count = structure.nodes.len() as u32;
            }
            if !out_leo_count.is_null() {
                *out_leo_count = structure.leos.len() as u32;
            }
            let id = L3D_NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let mut guard = L3D_REGISTRY.lock().unwrap();
            guard
                .get_or_insert_with(HashMap::new)
                .insert(id, Box::new(L3dHandleData { structure }));
            id
        }
        Err(e) => {
            if !out_err.is_null() {
                if let Ok(c) = CString::new(e.to_string()) {
                    *out_err = c.into_raw();
                }
            }
            0
        }
    }
}

/// Fill `*out` with node `idx`. Returns 0 on success, non-zero on an unknown
/// handle / index.
///
/// # Safety
/// `handle` must be live; `out` may be null.
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_l3d_node(handle: u64, idx: u32, out: *mut GldfL3dNode) -> i32 {
    let guard = L3D_REGISTRY.lock().unwrap();
    let Some(data) = guard.as_ref().and_then(|m| m.get(&handle)) else {
        return 1;
    };
    let Some(node) = data.structure.nodes.get(idx as usize) else {
        return 2;
    };
    if out.is_null() {
        return 0;
    }
    let mut n = GldfL3dNode {
        parent: node.parent.map(|p| p as i32).unwrap_or(-1),
        is_joint: node.is_joint as u8,
        has_mesh: node.mesh.is_some() as u8,
        axis_mask: 0,
        has_default_rotation: node.default_rotation_deg.is_some() as u8,
        world: cm_matrix(&node.world),
        local: cm_matrix(&node.local),
        local_pos_cm: [
            node.local_pos_m[0] * 100.0,
            node.local_pos_m[1] * 100.0,
            node.local_pos_m[2] * 100.0,
        ],
        local_rot_deg: node.local_rot_deg,
        axis_min: [0.0; 3],
        axis_max: [0.0; 3],
        axis_step: [0.0; 3],
        default_rotation_deg: node.default_rotation_deg.unwrap_or([0.0; 3]),
    };
    for (i, axis) in node.axes.iter().enumerate() {
        if let Some(a) = axis {
            n.axis_mask |= 1 << i;
            n.axis_min[i] = a.min;
            n.axis_max[i] = a.max;
            n.axis_step[i] = a.step;
        }
    }
    *out = n;
    0
}

/// Node `idx`'s `partName` as a Rust CString (free via
/// `gldf_unreal_string_free`); null on an unknown handle / index.
///
/// # Safety
/// `handle` must be live.
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_l3d_node_name(handle: u64, idx: u32) -> *mut c_char {
    let guard = L3D_REGISTRY.lock().unwrap();
    let Some(data) = guard.as_ref().and_then(|m| m.get(&handle)) else {
        return std::ptr::null_mut();
    };
    let Some(node) = data.structure.nodes.get(idx as usize) else {
        return std::ptr::null_mut();
    };
    CString::new(node.name.clone())
        .map(|c| c.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// Open node `idx`'s part mesh (node-local frame, cm) as a MESH handle for
/// `gldf_unreal_mesh_borrow` / `gldf_unreal_mesh_close`. Returns 0 when the
/// node has no mesh (joints, missing assets).
///
/// # Safety
/// `handle` must be live; `out_header` may be null.
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_l3d_node_mesh_open(
    handle: u64,
    idx: u32,
    out_header: *mut GldfMeshHeader,
) -> u64 {
    let data = {
        let guard = L3D_REGISTRY.lock().unwrap();
        let Some(h) = guard.as_ref().and_then(|m| m.get(&handle)) else {
            return 0;
        };
        let Some(node) = h.structure.nodes.get(idx as usize) else {
            return 0;
        };
        let Some(mesh) = node.mesh.as_ref() else {
            return 0;
        };
        MeshHandleData::from_parsed(mesh)
    };
    if !out_header.is_null() {
        *out_header = data.header();
    }
    let id = MESH_NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let mut guard = MESH_REGISTRY.lock().unwrap();
    guard
        .get_or_insert_with(HashMap::new)
        .insert(id, Box::new(data));
    id
}

/// Fill `*out` with LEO `idx` (owning node + rest-pose world shape).
/// Returns 0 on success, non-zero on an unknown handle / index.
///
/// # Safety
/// `handle` must be live; `out` may be null.
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_l3d_leo(handle: u64, idx: u32, out: *mut GldfL3dLeo) -> i32 {
    let guard = L3D_REGISTRY.lock().unwrap();
    let Some(data) = guard.as_ref().and_then(|m| m.get(&handle)) else {
        return 1;
    };
    let Some(leo) = data.structure.leos.get(idx as usize) else {
        return 2;
    };
    if !out.is_null() {
        *out = GldfL3dLeo {
            node: leo.node as i32,
            shape: leo_shape_from_placement(&leo.placement),
        };
    }
    0
}

/// Drop an L3D structure handle. No-op on zero / unknown.
///
/// # Safety
/// `handle` must be from `gldf_unreal_l3d_open` or 0.
#[no_mangle]
pub unsafe extern "C" fn gldf_unreal_l3d_close(handle: u64) {
    if handle == 0 {
        return;
    }
    let mut guard = L3D_REGISTRY.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        map.remove(&handle);
    }
}
