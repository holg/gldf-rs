//! Variant-scope override smoke tests for Pictures / Hyperlinks / Emergency.

use gldf_rs_editor::{GldfAction, GldfState, Scope};

#[test]
fn variant_picture_lands_on_variant_not_product() {
    let mut s = GldfState::default();
    s.apply(GldfAction::AddVariantSimple {
        name: "V1".to_string(),
    });
    s.apply(GldfAction::AddProductMetaPicture {
        scope: Scope::Variant(0),
        file_id: "file_1".to_string(),
        image_type: "Product Picture".to_string(),
    });

    // Product pictures must NOT contain the entry.
    assert!(s
        .product
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|m| m.pictures.as_ref())
        .is_none_or(|p| p.image.is_empty()));

    // Variant[0] pictures must contain it.
    let v = &s
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant[0];
    let pics = v.pictures.as_ref().expect("variant pictures created");
    assert_eq!(pics.image.len(), 1);
    assert_eq!(pics.image[0].file_id, "file_1");
}

#[test]
fn variant_hyperlink_lands_on_variant_marketing() {
    let mut s = GldfState::default();
    s.apply(GldfAction::AddVariantSimple {
        name: "V1".to_string(),
    });
    s.apply(GldfAction::AddHyperlink(Scope::Variant(0)));
    s.apply(GldfAction::UpdateHyperlink {
        scope: Scope::Variant(0),
        index: 0,
        href: "https://example.com".to_string(),
        language: Some("en".to_string()),
        region: None,
        country_code: None,
        value: "Datasheet".to_string(),
    });

    let v = &s
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant[0];
    let hs = v
        .descriptive_attributes
        .as_ref()
        .and_then(|a| a.marketing.as_ref())
        .and_then(|m| m.hyperlinks.as_ref())
        .expect("variant marketing.hyperlinks");
    assert_eq!(hs.hyperlink.len(), 1);
    assert_eq!(hs.hyperlink[0].href, "https://example.com");

    // Product-level should be empty.
    let prod_hs = s
        .product
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|m| m.descriptive_attributes.as_ref())
        .and_then(|a| a.marketing.as_ref())
        .and_then(|m| m.hyperlinks.as_ref());
    assert!(prod_hs.is_none_or(|h| h.hyperlink.is_empty()));
}

#[test]
fn variant_emergency_dispatches_to_variant_descriptive_attributes() {
    let mut s = GldfState::default();
    s.apply(GldfAction::AddVariantSimple {
        name: "V1".to_string(),
    });
    s.apply(GldfAction::SetDedicatedEmergencyLightingType(
        Scope::Variant(0),
        Some("Maintained".to_string()),
    ));
    s.apply(GldfAction::AddEmergencyFlux(Scope::Variant(0)));
    s.apply(GldfAction::UpdateEmergencyFlux {
        scope: Scope::Variant(0),
        index: 0,
        hours: 1,
        flux: 200,
    });

    let v = &s
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant[0];
    let e = v
        .descriptive_attributes
        .as_ref()
        .and_then(|a| a.emergency.as_ref())
        .expect("variant emergency");
    assert_eq!(
        e.dedicated_emergency_lighting_type.as_deref(),
        Some("Maintained")
    );
    let flux = e.duration_time_and_flux.as_ref().unwrap();
    assert_eq!(flux.flux.len(), 1);
    assert_eq!(flux.flux[0].hours, 1);
    assert_eq!(flux.flux[0].value, 200);
}
