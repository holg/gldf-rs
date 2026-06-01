//! LDT / IES goniometric viewer.
//!
//! Port of the Yew `LdtViewer` from `light-other`. Parses the bytes via
//! `eulumdat`/`IesParser`, then renders one of 10 diagram types (Polar,
//! Cartesian, Heatmap, 3D/Butterfly, BUG, LCS, Cone, Isocandela, Isolux,
//! V-H Floodlight) as inline SVG. The user clicks a tab to switch view.
//!
//! All rendering goes through eulumdat-rs's SVG renderers — no canvas, no
//! three-d. The mount uses our existing `SvgEmbed` component so the SVG
//! geometry survives the browser's HTML parser.

use crate::components::SvgEmbed;
use eulumdat::{
    diagram::{
        ButterflyDiagram, CartesianDiagram, ConeDiagram, FloodlightCartesianDiagram,
        HeatmapDiagram, IsocandelaDiagram, IsoluxDiagram, IsoluxParams, PolarDiagram, SvgTheme,
        YScale,
    },
    BugDiagram, PhotometricSummary,
};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Available diagram view types — matches the Yew `ViewType` enum exactly.
#[derive(Clone, PartialEq, Copy, Debug)]
pub enum LdtView {
    Polar,
    Cartesian,
    Heatmap,
    Butterfly,
    Bug,
    Lcs,
    Cone,
    Isocandela,
    Isolux,
    Floodlight,
}

impl LdtView {
    fn label(self) -> &'static str {
        match self {
            LdtView::Polar => "Polar",
            LdtView::Cartesian => "Cartesian",
            LdtView::Heatmap => "Heatmap",
            LdtView::Butterfly => "3D",
            LdtView::Bug => "BUG",
            LdtView::Lcs => "LCS",
            LdtView::Cone => "Cone",
            LdtView::Isocandela => "Isocd",
            LdtView::Isolux => "Isolux",
            LdtView::Floodlight => "V-H",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            LdtView::Polar => "◉",
            LdtView::Cartesian => "📊",
            LdtView::Heatmap => "🔥",
            LdtView::Butterfly => "🦋",
            LdtView::Bug => "🐛",
            LdtView::Lcs => "📐",
            LdtView::Cone => "🔦",
            LdtView::Isocandela => "🕯",
            LdtView::Isolux => "💡",
            LdtView::Floodlight => "🔆",
        }
    }

    fn all() -> &'static [LdtView] {
        &[
            LdtView::Polar,
            LdtView::Cartesian,
            LdtView::Heatmap,
            LdtView::Butterfly,
            LdtView::Bug,
            LdtView::Lcs,
            LdtView::Cone,
            LdtView::Isocandela,
            LdtView::Isolux,
            LdtView::Floodlight,
        ]
    }
}

/// Leptos LDT/IES viewer.
///
/// Renders the goniometric distribution for an `.ies` or `.ldt` file. The
/// view-type tab bar lets the user switch between the 10 diagram flavours
/// eulumdat-rs ships. SVG mounts via `SvgEmbed` so the geometry survives.
#[component]
pub fn LdtPanel(
    /// Raw `.ies` / `.ldt` bytes.
    bytes: Vec<u8>,
    /// Diagram width in SVG units.
    #[prop(default = 500.0)]
    width: f64,
    /// Diagram height in SVG units.
    #[prop(default = 400.0)]
    height: f64,
) -> impl IntoView {
    let view = RwSignal::new(LdtView::Polar);
    let is_zoomed = RwSignal::new(false);

    // Two derived SVG signals: the inline (small) and the zoomed (large)
    // version. Both react to the active view-type signal, so flipping a tab
    // updates both renderings in lockstep. The bytes are cloned twice
    // because each `Signal::derive` closure captures by move.
    let bytes_inline = bytes.clone();
    let svg_inline = Signal::derive(move || {
        generate_ldt_svg(&bytes_inline, width, height, view.get())
    });
    let bytes_zoom = bytes.clone();
    let svg_zoom = Signal::derive(move || {
        // Big render for the modal — same shape eulumdat-rs uses when the
        // Yew side opened the zoomed view.
        generate_ldt_svg(&bytes_zoom, 800.0, 650.0, view.get())
    });

    // Close on Esc — wire a doc-level keydown listener while the modal is up.
    // Without it the user has no escape from a fullscreen overlay; the click-
    // outside fallback works but Esc is the expected affordance.
    Effect::new(move |_| {
        if !is_zoomed.get() {
            return;
        }
        let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
            if e.key() == "Escape" {
                is_zoomed.set(false);
            }
        }) as Box<dyn FnMut(_)>);
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            let _ = doc.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
        }
        // Leak the closure — it lives until the page unloads. The listener
        // is fine to keep registered; checking `is_zoomed` inside is cheap.
        cb.forget();
    });

    let on_zoom_open = move |_| is_zoomed.set(true);
    let on_zoom_close = move |_| is_zoomed.set(false);

    view! {
        <div class="ldt-viewer">
            <div class="ldt-viewer-tabs" style="display: flex; flex-wrap: wrap; gap: 4px; margin-bottom: 8px;">
                {LdtView::all().iter().map(|vt| {
                    let vt = *vt;
                    let label = vt.label();
                    let icon = vt.icon();
                    let class = move || {
                        if view.get() == vt { "ldt-tab active" } else { "ldt-tab" }
                    };
                    let on_click = move |_| view.set(vt);
                    view! {
                        <button on:click=on_click class=class title=label style="padding: 4px 8px; cursor: pointer;">
                            <span class="tab-icon">{ icon }</span>
                            <span class="tab-label" style="margin-left: 4px;">{ label }</span>
                        </button>
                    }
                }).collect::<Vec<_>>()}
            </div>
            <div
                class="ldt-diagram clickable"
                style=format!("width: {}px; height: {}px; background: #1a1a2e; cursor: zoom-in;", width, height)
                on:click=on_zoom_open
                title="Click to enlarge"
            >
                <SvgEmbed svg=svg_inline style="display: block; width: 100%; height: 100%;".to_string() />
            </div>

            // Zoom modal — full-viewport overlay with a large render and the
            // same tab bar so the user can switch view types without closing.
            {move || if is_zoomed.get() {
                view! {
                    <div
                        on:click=on_zoom_close
                        style="position: fixed; inset: 0; background: rgba(0,0,0,0.85); z-index: 2000; display: flex; align-items: center; justify-content: center; padding: 20px;"
                    >
                        <div
                            on:click=move |e| e.stop_propagation()
                            style="background: #1a1a2e; border-radius: 8px; max-width: 95vw; max-height: 95vh; display: flex; flex-direction: column; padding: 16px;"
                        >
                            <div style="display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-bottom: 12px;">
                                <div class="ldt-viewer-tabs" style="display: flex; flex-wrap: wrap; gap: 4px;">
                                    {LdtView::all().iter().map(|vt| {
                                        let vt = *vt;
                                        let label = vt.label();
                                        let icon = vt.icon();
                                        let class = move || {
                                            if view.get() == vt { "ldt-tab active" } else { "ldt-tab" }
                                        };
                                        let on_click = move |_| view.set(vt);
                                        view! {
                                            <button on:click=on_click class=class title=label style="padding: 6px 10px; cursor: pointer;">
                                                <span class="tab-icon">{ icon }</span>
                                                <span class="tab-label" style="margin-left: 6px;">{ label }</span>
                                            </button>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                                <button
                                    on:click=on_zoom_close
                                    title="Close (Esc)"
                                    style="background: none; border: 1px solid #555; color: #ccc; border-radius: 4px; padding: 4px 12px; cursor: pointer; font-size: 16px;"
                                >
                                    { "✕" }
                                </button>
                            </div>
                            <div style="width: 800px; height: 650px; max-width: 90vw; max-height: 80vh;">
                                <SvgEmbed svg=svg_zoom style="display: block; width: 100%; height: 100%;".to_string() />
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}
        </div>
    }
}

fn generate_ldt_svg(buffer: &[u8], width: f64, height: f64, view: LdtView) -> String {
    // Encoding fallback: many IES files from Windows tooling have Latin-1
    // bytes in [MORE]/[LAMP] comment lines, so a hard from_utf8 would
    // wrongly reject them. We mirror the eulumdat lib's
    // read_with_encoding_fallback shape via the shared helper.
    let (content, _decode_kind) =
        crate::components::photometry_decode::decode_photometry_bytes(buffer);
    let ldt = match crate::components::photometry_decode::parse_photometry_with_fallback(&content)
    {
        Some((ldt, _kind)) => ldt,
        None => return error_svg(width, height, "Could not parse as IES or EULUMDAT"),
    };

    let theme = SvgTheme::dark();
    let summary = PhotometricSummary::from_eulumdat(&ldt);

    match view {
        LdtView::Polar => PolarDiagram::from_eulumdat(&ldt)
            .to_svg_with_summary(width, height, &theme, &summary),
        LdtView::Cartesian => CartesianDiagram::from_eulumdat(&ldt, width, height, 8)
            .to_svg_with_summary(width, height, &theme, &summary),
        LdtView::Heatmap => HeatmapDiagram::from_eulumdat(&ldt, width, height)
            .to_svg_with_summary(width, height, &theme, &summary),
        LdtView::Butterfly => ButterflyDiagram::from_eulumdat(&ldt, width, height, 60.0)
            .to_svg(width, height, &theme),
        LdtView::Bug => BugDiagram::from_eulumdat(&ldt)
            .to_svg_with_details(width.max(550.0), height, &theme),
        LdtView::Lcs => BugDiagram::from_eulumdat(&ldt)
            .to_lcs_svg(width.max(510.0), height, &theme),
        LdtView::Cone => ConeDiagram::from_eulumdat(&ldt, 8.0).to_svg(width, height, &theme),
        LdtView::Isocandela => IsocandelaDiagram::from_eulumdat(&ldt, width, height)
            .to_svg(width, height, &theme),
        LdtView::Isolux => {
            let params = IsoluxParams {
                mounting_height: 8.0,
                tilt_angle: 0.0,
                area_half_width: 20.0,
                area_half_depth: 20.0,
                grid_resolution: 50,
            };
            IsoluxDiagram::from_eulumdat(&ldt, width, height, params)
                .to_svg(width, height, &theme)
        }
        LdtView::Floodlight => {
            FloodlightCartesianDiagram::from_eulumdat(&ldt, width, height, YScale::Linear)
                .to_svg(width, height, &theme)
        }
    }
}

fn error_svg(width: f64, height: f64, message: &str) -> String {
    // Use named placeholders for the hex colours so the `#` in the source
    // doesn't trip the macro's tokenizer (it would see `#1a1a2e` as a numeric
    // prefix). The interpolated &str values render verbatim.
    let bg = "#1a1a2e";
    let fg = "#ff6b6b";
    format!(
        r#"<svg width="{w}" height="{h}" xmlns="http://www.w3.org/2000/svg">
            <rect width="100%" height="100%" fill="{bg}"/>
            <text x="50%" y="50%" text-anchor="middle" fill="{fg}" font-size="14">
                {msg}
            </text>
        </svg>"#,
        w = width,
        h = height,
        bg = bg,
        fg = fg,
        msg = message,
    )
}
