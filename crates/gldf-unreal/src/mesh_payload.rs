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

use crate::error::{ExportError, Result};

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

/// Resolve the first L3D for `variant_id` (same resolution path the
/// photometry pipeline uses) and parse its first OBJ asset into
/// [`GldfMeshData`].
pub fn build_first_mesh_for_variant(
    buf: &FileBufGldf,
    _variant_id: &str,
) -> Result<GldfMeshData> {
    // Phase 2 uses the first L3D in the bundle (mesh is variant-invariant
    // in v0). The variant_id is accepted for API symmetry with
    // photometry::build_ies_outputs_for_variant and for when Phase 3
    // wires per-variant geometry.
    let l3d_bytes = get_first_l3d_with_ldt(buf)
        .and_then(|m| m.l3d_content)
        .or_else(|| first_l3d_in_buffer(buf))
        .ok_or(ExportError::NoGeometry)?;

    let l3d = l3d_rs::from_buffer(&l3d_bytes);
    let obj_bytes = l3d
        .file
        .assets
        .iter()
        .find(|a| a.name.to_lowercase().ends_with(".obj"))
        .map(|a| a.content.clone())
        .ok_or(ExportError::NoGeometry)?;

    parse_obj(&obj_bytes)
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
        eprintln!(
            "[alurays] {} verts, {} normals, {} uvs, {} polys, {} corners, groups={:?}",
            mesh.positions.len(),
            mesh.normals.len(),
            mesh.uvs.len(),
            mesh.polygons.len(),
            mesh.corner_count(),
            mesh.material_groups(),
        );
    }
}
