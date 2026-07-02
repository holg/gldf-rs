//! Validation banner — collapsible summary of every problem the lib's
//! validators can detect from the in-memory model, without an XSD.
//!
//! Two sources, merged:
//!
//! * `gldf_rs::validation::validate_gldf(&product, &embedded_files)` —
//!   the schema-rule engine: reference integrity (Photometry/Spectrum
//!   `@fileId` → `<File id=…>`, Emitter → LightSource, etc.), missing
//!   required fields, and consistency checks.
//! * `gldf_rs::validation::validate(&product)` — the canonical
//!   closed-XSD-enum walker (`Violation`). IP codes, IK ratings, all
//!   the closed `<xs:enumeration>` slots.
//!
//! Both walkers live in `gldf-rs-lib` and are dep-free, so the same
//! diagnostics also surface in the CLI and TUI; this component is the
//! WASM viewer's mount point. Empty result = nothing to show; the
//! banner self-hides.

use crate::state::use_gldf;
use gldf_rs::validation::{
    validate, validate_gldf, xsd_validate_product_xml, ValidationLevel,
};
use leptos::prelude::*;

#[component]
pub fn ValidationBanner() -> impl IntoView {
    let state = use_gldf();
    let open = RwSignal::new(false);

    view! {
        {move || state.with(|s| {
            // Recompute both walkers on every state change. They walk
            // the full tree; for a typical luminaire (few KB of XML
            // worth of references) this is microseconds — cheap enough
            // not to bother memoising.
            let engine = validate_gldf(&s.product, &s.embedded_files);
            let canon = validate(&s.product);
            // XSD: serialize the in-memory model the same way `Save` does
            // and run it past uppsala. Empty findings = clean. Cost is a
            // few ms per state change — fast enough to leave un-memoised
            // since the rest of this closure already walks the whole tree.
            let xsd_findings: Vec<(String, String)> = match s.product.to_xml() {
                Ok(xml) => match xsd_validate_product_xml(&xml) {
                    Ok(fs) => fs
                        .into_iter()
                        .map(|f| {
                            let loc = match (f.line, f.column) {
                                (Some(l), Some(c)) => format!("line {l}, col {c}"),
                                (Some(l), _) => format!("line {l}"),
                                _ => "—".to_string(),
                            };
                            (loc, f.message)
                        })
                        .collect(),
                    Err(_) => Vec::new(),
                },
                Err(_) => Vec::new(),
            };

            // Bucket by severity. Engine `Error` / `Warning` / `Info`
            // map to red / amber / blue. Canonical violations are
            // always amber (closed-enum drift is fixable, not fatal).
            let mut errors: Vec<(String, String)> = Vec::new();
            let mut warnings: Vec<(String, String)> = Vec::new();
            let mut info: Vec<(String, String)> = Vec::new();
            let mut xsd_rows: Vec<(String, String)> = xsd_findings;
            for e in &engine.errors {
                let row = (e.path.clone(), e.message.clone());
                match e.level {
                    ValidationLevel::Error => errors.push(row),
                    ValidationLevel::Warning => warnings.push(row),
                    ValidationLevel::Info => info.push(row),
                }
            }
            for v in &canon {
                let msg = if v.hint.is_empty() {
                    format!("{} = {:?} — outside closed XSD enum", v.field, v.value)
                } else {
                    format!("{} = {:?} — {}", v.field, v.value, v.hint)
                };
                warnings.push((v.path.clone(), msg));
            }

            let total = errors.len() + warnings.len() + info.len() + xsd_rows.len();
            if total == 0 {
                return ().into_any();
            }

            // XSD findings count as errors for headline severity — they
            // mean the file won't validate against the official XSD,
            // which is the strictest gate downstream.
            let xsd_count = xsd_rows.len();
            let (bg, border, icon, headline) = if !errors.is_empty() || xsd_count > 0 {
                let n = errors.len() + xsd_count;
                ("#3a1818", "#a33", "⛔", format!("{} validation error{}", n,
                    if n == 1 { "" } else { "s" }))
            } else if !warnings.is_empty() {
                ("#3a2a18", "#a73", "⚠️", format!("{} warning{}", warnings.len(),
                    if warnings.len() == 1 { "" } else { "s" }))
            } else {
                ("#18283a", "#37a", "ℹ️", format!("{} note{}", info.len(),
                    if info.len() == 1 { "" } else { "s" }))
            };
            let summary = format!(
                "{headline} · {xsd} XSD · {warn} warning{ws} · {inf} note{is_}",
                xsd = xsd_count,
                warn = warnings.len(),
                ws = if warnings.len() == 1 { "" } else { "s" },
                inf = info.len(),
                is_ = if info.len() == 1 { "" } else { "s" },
            );
            // Move xsd_rows into the view via local binding for clone below.
            let xsd_rows_view = std::mem::take(&mut xsd_rows);

            view! {
                <div
                    data-testid="validation-banner"
                    style=format!("background: {bg}; border: 1px solid {border}; border-radius: 6px; padding: 10px 14px; margin-bottom: 12px; color: var(--text-primary);")
                >
                    <div
                        style="display: flex; justify-content: space-between; align-items: center; cursor: pointer; gap: 10px;"
                        on:click=move |_| open.update(|v| *v = !*v)
                    >
                        <div>
                            <span style="margin-right: 6px;">{ icon }</span>
                            <strong>{ summary }</strong>
                            <span style="margin-left: 10px; font-size: 11px; color: var(--text-tertiary);">
                                { "Model-side + XSD (rc-3) — runs in-browser via uppsala" }
                            </span>
                        </div>
                        <span style="font-size: 11px; color: var(--text-tertiary);">
                            { move || if open.get() { "Hide ▲" } else { "Show ▼" } }
                        </span>
                    </div>
                    {move || open.get().then(|| view! {
                        <div style="margin-top: 10px; font-family: var(--font-mono); font-size: 11px; max-height: 280px; overflow: auto; padding-right: 4px;">
                            <ViolationGroup label="XSD (rc-3)" tint="#ffb0b0" rows=xsd_rows_view.clone() />
                            <ViolationGroup label="Errors" tint="#ffb0b0" rows=errors.clone() />
                            <ViolationGroup label="Warnings" tint="#ffd28a" rows=warnings.clone() />
                            <ViolationGroup label="Notes" tint="#9fd0ff" rows=info.clone() />
                        </div>
                    })}
                </div>
            }
            .into_any()
        })}
    }
}

#[component]
fn ViolationGroup(label: &'static str, tint: &'static str, rows: Vec<(String, String)>) -> impl IntoView {
    if rows.is_empty() {
        return ().into_any();
    }
    view! {
        <div style="margin: 6px 0;">
            <div style=format!("color: {tint}; font-weight: 600; margin-bottom: 4px;")>{ label }</div>
            <ul style="margin: 0; padding-left: 18px; line-height: 1.45;">
                {rows.into_iter().map(|(path, msg)| view! {
                    <li>
                        <code style="color: var(--text-secondary);">{ path }</code>
                        <span style="margin: 0 6px; color: var(--text-tertiary);">{ "—" }</span>
                        <span>{ msg }</span>
                    </li>
                }).collect::<Vec<_>>()}
            </ul>
        </div>
    }
    .into_any()
}
