//! L3D OBJ → parsed mesh arrays for the Interchange translator.
//!
//! Phase 2 of the UE Interchange path. The C++ translator asks Rust
//! for the luminaire's geometry as flat vertex / normal / uv / index
//! arrays, then builds an `FMeshDescription` from them.
//!
//! Source frame is preserved exactly as the L3D OBJ has it (millimetres,
//! right-handed Z-up). **No coordinate transform happens here** — the
//! C++ side applies UE's standard `PositionToUEBasis` / `UVToUEBasis`
//! per element, exactly like UE's own OBJ translator
//! (`InterchangeOBJTranslator.cpp`). Keeping Rust ignorant of UE's
//! coordinate convention means the same arrays are reusable from Unity,
//! Blender, or any other consumer with its own axis rules.

use gldf_rs::mapping::get_first_l3d_with_ldt;
use gldf_rs::FileBufGldf;
use l3d_rs::{
    build_transform, get_scale, mat4_mul, mat4_scale, Geometry, GeometryFileDefinition, Luminaire,
    Mat4, MAT4_IDENTITY,
};

use crate::error::{ExportError, Result};

/// `get_scale("mm") = 0.001` converts L3D source units to **metres**.
/// UE's static-mesh import default is **centimetres**, so we follow the
/// unit-aware metre scale with this constant to land in cm. Handles
/// non-mm units (e.g. "in") correctly because `get_scale` already
/// normalises them to metres first.
const METERS_TO_UE_CM: f32 = 100.0;

/// One face corner: indices into the position / normal / uv arrays.
/// Mirrors a single `a/b/c` token of an OBJ `f` line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshCorner {
    pub position_idx: u32,
    /// `None` when the OBJ face token had no normal index (`a` or `a/b`).
    pub normal_idx: Option<u32>,
    /// `None` when the OBJ face token had no uv index (`a` or `a//c`).
    pub uv_idx: Option<u32>,
}

/// One polygon (n-gon preserved). `material_group` is the active
/// `usemtl` name when the face was declared (empty string if none).
#[derive(Debug, Clone)]
pub struct MeshPolygon {
    pub material_group: String,
    pub corners: Vec<MeshCorner>,
}

/// The shape of an L3D Light Emitting Object (the physical light-emitting
/// surface), used to choose the UE light type. Dimensions are in metres
/// (L3D LEO sizes are metres, unlike OBJ vertices which are mm).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LeoShape {
    /// Rectangular emitter (linear strips, panels). w × h in metres.
    Rectangle { width_m: f32, height_m: f32 },
    /// Round emitter (downlights, bollards). Diameter in metres.
    Circle { diameter_m: f32 },
    /// No shape declared — treat as a point.
    Point,
}

/// Find the first Light Emitting Object's shape in the GLDF's first L3D.
/// LEOs are geometry (variant-invariant in v0), so one lookup suffices.
/// Returns `Point` if there's no LEO or no L3D.
pub fn first_leo_shape(buf: &FileBufGldf) -> LeoShape {
    let Some(l3d_bytes) = get_first_l3d_with_ldt(buf)
        .and_then(|m| m.l3d_content)
        .or_else(|| first_l3d_in_buffer(buf))
    else {
        return LeoShape::Point;
    };
    let l3d = l3d_rs::from_buffer(&l3d_bytes);
    let Ok(luminaire) = Luminaire::from_xml(&l3d.file.structure) else {
        return LeoShape::Point;
    };
    find_leo(&luminaire.structure.geometry).unwrap_or(LeoShape::Point)
}

/// Recursively search a Geometry (and joint children) for the first LEO.
fn find_leo(geo: &Geometry) -> Option<LeoShape> {
    if let Some(leos) = &geo.light_emitting_objects {
        if let Some(leo) = leos.light_emitting_object.first() {
            if let Some(rect) = leo.rectangle.as_ref() {
                return Some(LeoShape::Rectangle {
                    width_m: rect.size_x as f32,
                    height_m: rect.size_y as f32,
                });
            }
            if let Some(circle) = leo.circle.as_ref() {
                return Some(LeoShape::Circle {
                    diameter_m: circle.diameter as f32,
                });
            }
            return Some(LeoShape::Point);
        }
    }
    if let Some(joints) = &geo.joints {
        for joint in &joints.joint {
            for child in &joint.geometries.geometry {
                if let Some(s) = find_leo(child) {
                    return Some(s);
                }
            }
        }
    }
    None
}

/// Parsed OBJ mesh in L3D source frame (mm, right-handed Z-up).
#[derive(Debug, Clone, Default)]
pub struct GldfMeshData {
    pub positions: Vec<[f32; 3]>,
    /// Empty when the OBJ has no `vn` lines.
    pub normals: Vec<[f32; 3]>,
    /// Empty when the OBJ has no `vt` lines.
    pub uvs: Vec<[f32; 2]>,
    pub polygons: Vec<MeshPolygon>,
}

impl GldfMeshData {
    /// Total face corners across all polygons (sum of polygon vertex
    /// counts). Useful for the FFI header so C++ can pre-size buffers.
    pub fn corner_count(&self) -> usize {
        self.polygons.iter().map(|p| p.corners.len()).sum()
    }

    /// Distinct material-group names in first-seen order.
    pub fn material_groups(&self) -> Vec<String> {
        let mut seen = Vec::new();
        for p in &self.polygons {
            if !seen.iter().any(|g: &String| g == &p.material_group) {
                seen.push(p.material_group.clone());
            }
        }
        seen
    }
}

/// Resolve the first L3D for `variant_id` and assemble its FULL geometry
/// into one merged [`GldfMeshData`].
///
/// L3D luminaires are multi-part assemblies: a root `Geometry` plus child
/// parts connected through joints, each part referencing its own OBJ at
/// its own position/rotation, in declared units (mm/in/…). This walks the
/// whole part tree (same traversal `l3d_rs`'s FBX exporter uses), bakes
/// each part's accumulated world transform + unit→cm scale into its
/// vertices, and merges everything into a single mesh in UE-centimetre
/// scale.
///
/// (`variant_id` is accepted for API symmetry with
/// `photometry::build_ies_outputs_for_variant`; geometry is
/// variant-invariant in v0 but Phase 3 may wire per-variant parts.)
pub fn build_first_mesh_for_variant(
    buf: &FileBufGldf,
    _variant_id: &str,
) -> Result<GldfMeshData> {
    let l3d_bytes = get_first_l3d_with_ldt(buf)
        .and_then(|m| m.l3d_content)
        .or_else(|| first_l3d_in_buffer(buf))
        .ok_or(ExportError::NoGeometry)?;

    let l3d = l3d_rs::from_buffer(&l3d_bytes);

    // Parse the structure to learn the part tree + geometry-file defs.
    let luminaire = Luminaire::from_xml(&l3d.file.structure)
        .map_err(|e| ExportError::Internal(format!("L3D structure parse failed: {e}")))?;
    let defs = &luminaire.geometry_definitions.geometry_file_definition;

    // Snapshot the assets as (name, bytes) so the recursion borrows a
    // plain slice rather than capturing `l3d` in a closure (which would
    // tangle lifetimes).
    let assets: Vec<(&str, &[u8])> = l3d
        .file
        .assets
        .iter()
        .map(|a| (a.name.as_str(), a.content.as_slice()))
        .collect();

    let mut merged = GldfMeshData::default();
    collect_geometry(
        &luminaire.structure.geometry,
        defs,
        &MAT4_IDENTITY,
        &assets,
        &mut merged,
    )?;

    if merged.positions.is_empty() || merged.polygons.is_empty() {
        return Err(ExportError::NoGeometry);
    }
    Ok(merged)
}

/// Recursively walk an L3D `Geometry` (and its joint-connected children),
/// baking each part's world transform + unit scale into the merged mesh.
/// Mirrors `l3d_rs`'s FBX exporter `collect_nodes`, but bakes vertices
/// instead of emitting per-node transforms.
fn collect_geometry(
    geo: &Geometry,
    defs: &[GeometryFileDefinition],
    parent_world: &Mat4,
    assets: &[(&str, &[u8])],
    out: &mut GldfMeshData,
) -> Result<()> {
    let local = build_transform(&geo.position, &geo.rotation);
    let accumulated = mat4_mul(parent_world, &local);

    // This part's mesh, if its GeometryReference resolves to a def + asset.
    let geom_id = &geo.geometry_reference.geometry_id;
    if let Some(def) = defs.iter().find(|d| &d.id == geom_id) {
        let path = format!("{}/{}", def.id, def.filename);
        // unit → metres (get_scale) → UE centimetres.
        let unit_scale = get_scale(&def.units) * METERS_TO_UE_CM;
        let world_scaled = mat4_mul(&accumulated, &mat4_scale(unit_scale));

        if let Some((_, bytes)) = assets.iter().find(|(name, _)| *name == path) {
            let part = parse_obj(bytes)?;
            append_transformed(&part, &world_scaled, out);
        }
        // Missing asset: skip this part rather than fail the whole import.
    }

    // Recurse into joint-connected child geometries.
    if let Some(joints) = &geo.joints {
        for joint in &joints.joint {
            let joint_local = build_transform(&joint.position, &joint.rotation);
            let joint_world = mat4_mul(&accumulated, &joint_local);
            for child in &joint.geometries.geometry {
                collect_geometry(child, defs, &joint_world, assets, out)?;
            }
        }
    }
    Ok(())
}

/// Append a parsed part to `out`, transforming positions by `world`
/// (full 4×4, w=1) and normals by its rotation/scale (upper 3×3, w=0).
/// Index references in the part's polygons are rebased onto `out`'s
/// current array lengths so the merge stays consistent.
fn append_transformed(part: &GldfMeshData, world: &Mat4, out: &mut GldfMeshData) {
    let pos_base = out.positions.len() as u32;
    let nrm_base = out.normals.len() as u32;
    let uv_base = out.uvs.len() as u32;

    for p in &part.positions {
        out.positions.push(transform_point(world, *p));
    }
    for n in &part.normals {
        out.normals.push(normalize3(transform_dir(world, *n)));
    }
    for uv in &part.uvs {
        out.uvs.push(*uv);
    }
    for poly in &part.polygons {
        let corners = poly
            .corners
            .iter()
            .map(|c| MeshCorner {
                position_idx: c.position_idx + pos_base,
                normal_idx: c.normal_idx.map(|n| n + nrm_base),
                uv_idx: c.uv_idx.map(|u| u + uv_base),
            })
            .collect();
        out.polygons.push(MeshPolygon {
            material_group: poly.material_group.clone(),
            corners,
        });
    }
}

/// Transform a position by a column-major 4×4 matrix (w = 1).
fn transform_point(m: &Mat4, p: [f32; 3]) -> [f32; 3] {
    [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
        m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
    ]
}

/// Transform a direction by the upper-3×3 of a column-major matrix
/// (w = 0; ignores translation). Good enough for normals under the
/// rigid+uniform-scale transforms L3D uses (no non-uniform shear).
fn transform_dir(m: &Mat4, v: [f32; 3]) -> [f32; 3] {
    [
        m[0] * v[0] + m[4] * v[1] + m[8] * v[2],
        m[1] * v[0] + m[5] * v[1] + m[9] * v[2],
        m[2] * v[0] + m[6] * v[1] + m[10] * v[2],
    ]
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 1e-8 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        v
    }
}

/// Scan the GLDF's raw file buffer for the first `.l3d` entry.
/// Fallback when the variant→geometry mapping doesn't resolve.
fn first_l3d_in_buffer(buf: &FileBufGldf) -> Option<Vec<u8>> {
    buf.files.iter().find_map(|f| {
        let looks_l3d = f
            .name
            .as_ref()
            .map(|n| n.to_lowercase().ends_with(".l3d"))
            .unwrap_or(false)
            || f.path
                .as_ref()
                .map(|p| p.to_lowercase().ends_with(".l3d"))
                .unwrap_or(false);
        if looks_l3d {
            f.content.clone()
        } else {
            None
        }
    })
}

/// Parse Wavefront OBJ bytes into [`GldfMeshData`]. Handles the common
/// face-token forms `a`, `a/b`, `a/b/c`, `a//c`, and negative
/// (relative) indices. Ignores everything we don't model (`g`, `o`,
/// `s`, smoothing, etc.) but tracks `usemtl` for per-polygon material
/// grouping.
fn parse_obj(bytes: &[u8]) -> Result<GldfMeshData> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| ExportError::Internal(format!("OBJ not UTF-8: {e}")))?;

    let mut data = GldfMeshData::default();
    let mut current_material = String::new();

    // Resolve an OBJ index (1-based, negatives are relative-from-end)
    // to a 0-based u32. `len` is the count of elements parsed so far.
    let resolve = |raw: i64, len: usize, lineno: usize, what: &str| -> Result<u32> {
        let idx = if raw > 0 {
            raw - 1
        } else if raw < 0 {
            len as i64 + raw
        } else {
            return Err(ExportError::Internal(format!(
                "OBJ line {}: zero {} index",
                lineno + 1,
                what
            )));
        };
        if idx < 0 || idx as usize >= len {
            return Err(ExportError::Internal(format!(
                "OBJ line {}: {} index {} out of range (have {})",
                lineno + 1,
                what,
                raw,
                len
            )));
        }
        Ok(idx as u32)
    };

    for (lineno, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_ascii_whitespace();
        let Some(tag) = parts.next() else { continue };

        match tag {
            "v" => {
                let c: Vec<&str> = parts.collect();
                if c.len() < 3 {
                    return Err(ExportError::Internal(format!(
                        "OBJ line {}: vertex needs 3 coords",
                        lineno + 1
                    )));
                }
                data.positions.push([
                    parse_f32(c[0], lineno, "vertex x")?,
                    parse_f32(c[1], lineno, "vertex y")?,
                    parse_f32(c[2], lineno, "vertex z")?,
                ]);
            }
            "vn" => {
                let c: Vec<&str> = parts.collect();
                if c.len() < 3 {
                    return Err(ExportError::Internal(format!(
                        "OBJ line {}: normal needs 3 coords",
                        lineno + 1
                    )));
                }
                data.normals.push([
                    parse_f32(c[0], lineno, "normal x")?,
                    parse_f32(c[1], lineno, "normal y")?,
                    parse_f32(c[2], lineno, "normal z")?,
                ]);
            }
            "vt" => {
                let c: Vec<&str> = parts.collect();
                if c.len() < 2 {
                    return Err(ExportError::Internal(format!(
                        "OBJ line {}: texcoord needs 2 coords",
                        lineno + 1
                    )));
                }
                data.uvs.push([
                    parse_f32(c[0], lineno, "uv u")?,
                    parse_f32(c[1], lineno, "uv v")?,
                ]);
            }
            "usemtl" => {
                current_material = parts.next().unwrap_or("").to_string();
            }
            "f" => {
                let mut corners = Vec::new();
                for tok in parts {
                    // Token: "p", "p/t", "p/t/n", "p//n".
                    let mut seg = tok.split('/');
                    let p_str = seg.next().unwrap_or("");
                    let t_str = seg.next().unwrap_or("");
                    let n_str = seg.next().unwrap_or("");

                    let p_raw: i64 = p_str.parse().map_err(|_| {
                        ExportError::Internal(format!(
                            "OBJ line {}: bad position index '{}'",
                            lineno + 1,
                            p_str
                        ))
                    })?;
                    let position_idx = resolve(p_raw, data.positions.len(), lineno, "position")?;

                    let uv_idx = if t_str.is_empty() {
                        None
                    } else {
                        let t_raw: i64 = t_str.parse().map_err(|_| {
                            ExportError::Internal(format!(
                                "OBJ line {}: bad uv index '{}'",
                                lineno + 1,
                                t_str
                            ))
                        })?;
                        Some(resolve(t_raw, data.uvs.len(), lineno, "uv")?)
                    };

                    let normal_idx = if n_str.is_empty() {
                        None
                    } else {
                        let n_raw: i64 = n_str.parse().map_err(|_| {
                            ExportError::Internal(format!(
                                "OBJ line {}: bad normal index '{}'",
                                lineno + 1,
                                n_str
                            ))
                        })?;
                        Some(resolve(n_raw, data.normals.len(), lineno, "normal")?)
                    };

                    corners.push(MeshCorner {
                        position_idx,
                        normal_idx,
                        uv_idx,
                    });
                }
                if corners.len() >= 3 {
                    data.polygons.push(MeshPolygon {
                        material_group: current_material.clone(),
                        corners,
                    });
                }
                // Degenerate faces (< 3 corners) are silently dropped.
            }
            _ => {} // g, o, s, mtllib, etc. — ignored
        }
    }

    if data.positions.is_empty() || data.polygons.is_empty() {
        return Err(ExportError::Internal(
            "OBJ has no usable geometry (no vertices or no faces)".into(),
        ));
    }

    Ok(data)
}

fn parse_f32(s: &str, lineno: usize, what: &str) -> Result<f32> {
    s.parse::<f32>().map_err(|_| {
        ExportError::Internal(format!("OBJ line {}: bad {} '{}'", lineno + 1, what, s))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_triangle_with_normals_and_uvs() {
        let obj = "\
v 0 0 0
v 1 0 0
v 0 1 0
vn 0 0 1
vt 0 0
vt 1 0
vt 0 1
f 1/1/1 2/2/1 3/3/1
";
        let m = parse_obj(obj.as_bytes()).unwrap();
        assert_eq!(m.positions.len(), 3);
        assert_eq!(m.normals.len(), 1);
        assert_eq!(m.uvs.len(), 3);
        assert_eq!(m.polygons.len(), 1);
        let poly = &m.polygons[0];
        assert_eq!(poly.corners.len(), 3);
        assert_eq!(poly.corners[0].position_idx, 0);
        assert_eq!(poly.corners[0].normal_idx, Some(0));
        assert_eq!(poly.corners[1].uv_idx, Some(1));
        assert_eq!(m.corner_count(), 3);
    }

    #[test]
    fn parse_double_slash_no_uv() {
        let obj = "\
v 0 0 0
v 1 0 0
v 0 1 0
vn 0 0 1
f 1//1 2//1 3//1
";
        let m = parse_obj(obj.as_bytes()).unwrap();
        assert_eq!(m.uvs.len(), 0);
        assert_eq!(m.polygons[0].corners[0].uv_idx, None);
        assert_eq!(m.polygons[0].corners[0].normal_idx, Some(0));
    }

    #[test]
    fn parse_position_only_face() {
        let obj = "\
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
";
        let m = parse_obj(obj.as_bytes()).unwrap();
        let c = m.polygons[0].corners[0];
        assert_eq!(c.position_idx, 0);
        assert_eq!(c.normal_idx, None);
        assert_eq!(c.uv_idx, None);
    }

    #[test]
    fn usemtl_groups_polygons() {
        let obj = "\
v 0 0 0
v 1 0 0
v 0 1 0
v 1 1 0
usemtl Body
f 1 2 3
usemtl Lens
f 2 4 3
";
        let m = parse_obj(obj.as_bytes()).unwrap();
        assert_eq!(m.polygons.len(), 2);
        assert_eq!(m.polygons[0].material_group, "Body");
        assert_eq!(m.polygons[1].material_group, "Lens");
        assert_eq!(m.material_groups(), vec!["Body", "Lens"]);
    }

    #[test]
    fn empty_geometry_errors() {
        let obj = "# just a comment\nvn 0 0 1\n";
        assert!(parse_obj(obj.as_bytes()).is_err());
    }

    #[test]
    fn negative_relative_indices() {
        // After 3 vertices, `-1` = last (idx 2), `-3` = first (idx 0).
        let obj = "\
v 0 0 0
v 1 0 0
v 0 1 0
f -3 -2 -1
";
        let m = parse_obj(obj.as_bytes()).unwrap();
        let c = &m.polygons[0].corners;
        assert_eq!(c[0].position_idx, 0);
        assert_eq!(c[1].position_idx, 1);
        assert_eq!(c[2].position_idx, 2);
    }

    fn fixture(name: &str) -> Option<PathBuf> {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("tests/data")
            .join(name);
        p.exists().then(|| p.canonicalize().unwrap())
    }

    #[test]
    fn alurays_first_mesh_has_geometry() {
        let Some(p) = fixture("alurays-3000mm.gldf") else {
            eprintln!("skipping: fixture missing");
            return;
        };
        let exporter = crate::Exporter::from_path(&p).unwrap();
        let buf = exporter.file_buf();
        let vid = buf
            .gldf
            .product_definitions
            .variants
            .as_ref()
            .and_then(|v| v.variant.first())
            .map(|v| v.id.clone())
            .expect("alurays has a variant");

        let mesh = build_first_mesh_for_variant(buf, &vid).expect("mesh parse");
        assert!(!mesh.positions.is_empty(), "has vertices");
        assert!(!mesh.polygons.is_empty(), "has polygons");
        assert!(mesh.corner_count() >= mesh.polygons.len() * 3);

        // Scale sanity: alurays-3000mm is a 3 m linear pendant. After the
        // mm→cm conversion its longest axis should be ~300 cm. Compute the
        // bounding-box extents.
        let bbox = bounding_extents(&mesh);
        let max_extent = bbox[0].max(bbox[1]).max(bbox[2]);
        assert!(
            max_extent > 100.0 && max_extent < 1000.0,
            "expected ~300cm longest extent after mm→cm scale, got {max_extent:.1}cm \
             (bbox = {bbox:?})"
        );

        eprintln!(
            "[alurays] {} verts, {} normals, {} uvs, {} polys, {} corners, bbox(cm)={:?}, groups={:?}",
            mesh.positions.len(),
            mesh.normals.len(),
            mesh.uvs.len(),
            mesh.polygons.len(),
            mesh.corner_count(),
            bbox,
            mesh.material_groups(),
        );
    }

    #[test]
    fn slv_tria2_assembles_all_parts() {
        // SLV Tria 2 is a multi-part assembly: a `base` (geom_1) with two
        // joint branches, each instantiating a connector (geom_2) + head
        // (geom_3). So the merged mesh must contain MORE geometry than any
        // single part — the bug we hit (only base.obj imported) would show
        // far fewer vertices.
        let Some(p) = fixture("SLV - Tria 2.gldf") else {
            eprintln!("skipping: fixture missing");
            return;
        };
        let exporter = crate::Exporter::from_path(&p).unwrap();
        let buf = exporter.file_buf();
        let vid = buf
            .gldf
            .product_definitions
            .variants
            .as_ref()
            .and_then(|v| v.variant.first())
            .map(|v| v.id.clone())
            .expect("SLV has a variant");

        let mesh = build_first_mesh_for_variant(buf, &vid).expect("mesh assemble");

        // base.obj alone parsed standalone, for comparison. Use the same
        // resolution path build_first_mesh_for_variant does (strict
        // mapping with in-buffer fallback) — SLV's variant doesn't use
        // ModelGeometryReference so the strict lookup alone returns None.
        let l3d_bytes = gldf_rs::mapping::get_first_l3d_with_ldt(buf)
            .and_then(|m| m.l3d_content)
            .or_else(|| first_l3d_in_buffer(buf))
            .expect("SLV has L3D");
        let l3d = l3d_rs::from_buffer(&l3d_bytes);
        let base_only = l3d
            .file
            .assets
            .iter()
            .find(|a| a.name == "geom_1/base.obj")
            .map(|a| parse_obj(&a.content).unwrap())
            .expect("base.obj present");

        assert!(
            mesh.positions.len() > base_only.positions.len(),
            "assembled mesh ({} verts) must exceed base-only ({} verts) — \
             multi-part walk is working",
            mesh.positions.len(),
            base_only.positions.len()
        );

        let bbox = bounding_extents(&mesh);
        eprintln!(
            "[SLV Tria 2] assembled {} verts ({} polys) vs base-only {} verts; bbox(cm)={:?}",
            mesh.positions.len(),
            mesh.polygons.len(),
            base_only.positions.len(),
            bbox,
        );
    }

    /// Axis-aligned bounding-box extents (max - min) per axis.
    fn bounding_extents(m: &GldfMeshData) -> [f32; 3] {
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for p in &m.positions {
            for i in 0..3 {
                min[i] = min[i].min(p[i]);
                max[i] = max[i].max(p[i]);
            }
        }
        [max[0] - min[0], max[1] - min[1], max[2] - min[2]]
    }
}
