//! Export functions.
//!
//! First slice exposes just `(GLDF-EXPORT-XML <out-path>)` — round-trips
//! the loaded product through `GldfProduct::to_xml` and writes to disk.
//! TM-33 / IES / IFC per-LightSource exports will land in a follow-up
//! once we agree the lookup ergonomics (some need a light-source id).

use acadlisp::{Expr, ForeignContext, Interpreter};

use crate::{arg_error, expr_to_string, state_ref};

pub fn register_all(interp: &mut Interpreter) {
    interp.register_fn("GLDF-EXPORT-XML", gldf_export_xml);
}

fn gldf_export_xml(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let path = match args.first().and_then(expr_to_string) {
        Some(p) => p,
        None => return arg_error(ctx.output, "GLDF-EXPORT-XML", "<out-path-string>", args),
    };
    let Some(p) = state_ref(ctx).and_then(|s| s.product.as_ref()) else {
        ctx.output
            .push("(GLDF-EXPORT-XML) no product loaded\n".to_string());
        return Expr::Nil;
    };
    let xml = match p.to_xml() {
        Ok(x) => x,
        Err(e) => {
            ctx.output
                .push(format!("(GLDF-EXPORT-XML) serialize: {e}\n"));
            return Expr::Nil;
        }
    };
    if let Err(e) = std::fs::write(&path, xml) {
        ctx.output
            .push(format!("(GLDF-EXPORT-XML {path:?}) write: {e}\n"));
        return Expr::Nil;
    }
    Expr::T
}
