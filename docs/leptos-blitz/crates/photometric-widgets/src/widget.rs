//! Blitz `Widget` implementations for photometric data.
//!
//! Attach to a DOM node via `DocumentMutator::set_custom_widget` /
//! `BaseDocument::set_custom_widget`. The node behaves like a replaced
//! element (think <img>): CSS sizes it, we paint it.
//!
//! Attribute protocol (drives reactivity from leptos without touching the
//! widget directly — `attribute_changed` fires on `set_attribute`):
//!   - `c-plane`: float degrees; absent = C0-C180/C90-C270 overview
//!   - `theme`:   "dark" | "light"

use crate::polar::{paint_ldt_polar, PolarStyle};
use anyrender::Scene;
use blitz_dom::node::{ComputedStyles, Widget};
use blitz_traits::events::UiEvent;
use eulumdat::Eulumdat;

pub struct PolarWidget {
    ldt: Eulumdat,
    c_plane: Option<f64>,
    style: PolarStyle,
}

impl PolarWidget {
    pub fn new(ldt: Eulumdat) -> Self {
        Self {
            ldt,
            c_plane: None,
            style: PolarStyle::default(),
        }
    }

    pub fn from_ldt_str(content: &str) -> eulumdat::Result<Self> {
        Ok(Self::new(Eulumdat::parse(content)?))
    }

    /// Extract photometry from a GLDF container by file id and build the
    /// widget. GLDF ships LDT/IES per variant; `gldf-rs` hands us the raw
    /// content via `get_ldc_by_id`.
    #[cfg(feature = "gldf")]
    pub fn from_gldf(
        product: &gldf_rs::GldfProduct,
        file_id: &str,
    ) -> anyhow::Result<Self> {
        let ldc = product.get_ldc_by_id(file_id.to_string())?;
        Ok(Self::new(Eulumdat::parse(&ldc)?))
    }
}

impl Widget for PolarWidget {
    fn attribute_changed(&mut self, name: &str, _old: Option<&str>, new: Option<&str>) {
        match name {
            "c-plane" => self.c_plane = new.and_then(|v| v.parse::<f64>().ok()),
            "theme" => {
                self.style = match new {
                    Some("dark") => PolarStyle::dark(),
                    _ => PolarStyle::default(),
                }
            }
            _ => {}
        }
    }

    fn handle_event(&mut self, _event: &UiEvent) {
        // Phase 2: hover → tooltip with (C, gamma, cd/klm) via hit math;
        // wheel → cycle c-plane. Spike: none.
    }

    fn paint(
        &mut self,
        _render_ctx: &mut dyn anyrender::RenderContext,
        _styles: &ComputedStyles,
        width: u32,
        height: u32,
        scale: f64,
    ) -> Scene {
        let mut scene = Scene::new();
        paint_ldt_polar(
            &mut scene,
            &self.ldt,
            self.c_plane,
            width as f64,
            height as f64,
            scale,
            &self.style,
        );
        scene
    }
}
