//! L3D 3D viewer component

use yew::prelude::*;
use gloo::console::log;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::draw_l3d::{DrawL3d, start_render_loop};

/// Global counter for unique viewer IDs
static VIEWER_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Simple async delay using Promise
async fn delay_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .unwrap();
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[derive(Properties, PartialEq)]
pub struct L3dViewerProps {
    pub l3d_data: Vec<u8>,
    #[prop_or(600)]
    pub width: u32,
    #[prop_or(600)]
    pub height: u32,
}

#[function_component(L3dViewer)]
pub fn l3d_viewer(props: &L3dViewerProps) -> Html {
    // Generate unique ID for this viewer instance
    let viewer_id = use_state(|| VIEWER_COUNTER.fetch_add(1, Ordering::SeqCst));

    log!(format!("[L3D-{}] Component rendering, data size: {} bytes", *viewer_id, props.l3d_data.len()));

    let canvas_ref = use_node_ref();
    let renderer_ref = use_state(|| None::<Rc<RefCell<DrawL3d>>>);
    let is_dragging = use_state(|| false);
    let last_pos = use_state(|| (0.0f32, 0.0f32));
    let initialized = use_state(|| false);

    // Initialize renderer when canvas is mounted AND we have data
    {
        let canvas_ref = canvas_ref.clone();
        let renderer_ref = renderer_ref.clone();
        let l3d_data = props.l3d_data.clone();
        let viewer_id = *viewer_id;
        let initialized = initialized.clone();

        // Use l3d_data length as dependency to re-trigger if data changes
        let data_len = props.l3d_data.len();

        use_effect_with(data_len, move |_| {
            log!(format!("[L3D-{}] Effect triggered, data_len: {}, initialized: {}", viewer_id, data_len, *initialized));

            // Only initialize if we have data and haven't initialized yet
            if data_len > 0 && !*initialized {
                let canvas_ref = canvas_ref.clone();
                let renderer_ref = renderer_ref.clone();
                let l3d_data = l3d_data.clone();
                let initialized = initialized.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    // Stagger delay based on viewer ID to avoid resource contention
                    let delay = 100 + (viewer_id * 50) as i32;
                    log!(format!("[L3D-{}] Waiting {}ms before init", viewer_id, delay));
                    delay_ms(delay).await;

                    log!(format!("[L3D-{}] Attempting to get canvas", viewer_id));
                    if let Some(canvas) = canvas_ref.cast::<web_sys::HtmlCanvasElement>() {
                        log!(format!("[L3D-{}] Canvas found: {}x{}", viewer_id, canvas.width(), canvas.height()));

                        // Parse L3D data
                        log!(format!("[L3D-{}] Parsing L3D data, {} bytes", viewer_id, l3d_data.len()));
                        log!(format!("[L3D-{}] First 20 bytes: {:?}", viewer_id, &l3d_data[..20.min(l3d_data.len())]));

                        let l3d = l3d_rs::from_buffer(&l3d_data);

                        log!(format!("[L3D-{}] L3D parsed successfully", viewer_id));
                        log!(format!("[L3D-{}] - model.parts.len() = {}", viewer_id, l3d.model.parts.len()));
                        log!(format!("[L3D-{}] - file.assets.len() = {}", viewer_id, l3d.file.assets.len()));

                        // Log each part
                        for (i, part) in l3d.model.parts.iter().enumerate() {
                            log!(format!("[L3D-{}] - Part {}: path={}", viewer_id, i, part.path));
                        }

                        if !l3d.model.parts.is_empty() {
                            // Create renderer
                            log!(format!("[L3D-{}] Creating DrawL3d renderer...", viewer_id));
                            match DrawL3d::create(canvas) {
                                Ok(mut draw_l3d) => {
                                    log!(format!("[L3D-{}] DrawL3d created successfully", viewer_id));
                                    draw_l3d.set_model(&l3d);
                                    log!(format!("[L3D-{}] Model set on renderer", viewer_id));

                                    let renderer = Rc::new(RefCell::new(draw_l3d));
                                    renderer_ref.set(Some(renderer.clone()));

                                    // Start render loop
                                    log!(format!("[L3D-{}] Starting render loop", viewer_id));
                                    start_render_loop(renderer);

                                    initialized.set(true);
                                    log!(format!("[L3D-{}] Initialization complete!", viewer_id));
                                }
                                Err(e) => {
                                    log!(format!("[L3D-{}] Failed to create renderer: {}", viewer_id, e));
                                }
                            }
                        } else {
                            log!(format!("[L3D-{}] No model parts found in L3D file", viewer_id));
                        }
                    } else {
                        log!(format!("[L3D-{}] Canvas element not found!", viewer_id));
                    }
                });
            }

            || {}
        });
    }

    // Mouse event handlers for orbit control
    let on_mouse_down = {
        let is_dragging = is_dragging.clone();
        let last_pos = last_pos.clone();
        Callback::from(move |e: MouseEvent| {
            is_dragging.set(true);
            last_pos.set((e.client_x() as f32, e.client_y() as f32));
        })
    };

    let on_mouse_up = {
        let is_dragging = is_dragging.clone();
        Callback::from(move |_: MouseEvent| {
            is_dragging.set(false);
        })
    };

    let on_mouse_move = {
        let is_dragging = is_dragging.clone();
        let last_pos = last_pos.clone();
        let renderer_ref = renderer_ref.clone();
        Callback::from(move |e: MouseEvent| {
            if *is_dragging {
                let (lx, ly) = *last_pos;
                let dx = e.client_x() as f32 - lx;
                let dy = e.client_y() as f32 - ly;
                last_pos.set((e.client_x() as f32, e.client_y() as f32));

                if let Some(ref renderer) = *renderer_ref {
                    renderer.borrow_mut().orbit((dx, dy));
                }
            }
        })
    };

    let on_mouse_leave = {
        let is_dragging = is_dragging.clone();
        Callback::from(move |_: MouseEvent| {
            is_dragging.set(false);
        })
    };

    let on_wheel = {
        let renderer_ref = renderer_ref.clone();
        Callback::from(move |e: WheelEvent| {
            e.prevent_default();
            if let Some(ref renderer) = *renderer_ref {
                let delta_y = e.delta_y() as f32 * 0.01;
                renderer.borrow_mut().zoom((0.0, delta_y));
            }
        })
    };

    // Account for device pixel ratio
    let dpr = web_sys::window()
        .map(|w| w.device_pixel_ratio())
        .unwrap_or(1.0);
    let canvas_width = (props.width as f64 * dpr) as u32;
    let canvas_height = (props.height as f64 * dpr) as u32;

    html! {
        <div class="l3d-viewer">
            <canvas
                ref={canvas_ref}
                width={canvas_width.to_string()}
                height={canvas_height.to_string()}
                style={format!("width: {}px; height: {}px; cursor: grab;", props.width, props.height)}
                onmousedown={on_mouse_down}
                onmouseup={on_mouse_up}
                onmousemove={on_mouse_move}
                onmouseleave={on_mouse_leave}
                onwheel={on_wheel}
            />
            <p class="text-sm text-gray-400 mt-2">
                {"Drag to rotate • Scroll to zoom"}
            </p>
        </div>
    }
}
