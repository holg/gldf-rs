//! L3D 3D viewer.
//!
//! Port of the Yew `L3dViewer` from `light-other`. Mounts a `<canvas>`,
//! parses the `.l3d` bytes via `l3d_rs::from_buffer`, creates a `DrawL3d`
//! renderer against the canvas, and starts the render loop. Drag to orbit,
//! scroll to zoom.
//!
//! `DrawL3d` itself lives in `crate::draw_l3d` (already framework-agnostic,
//! kept across the Yew→Leptos cut). This file is just the canvas mount +
//! event handlers.
//!
//! The renderer state (`Rc<RefCell<Option<DrawL3d>>>`) is held in a plain
//! `Rc` cloned into each closure. Leptos's `StoredValue` would need
//! `Send + Sync` (the framework-managed Send/Sync API) which `DrawL3d` is
//! not — three-d's WebGL handles aren't Sync. WASM is single-threaded so
//! the safety borrow rules of `RefCell` are sufficient.

use crate::draw_l3d::{start_render_loop, DrawL3d};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use wasm_bindgen::JsCast;

/// Per-instance counter so we can stagger init delays and tag log lines.
static VIEWER_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Wait `ms` milliseconds via `setTimeout` — the renderer wants the canvas
/// to be fully laid out before grabbing its size.
async fn delay_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[component]
pub fn L3dPanel(
    /// Raw `.l3d` bytes.
    bytes: Vec<u8>,
    /// Logical CSS width.
    #[prop(default = 600)]
    width: u32,
    /// Logical CSS height.
    #[prop(default = 600)]
    height: u32,
) -> impl IntoView {
    let viewer_id = VIEWER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    // Plain Rc shared across init effect + mouse handlers. WASM is
    // single-threaded so the RefCell borrows are race-free.
    let renderer: Rc<RefCell<Option<Rc<RefCell<DrawL3d>>>>> = Rc::new(RefCell::new(None));
    let is_dragging = Rc::new(RefCell::new(false));
    let last_pos: Rc<RefCell<(f32, f32)>> = Rc::new(RefCell::new((0.0, 0.0)));

    // First mount: stagger renderer creation per-instance so multiple
    // panels don't fight for GPU resources on first mount.
    //
    // Subsequent updates to `bytes` (e.g. user edits a `HousingColor`
    // → the L3D's synthesized MTL is rewritten with a new RAL colour →
    // `embedded_files[file_id]` changes → this component re-renders
    // with new bytes) reuse the existing `DrawL3d` and just call
    // `set_model` again. That keeps the WebGL context, camera/orbit
    // state, and render loop alive across edits — no flicker, no
    // context-loss risk.
    {
        let bytes_first = bytes.clone();
        let renderer = renderer.clone();
        Effect::new(move |prev: Option<bool>| {
            if prev.is_some() {
                return true;
            }
            let bytes = bytes_first.clone();
            let canvas_ref = canvas_ref;
            let renderer = renderer.clone();
            spawn_local(async move {
                let delay = 100 + (viewer_id * 50) as i32;
                delay_ms(delay).await;
                let Some(canvas_el) = canvas_ref.get_untracked() else {
                    log::warn!("[L3D-{}] canvas element not mounted", viewer_id);
                    return;
                };
                let canvas: web_sys::HtmlCanvasElement = match (*canvas_el).clone().dyn_into() {
                    Ok(c) => c,
                    Err(_) => return,
                };
                let l3d = l3d_rs::from_buffer(&bytes);
                if l3d.model.parts.is_empty() {
                    log::warn!("[L3D-{}] no model parts", viewer_id);
                    return;
                }
                match DrawL3d::create(canvas) {
                    Ok(mut draw_l3d) => {
                        draw_l3d.set_model(&l3d);
                        let rc = Rc::new(RefCell::new(draw_l3d));
                        *renderer.borrow_mut() = Some(rc.clone());
                        start_render_loop(rc);
                        log::info!("[L3D-{}] initialised", viewer_id);
                    }
                    Err(e) => log::error!("[L3D-{}] DrawL3d::create failed: {}", viewer_id, e),
                }
            });
            true
        });
    }

    // Hot-reload: every time `bytes` changes (new component prop value),
    // re-parse the L3D and push the new model into the existing
    // renderer. Skips the first run — the mount effect above handles
    // that. If the renderer hasn't booted yet (slow mount), this no-ops
    // and the mount effect will pick up the latest `bytes` when it runs.
    {
        let renderer = renderer.clone();
        Effect::new(move |prev: Option<()>| {
            // Read bytes so Leptos tracks the prop's identity for
            // subsequent re-renders. Leptos passes props by value;
            // when the parent updates `bytes`, this closure re-runs.
            let new_bytes = bytes.clone();
            if prev.is_none() {
                return;
            }
            let Some(rc) = renderer.borrow().clone() else {
                log::debug!(
                    "[L3D-{}] hot-reload skipped: renderer not yet initialised",
                    viewer_id
                );
                return;
            };
            let l3d = l3d_rs::from_buffer(&new_bytes);
            if l3d.model.parts.is_empty() {
                log::warn!("[L3D-{}] hot-reload: no model parts", viewer_id);
                return;
            }
            rc.borrow_mut().set_model(&l3d);
            log::info!("[L3D-{}] hot-reloaded after bytes change", viewer_id);
        });
    }

    // Mouse handlers — orbit (drag) and zoom (wheel).
    let on_mouse_down = {
        let is_dragging = is_dragging.clone();
        let last_pos = last_pos.clone();
        move |e: web_sys::MouseEvent| {
            *is_dragging.borrow_mut() = true;
            *last_pos.borrow_mut() = (e.client_x() as f32, e.client_y() as f32);
        }
    };
    let on_mouse_up = {
        let is_dragging = is_dragging.clone();
        move |_: web_sys::MouseEvent| {
            *is_dragging.borrow_mut() = false;
        }
    };
    let on_mouse_move = {
        let is_dragging = is_dragging.clone();
        let last_pos = last_pos.clone();
        let renderer = renderer.clone();
        move |e: web_sys::MouseEvent| {
            if !*is_dragging.borrow() {
                return;
            }
            let (lx, ly) = *last_pos.borrow();
            let dx = e.client_x() as f32 - lx;
            let dy = e.client_y() as f32 - ly;
            *last_pos.borrow_mut() = (e.client_x() as f32, e.client_y() as f32);
            if let Some(rc) = renderer.borrow().clone() {
                rc.borrow_mut().orbit((dx, dy));
            }
        }
    };
    let on_mouse_leave = {
        let is_dragging = is_dragging.clone();
        move |_: web_sys::MouseEvent| {
            *is_dragging.borrow_mut() = false;
        }
    };
    let on_wheel = {
        let renderer = renderer.clone();
        move |e: web_sys::WheelEvent| {
            e.prevent_default();
            if let Some(rc) = renderer.borrow().clone() {
                let dy = e.delta_y() as f32 * 0.01;
                rc.borrow_mut().zoom((0.0, dy));
            }
        }
    };

    // Account for device pixel ratio — canvas backing store is dpr× the CSS size.
    let dpr = web_sys::window()
        .map(|w| w.device_pixel_ratio())
        .unwrap_or(1.0);
    let canvas_w = (width as f64 * dpr) as u32;
    let canvas_h = (height as f64 * dpr) as u32;
    let canvas_style = format!("width: {}px; height: {}px; cursor: grab;", width, height);

    view! {
        <div class="l3d-viewer">
            <canvas
                node_ref=canvas_ref
                width=canvas_w.to_string()
                height=canvas_h.to_string()
                style=canvas_style
                on:mousedown=on_mouse_down
                on:mouseup=on_mouse_up
                on:mousemove=on_mouse_move
                on:mouseleave=on_mouse_leave
                on:wheel=on_wheel
            />
            <p style="font-size: 12px; color: var(--text-tertiary); margin-top: 8px;">
                { "Drag to rotate • Scroll to zoom" }
            </p>
        </div>
    }
}
