//! `.udatasmith` XML serializer.
//!
//! Minimal hand-written writer over `std::fmt::Write`. We use `format!`/
//! `write!` rather than quick-xml's serializer because the Datasmith dialect
//! has a fixed, small element set and the explicit writer makes the output
//! easy to diff against UE-emitted reference files.
//!
//! The schema names below (`<DatasmithUnrealScene>`, `<StaticMesh>`,
//! `<ActorLight type="Spot">`, `<Ies file="…">`, etc.) target Datasmith
//! 0.24 / UE 5.4. **They MUST be verified against a UE-emitted
//! `.udatasmith` before Phase 3 sign-off** — the reference fixture lives
//! at `tests/data/datasmith/reference.udatasmith` (committed alongside
//! the first Phase 3 milestone).

use std::fmt::Write;

use super::elements::{
    Actor, ActorChild, ActorLight, ActorMesh, DatasmithDocument, LightType, StaticMesh, Transform,
};

/// Serialize the document to a UTF-8 string. Always emits LF line endings,
/// two-space indents — matches the style of UE's own Datasmith exporter
/// output.
pub fn write_document(doc: &DatasmithDocument) -> String {
    let mut out = String::with_capacity(2048);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(out, "<DatasmithUnrealScene>");
    let _ = writeln!(out, "  <Version>{}</Version>", xml_escape(&doc.version));
    let _ = writeln!(
        out,
        "  <SDKVersion>{}</SDKVersion>",
        xml_escape(&doc.sdk_version)
    );
    let _ = writeln!(out, "  <Host>{}</Host>", xml_escape(&doc.host));
    let _ = writeln!(
        out,
        "  <ResourcePath>{}</ResourcePath>",
        xml_escape(&doc.resource_path)
    );

    for mesh in &doc.static_meshes {
        write_static_mesh(&mut out, mesh, 1);
    }
    for actor in &doc.actors {
        write_actor(&mut out, actor, 1);
    }

    let _ = writeln!(out, "</DatasmithUnrealScene>");
    out
}

fn write_static_mesh(out: &mut String, m: &StaticMesh, depth: usize) {
    let ind = indent(depth);
    let _ = writeln!(
        out,
        "{ind}<StaticMesh name=\"{}\" label=\"{}\">",
        xml_escape(&m.name),
        xml_escape(&m.label),
    );
    let _ = writeln!(
        out,
        "{ind}  <file path=\"{}\"/>",
        xml_escape(&m.file_path)
    );
    let _ = writeln!(out, "{ind}</StaticMesh>");
}

fn write_actor(out: &mut String, a: &Actor, depth: usize) {
    let ind = indent(depth);
    let _ = writeln!(
        out,
        "{ind}<Actor name=\"{}\" layer=\"{}\">",
        xml_escape(&a.name),
        xml_escape(&a.layer),
    );
    write_transform(out, &a.transform, depth + 1);
    if !a.children.is_empty() {
        let _ = writeln!(out, "{ind}  <children>");
        for c in &a.children {
            match c {
                ActorChild::Mesh(am) => write_actor_mesh(out, am, depth + 2),
                ActorChild::Light(al) => write_actor_light(out, al, depth + 2),
            }
        }
        let _ = writeln!(out, "{ind}  </children>");
    }
    let _ = writeln!(out, "{ind}</Actor>");
}

fn write_actor_mesh(out: &mut String, m: &ActorMesh, depth: usize) {
    let ind = indent(depth);
    let _ = writeln!(
        out,
        "{ind}<ActorMesh name=\"{}\" mesh=\"{}\">",
        xml_escape(&m.name),
        xml_escape(&m.mesh_ref),
    );
    write_transform(out, &m.transform, depth + 1);
    let _ = writeln!(out, "{ind}</ActorMesh>");
}

fn write_actor_light(out: &mut String, l: &ActorLight, depth: usize) {
    let ind = indent(depth);
    let _ = writeln!(
        out,
        "{ind}<ActorLight name=\"{}\" type=\"{}\">",
        xml_escape(&l.name),
        light_type_attr(l.light_type),
    );
    write_transform(out, &l.transform, depth + 1);
    let _ = writeln!(
        out,
        "{ind}  <Intensity>{}</Intensity>",
        fmt_f(l.intensity_lumens),
    );
    let _ = writeln!(out, "{ind}  <IntensityUnits>Lumens</IntensityUnits>");
    if let Some(k) = l.color_temperature_k {
        let _ = writeln!(
            out,
            "{ind}  <Color usetemp=\"1\"><Temperature>{k}</Temperature></Color>",
        );
    }
    if let Some(ref ies) = l.ies_file {
        let _ = writeln!(out, "{ind}  <Ies file=\"{}\"/>", xml_escape(ies));
        let _ = writeln!(
            out,
            "{ind}  <IesBrightnessScale>1.0</IesBrightnessScale>"
        );
    }
    let _ = writeln!(out, "{ind}</ActorLight>");
}

fn write_transform(out: &mut String, t: &Transform, depth: usize) {
    let ind = indent(depth);
    let _ = writeln!(
        out,
        "{ind}<Transform tx=\"{}\" ty=\"{}\" tz=\"{}\" rx=\"{}\" ry=\"{}\" rz=\"{}\" sx=\"{}\" sy=\"{}\" sz=\"{}\"/>",
        fmt_f(t.tx), fmt_f(t.ty), fmt_f(t.tz),
        fmt_f(t.rx), fmt_f(t.ry), fmt_f(t.rz),
        fmt_f(t.sx), fmt_f(t.sy), fmt_f(t.sz),
    );
}

fn light_type_attr(t: LightType) -> &'static str {
    match t {
        LightType::Point => "Point",
        LightType::Spot => "Spot",
        LightType::Area => "Area",
    }
}

fn indent(depth: usize) -> String {
    "  ".repeat(depth)
}

/// Escape the five XML special characters in an attribute or text node.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

fn fmt_f(x: f64) -> String {
    if x == 0.0 {
        "0".to_string()
    } else {
        format!("{x}")
    }
}

#[cfg(test)]
mod tests {
    use super::super::elements::*;
    use super::*;

    #[test]
    fn minimal_document_has_expected_structure() {
        let doc = DatasmithDocument {
            version: "0.24".into(),
            sdk_version: "5.4".into(),
            host: "gldf-to-datasmith".into(),
            resource_path: "Assets".into(),
            static_meshes: vec![StaticMesh {
                name: "body".into(),
                label: "body".into(),
                file_path: "Assets/Geometry/body.obj".into(),
            }],
            actors: vec![Actor {
                name: "Lum".into(),
                layer: "GLDF".into(),
                transform: Transform::identity(),
                children: vec![ActorChild::Mesh(ActorMesh {
                    name: "Body".into(),
                    mesh_ref: "body".into(),
                    transform: Transform::identity(),
                })],
            }],
        };
        let xml = write_document(&doc);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<DatasmithUnrealScene>"));
        assert!(xml.contains("<Version>0.24</Version>"));
        assert!(xml.contains("<SDKVersion>5.4</SDKVersion>"));
        assert!(xml.contains("<StaticMesh name=\"body\" label=\"body\">"));
        assert!(xml.contains("<file path=\"Assets/Geometry/body.obj\"/>"));
        assert!(xml.contains("<Actor name=\"Lum\" layer=\"GLDF\">"));
        assert!(xml.contains("<ActorMesh name=\"Body\" mesh=\"body\">"));
        assert!(xml.ends_with("</DatasmithUnrealScene>\n"));
    }

    #[test]
    fn xml_escapes_special_characters() {
        let doc = DatasmithDocument {
            version: "0.24".into(),
            sdk_version: "5.4".into(),
            host: "test".into(),
            resource_path: "Assets".into(),
            static_meshes: vec![StaticMesh {
                name: "a&b<c>d".into(),
                label: "x\"y".into(),
                file_path: "p".into(),
            }],
            actors: vec![],
        };
        let xml = write_document(&doc);
        assert!(xml.contains("name=\"a&amp;b&lt;c&gt;d\""));
        assert!(xml.contains("label=\"x&quot;y\""));
    }

    #[test]
    fn actor_light_with_ies_emits_intensity_and_ies_ref() {
        let doc = DatasmithDocument {
            version: "0.24".into(),
            sdk_version: "5.4".into(),
            host: "test".into(),
            resource_path: "Assets".into(),
            static_meshes: vec![],
            actors: vec![Actor {
                name: "Lum".into(),
                layer: "GLDF".into(),
                transform: Transform::identity(),
                children: vec![ActorChild::Light(ActorLight {
                    name: "Emitter1".into(),
                    light_type: LightType::Spot,
                    transform: Transform::identity(),
                    intensity_lumens: 1800.0,
                    color_temperature_k: Some(4000),
                    ies_file: Some("Assets/Ies/emitter1.ies".into()),
                })],
            }],
        };
        let xml = write_document(&doc);
        assert!(xml.contains("<ActorLight name=\"Emitter1\" type=\"Spot\">"));
        assert!(xml.contains("<Intensity>1800</Intensity>"));
        assert!(xml.contains("<IntensityUnits>Lumens</IntensityUnits>"));
        assert!(xml.contains("<Temperature>4000</Temperature>"));
        assert!(xml.contains("<Ies file=\"Assets/Ies/emitter1.ies\"/>"));
    }
}
