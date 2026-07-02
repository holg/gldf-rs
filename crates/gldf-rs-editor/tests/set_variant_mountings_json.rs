//! Verify the JSON envelope shape the Trahe ACME demo dispatches for
//! `SetVariantMountings`. Same defence as the ProductNumber test: the
//! demo bypasses the editor UI for many actions and hand-builds the
//! payload from Python — getting the field names or @attribute markers
//! wrong silently drops the action and the exported GLDF then omits
//! `<Mountings>` from every variant.
//!
//! The Mountings → Ceiling → Recessed shape uses quick-xml's `@…`
//! attribute marker (e.g. `@recessedDepth`). Child elements are bare
//! PascalCase keys (`RectangularCutout`, `Width`).

use gldf_rs_editor::GldfAction;

#[test]
fn demo_payload_deserialises_into_action() {
    // The exact JSON the e2e demo's Python builds (see Act 12 in
    // tests/e2e/demo_trahe_acme.py).
    let raw = r#"{
        "action": "SetVariantMountings",
        "data": {
            "index": 0,
            "payload": {
                "Ceiling": {
                    "Recessed": {
                        "@recessedDepth": 50,
                        "RectangularCutout": {
                            "Width": 90,
                            "Length": 170,
                            "Depth": 50
                        }
                    }
                }
            }
        }
    }"#;

    let action: GldfAction = serde_json::from_str(raw).expect("valid action JSON");

    match action {
        GldfAction::SetVariantMountings { index, payload } => {
            assert_eq!(index, 0);
            let m = payload.expect("Mountings payload present");
            let ceiling = m.ceiling.as_ref().expect("Ceiling branch set");
            let recessed = ceiling.recessed.as_ref().expect("Recessed branch set");
            assert_eq!(recessed.recessed_depth, 50);
            let cutout = recessed
                .rectangular_cutout
                .as_ref()
                .expect("RectangularCutout set");
            assert_eq!(cutout.width, 90);
            assert_eq!(cutout.length, 170);
            assert_eq!(cutout.depth, 50);
        }
        other => panic!("expected SetVariantMountings, got {other:?}"),
    }
}
