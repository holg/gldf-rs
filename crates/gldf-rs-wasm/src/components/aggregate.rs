//! Field-aggregation primitives for the scope-aware read views.
//!
//! GLDF's DescriptiveAttributes live at two levels: `ProductMetaData` (the
//! family default) and per-`Variant` (per-SKU override). The viewer rolls
//! both into one row: a uniform value when every variant agrees (or only
//! the default is set), or a "varies" row with per-value chips when they
//! diverge. This is the engine that classifies any single field, plus the
//! Leptos renderer that turns the result into a dl-row.
//!
//! Same shape as the Yew side's `FieldAggregate` + `aggregate_field` +
//! `render_row` trio so the read views look identical.

use gldf_rs::gldf::product_definitions::Variant;
use leptos::prelude::*;

/// One field aggregated across the product default + every variant.
#[derive(Debug, Clone)]
pub enum FieldAggregate {
    /// No source has the value. Caller should hide the row entirely.
    Empty,
    /// All sources agree. Single value to display.
    Uniform(String),
    /// Variants disagree. One entry per distinct value; the inner Vec lists
    /// variant ids that hold that value.
    Varies(Vec<(String, Vec<String>)>),
}

/// Aggregate a single field over the product default plus every variant's
/// override.
///
/// `default_value` is the family-level value (e.g.
/// `ProductMetaData/.../Electrical.electrical_safety_class`).
/// `variant_values` is `(variant_id, Option<value>)` for every variant.
/// `None` for a variant means "inherit / unset"; with a default present,
/// it folds into the default bucket.
pub fn aggregate_field(
    default_value: Option<String>,
    variant_values: Vec<(String, Option<String>)>,
) -> FieldAggregate {
    use std::collections::BTreeMap;

    let mut by_value: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut inheriting: Vec<String> = Vec::new();

    for (vid, val) in variant_values {
        match val {
            Some(v) => by_value.entry(v).or_default().push(vid),
            None => inheriting.push(vid),
        }
    }

    // Variants that don't override carry the default value if there is one.
    if let Some(ref d) = default_value {
        if !inheriting.is_empty() {
            by_value
                .entry(d.clone())
                .or_default()
                .extend(inheriting.iter().cloned());
        }
    }

    match by_value.len() {
        0 => match default_value {
            Some(d) => FieldAggregate::Uniform(d),
            None => FieldAggregate::Empty,
        },
        1 => {
            let (v, _) = by_value.into_iter().next().unwrap();
            FieldAggregate::Uniform(v)
        }
        _ => FieldAggregate::Varies(by_value.into_iter().collect()),
    }
}

/// Render the aggregate as one dl-row. Empty hides the row. Uniform shows
/// label / value. Varies shows the label with a "varies" badge, plus one
/// bucket per distinct value with the contributing variant chips.
///
/// `label_of` maps variant id → display label for the chips (usually the
/// variant's product number or id; see [`variant_chip_label`]).
pub fn render_aggregate_row(
    label: &'static str,
    agg: &FieldAggregate,
    label_of: &std::collections::HashMap<String, String>,
) -> impl IntoView {
    match agg {
        FieldAggregate::Empty => ().into_any(),
        FieldAggregate::Uniform(v) => {
            let v = v.clone();
            view! {
                <div class="aggregate-row">
                    <span class="aggregate-row-label">{ label }</span>
                    <span class="aggregate-row-value">{ v }</span>
                </div>
            }
            .into_any()
        }
        FieldAggregate::Varies(buckets) => {
            let buckets = buckets.clone();
            let label_of = label_of.clone();
            view! {
                <div class="aggregate-row varies">
                    <span class="aggregate-row-label">
                        { label }
                        <span class="varies-badge" title="This field varies between variants">
                            { "varies" }
                        </span>
                    </span>
                    <div class="aggregate-row-buckets">
                        {buckets.into_iter().map(|(value, vids)| {
                            let label_of = label_of.clone();
                            view! {
                                <div class="aggregate-bucket">
                                    <span class="bucket-value">{ value }</span>
                                    <span class="bucket-chips">
                                        {vids.into_iter().map(|vid| {
                                            let display = label_of.get(&vid).cloned().unwrap_or_else(|| vid.clone());
                                            let title = format!("Variant {}", vid);
                                            view! {
                                                <span
                                                    class="variant-chip"
                                                    title=title
                                                    style="display: inline-block; padding: 2px 8px; margin: 2px; background: var(--bg-secondary); border-radius: 4px; font-size: 11px;"
                                                >
                                                    { display }
                                                </span>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </span>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            }
            .into_any()
        }
    }
}

/// Display label for a variant chip — product number when available,
/// variant id otherwise.
pub fn variant_chip_label(variant: &Variant) -> String {
    if let Some(ref pn) = variant.product_number {
        if let Some(loc) = pn.locale.iter().find(|l| !l.value.is_empty()) {
            return loc.value.clone();
        }
    }
    variant.id.clone()
}
