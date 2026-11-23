//! Editor-authoring Lisp verbs — BUILD / REFACTOR the GLDF product.
//!
//! These are the "no-JSON, all-Lisp" surface: each verb is a named
//! primitive that emits an opaque [`SceneCommand::EditorAction`] /
//! [`SceneCommand::NeedFiles`] carrying the editor's wire-format JSON.
//! The engine never depends on `gldf-rs-editor` — it only produces
//! the `{"action":…,"data":…}` strings; the WASM panel resolves any
//! `{"@file":"name"}` byte-refs against dropped files and dispatches
//! through the editor's existing `apply_action_json` path.
//!
//! File-referencing verbs (`GLDF-ADD-PHOTOMETRY` etc.) put the
//! filename into BOTH the `@file` ref and the `filename` field, so the
//! panel can fill the bytes from the matching dropped file.
//!
//! Naming mirrors the query verbs: uppercase `GLDF-…`.

use acadlisp::{Expr, ForeignContext, Interpreter};
use serde_json::json;

use crate::{arg_error, expr_to_f32, expr_to_string, expr_to_usize, state_mut, SceneCommand};

pub fn register_all(interp: &mut Interpreter) {
    interp.register_fn("NEEDS-FILES", needs_files);
    interp.register_fn("GLDF-WIZARD", gldf_wizard);
    interp.register_fn("GLDF-SET-MANUFACTURER", gldf_set_manufacturer);
    interp.register_fn("GLDF-SET-AUTHOR", gldf_set_author);
    interp.register_fn("GLDF-SET-NAME", gldf_set_name);
    interp.register_fn("GLDF-NEW", gldf_new);
    interp.register_fn("GLDF-ADD-PHOTOMETRY", gldf_add_photometry);
    interp.register_fn("GLDF-ADD-SENSOR", gldf_add_sensor);
    interp.register_fn("GLDF-ADD-SPECTRUM", gldf_add_spectrum);
    interp.register_fn("GLDF-SET-GEOMETRY", gldf_set_geometry);
    interp.register_fn("GLDF-ADD-VARIANT", gldf_add_variant);
    interp.register_fn("GLDF-ADD-LIGHTSOURCE", gldf_add_lightsource);
    interp.register_fn("GLDF-ADD-EMITTER", gldf_add_emitter);
    interp.register_fn("GLDF-VARIANT-GEOMETRY", gldf_variant_geometry);
    interp.register_fn("GLDF-LUMENS", gldf_lumens);
    interp.register_fn("GLDF-HOUSING-COLOR", gldf_housing_color);
    interp.register_fn("GLDF-APPLICATIONS", gldf_applications);
    interp.register_fn("GLDF-MARKET", gldf_market);
    interp.register_fn("GLDF-APPROVAL", gldf_approval);
    interp.register_fn("GLDF-ELECTRICAL", gldf_electrical);
    interp.register_fn("GLDF-MECHANICAL", gldf_mechanical);
    interp.register_fn("GLDF-RENAME-VARIANT", gldf_rename_variant);
    interp.register_fn("GLDF-ADD-SENSOR-EMITTER", gldf_add_sensor_emitter);
    interp.register_fn("GLDF-PRODUCT-PICTURE", gldf_product_picture);
    interp.register_fn("GLDF-SENSOR-CHARS", gldf_sensor_chars);
    interp.register_fn("GLDF-VARIANT-GTIN", gldf_variant_gtin);
    interp.register_fn("GLDF-SIZE", gldf_size);
    interp.register_fn("GLDF-EMERGENCY", gldf_emergency);
}

/// Decode a scope arg: a non-negative integer N → `Variant(N)`; a
/// negative number, missing, or the string "product" → `Product`.
/// The metadata verbs all take this as their first argument so a
/// script can target the product default or a specific variant.
fn scope_arg(e: Option<&Expr>) -> serde_json::Value {
    match e {
        Some(Expr::Integer(n)) if *n >= 0 => json!({ "Variant": *n }),
        Some(Expr::Real(r)) if *r >= 0.0 => json!({ "Variant": *r as i64 }),
        _ => json!("Product"),
    }
}

/// Dispatch a `WizardApply` carrying the given WizardDraft JSON value.
/// The panel resolves any `{"@file":…}` byte-refs, deserializes the
/// draft, and runs the SAME `apply_wizard` reducer the New-GLDF wizard
/// uses — which stamps the XSD-required header (UniqueGldfId, creation
/// time, UniqueProductId), a light source, an emitter bound to the
/// photometry + light source, a ProductNumber-bearing variant, and the
/// geometry binding. That's why a `GLDF-WIZARD` build validates while a
/// pile of granular `GLDF-ADD-*` actions does not.
fn emit_wizard(ctx: &mut ForeignContext, draft: serde_json::Value) -> Expr {
    let json_str = draft.to_string();
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::WizardApply {
            draft_json: json_str,
        });
        Expr::T
    } else {
        ctx.output.push("(GLDF-WIZARD) state missing\n".to_string());
        Expr::Nil
    }
}

/// `(GLDF-WIZARD manufacturer name author photometry-file geometry-file
///   variant-name [sensor-file] [emergency-file])` — build a COMPLETE,
/// XSD-valid product in one call via the shared wizard reducer. Pass
/// `""` (or omit trailing args) to skip a field.
///
/// - File args (`photometry`/`geometry`/`sensor`/`emergency`) must also
///   appear in `(NEEDS-FILES …)` so the panel can prompt for them; they
///   become `@file` byte-refs the panel resolves before applying.
/// - The emitter is auto-bound to the predicted `photometry_1` /
///   `lightsource_1` ids — valid because the wizard reducer mints them
///   in that exact order on a fresh document (same as the GUI wizard's
///   Emitter step placeholders).
/// - A `sensor-file` mints a `<Sensor>`; an `emergency-file` adds a
///   second `<Photometry>` for the emergency beam.
fn gldf_wizard(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let arg = |i: usize| args.get(i).and_then(expr_to_string).unwrap_or_default();
    let manufacturer = arg(0);
    let name = arg(1);
    let author = arg(2);
    let photometry = arg(3);
    let geometry = arg(4);
    let variant = arg(5);
    let sensor = arg(6);
    let emergency = arg(7);

    if manufacturer.trim().is_empty() && name.trim().is_empty() {
        return arg_error(
            ctx.output,
            "GLDF-WIZARD",
            "<manufacturer> <name> [author] [photometry] [geometry] [variant] [sensor] [emergency]",
            args,
        );
    }

    // Build a WizardDraft as JSON. Field names match
    // `gldf_rs_editor::WizardDraft`'s serde shape (snake_case). Byte
    // fields carry `{"@file":name}` so the panel fills them from dropped
    // files; serde then reads the resolved array as `Some(Vec<u8>)`.
    // ls_name doubles as the product/light-source display name; ls_kind
    // "Fixed" + an auto-bound emitter give a complete optical chain.
    let mut draft = json!({
        "header_author": author,
        "header_manufacturer": manufacturer,
        "header_created_with": "gldf.icu Lisp REPL",
        "header_default_language": "en",
        "ls_kind": "Fixed",
        "ls_name": name,
        "ls_manufacturer": manufacturer,
        // FixedLightSource/RatedInputPower is REQUIRED by the XSD — an
        // empty string parses to None and the element is dropped, which
        // fails validation ("Expected at least 1 RatedInputPower"). Give
        // a sane default; the user edits it in the LightSources page.
        "ls_rated_power": "30",
        "variant_name": if variant.trim().is_empty() { name.clone() } else { variant },
        "emitter_draft": {
            "child_name": name,
            "kind": "Fixed",
            "photometry_id": "photometry_1",
            "fixed_light_source_id": "lightsource_1",
            "multi_channel_light_source_id": "",
            "sensor_id": ""
        },
        "driver_enabled": false,
        "driver_name": "",
        "driver_dimmable": false,
        "driver_standby_power": "",
        // Default to a recessed ceiling downlight (the typical Prolumia
        // luminaire) so the variant ships a <Mountings> block. The user
        // can change it on the Variants page.
        "mounting_type": "ceiling/recessed",
        "mounting_recessed_depth_mm": "100",
        "mounting_pendant_length_mm": "",
        "mounting_height_mm": "",
        "mounting_pole_height_mm": "",
        "photometry_filename": photometry,
        "sensor_filename": sensor,
        "emergency_filename": emergency,
        "geometry_filename": geometry,
    });

    // Attach @file byte-refs only for the files that were named — an
    // empty filename must NOT become a bogus `{"@file":""}` ref.
    if !photometry.trim().is_empty() {
        draft["photometry_bytes"] = json!({ "@file": photometry });
    }
    if !geometry.trim().is_empty() {
        draft["geometry_bytes"] = json!({ "@file": geometry });
    }
    if !sensor.trim().is_empty() {
        draft["sensor_bytes"] = json!({ "@file": sensor });
    }
    if !emergency.trim().is_empty() {
        draft["emergency_bytes"] = json!({ "@file": emergency });
    }

    emit_wizard(ctx, draft)
}

/// Dispatch an `EditorAction` carrying the given action JSON value.
/// Returns `T`. The `value` is the full `{"action":…,"data":…}` object.
fn emit_action(ctx: &mut ForeignContext, value: serde_json::Value) -> Expr {
    let json_str = value.to_string();
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::EditorAction {
            action_json: json_str,
        });
        Expr::T
    } else {
        ctx.output
            .push("(editor action) state missing\n".to_string());
        Expr::Nil
    }
}

/// `(NEEDS-FILES "a.ldt" "b.l3d" …)` — declare the files this script
/// needs. The header block. Args are filenames (strings). Returns the
/// count. The panel reads this pre-run to know what to prompt for.
fn needs_files(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let names: Vec<String> = args.iter().filter_map(expr_to_string).collect();
    if names.is_empty() {
        return arg_error(ctx.output, "NEEDS-FILES", "one or more filenames", args);
    }
    let n = names.len();
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::NeedFiles { names });
        Expr::Integer(n as i32)
    } else {
        ctx.output.push("(NEEDS-FILES) state missing\n".to_string());
        Expr::Nil
    }
}

/// `(GLDF-SET-MANUFACTURER "SLV")` → `SetManufacturer` (tuple variant).
fn gldf_set_manufacturer(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(s) = args.first().and_then(expr_to_string) else {
        return arg_error(ctx.output, "GLDF-SET-MANUFACTURER", "<string>", args);
    };
    emit_action(ctx, json!({ "action": "SetManufacturer", "data": s }))
}

/// `(GLDF-SET-AUTHOR "Holger Trahe")` → `SetAuthor` (tuple variant).
fn gldf_set_author(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(s) = args.first().and_then(expr_to_string) else {
        return arg_error(ctx.output, "GLDF-SET-AUTHOR", "<string>", args);
    };
    emit_action(ctx, json!({ "action": "SetAuthor", "data": s }))
}

/// `(GLDF-SET-NAME "Lumina Pro")` → `SetProductMetaName` with an `en`
/// Locale. `SetProductMetaName` takes a `LocaleFoo { locale: [{language,
/// $text}] }`.
fn gldf_set_name(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(s) = args.first().and_then(expr_to_string) else {
        return arg_error(ctx.output, "GLDF-SET-NAME", "<string>", args);
    };
    emit_action(
        ctx,
        json!({
            "action": "SetProductMetaName",
            "data": { "Locale": [{ "@language": "en", "$text": s }] }
        }),
    )
}

/// `(GLDF-NEW "SLV" "Lumina Pro")` — convenience: set manufacturer +
/// product name in one call. Emits TWO actions (no file needed, so we
/// avoid the wizard). Returns `T` if both dispatched.
fn gldf_new(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(mfr) = args.first().and_then(expr_to_string) else {
        return arg_error(ctx.output, "GLDF-NEW", "<manufacturer> <name>", args);
    };
    let name = args.get(1).and_then(expr_to_string);
    let a = emit_action(ctx, json!({ "action": "SetManufacturer", "data": mfr }));
    if let Some(name) = name {
        let _ = emit_action(
            ctx,
            json!({
                "action": "SetProductMetaName",
                "data": { "Locale": [{ "@language": "en", "$text": name }] }
            }),
        );
    }
    a
}

/// `(GLDF-ADD-PHOTOMETRY "middle.ldt")` → `AddPhotometryFile` with the
/// bytes pulled from the dropped file of the same name.
fn gldf_add_photometry(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(f) = args.first().and_then(expr_to_string) else {
        return arg_error(ctx.output, "GLDF-ADD-PHOTOMETRY", "<filename>", args);
    };
    emit_action(
        ctx,
        json!({
            "action": "AddPhotometryFile",
            "data": { "bytes": { "@file": f }, "filename": f }
        }),
    )
}

/// `(GLDF-ADD-SENSOR "sensor.ldt")` → `AddSensorFile`.
fn gldf_add_sensor(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(f) = args.first().and_then(expr_to_string) else {
        return arg_error(ctx.output, "GLDF-ADD-SENSOR", "<filename>", args);
    };
    emit_action(
        ctx,
        json!({
            "action": "AddSensorFile",
            "data": { "bytes": { "@file": f }, "filename": f }
        }),
    )
}

/// `(GLDF-ADD-SPECTRUM "spectrum_2700K.spd" "ls1")` → `AddSpectrumFile`
/// bound to the given light-source id. The light-source id is
/// required by the action (it links the spectrum on creation).
fn gldf_add_spectrum(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(f) = args.first().and_then(expr_to_string) else {
        return arg_error(
            ctx.output,
            "GLDF-ADD-SPECTRUM",
            "<filename> <light-source-id>",
            args,
        );
    };
    let ls_id = args.get(1).and_then(expr_to_string).unwrap_or_default();
    emit_action(
        ctx,
        json!({
            "action": "AddSpectrumFile",
            "data": { "bytes": { "@file": f }, "filename": f, "light_source_id": ls_id }
        }),
    )
}

/// `(GLDF-SET-GEOMETRY "example_007.l3d")` → `ImportL3dFile`.
fn gldf_set_geometry(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(f) = args.first().and_then(expr_to_string) else {
        return arg_error(ctx.output, "GLDF-SET-GEOMETRY", "<filename>", args);
    };
    emit_action(
        ctx,
        json!({
            "action": "ImportL3dFile",
            "data": { "bytes": { "@file": f }, "filename": f }
        }),
    )
}

/// `(GLDF-ADD-VARIANT "Lumina Pro 4000K")` → `AddVariantSimple`.
/// (The variant's geometry/mounting/emitter bindings are separate
/// follow-up verbs — this just creates the named `<Variant>`.) An
/// optional second arg is accepted and ignored for now (room for an
/// id vs. name split later).
fn gldf_add_variant(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(name) = args.first().and_then(expr_to_string) else {
        return arg_error(ctx.output, "GLDF-ADD-VARIANT", "<name>", args);
    };
    // Touch the second arg through expr_to_usize/str so a future
    // (id name) form is forward-compatible; ignored today.
    let _ = args.get(1).and_then(expr_to_usize);
    emit_action(
        ctx,
        json!({ "action": "AddVariantSimple", "data": { "name": name } }),
    )
}

/// `(GLDF-ADD-LIGHTSOURCE id name cct [power])` → `AddFixedLightSource`
/// with an EXPLICIT id (so emitters can reference it by name without
/// guessing auto-ids), a display name, a CorrelatedColorTemperature
/// (Kelvin) under ColorInformation, and an optional RatedInputPower (W).
///
/// The demo has 4 light sources differing by CCT (4000K/3000K/5000K +
/// a 3000K emergency module) — this verb builds each so the variants
/// can bind LEO0/LEO1 to the right colour, which is what makes the 3D
/// viewer show distinct CCTs per variant.
fn gldf_add_lightsource(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(id) = args.first().and_then(expr_to_string) else {
        return arg_error(
            ctx.output,
            "GLDF-ADD-LIGHTSOURCE",
            "<id> <name> <cct-kelvin> [power-w]",
            args,
        );
    };
    let name = args.get(1).and_then(expr_to_string).unwrap_or_default();
    let cct = args.get(2).and_then(expr_to_f32).map(|c| c as i64);
    // RatedInputPower is XSD-required; default to 1 W if omitted so the
    // light source always validates.
    let power = args.get(3).and_then(expr_to_f32).unwrap_or(1.0) as f64;

    let mut ls = json!({
        "@id": id,
        "Name": { "Locale": [{ "@language": "en", "$text": name }] },
        "RatedInputPower": power,
    });
    if let Some(cct) = cct {
        ls["ColorInformation"] = json!({ "CorrelatedColorTemperature": cct });
    }
    emit_action(
        ctx,
        json!({ "action": "AddFixedLightSource", "data": { "payload": ls } }),
    )
}

/// `(GLDF-ADD-EMITTER photometry-id light-source-id [child-name] [lumens])`
/// → `AddEmitterWithChild` (Fixed kind) — creates an `<Emitter>` whose
/// FixedLightEmitter binds the photometry to the light source. This is
/// what turns an embedded photometry into a real optical chain
/// `(GLDF-ADD-EMITTER emitter-id photometry-id light-source-id [leo] [lumens])`
/// → `AddFixedEmitter` — creates an `<Emitter>` with an EXPLICIT id
/// whose FixedLightEmitter binds the photometry to the light source and
/// carries the lumens, all in one step. Using explicit ids (e.g.
/// `"em_v1_leo0"`) means later `GLDF-VARIANT-GEOMETRY` references are
/// stable — no predicting `emitter_N` auto-ids, no "0 lm" gaps.
///
/// `leo` is the LightEmitter child name (also the L3D part name, e.g.
/// "LEO0"). `lumens` defaults to 0 if omitted (set it so the 3D scene
/// isn't dark).
fn gldf_add_emitter(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(emitter_id) = args.first().and_then(expr_to_string) else {
        return arg_error(
            ctx.output,
            "GLDF-ADD-EMITTER",
            "<emitter-id> <photometry-id> <light-source-id> [leo] [lumens]",
            args,
        );
    };
    let Some(photometry_id) = args.get(1).and_then(expr_to_string) else {
        return arg_error(
            ctx.output,
            "GLDF-ADD-EMITTER",
            "<emitter-id> <photometry-id> <light-source-id> [leo] [lumens]",
            args,
        );
    };
    let ls_id = args.get(2).and_then(expr_to_string).unwrap_or_default();
    let leo = args.get(3).and_then(expr_to_string);
    let lumens = args.get(4).and_then(expr_to_f32).unwrap_or(0.0) as i64;
    emit_action(
        ctx,
        json!({
            "action": "AddFixedEmitter",
            "data": {
                "emitter_id": emitter_id,
                "child_name": leo,
                "photometry_id": photometry_id,
                "light_source_id": ls_id,
                "rated_luminous_flux": lumens
            }
        }),
    )
}

/// `(GLDF-VARIANT-GEOMETRY variant-index geometry-id emitter-id
///   [external-name])` → `SetVariantGeometryModel` — point the variant
/// at the L3D `<ModelGeometry>` and bind one emitter into it, so the
/// variant actually has light configured (not an empty `<Variant>`).
///
/// `variant-index` is 0-based in document order. After `GLDF-WIZARD`
/// the wizard's own variant is index 0 (already bound); the variants
/// added by `GLDF-ADD-VARIANT` are 1, 2, 3, …. `external-name` is the
/// object name inside the L3D the emitter sits on (defaults to the
/// emitter id).
fn gldf_variant_geometry(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let usage = "<variant-index> <geometry-id> <emitter> <leo> [<emitter> <leo> …]";
    let Some(variant_idx) = args.first().and_then(expr_to_usize) else {
        return arg_error(ctx.output, "GLDF-VARIANT-GEOMETRY", usage, args);
    };
    let Some(geometry_id) = args.get(1).and_then(expr_to_string) else {
        return arg_error(ctx.output, "GLDF-VARIANT-GEOMETRY", usage, args);
    };
    // Remaining args are (emitter-id, external-name/LEO) PAIRS, so a
    // variant can light multiple LEOs (LEO0 + LEO1) like the demo. An
    // odd trailing arg reuses the emitter id as its own external name.
    let rest: Vec<String> = args.iter().skip(2).filter_map(expr_to_string).collect();
    if rest.is_empty() {
        return arg_error(ctx.output, "GLDF-VARIANT-GEOMETRY", usage, args);
    }
    let mut bindings = Vec::new();
    let mut i = 0;
    while i < rest.len() {
        let emitter_id = rest[i].clone();
        let external_name = rest
            .get(i + 1)
            .cloned()
            .unwrap_or_else(|| emitter_id.clone());
        bindings.push(json!({ "emitter_id": emitter_id, "external_name": external_name }));
        i += 2;
    }
    emit_action(
        ctx,
        json!({
            "action": "SetVariantGeometryModel",
            "data": {
                "variant_idx": variant_idx,
                "geometry_id": geometry_id,
                "bindings": bindings
            }
        }),
    )
}

/// `(GLDF-LUMENS emitter-id lumens)` → set `RatedLuminousFlux` on the
/// first FixedLightEmitter child of the emitter. Without this the
/// emitter outputs 0 lm and the Bevy 3D viewer renders a dark scene.
fn gldf_lumens(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(emitter_id) = args.first().and_then(expr_to_string) else {
        return arg_error(ctx.output, "GLDF-LUMENS", "<emitter-id> <lumens>", args);
    };
    let lumens = args.get(1).and_then(expr_to_f32).unwrap_or(0.0) as i64;
    emit_action(
        ctx,
        json!({
            "action": "SetFixedLightEmitterFlux",
            "data": { "emitter_id": emitter_id, "index": 0, "rated_luminous_flux": lumens }
        }),
    )
}

/// `(GLDF-HOUSING-COLOR variant-index ral "name")` → add one
/// HousingColor (RAL number + locale name) at the given scope. Emits
/// `AddHousingColor` then `UpdateHousingColor{index:0}` — assumes this
/// is the scope's first/only housing color (true for a fresh build).
/// At variant scope this also drives the Bevy housing-color override,
/// so the variant actually shows its RAL colour in 3D.
fn gldf_housing_color(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let scope = scope_arg(args.first());
    let ral = args.get(1).and_then(expr_to_f32).map(|r| r as i64);
    let name = args.get(2).and_then(expr_to_string).unwrap_or_default();
    let _ = emit_action(ctx, json!({ "action": "AddHousingColor", "data": scope }));
    emit_action(
        ctx,
        json!({
            "action": "UpdateHousingColor",
            "data": { "scope": scope, "index": 0, "ral": ral, "language": "", "value": name }
        }),
    )
}

/// `(GLDF-APPLICATIONS variant-index "Office" "Industrial" …)` → set
/// the Applications list at the given scope (replaces any existing).
fn gldf_applications(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let scope = scope_arg(args.first());
    let apps: Vec<String> = args.iter().skip(1).filter_map(expr_to_string).collect();
    if apps.is_empty() {
        return arg_error(
            ctx.output,
            "GLDF-APPLICATIONS",
            "<variant-index> <app> [app …]",
            args,
        );
    }
    emit_action(
        ctx,
        json!({ "action": "SetApplications", "data": [scope, apps] }),
    )
}

/// `(GLDF-MARKET variant-index "Europe")` → add one Markets/Region
/// locale entry at the given scope (`AddMarketRegion` + `Update…`).
fn gldf_market(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let scope = scope_arg(args.first());
    let value = args.get(1).and_then(expr_to_string).unwrap_or_default();
    let _ = emit_action(ctx, json!({ "action": "AddMarketRegion", "data": scope }));
    emit_action(
        ctx,
        json!({
            "action": "UpdateMarketRegion",
            "data": { "scope": scope, "index": 0, "language": "", "value": value }
        }),
    )
}

/// `(GLDF-APPROVAL variant-index "ENEC")` → add one ApprovalMark at
/// the given scope (`AddApprovalMark` + `Update…`).
fn gldf_approval(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let scope = scope_arg(args.first());
    let value = args.get(1).and_then(expr_to_string).unwrap_or_default();
    let _ = emit_action(ctx, json!({ "action": "AddApprovalMark", "data": scope }));
    emit_action(
        ctx,
        json!({
            "action": "UpdateApprovalMark",
            "data": { "scope": scope, "index": 0, "value": value }
        }),
    )
}

/// `(GLDF-ELECTRICAL variant-index "ClassII" "IP20")` → set the
/// Electrical safety class + ingress-protection IP code at the given
/// scope. Pass `""` to skip either field.
fn gldf_electrical(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let scope = scope_arg(args.first());
    let safety = args.get(1).and_then(expr_to_string).unwrap_or_default();
    let ip = args.get(2).and_then(expr_to_string).unwrap_or_default();
    if !safety.trim().is_empty() {
        let _ = emit_action(
            ctx,
            json!({ "action": "SetElectricalSafetyClass", "data": [scope, safety] }),
        );
    }
    if !ip.trim().is_empty() {
        let _ = emit_action(
            ctx,
            json!({ "action": "SetIngressProtectionIPCode", "data": [scope, ip] }),
        );
    }
    Expr::T
}

/// `(GLDF-MECHANICAL variant-index "IK08" weight-kg)` → set the
/// Mechanical IK rating + weight at the given scope. Pass `""` / omit
/// to skip either field.
fn gldf_mechanical(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let scope = scope_arg(args.first());
    let ik = args.get(1).and_then(expr_to_string).unwrap_or_default();
    let weight = args.get(2).and_then(expr_to_f32).map(|w| w as f64);
    if !ik.trim().is_empty() {
        let _ = emit_action(
            ctx,
            json!({ "action": "SetMechanicalIKRating", "data": [scope, ik] }),
        );
    }
    if let Some(w) = weight {
        let _ = emit_action(
            ctx,
            json!({ "action": "SetMechanicalWeight", "data": [scope, w] }),
        );
    }
    Expr::T
}

/// `(GLDF-RENAME-VARIANT old-id new-id)` → `RenameVariant` — give a
/// variant a meaningful id (e.g. `v_ral6018_yellowgreen`) instead of
/// the auto `variant_1`. Cascades into any references. After
/// `AddVariantSimple` the auto ids are `variant_1`, `variant_2`, ….
fn gldf_rename_variant(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(old_id) = args.first().and_then(expr_to_string) else {
        return arg_error(ctx.output, "GLDF-RENAME-VARIANT", "<old-id> <new-id>", args);
    };
    let Some(new_id) = args.get(1).and_then(expr_to_string) else {
        return arg_error(ctx.output, "GLDF-RENAME-VARIANT", "<old-id> <new-id>", args);
    };
    emit_action(
        ctx,
        json!({ "action": "RenameVariant", "data": { "old_id": old_id, "new_id": new_id } }),
    )
}

/// `(GLDF-PRODUCT-PICTURE "product_picture.jpg" ["Product Picture"])` →
/// `AddProductPictureFile` — embed the image AND reference it as a
/// product `<Pictures>/<Image>` in one step. The file must be in
/// `(NEEDS-FILES …)`. `image-type` defaults to "Product Picture".
fn gldf_product_picture(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(f) = args.first().and_then(expr_to_string) else {
        return arg_error(
            ctx.output,
            "GLDF-PRODUCT-PICTURE",
            "<filename> [image-type]",
            args,
        );
    };
    let image_type = args
        .get(1)
        .and_then(expr_to_string)
        .unwrap_or_else(|| "Product Picture".to_string());
    emit_action(
        ctx,
        json!({
            "action": "AddProductPictureFile",
            "data": { "bytes": { "@file": f }, "filename": f, "image_type": image_type }
        }),
    )
}

/// `(GLDF-ADD-SENSOR-EMITTER sensor-id)` → `AddEmitterWithChild` of kind
/// Sensor, binding the `<Sensor>` (minted by GLDF-ADD-SENSOR / the
/// wizard's sensor file) into an emitter so it can be referenced by a
/// variant's geometry like any other emitter. Without this the sensor
/// sits unused. The wizard's sensor id is `sensor_1`.
fn gldf_add_sensor_emitter(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(sensor_id) = args.first().and_then(expr_to_string) else {
        return arg_error(ctx.output, "GLDF-ADD-SENSOR-EMITTER", "<sensor-id>", args);
    };
    emit_action(
        ctx,
        json!({
            "action": "AddEmitterWithChild",
            "data": {
                "child_name": null,
                "kind": "Sensor",
                "photometry_id": "",
                "fixed_light_source_id": "",
                "multi_channel_light_source_id": "",
                "sensor_id": sensor_id
            }
        }),
    )
}

/// `(GLDF-SENSOR-CHARS index "Round" "Passive Infrared" "Motion Detector")`
/// → set the n-th `<Sensor>`'s DetectorCharacteristic / DetectionMethod
/// / DetectorType (one each — the actions take lists but the demo uses
/// a single value per axis). XSD enums:
///   characteristic ∈ {Round, Square, Other}
///   method incl. "Passive Infrared"; type incl. "Motion Detector".
fn gldf_sensor_chars(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(index) = args.first().and_then(expr_to_usize) else {
        return arg_error(
            ctx.output,
            "GLDF-SENSOR-CHARS",
            "<sensor-index> <characteristic> <method> <type>",
            args,
        );
    };
    let characteristic = args.get(1).and_then(expr_to_string);
    let method = args.get(2).and_then(expr_to_string);
    let dtype = args.get(3).and_then(expr_to_string);
    if let Some(v) = characteristic {
        let _ = emit_action(
            ctx,
            json!({ "action": "SetSensorDetectorCharacteristics", "data": { "index": index, "values": [v] } }),
        );
    }
    if let Some(v) = method {
        let _ = emit_action(
            ctx,
            json!({ "action": "SetSensorDetectionMethods", "data": { "index": index, "values": [v] } }),
        );
    }
    if let Some(v) = dtype {
        let _ = emit_action(
            ctx,
            json!({ "action": "SetSensorDetectorTypes", "data": { "index": index, "values": [v] } }),
        );
    }
    Expr::T
}

/// `(GLDF-VARIANT-GTIN variant-index "4260...")` → set the variant's
/// GTIN attribute. A GTIN per variant is an unwritten must-have for
/// real catalogues.
fn gldf_variant_gtin(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let Some(index) = args.first().and_then(expr_to_usize) else {
        return arg_error(
            ctx.output,
            "GLDF-VARIANT-GTIN",
            "<variant-index> <gtin>",
            args,
        );
    };
    let gtin = args.get(1).and_then(expr_to_string);
    emit_action(
        ctx,
        json!({ "action": "SetVariantGTIN", "data": { "index": index, "gtin": gtin } }),
    )
}

/// `(GLDF-SIZE scope length width [height])` → set Mechanical
/// ProductSize length / width / (optional) height in mm at the given
/// scope (-1/"product" or a variant index).
fn gldf_size(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let scope = scope_arg(args.first());
    let length = args.get(1).and_then(expr_to_f32).map(|v| v as i64);
    let width = args.get(2).and_then(expr_to_f32).map(|v| v as i64);
    let height = args.get(3).and_then(expr_to_f32).map(|v| v as i64);
    if let Some(l) = length {
        let _ = emit_action(
            ctx,
            json!({ "action": "SetMechanicalLength", "data": [scope, l] }),
        );
    }
    if let Some(w) = width {
        let _ = emit_action(
            ctx,
            json!({ "action": "SetMechanicalWidth", "data": [scope, w] }),
        );
    }
    if let Some(h) = height {
        let _ = emit_action(
            ctx,
            json!({ "action": "SetMechanicalHeight", "data": [scope, h] }),
        );
    }
    Expr::T
}

/// `(GLDF-EMERGENCY scope "Guide Light" hours flux)` → set the
/// DedicatedEmergencyLightingType + add one DurationTimeAndFlux entry
/// (hours rated runtime, flux in lm). E.g. (GLDF-EMERGENCY -1
/// "Guide Light" 5 100). Type ∈ XSD EmergencyLightingType enum.
fn gldf_emergency(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let scope = scope_arg(args.first());
    let kind = args.get(1).and_then(expr_to_string);
    let hours = args.get(2).and_then(expr_to_f32).map(|v| v as i64);
    let flux = args.get(3).and_then(expr_to_f32).map(|v| v as i64);
    if let Some(k) = kind {
        let _ = emit_action(
            ctx,
            json!({ "action": "SetDedicatedEmergencyLightingType", "data": [scope, k] }),
        );
    }
    if let (Some(h), Some(f)) = (hours, flux) {
        let _ = emit_action(ctx, json!({ "action": "AddEmergencyFlux", "data": scope }));
        let _ = emit_action(
            ctx,
            json!({ "action": "UpdateEmergencyFlux", "data": { "scope": scope, "index": 0, "hours": h, "flux": f } }),
        );
    }
    Expr::T
}

#[cfg(test)]
mod tests {
    use crate::{create_gldf_interpreter, GldfState, SceneCommand};

    /// Run a script and return the dispatched command log.
    fn run(src: &str) -> Vec<SceneCommand> {
        let mut interp = create_gldf_interpreter();
        interp.run(src);
        interp
            .user_data
            .downcast_ref::<GldfState>()
            .unwrap()
            .log
            .clone()
    }

    #[test]
    fn needs_files_records_manifest() {
        let log = run(r#"(NEEDS-FILES "a.ldt" "b.l3d")"#);
        assert_eq!(log.len(), 1);
        match &log[0] {
            SceneCommand::NeedFiles { names } => {
                assert_eq!(names, &vec!["a.ldt".to_string(), "b.l3d".to_string()]);
            }
            other => panic!("expected NeedFiles, got {other:?}"),
        }
    }

    #[test]
    fn set_manufacturer_emits_tuple_action() {
        let log = run(r#"(GLDF-SET-MANUFACTURER "SLV")"#);
        match &log[0] {
            SceneCommand::EditorAction { action_json } => {
                assert!(action_json.contains("\"action\":\"SetManufacturer\""));
                assert!(action_json.contains("\"data\":\"SLV\""));
            }
            other => panic!("expected EditorAction, got {other:?}"),
        }
    }

    #[test]
    fn add_photometry_emits_file_ref() {
        let log = run(r#"(GLDF-ADD-PHOTOMETRY "middle.ldt")"#);
        match &log[0] {
            SceneCommand::EditorAction { action_json } => {
                assert!(action_json.contains("AddPhotometryFile"));
                assert!(action_json.contains("\"@file\":\"middle.ldt\""));
                assert!(action_json.contains("\"filename\":\"middle.ldt\""));
            }
            other => panic!("expected EditorAction, got {other:?}"),
        }
    }

    #[test]
    fn gldf_new_emits_manufacturer_and_name() {
        let log = run(r#"(GLDF-NEW "Prolumia" "Lumina Pro")"#);
        assert_eq!(log.len(), 2);
        let joined: String = log
            .iter()
            .map(|c| match c {
                SceneCommand::EditorAction { action_json } => action_json.clone(),
                _ => String::new(),
            })
            .collect();
        assert!(joined.contains("SetManufacturer"));
        assert!(joined.contains("Prolumia"));
        assert!(joined.contains("SetProductMetaName"));
        assert!(joined.contains("Lumina Pro"));
    }

    #[test]
    fn full_build_script_logs_in_order() {
        let log = run(r#"
            (NEEDS-FILES "middle.ldt" "example_007.l3d")
            (GLDF-NEW "Prolumia" "Lumina Pro")
            (GLDF-ADD-PHOTOMETRY "middle.ldt")
            (GLDF-SET-GEOMETRY "example_007.l3d")
            (GLDF-ADD-VARIANT "RAL6018")
            "#);
        // NeedFiles + (Manufacturer + Name) + Photometry + Geometry + Variant
        assert_eq!(log.len(), 6);
        assert!(matches!(log[0], SceneCommand::NeedFiles { .. }));
        assert!(matches!(log[5], SceneCommand::EditorAction { .. }));
    }
}
