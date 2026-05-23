//! OBJ rewriting from L3D coordinate frame to Unreal Engine frame.
//!
//! Reads an L3D-bundled Wavefront OBJ (mm, right-handed Z-up), and emits
//! an OBJ in the chosen Unreal unit system (cm or m, left-handed Z-up).
//!
//! The transform is line-by-line, not parse-into-mesh-then-emit:
//!
//! * `v x y z`   → scale + negate y (positions, [`coords::point_l3d_to_ue`])
//! * `vn x y z`  → negate y only (normals are directions; no scale)
//! * `f a b c …` → reverse polygon vertex order (every winding flips because
//!                 the transform is orientation-reversing — `det(M) < 0`)
//! * `mtllib`, `usemtl`, `vt`, `g`, `o`, `s`, `#`, blanks → passthrough
//!
//! Anything we don't recognise is also passed through verbatim — better to
//! ship slightly more text than to drop data we don't understand.

use crate::coords::point_l3d_to_ue;
use crate::error::Result;
use crate::options::UnitSystem;
use glam::DVec3;

/// Rewrite OBJ bytes from L3D frame into Unreal frame. Returns the new
/// OBJ as a UTF-8 string. Lines that don't parse as numbers fall through
/// unchanged (with a debug-only stderr note in tests).
pub fn rewrite_obj_to_ue(obj_bytes: &[u8], units: UnitSystem) -> Result<String> {
    let input = std::str::from_utf8(obj_bytes).map_err(|e| {
        crate::error::ExportError::Internal(format!("OBJ is not valid UTF-8: {e}"))
    })?;

    // Preallocate ~= input size; rewrite barely changes line lengths.
    let mut out = String::with_capacity(input.len());

    for line in input.split_inclusive('\n') {
        // Strip the trailing newline (if any) for inspection, but remember
        // to re-emit it. split_inclusive keeps it on the last char.
        let (body, eol) = split_eol(line);
        let trimmed = body.trim_start();

        if trimmed.starts_with("v ") {
            rewrite_vertex_line(body, units, &mut out);
        } else if trimmed.starts_with("vn ") {
            rewrite_normal_line(body, &mut out);
        } else if trimmed.starts_with("f ") {
            rewrite_face_line(body, &mut out);
        } else {
            out.push_str(body);
        }
        out.push_str(eol);
    }

    Ok(out)
}

/// Split a line into (body, line-ending). `line` may contain `\r\n`, `\n`,
/// or no terminator (last line).
fn split_eol(line: &str) -> (&str, &str) {
    if let Some(s) = line.strip_suffix("\r\n") {
        (s, "\r\n")
    } else if let Some(s) = line.strip_suffix('\n') {
        (s, "\n")
    } else {
        (line, "")
    }
}

fn rewrite_vertex_line(body: &str, units: UnitSystem, out: &mut String) {
    // Format: "v <x> <y> <z> [w]"  — we ignore w (rarely used).
    let mut iter = body.split_whitespace();
    let _ = iter.next(); // "v"
    let (x, y, z) = match (
        iter.next().and_then(parse_f64),
        iter.next().and_then(parse_f64),
        iter.next().and_then(parse_f64),
    ) {
        (Some(x), Some(y), Some(z)) => (x, y, z),
        _ => {
            // Couldn't parse — emit verbatim.
            out.push_str(body);
            return;
        }
    };
    let p = point_l3d_to_ue(DVec3::new(x, y, z), units);
    // Format with enough precision to round-trip floats (g for compactness,
    // f64 default has 15+ sig digits — plenty for mm-scale geometry).
    use std::fmt::Write;
    let _ = write!(out, "v {} {} {}", fmt_f(p.x), fmt_f(p.y), fmt_f(p.z));
}

fn rewrite_normal_line(body: &str, out: &mut String) {
    // Format: "vn <x> <y> <z>"  — directions only, no scale.
    let mut iter = body.split_whitespace();
    let _ = iter.next(); // "vn"
    let (x, y, z) = match (
        iter.next().and_then(parse_f64),
        iter.next().and_then(parse_f64),
        iter.next().and_then(parse_f64),
    ) {
        (Some(x), Some(y), Some(z)) => (x, y, z),
        _ => {
            out.push_str(body);
            return;
        }
    };
    use std::fmt::Write;
    let _ = write!(out, "vn {} {} {}", fmt_f(x), fmt_f(-y), fmt_f(z));
}

fn rewrite_face_line(body: &str, out: &mut String) {
    // Format: "f v1[/vt1[/vn1]] v2[/vt2[/vn2]] v3..."
    // We keep each vertex token (a/b/c) intact and just reverse their
    // order. This is the polygon winding flip.
    let mut iter = body.split_whitespace();
    let _ = iter.next(); // "f"
    let tokens: Vec<&str> = iter.collect();
    if tokens.len() < 3 {
        out.push_str(body);
        return;
    }
    out.push('f');
    for tok in tokens.iter().rev() {
        out.push(' ');
        out.push_str(tok);
    }
}

fn parse_f64(s: &str) -> Option<f64> {
    s.parse::<f64>().ok()
}

/// Format f64 compactly, stripping a redundant trailing `.0` from
/// integral values to keep the OBJ readable. Precision is `{}` (f64
/// default ~15 digits) — enough for any GLDF geometry.
fn fmt_f(x: f64) -> String {
    // Avoid `-0`-looking output; collapse to `0`.
    if x == 0.0 {
        return "0".to_string();
    }
    let s = format!("{x}");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_lines_scaled_and_y_negated() {
        let input = "v 1000 2000 3000\n";
        let out = rewrite_obj_to_ue(input.as_bytes(), UnitSystem::Cm).unwrap();
        // mm → cm: divide by 10. Y negated.
        assert_eq!(out, "v 100 -200 300\n");
    }

    #[test]
    fn normal_lines_y_negated_no_scale() {
        let input = "vn 0 1 0\n";
        let out = rewrite_obj_to_ue(input.as_bytes(), UnitSystem::Cm).unwrap();
        assert_eq!(out, "vn 0 -1 0\n");
    }

    #[test]
    fn face_lines_reversed_for_winding_flip() {
        let input = "f 1/1/1 2/2/2 3/3/3\n";
        let out = rewrite_obj_to_ue(input.as_bytes(), UnitSystem::Cm).unwrap();
        assert_eq!(out, "f 3/3/3 2/2/2 1/1/1\n");
    }

    #[test]
    fn face_simple_indices_reversed() {
        let input = "f 1 2 3 4\n";
        let out = rewrite_obj_to_ue(input.as_bytes(), UnitSystem::Cm).unwrap();
        // Quad: same rule, reverse polygon order.
        assert_eq!(out, "f 4 3 2 1\n");
    }

    #[test]
    fn other_lines_pass_through() {
        let input = "# comment\nmtllib luminaire.mtl\nusemtl Body\ng Part1\nvt 0.5 0.5\n";
        let out = rewrite_obj_to_ue(input.as_bytes(), UnitSystem::Cm).unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn units_m_scales_to_metres() {
        let input = "v 1000 0 0\n";
        let out = rewrite_obj_to_ue(input.as_bytes(), UnitSystem::M).unwrap();
        assert_eq!(out, "v 1 0 0\n");
    }

    #[test]
    fn known_fixture_triangle_round_trip() {
        // A 1 mm RH triangle in the XY plane (CCW from +Z looking down)
        // becomes a 0.1 cm triangle with Y flipped and winding reversed.
        let input = "\
v 0 0 0
v 1 0 0
v 0 1 0
vn 0 0 1
f 1//1 2//1 3//1
";
        let out = rewrite_obj_to_ue(input.as_bytes(), UnitSystem::Cm).unwrap();
        let expected = "\
v 0 0 0
v 0.1 0 0
v 0 -0.1 0
vn 0 0 1
f 3//1 2//1 1//1
";
        assert_eq!(out, expected);
    }

    #[test]
    fn crlf_line_endings_preserved() {
        let input = "v 1000 0 0\r\nv 0 1000 0\r\n";
        let out = rewrite_obj_to_ue(input.as_bytes(), UnitSystem::Cm).unwrap();
        assert_eq!(out, "v 100 0 0\r\nv 0 -100 0\r\n");
    }
}
