//! Verify the `SetVariantProductNumber` action's JSON shape so the
//! Trahe ACME e2e demo dispatches it correctly. The demo bypasses the
//! editor UI for many actions and hand-builds the payload from Python;
//! getting the tuple-variant shape wrong silently drops the action
//! and the exported GLDF then omits `<ProductNumber>` from every
//! variant. That's how the gldf-search.de reader crash was missed.
//!
//! The test deserialises the exact payload shape the demo sends and
//! asserts it round-trips through the action enum unchanged.

use gldf_rs::gldf::header::Locale;
use gldf_rs::gldf::LocaleFoo;
use gldf_rs_editor::GldfAction;

#[test]
fn demo_payload_deserialises_into_action() {
    // The exact JSON the e2e demo's Python builds (see Act 7 in
    // tests/e2e/demo_trahe_acme.py: `SetVariantProductNumber`).
    //
    // LocaleFoo has `#[serde(rename = "Locale")]` on its `locale` field,
    // and Locale uses quick-xml's `@language` (attribute) / `$text`
    // (element text) marker keys. So the JSON `data[1]` shape is
    // `{"Locale": [{"@language": "en", "$text": "..."}]}` — NOT
    // `{"locale": [{"language": "en", "value": "..."}]}`. Getting this
    // wrong is what previously left every variant's ProductNumber
    // unset in the demo's exported GLDF (and crashed downstream
    // readers that hard-deref `.ProductNumber.Locale`).
    let raw = r#"{
        "action": "SetVariantProductNumber",
        "data": [0, { "Locale": [{ "@language": "en", "$text": "LP-Y-6018" }] }]
    }"#;

    let action: GldfAction = serde_json::from_str(raw).expect("valid action JSON");

    match action {
        GldfAction::SetVariantProductNumber(idx, locale_foo) => {
            assert_eq!(idx, 0);
            assert_eq!(locale_foo.locale.len(), 1);
            assert_eq!(locale_foo.locale[0].language, "en");
            assert_eq!(locale_foo.locale[0].value, "LP-Y-6018");
        }
        other => panic!("expected SetVariantProductNumber, got {other:?}"),
    }
}

#[test]
fn round_trip_via_to_string() {
    // The reverse: serialise an instance the same way the editor's
    // own JS bridge does, then deserialise. Confirms the round-trip
    // is identity for the demo's payload shape.
    let action = GldfAction::SetVariantProductNumber(
        3,
        LocaleFoo {
            locale: vec![Locale {
                language: "en".to_string(),
                value: "LP-E-9005".to_string(),
            }],
        },
    );
    let json = serde_json::to_string(&action).expect("serialise");
    let back: GldfAction = serde_json::from_str(&json).expect("deserialise");
    // GldfAction doesn't impl PartialEq (intentional — most variants carry
    // floating-point / collection payloads where equality isn't well
    // defined). Re-serialise both sides and compare the resulting JSON.
    let json2 = serde_json::to_string(&back).expect("re-serialise");
    assert_eq!(json, json2);

    // And confirm the structural shape of the payload — it's a tagged
    // tuple-variant so `data` is a JSON array `[idx, locale]`. Inside
    // locale, quick-xml's @language / $text marker keys (NOT
    // `language` / `value`).
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["action"], "SetVariantProductNumber");
    assert!(value["data"].is_array());
    assert_eq!(value["data"][0], 3);
    assert_eq!(value["data"][1]["Locale"][0]["@language"], "en");
    assert_eq!(value["data"][1]["Locale"][0]["$text"], "LP-E-9005");
}
