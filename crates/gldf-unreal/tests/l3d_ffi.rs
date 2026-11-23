//! `gldf_unreal_l3d_*` — the articulated L3D structure: SLV Tria 2 is a
//! base with two heads, each on a two-joint chain (base-head-con-joint →
//! head-joint), the LEO on the head geometry. Joint limits come from the
//! manufacturer's L3D and are what a consumer may animate within.

use std::ffi::{CStr, CString};
use std::path::PathBuf;

use gldf_unreal::ffi::{gldf_unreal_mesh_close, gldf_unreal_string_free, GldfMeshHeader};
use gldf_unreal::ffi_l3d::{
    gldf_unreal_l3d_close, gldf_unreal_l3d_leo, gldf_unreal_l3d_node,
    gldf_unreal_l3d_node_mesh_open, gldf_unreal_l3d_node_name, gldf_unreal_l3d_open, GldfL3dLeo,
    GldfL3dNode,
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

#[test]
fn tria2_has_two_articulated_heads() {
    let path = data("SLV - Tria 2.gldf");
    let (mut nodes, mut leos) = (0u32, 0u32);
    let mut err = std::ptr::null_mut();
    let h = unsafe { gldf_unreal_l3d_open(path.as_ptr(), &mut nodes, &mut leos, &mut err) };
    assert!(h != 0, "l3d_open failed");
    assert_eq!(leos, 2, "two heads → two LEOs");

    let mut joints = 0;
    let mut meshes = 0;
    for i in 0..nodes {
        let mut n = GldfL3dNode::default();
        assert_eq!(unsafe { gldf_unreal_l3d_node(h, i, &mut n) }, 0);
        let raw = unsafe { gldf_unreal_l3d_node_name(h, i) };
        let name = unsafe { CStr::from_ptr(raw) }
            .to_string_lossy()
            .into_owned();
        unsafe { gldf_unreal_string_free(raw) };
        eprintln!(
            "node {i} '{name}' parent {} joint {} mesh {} axes {:#05b} min {:?} max {:?} t=({:.1},{:.1},{:.1}) cm",
            n.parent, n.is_joint, n.has_mesh, n.axis_mask, n.axis_min, n.axis_max, n.world[12], n.world[13], n.world[14]
        );
        if i == 0 {
            assert_eq!(n.parent, -1, "node 0 is the root");
        } else {
            assert!(
                n.parent >= 0 && (n.parent as u32) < i,
                "parents come before children"
            );
        }
        if n.is_joint != 0 {
            joints += 1;
            assert!(n.axis_mask != 0, "a joint rotates about at least one axis");
        }
        if n.has_mesh != 0 {
            let mut hdr = GldfMeshHeader::default();
            let mh = unsafe { gldf_unreal_l3d_node_mesh_open(h, i, &mut hdr) };
            assert!(mh != 0 && hdr.vertex_count > 0, "mesh node has vertices");
            unsafe { gldf_unreal_mesh_close(mh) };
            meshes += 1;
        }
    }
    assert_eq!(joints, 4, "two heads × (connector joint + head joint)");
    assert!(meshes >= 3, "base + two heads (got {meshes})");

    for i in 0..leos {
        let mut l = GldfL3dLeo::default();
        assert_eq!(unsafe { gldf_unreal_l3d_leo(h, i, &mut l) }, 0);
        let mut owner = GldfL3dNode::default();
        assert_eq!(
            unsafe { gldf_unreal_l3d_node(h, l.node as u32, &mut owner) },
            0
        );
        assert_eq!(owner.is_joint, 0, "a LEO sits on a geometry node");
        assert!(owner.parent >= 0, "the head is below a joint, not the root");
        eprintln!(
            "leo {i} on node {} at ({:.1},{:.1},{:.1}) cm",
            l.node, l.shape.pos_x, l.shape.pos_y, l.shape.pos_z
        );
    }
    unsafe { gldf_unreal_l3d_close(h) };
}

#[test]
fn single_part_file_is_one_node() {
    let path = data("alurays-3500mm.gldf");
    let (mut nodes, mut leos) = (0u32, 0u32);
    let h =
        unsafe { gldf_unreal_l3d_open(path.as_ptr(), &mut nodes, &mut leos, std::ptr::null_mut()) };
    assert!(h != 0);
    assert_eq!(nodes, 1);
    assert_eq!(leos, 1);
    unsafe { gldf_unreal_l3d_close(h) };
}
