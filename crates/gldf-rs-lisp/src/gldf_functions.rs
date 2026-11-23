//! Query functions for the GLDF product tree.
//!
//! Naming convention mirrors bimifc-lisp: uppercase `GLDF-…`. AutoLISP
//! is case-insensitive on symbol lookup but uppercase reads better in
//! scripts (`(GLDF-VARIANT-COUNT)`).

use acadlisp::{Expr, ForeignContext, Interpreter};
use gldf_rs::GldfProduct;

use crate::{arg_error, expr_to_string, expr_to_usize, state_mut, state_ref, SceneCommand};

pub fn register_all(interp: &mut Interpreter) {
    interp.register_fn("GLDF-LOAD", gldf_load);
    interp.register_fn("GLDF-VARIANT-COUNT", gldf_variant_count);
    interp.register_fn("GLDF-VARIANT-NAMES", gldf_variant_names);
    interp.register_fn("GLDF-VARIANT-SET", gldf_variant_set);
    interp.register_fn("GLDF-VARIANT-ACTIVE", gldf_variant_active);
    interp.register_fn("GLDF-MANUFACTURER", gldf_manufacturer);
    interp.register_fn("GLDF-EMITTER-IDS", gldf_emitter_ids);
}

/// `(GLDF-LOAD <path>)` — load a .gldf from disk into the interpreter's
/// state. Returns `T` on success, `Nil` on failure (and pushes the
/// error onto `ctx.output`).
fn gldf_load(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let path = match args.first().and_then(expr_to_string) {
        Some(p) => p,
        None => return arg_error(ctx.output, "GLDF-LOAD", "<path-string>", args),
    };
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            ctx.output.push(format!("(GLDF-LOAD {path:?}) read: {e}\n"));
            return Expr::Nil;
        }
    };
    let bundle = match GldfProduct::load_gldf_from_buf_all(bytes) {
        Ok(b) => b,
        Err(e) => {
            ctx.output
                .push(format!("(GLDF-LOAD {path:?}) parse: {e}\n"));
            return Expr::Nil;
        }
    };
    let Some(state) = state_mut(ctx) else {
        ctx.output.push("(GLDF-LOAD) state missing\n".to_string());
        return Expr::Nil;
    };
    state.product = Some(bundle.gldf);
    state.active_variant = 0;
    Expr::T
}

/// `(GLDF-VARIANT-COUNT)` — number of `<Variant>` entries on the
/// current product. Returns 0 if nothing is loaded.
fn gldf_variant_count(ctx: &mut ForeignContext, _args: &[Expr]) -> Expr {
    let count = state_ref(ctx)
        .and_then(|s| s.product.as_ref())
        .and_then(|p| p.product_definitions.variants.as_ref())
        .map(|v| v.variant.len())
        .unwrap_or(0);
    Expr::Integer(count as i32)
}

/// `(GLDF-VARIANT-NAMES)` — list of variant ids as strings. Empty list
/// if no product is loaded.
fn gldf_variant_names(ctx: &mut ForeignContext, _args: &[Expr]) -> Expr {
    let names: Vec<Expr> = state_ref(ctx)
        .and_then(|s| s.product.as_ref())
        .and_then(|p| p.product_definitions.variants.as_ref())
        .map(|vs| {
            vs.variant
                .iter()
                .map(|v| Expr::String(v.id.clone()))
                .collect()
        })
        .unwrap_or_default();
    Expr::list(names)
}

/// `(GLDF-VARIANT-SET <index>)` — change the active variant index.
/// Returns the new index as an integer, or `Nil` if out of range.
fn gldf_variant_set(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let idx = match args.first().and_then(expr_to_usize) {
        Some(i) => i,
        None => return arg_error(ctx.output, "GLDF-VARIANT-SET", "<index>", args),
    };
    let n = state_ref(ctx)
        .and_then(|s| s.product.as_ref())
        .and_then(|p| p.product_definitions.variants.as_ref())
        .map(|v| v.variant.len())
        .unwrap_or(0);
    if idx >= n {
        ctx.output
            .push(format!("(GLDF-VARIANT-SET {idx}) out of range (0..{n})\n"));
        return Expr::Nil;
    }
    if let Some(s) = state_mut(ctx) {
        s.active_variant = idx;
        s.dispatch(SceneCommand::SetActiveVariant { index: idx });
    }
    Expr::Integer(idx as i32)
}

/// `(GLDF-VARIANT-ACTIVE)` — active variant index as an integer.
fn gldf_variant_active(ctx: &mut ForeignContext, _args: &[Expr]) -> Expr {
    let idx = state_ref(ctx).map(|s| s.active_variant).unwrap_or(0);
    Expr::Integer(idx as i32)
}

/// `(GLDF-MANUFACTURER)` — manufacturer string from the Header. Empty
/// string when nothing loaded.
fn gldf_manufacturer(ctx: &mut ForeignContext, _args: &[Expr]) -> Expr {
    let mfr = state_ref(ctx)
        .and_then(|s| s.product.as_ref())
        .map(|p| p.header.manufacturer.clone())
        .unwrap_or_default();
    Expr::String(mfr)
}

/// `(GLDF-EMITTER-IDS)` — list of every emitter id on the loaded
/// product (across all variants and the GeneralDefinitions/Emitters
/// table). Useful for "what can I tilt?" introspection.
fn gldf_emitter_ids(ctx: &mut ForeignContext, _args: &[Expr]) -> Expr {
    let mut ids: Vec<Expr> = Vec::new();
    if let Some(p) = state_ref(ctx).and_then(|s| s.product.as_ref()) {
        if let Some(em) = p.general_definitions.emitters.as_ref() {
            for e in &em.emitter {
                ids.push(Expr::String(e.id.clone()));
            }
        }
    }
    Expr::list(ids)
}
