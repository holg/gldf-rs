//! Variant CRUD smoke: Duplicate / Move / GTIN / sortOrder.

use gldf_rs_editor::{GldfAction, GldfState};

fn seed_variant(name: &str) -> GldfState {
    let mut s = GldfState::default();
    s.apply(GldfAction::AddVariantSimple {
        name: name.to_string(),
    });
    s
}

#[test]
fn duplicate_variant_creates_sibling_with_fresh_id() {
    let mut s = seed_variant("Standard 2700K");
    s.apply(GldfAction::SetVariantGTIN {
        index: 0,
        gtin: Some("0123456789012".to_string()),
    });

    s.apply(GldfAction::DuplicateVariant { src_idx: 0 });
    let vs = &s
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant;
    assert_eq!(vs.len(), 2, "two variants after duplicate");
    assert_ne!(vs[0].id, vs[1].id, "fresh id on clone");
    assert_eq!(vs[1].gtin.as_deref(), Some("0123456789012"), "GTIN cloned");
    let cloned_name = vs[1]
        .name
        .as_ref()
        .and_then(|n| n.locale.first())
        .map(|l| l.value.as_str());
    assert_eq!(
        cloned_name,
        Some("Standard 2700K (copy)"),
        "name gets (copy) suffix"
    );
}

#[test]
fn move_variant_swaps_positions() {
    let mut s = seed_variant("A");
    s.apply(GldfAction::AddVariantSimple {
        name: "B".to_string(),
    });
    s.apply(GldfAction::AddVariantSimple {
        name: "C".to_string(),
    });
    let ids_before: Vec<String> = s
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant
        .iter()
        .map(|v| v.id.clone())
        .collect();
    // Move A (index 0) down by 1 → expect [B, A, C].
    s.apply(GldfAction::MoveVariant { index: 0, delta: 1 });
    let ids_after: Vec<String> = s
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant
        .iter()
        .map(|v| v.id.clone())
        .collect();
    assert_eq!(
        ids_after,
        vec![
            ids_before[1].clone(),
            ids_before[0].clone(),
            ids_before[2].clone()
        ]
    );
}

#[test]
fn move_variant_out_of_bounds_is_noop() {
    let mut s = seed_variant("solo");
    s.apply(GldfAction::MoveVariant {
        index: 0,
        delta: -1,
    });
    assert_eq!(
        s.product
            .product_definitions
            .variants
            .as_ref()
            .unwrap()
            .variant
            .len(),
        1
    );
}

#[test]
fn set_sort_order_writes_attribute() {
    let mut s = seed_variant("v");
    s.apply(GldfAction::SetVariantSortOrder {
        index: 0,
        sort_order: Some(7),
    });
    let v = &s
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant[0];
    assert_eq!(v.sort_order, Some(7));
    s.apply(GldfAction::SetVariantSortOrder {
        index: 0,
        sort_order: None,
    });
    let v = &s
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant[0];
    assert_eq!(v.sort_order, None);
}

#[test]
fn set_gtin_empty_string_clears_field() {
    let mut s = seed_variant("v");
    s.apply(GldfAction::SetVariantGTIN {
        index: 0,
        gtin: Some("".to_string()),
    });
    let v = &s
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant[0];
    assert_eq!(v.gtin, None, "empty string should clear gtin");
}

#[test]
fn set_variant_symbol_set_and_clear() {
    let mut s = seed_variant("v");
    s.apply(GldfAction::SetVariantSymbol {
        index: 0,
        file_id: Some("file_logo".to_string()),
    });
    let v = &s
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant[0];
    assert_eq!(
        v.symbol.as_ref().map(|sym| sym.file_id.as_str()),
        Some("file_logo")
    );

    s.apply(GldfAction::SetVariantSymbol {
        index: 0,
        file_id: None,
    });
    let v = &s
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant[0];
    assert!(v.symbol.is_none(), "None clears the field");

    // Empty string also clears (so the "(none)" UI option works).
    s.apply(GldfAction::SetVariantSymbol {
        index: 0,
        file_id: Some("file_logo".to_string()),
    });
    s.apply(GldfAction::SetVariantSymbol {
        index: 0,
        file_id: Some("".to_string()),
    });
    let v = &s
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant[0];
    assert!(v.symbol.is_none(), "empty string clears");
}
