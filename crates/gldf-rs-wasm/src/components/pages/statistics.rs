//! Statistics page — counts dashboard with 6 stat cards.
//!
//! Mirrors `view_statistics` from the Yew side. Pure read-only.

use crate::state::use_gldf;
use leptos::prelude::*;

#[component]
pub fn StatisticsPage() -> impl IntoView {
    let state = use_gldf();

    view! {
        {move || state.with(|s| {
            let files_count = s.embedded_files.len();
            let ls = s.product.general_definitions.light_sources.as_ref();
            let fixed_ls = ls.map(|l| l.fixed_light_source.len()).unwrap_or(0);
            let changeable_ls = ls.map(|l| l.changeable_light_source.len()).unwrap_or(0);
            let variants_count = s
                .product
                .product_definitions
                .variants
                .as_ref()
                .map(|v| v.variant.len())
                .unwrap_or(0);
            let photometries_count = s
                .product
                .general_definitions
                .photometries
                .as_ref()
                .map(|p| p.photometry.len())
                .unwrap_or(0);
            let geometries_count = s
                .product
                .general_definitions
                .geometries
                .as_ref()
                .map(|g| g.simple_geometry.len() + g.model_geometry.len())
                .unwrap_or(0);

            view! {
                <div class="stats-grid" style="grid-template-columns: repeat(3, 1fr);">
                    <div class="stat-card">
                        <div class="icon blue">{ "📁" }</div>
                        <div class="value">{ files_count }</div>
                        <div class="label">{ "Embedded Files" }</div>
                    </div>
                    <div class="stat-card">
                        <div class="icon yellow">{ "💡" }</div>
                        <div class="value">{ fixed_ls }</div>
                        <div class="label">{ "Fixed Light Sources" }</div>
                    </div>
                    <div class="stat-card">
                        <div class="icon orange">{ "💡" }</div>
                        <div class="value">{ changeable_ls }</div>
                        <div class="label">{ "Changeable Light Sources" }</div>
                    </div>
                    <div class="stat-card">
                        <div class="icon purple">{ "📦" }</div>
                        <div class="value">{ variants_count }</div>
                        <div class="label">{ "Variants" }</div>
                    </div>
                    <div class="stat-card">
                        <div class="icon orange">{ "☀️" }</div>
                        <div class="value">{ photometries_count }</div>
                        <div class="label">{ "Photometries" }</div>
                    </div>
                    <div class="stat-card">
                        <div class="icon green">{ "🧊" }</div>
                        <div class="value">{ geometries_count }</div>
                        <div class="label">{ "Geometries" }</div>
                    </div>
                </div>
            }
        })}
    }
}
