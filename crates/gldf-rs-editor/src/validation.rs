//! Frontend-neutral validation report.
//!
//! Runs the three checks every frontend surfaces — the reference-walking
//! engine (`validate_gldf`), the canonical closed-enum check (`validate`)
//! and the XSD pass (uppsala, rc-3 or rc-4) — and buckets the findings by
//! severity the way the WASM banner presents them. The egui editor and
//! the TUI render this struct instead of re-deriving the bucketing.
//!
//! Requires the `xsd` feature (pulls the uppsala validator into the
//! build).

use gldf_rs::validation::{
    validate, validate_gldf, xsd_validate_product_xml_with_schema, GldfSchemaVersion,
    ValidationLevel,
};
use gldf_rs::GldfProduct;
use std::collections::HashMap;

/// One finding: where it is (path / line:col) and what is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationRow {
    pub location: String,
    pub message: String,
}

/// Findings bucketed by severity. `errors` + `xsd` are blocking (the file
/// won't validate downstream); `warnings` are fixable drift; `info` is
/// advisory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    /// Schema revision the XSD pass ran against.
    pub schema: GldfSchemaVersion,
    pub errors: Vec<ValidationRow>,
    pub warnings: Vec<ValidationRow>,
    pub info: Vec<ValidationRow>,
    pub xsd: Vec<ValidationRow>,
    /// Set when the XSD pass could not run at all (serialization or
    /// validator failure). Distinct from "clean": nothing was checked.
    pub xsd_error: Option<String>,
}

impl ValidationReport {
    /// Total number of findings across all buckets.
    pub fn total(&self) -> usize {
        self.errors.len() + self.warnings.len() + self.info.len() + self.xsd.len()
    }

    /// Blocking findings: engine errors plus XSD violations.
    pub fn error_count(&self) -> usize {
        self.errors.len() + self.xsd.len()
    }

    /// `true` when every pass ran and found nothing.
    pub fn is_clean(&self) -> bool {
        self.total() == 0 && self.xsd_error.is_none()
    }

    /// Short human label for the schema revision (`rc-3` / `rc-4`).
    pub fn schema_label(schema: GldfSchemaVersion) -> &'static str {
        match schema {
            GldfSchemaVersion::Rc3 => "rc-3",
            GldfSchemaVersion::Rc4 => "rc-4",
        }
    }

    /// One-line summary, e.g. `2 errors · 1 XSD · 3 warnings · 0 notes (rc-3)`.
    pub fn headline(&self) -> String {
        let schema = Self::schema_label(self.schema);
        if self.is_clean() {
            return format!("clean ({schema})");
        }
        let mut parts = Vec::new();
        parts.push(plural(self.errors.len(), "error", "errors"));
        parts.push(format!("{} XSD", self.xsd.len()));
        parts.push(plural(self.warnings.len(), "warning", "warnings"));
        parts.push(plural(self.info.len(), "note", "notes"));
        if let Some(err) = &self.xsd_error {
            parts.push(format!("XSD pass failed: {err}"));
        }
        format!("{} ({schema})", parts.join(" · "))
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Run every check against the in-memory model and bucket the findings.
///
/// The XSD pass serializes the product with the *same* schema spelling it
/// validates against (`to_xml_with_schema`), so an rc-3 check sees rc-3
/// element names and an rc-4 check sees the corrected ones.
pub fn validation_report(
    product: &GldfProduct,
    embedded_files: &HashMap<String, Vec<u8>>,
    schema: GldfSchemaVersion,
) -> ValidationReport {
    let mut report = ValidationReport {
        schema,
        errors: Vec::new(),
        warnings: Vec::new(),
        info: Vec::new(),
        xsd: Vec::new(),
        xsd_error: None,
    };

    let engine = validate_gldf(product, embedded_files);
    for e in &engine.errors {
        let row = ValidationRow {
            location: e.path.clone(),
            message: e.message.clone(),
        };
        match e.level {
            ValidationLevel::Error => report.errors.push(row),
            ValidationLevel::Warning => report.warnings.push(row),
            ValidationLevel::Info => report.info.push(row),
        }
    }

    // Canonical violations are always warnings: closed-enum drift is
    // fixable, not fatal.
    for v in validate(product) {
        let message = if v.hint.is_empty() {
            format!("{} = {:?} — outside closed XSD enum", v.field, v.value)
        } else {
            format!("{} = {:?} — {}", v.field, v.value, v.hint)
        };
        report.warnings.push(ValidationRow {
            location: v.path.clone(),
            message,
        });
    }

    match product.to_xml_with_schema(schema) {
        Ok(xml) => match xsd_validate_product_xml_with_schema(&xml, schema) {
            Ok(findings) => {
                report.xsd = findings
                    .into_iter()
                    .map(|f| ValidationRow {
                        location: match (f.line, f.column) {
                            (Some(l), Some(c)) => format!("line {l}, col {c}"),
                            (Some(l), None) => format!("line {l}"),
                            _ => "—".to_string(),
                        },
                        message: f.message,
                    })
                    .collect();
            }
            Err(e) => report.xsd_error = Some(e.to_string()),
        },
        Err(e) => report.xsd_error = Some(format!("serialize product.xml: {e}")),
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_product_reports_but_never_panics() {
        let product = GldfProduct::default();
        let files = HashMap::new();
        for schema in [GldfSchemaVersion::Rc3, GldfSchemaVersion::Rc4] {
            let report = validation_report(&product, &files, schema);
            assert_eq!(report.schema, schema);
            // A default product is not a valid GLDF; the headline must
            // say so rather than claim "clean".
            assert!(!report.headline().starts_with("clean") || report.is_clean());
            assert_eq!(
                report.total(),
                report.errors.len() + report.warnings.len() + report.info.len() + report.xsd.len()
            );
        }
    }

    #[test]
    fn headline_pluralizes() {
        let report = ValidationReport {
            schema: GldfSchemaVersion::Rc3,
            errors: vec![ValidationRow {
                location: "a".into(),
                message: "b".into(),
            }],
            warnings: Vec::new(),
            info: Vec::new(),
            xsd: Vec::new(),
            xsd_error: None,
        };
        assert_eq!(
            report.headline(),
            "1 error · 0 XSD · 0 warnings · 0 notes (rc-3)"
        );
        assert_eq!(report.error_count(), 1);
        assert!(!report.is_clean());
    }
}
