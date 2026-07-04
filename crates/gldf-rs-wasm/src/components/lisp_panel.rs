//! Floating Lisp REPL overlay.
//!
//! Mirrors the bimifc-leptos `lisp_panel` pattern: a `thread_local!`
//! `RefCell<Interpreter>` plus a Leptos component with a textarea +
//! Run button + output pane. The interpreter is pre-loaded with
//! gldf-rs-lisp's full function set, AND its `SceneBridge` is wired
//! to the existing `gldf-bevy` `#[wasm_bindgen]` camera/joint entries
//! so a script that says
//!
//!     (CAMERA-SET 0.5 0.7 1.2 0.0 0.0 0.0)
//!     (LUMINAIRE-TILT "l3d://LEO0" 20.0 0.0 0.0)
//!
//! actually moves the camera and tilts the inner ring on the Bevy
//! canvas overlaid on the main editor.
//!
//! The panel is *floating*, not routed — opening it via a toolbar
//! button keeps cross-page changes to a minimum (no NavItem variant,
//! no match arm sprawl). Visible state lives in a Leptos signal
//! threaded down from the shell.

use leptos::prelude::*;
use std::cell::RefCell;
use wasm_bindgen::JsValue;

use gldf_rs_lisp::{create_gldf_interpreter, GldfState, SceneCommand};

thread_local! {
    /// One interpreter per browser tab, lives for the page session.
    /// Same pattern as bimifc-leptos's INTERP.
    static INTERP: RefCell<acadlisp::Interpreter> = RefCell::new(make_interp());

    /// Outbox for SceneCommands dispatched during a single `run`. The
    /// bridge appends to this; after the synchronous interpreter
    /// returns, `drain_outbox_async` walks the queue and forwards each
    /// command to JS, inserting setTimeout pauses for `Sleep` entries.
    /// That's what makes `(SLEEP 1500)` between two variant swaps
    /// actually wait — the synchronous Lisp dispatcher can't pause
    /// itself, but the JS-side drainer can.
    static OUTBOX: RefCell<Vec<SceneCommand>> = const { RefCell::new(Vec::new()) };

    /// Set by the Stop button to abort the in-flight drain chain. Each
    /// scheduled `drain_step` checks this before forwarding the next
    /// command, so pressing Stop cuts the choreography on the next
    /// micro-yield boundary (≤ a few ms after click + any in-flight
    /// `SLEEP` shortens to its current `setTimeout`'s remaining time).
    /// Cleared at the start of each new `run`.
    static STOP_REQUESTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };

    /// Editor-authoring commands collected during a run, in dispatch
    /// order. Unlike the 3D `SceneCommand`s (which forward to the Bevy
    /// canvas via `window.*`), `EditorAction` / `WizardApply` /
    /// `NeedFiles` mutate the live `GLDF_CTX` document. They can't fire
    /// from the bridge closure (it has no handle to the Leptos signal),
    /// so the bridge stashes them here and the `on_run` handler drains
    /// them after the synchronous interpreter run, resolving `@file`
    /// byte-refs against the user's dropped files first.
    static EDITOR_OUTBOX: RefCell<Vec<SceneCommand>> = const { RefCell::new(Vec::new()) };
}

/// Public: pressed-the-Stop-button entry. Sets the abort flag so the
/// next scheduled `drain_step` bails. Camera/tilt animations already in
/// the Bevy queue will still play out their remaining duration — that's
/// a separate animation system in `gldf-bevy/src/lib.rs`. Call
/// `gldf_bevy_reset_tilts` for a hard stop on tilts.
fn request_stop() {
    STOP_REQUESTED.with(|f| f.set(true));
    // Cancel any pending tilt animations on the Bevy side so the user
    // gets a hard stop, not just "no more new commands". Camera glide
    // is harder to stop mid-flight; it'll fade to its current target.
    call_window_fn("gldf_bevy_reset_tilts", &[]);
}

/// Call a `window.<name>(...args)` JS global. Returns silently if
/// `window` is missing or the function isn't there yet — typical when
/// the Bevy canvas hasn't been mounted yet. The script keeps running.
fn call_window_fn(name: &str, args: &[JsValue]) {
    let Some(win) = web_sys::window() else {
        return;
    };
    let Ok(prop) = js_sys::Reflect::get(&win, &JsValue::from_str(name)) else {
        return;
    };
    let Ok(func) = prop.dyn_into::<js_sys::Function>() else {
        return;
    };
    let arr = js_sys::Array::new();
    for a in args {
        arr.push(a);
    }
    let _ = func.apply(&JsValue::NULL, &arr);
}

use wasm_bindgen::JsCast;

/// Fetch a bundled asset (copied into `dist/` via index.html
/// `copy-file`) by relative name, resolving against the page URL so it
/// works on gldf.icu, the :8052 dev server, and offline file:// bundles
/// alike. Mirrors `main.rs::fetch_bytes` (private there). Used by the λ
/// panel to auto-load the demo source files the Prolumia build needs,
/// so the demo runs with one ▶ Run and no manual file-drop.
async fn fetch_bytes(path: &str) -> Result<Vec<u8>, String> {
    let url = if path.starts_with("http://")
        || path.starts_with("https://")
        || path.starts_with("file://")
    {
        path.to_string()
    } else if let Some(base) = web_sys::window().and_then(|w| w.location().href().ok()) {
        match web_sys::Url::new_with_base(path, &base) {
            Ok(u) => u.href(),
            Err(_) => path.to_string(),
        }
    } else {
        path.to_string()
    };
    let resp = reqwest_wasm::get(&url).await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    Ok(bytes.to_vec())
}

/// Build a fresh interpreter with the Bevy bridge wired.
///
/// The gldf-bevy `#[wasm_bindgen]` entry points live in a SEPARATE
/// WASM module that bevy-loader.js loads on demand — they're not
/// linkable from this crate. So instead of calling them as Rust
/// functions, we look them up as `window.gldf_bevy_set_camera` /
/// `window.gldf_bevy_luminaire_tilt` and invoke them through
/// `js_sys::Reflect`. When the Bevy module isn't loaded yet (user
/// hasn't visited the 3D viewer page), the calls are no-ops and the
/// script continues — the SceneCommand stays in the engine's log so
/// the user can scroll back and replay.
/// Forward one SceneCommand to its window-side JS function. Pulled
/// out of the bridge closure so `drain_outbox_async` can reuse it.
fn forward_command(cmd: &SceneCommand) {
    match cmd {
        SceneCommand::CameraSet { eye, target } => {
            // Bevy's Camera3d looks down its local -Z by default, so
            // `Transform::forward()` returns the negated Z axis after
            // any yaw/pitch rotation. We have `(eye, target)` in world
            // space and need yaw/pitch such that the rotated forward
            // points from eye toward target.
            //
            // Decomposition (EulerRot::YXZ, yaw around Y, pitch around X):
            //   forward = ( -sin(yaw)·cos(pitch),  sin(pitch),  -cos(yaw)·cos(pitch) )
            //
            // For a desired forward `n = (target - eye).normalize()`:
            //   pitch = asin(n.y)
            //   yaw   = atan2(-n.x, -n.z)
            //
            // (The previous formula used `atan2(n.x, n.z)` which assumed a
            // +Z-forward camera — flipping yaw by 180° and producing the
            // mirror-image of the requested look direction.)
            let dir = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
            let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
            let (yaw, pitch) = if len > 0.0001 {
                let n = [dir[0] / len, dir[1] / len, dir[2] / len];
                ((-n[0]).atan2(-n[2]), n[1].asin())
            } else {
                (0.0, 0.0)
            };
            call_window_fn(
                "gldf_bevy_set_camera",
                &[
                    JsValue::from_f64(eye[0] as f64),
                    JsValue::from_f64(eye[1] as f64),
                    JsValue::from_f64(eye[2] as f64),
                    JsValue::from_f64(yaw as f64),
                    JsValue::from_f64(pitch as f64),
                ],
            );
        }
        SceneCommand::LuminaireTilt {
            joint_path,
            euler_deg,
        } => {
            let bare = joint_path
                .strip_prefix("l3d://")
                .unwrap_or(joint_path.as_str());
            call_window_fn(
                "gldf_bevy_luminaire_tilt",
                &[
                    JsValue::from_str(bare),
                    JsValue::from_f64(euler_deg[0] as f64),
                    JsValue::from_f64(euler_deg[1] as f64),
                    JsValue::from_f64(euler_deg[2] as f64),
                ],
            );
        }
        SceneCommand::CameraAnimate {
            eye,
            target,
            duration_ms,
        } => {
            call_window_fn(
                "gldf_bevy_camera_animate",
                &[
                    JsValue::from_f64(eye[0] as f64),
                    JsValue::from_f64(eye[1] as f64),
                    JsValue::from_f64(eye[2] as f64),
                    JsValue::from_f64(target[0] as f64),
                    JsValue::from_f64(target[1] as f64),
                    JsValue::from_f64(target[2] as f64),
                    JsValue::from_f64(*duration_ms as f64),
                ],
            );
        }
        SceneCommand::LuminaireAnimate {
            joint_path,
            euler_deg,
            duration_ms,
        } => {
            let bare = joint_path
                .strip_prefix("l3d://")
                .unwrap_or(joint_path.as_str());
            call_window_fn(
                "gldf_bevy_luminaire_animate",
                &[
                    JsValue::from_str(bare),
                    JsValue::from_f64(euler_deg[0] as f64),
                    JsValue::from_f64(euler_deg[1] as f64),
                    JsValue::from_f64(euler_deg[2] as f64),
                    JsValue::from_f64(*duration_ms as f64),
                ],
            );
        }
        SceneCommand::SetActiveVariant { index } => {
            call_window_fn(
                "gldf_set_active_variant",
                &[JsValue::from_f64(*index as f64)],
            );
        }
        SceneCommand::ShowView3d => {
            // Switch the editor to the 3D Design view; the Bevy canvas
            // mounts on demand. The build-then-animate demo follows this
            // with a SLEEP so the canvas has time to load before the
            // first camera/tilt command.
            call_window_fn("gldf_nav_3d", &[]);
        }
        SceneCommand::Sleep { .. } => {
            // Sleep is handled by the drainer's setTimeout chain, not
            // by forward_command itself — it must never reach here
            // outside a drained run.
        }
        SceneCommand::ResetTilts => {
            // Snap every LightEmitterJoint back to its as-loaded
            // base rotation; cancels in-flight tilt animations.
            // Wired through to `gldf_bevy_reset_tilts` in
            // `crates/gldf-bevy/src/lib.rs`.
            call_window_fn("gldf_bevy_reset_tilts", &[]);
        }
        SceneCommand::SetAmbient { brightness } => {
            call_window_fn(
                "gldf_bevy_set_ambient",
                &[JsValue::from_f64(*brightness as f64)],
            );
        }
        SceneCommand::SetRoomSize {
            width,
            length,
            height,
        } => {
            call_window_fn(
                "gldf_bevy_set_room_size",
                &[
                    JsValue::from_f64(*width as f64),
                    JsValue::from_f64(*length as f64),
                    JsValue::from_f64(*height as f64),
                ],
            );
        }
        SceneCommand::SetSurfaceColor { surface, r, g, b } => {
            call_window_fn(
                "gldf_bevy_set_surface_color",
                &[
                    JsValue::from_str(surface_name(*surface)),
                    JsValue::from_f64(*r as f64),
                    JsValue::from_f64(*g as f64),
                    JsValue::from_f64(*b as f64),
                ],
            );
        }
        SceneCommand::SetSurfaceReflectance { surface, value } => {
            call_window_fn(
                "gldf_bevy_set_surface_reflectance",
                &[
                    JsValue::from_str(surface_name(*surface)),
                    JsValue::from_f64(*value as f64),
                ],
            );
        }
        SceneCommand::SetSurfaceEmissive { surface, r, g, b } => {
            call_window_fn(
                "gldf_bevy_set_surface_emissive",
                &[
                    JsValue::from_str(surface_name(*surface)),
                    JsValue::from_f64(*r as f64),
                    JsValue::from_f64(*g as f64),
                    JsValue::from_f64(*b as f64),
                ],
            );
        }
        // Editor-authoring commands never reach the Bevy bridge. They
        // mutate the live GLDF document and are drained separately by
        // `apply_editor_outbox` after the run (which has the Leptos
        // signal + dropped-file map). The bridge closure routes them
        // into EDITOR_OUTBOX instead of here; this arm exists only so
        // `forward_command` stays exhaustive.
        SceneCommand::EditorAction { .. }
        | SceneCommand::WizardApply { .. }
        | SceneCommand::NeedFiles { .. } => {}
    }
}

/// Convert a `RoomSurface` tag into the string label the Bevy bridge
/// matches on. Keeping this in one place means the JS-side dispatch
/// table never has to learn about Rust enums.
fn surface_name(surface: gldf_rs_lisp::RoomSurface) -> &'static str {
    use gldf_rs_lisp::RoomSurface::*;
    match surface {
        Walls => "walls",
        Floor => "floor",
        Ceiling => "ceiling",
    }
}

/// Drain the OUTBOX after `interp.run()` completes. Walks commands in
/// dispatch order; for each non-Sleep command, calls `forward_command`
/// immediately; for `Sleep`, schedules the remainder via setTimeout so
/// the next dispatch happens AFTER the requested ms have elapsed.
///
/// This is what makes `(SLEEP 1500)` between two `(GLDF-VARIANT-SET …)`s
/// actually wait — the synchronous Lisp dispatcher writes Sleep into
/// the OUTBOX, and the drainer turns it into a real pause on the JS
/// event loop.
fn drain_outbox_async(on_done: std::rc::Rc<dyn Fn()>) {
    STOP_REQUESTED.with(|f| f.set(false));
    let commands: Vec<SceneCommand> = OUTBOX.with(|c| std::mem::take(&mut *c.borrow_mut()));
    drain_step(commands, 0, on_done);
}

fn drain_step(commands: Vec<SceneCommand>, idx: usize, on_done: std::rc::Rc<dyn Fn()>) {
    if STOP_REQUESTED.with(|f| f.get()) || idx >= commands.len() {
        on_done();
        return;
    }
    // Pick the wait-duration before yielding to the next command.
    // `Sleep` is obvious. `ANIMATE-CAMERA` / `ANIMATE-TILT` implicitly
    // block for their `duration_ms` — without this, the drainer reports
    // the script "done" before the animations even start (the queue
    // walks past them in microseconds), so the Stop button flips back
    // to Run almost immediately and total runtime is ≈ sum(SLEEP) only.
    // With this, a script reading "tilt 2s, sleep 0.5s, tilt 2s" takes
    // ~4.5 s wall-clock as the user expects.
    let (wait_ms, forward) = match &commands[idx] {
        SceneCommand::Sleep { duration_ms } => ((*duration_ms).max(0.0) as i32, false),
        SceneCommand::CameraAnimate { duration_ms, .. }
        | SceneCommand::LuminaireAnimate { duration_ms, .. } => {
            ((*duration_ms).max(0.0) as i32, true)
        }
        _ => (0, true),
    };
    if forward {
        forward_command(&commands[idx]);
    }
    let Some(win) = web_sys::window() else {
        on_done();
        return;
    };
    let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
        drain_step(commands, idx + 1, on_done);
    });
    if let Ok(func) = cb.dyn_into::<js_sys::Function>() {
        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(&func, wait_ms);
    }
}

fn make_interp() -> acadlisp::Interpreter {
    let mut interp = create_gldf_interpreter();
    if let Some(state) = interp.user_data.downcast_mut::<GldfState>() {
        // Hybrid dispatch model:
        //
        //   * "Stateless" room tweaks (ambient, room-size, surface
        //     colour/reflectance) and camera snaps (CAMERA-SET,
        //     LUMINAIRE-TILT, ResetTilts) fire IMMEDIATELY via
        //     `forward_command`. They're cheap, don't race anything,
        //     and the script's GET- companions need to see the new
        //     value on the very next line.
        //
        //   * Animation and variant-swap commands (ANIMATE-CAMERA,
        //     ANIMATE-TILT, GLDF-VARIANT-SET, Sleep) go into OUTBOX
        //     and are drained by `drain_outbox_async` through the
        //     setTimeout chain — so animations actually pause for
        //     their `duration_ms`, sleeps actually sleep, and variant
        //     swaps don't trample each other's canvas reloads.
        //
        // Without this split, every `(SET-AMBIENT 100) (GET-AMBIENT)`
        // pair returned the OLD value because the SET hadn't flushed
        // yet.
        state.bridge = Some(Box::new(|cmd| match &cmd {
            SceneCommand::SetAmbient { .. }
            | SceneCommand::SetRoomSize { .. }
            | SceneCommand::SetSurfaceColor { .. }
            | SceneCommand::SetSurfaceReflectance { .. }
            | SceneCommand::SetSurfaceEmissive { .. }
            | SceneCommand::CameraSet { .. }
            | SceneCommand::LuminaireTilt { .. }
            | SceneCommand::ResetTilts => {
                forward_command(&cmd);
            }
            SceneCommand::EditorAction { .. }
            | SceneCommand::WizardApply { .. }
            | SceneCommand::NeedFiles { .. } => {
                // Editor-authoring commands mutate the live document, not
                // the Bevy canvas. Collect them in dispatch order; the
                // `on_run` handler resolves `@file` refs against dropped
                // files and applies them to GLDF_CTX after the run.
                EDITOR_OUTBOX.with(|c| c.borrow_mut().push(cmd));
            }
            _ => {
                OUTBOX.with(|c| c.borrow_mut().push(cmd));
            }
        }));
        register_room_getters(&mut interp);
        return interp;
    }
    interp
}

/// Register GET- foreign-fns on the interpreter. These can't live in
/// `gldf-rs-lisp` because the lib crate has no way to call JS — the
/// snapshot cells holding the live room state live inside the Bevy
/// WASM bundle and are reachable only through `window.gldf_bevy_get_*`.
/// The setters in `gldf-rs-lisp` drop SceneCommands into OUTBOX so the
/// dispatch is asynchronous; getters need a synchronous return value
/// inside the foreign-fn, so they call JS directly here.
///
/// All eight getters take no arguments and return Real / List of
/// Reals. If the Bevy bundle hasn't loaded yet (user hasn't visited
/// the 3D viewer page), the JS function is missing on `window` and
/// the getter returns Nil — the script keeps running.
fn register_room_getters(interp: &mut acadlisp::Interpreter) {
    use acadlisp::{Expr, ForeignContext};

    /// Call `window.<fn_name>()` with no arguments and return the
    /// result. Returns `JsValue::NULL` on any failure.
    fn call_window_get(fn_name: &str) -> wasm_bindgen::JsValue {
        let Some(win) = web_sys::window() else {
            return wasm_bindgen::JsValue::NULL;
        };
        let Ok(prop) = js_sys::Reflect::get(&win, &wasm_bindgen::JsValue::from_str(fn_name))
        else {
            return wasm_bindgen::JsValue::NULL;
        };
        let Ok(func) = prop.dyn_into::<js_sys::Function>() else {
            return wasm_bindgen::JsValue::NULL;
        };
        func.call0(&wasm_bindgen::JsValue::NULL)
            .unwrap_or(wasm_bindgen::JsValue::NULL)
    }

    /// Same but with one string argument (used for the surface getter).
    fn call_window_get_with_str(fn_name: &str, arg: &str) -> wasm_bindgen::JsValue {
        let Some(win) = web_sys::window() else {
            return wasm_bindgen::JsValue::NULL;
        };
        let Ok(prop) = js_sys::Reflect::get(&win, &wasm_bindgen::JsValue::from_str(fn_name))
        else {
            return wasm_bindgen::JsValue::NULL;
        };
        let Ok(func) = prop.dyn_into::<js_sys::Function>() else {
            return wasm_bindgen::JsValue::NULL;
        };
        func.call1(
            &wasm_bindgen::JsValue::NULL,
            &wasm_bindgen::JsValue::from_str(arg),
        )
        .unwrap_or(wasm_bindgen::JsValue::NULL)
    }

    /// Pluck a named f64 from a JS object via Reflect::get, returning
    /// 0.0 on miss. Used by the JSON-blob getters.
    fn get_f64(obj: &wasm_bindgen::JsValue, key: &str) -> f64 {
        js_sys::Reflect::get(obj, &wasm_bindgen::JsValue::from_str(key))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    }

    // (GET-AMBIENT) — returns the current brightness as a Real.
    interp.register_fn("GET-AMBIENT", |_ctx: &mut ForeignContext, _args: &[Expr]| {
        let v = call_window_get("gldf_bevy_get_ambient");
        v.as_f64()
            .map(|f| Expr::Real(f))
            .unwrap_or(Expr::Nil)
    });

    // (GET-ROOM-SIZE) — returns a list (width length height) in metres.
    interp.register_fn(
        "GET-ROOM-SIZE",
        |_ctx: &mut ForeignContext, _args: &[Expr]| {
            let obj = call_window_get("gldf_bevy_get_room_size");
            if obj.is_null() || obj.is_undefined() {
                return Expr::Nil;
            }
            Expr::List(vec![
                Expr::Real(get_f64(&obj, "width")),
                Expr::Real(get_f64(&obj, "length")),
                Expr::Real(get_f64(&obj, "height")),
            ])
        },
    );

    /// Build a (GET-{surface}-COLOR) closure that returns (r g b).
    fn make_color_getter(
        surface: &'static str,
    ) -> impl Fn(&mut acadlisp::ForeignContext, &[Expr]) -> Expr + 'static {
        move |_ctx: &mut acadlisp::ForeignContext, _args: &[Expr]| {
            let obj = call_window_get_with_str("gldf_bevy_get_surface", surface);
            if obj.is_null() || obj.is_undefined() {
                return Expr::Nil;
            }
            Expr::List(vec![
                Expr::Real(get_f64(&obj, "r")),
                Expr::Real(get_f64(&obj, "g")),
                Expr::Real(get_f64(&obj, "b")),
            ])
        }
    }
    interp.register_fn("GET-WALLS-COLOR", make_color_getter("walls"));
    interp.register_fn("GET-FLOOR-COLOR", make_color_getter("floor"));
    interp.register_fn("GET-CEILING-COLOR", make_color_getter("ceiling"));

    /// Build a (GET-{surface}-REFLECTANCE) closure that returns a Real.
    fn make_reflectance_getter(
        surface: &'static str,
    ) -> impl Fn(&mut acadlisp::ForeignContext, &[Expr]) -> Expr + 'static {
        move |_ctx: &mut acadlisp::ForeignContext, _args: &[Expr]| {
            let obj = call_window_get_with_str("gldf_bevy_get_surface", surface);
            if obj.is_null() || obj.is_undefined() {
                return Expr::Nil;
            }
            Expr::Real(get_f64(&obj, "reflectance"))
        }
    }
    interp.register_fn("GET-WALLS-REFLECTANCE", make_reflectance_getter("walls"));
    interp.register_fn("GET-FLOOR-REFLECTANCE", make_reflectance_getter("floor"));
    interp.register_fn(
        "GET-CEILING-REFLECTANCE",
        make_reflectance_getter("ceiling"),
    );

    /// Build a (GET-{surface}-EMISSIVE) closure returning (er eg eb).
    fn make_emissive_getter(
        surface: &'static str,
    ) -> impl Fn(&mut acadlisp::ForeignContext, &[Expr]) -> Expr + 'static {
        move |_ctx: &mut acadlisp::ForeignContext, _args: &[Expr]| {
            let obj = call_window_get_with_str("gldf_bevy_get_surface", surface);
            if obj.is_null() || obj.is_undefined() {
                return Expr::Nil;
            }
            Expr::List(vec![
                Expr::Real(get_f64(&obj, "er")),
                Expr::Real(get_f64(&obj, "eg")),
                Expr::Real(get_f64(&obj, "eb")),
            ])
        }
    }
    interp.register_fn("GET-WALLS-EMISSIVE", make_emissive_getter("walls"));
    interp.register_fn("GET-FLOOR-EMISSIVE", make_emissive_getter("floor"));
    interp.register_fn("GET-CEILING-EMISSIVE", make_emissive_getter("ceiling"));

    // (GET-CAMERA) — returns (eye-x eye-y eye-z target-x target-y target-z).
    // The bevy bridge gives us {x,y,z,yaw,pitch,fx,fy,fz} — we derive
    // a unit-length target by `eye + forward * 2`. Matches what the
    // C-key recorder produces, so round-trips through `CAMERA-SET`.
    interp.register_fn("GET-CAMERA", |_ctx: &mut ForeignContext, _args: &[Expr]| {
        let obj = call_window_get("gldf_bevy_get_camera");
        if obj.is_null() || obj.is_undefined() {
            return Expr::Nil;
        }
        let ex = get_f64(&obj, "x");
        let ey = get_f64(&obj, "y");
        let ez = get_f64(&obj, "z");
        let fx = get_f64(&obj, "fx");
        let fy = get_f64(&obj, "fy");
        let fz = get_f64(&obj, "fz");
        Expr::List(vec![
            Expr::Real(ex),
            Expr::Real(ey),
            Expr::Real(ez),
            Expr::Real(ex + fx * 2.0),
            Expr::Real(ey + fy * 2.0),
            Expr::Real(ez + fz * 2.0),
        ])
    });
}

/// Run Lisp source against the thread-local interpreter and return
/// `(combined stdout, result line)`. The combined stdout is everything
/// the foreign functions pushed onto `ctx.output`; the result line is
/// the last non-Nil top-level expression, formatted via Display.
///
/// Before each Run, the editor's current `GldfProduct` is copied into
/// the interpreter's `GldfState` so query functions
/// (`GLDF-VARIANT-COUNT`, `GLDF-VARIANT-SET`, `GLDF-MANUFACTURER`, …)
/// see the same document the user is editing. Without this sync the
/// Lisp side starts empty and every `(GLDF-VARIANT-SET 1)` fails with
/// "out of range (0..0)".
fn run_lisp(
    code: &str,
    product: Option<gldf_rs::GldfProduct>,
) -> (String, String) {
    let (out, result) = INTERP.with(|cell| {
        let interp = &mut *cell.borrow_mut();
        if let Some(state) = interp.user_data.downcast_mut::<GldfState>() {
            state.product = product;
        }
        // Clear any leftovers from a previous run (e.g. a Sleep that
        // was still mid-chain when the user pressed Run again).
        OUTBOX.with(|c| c.borrow_mut().clear());
        let results = interp.run(code);
        let out = interp.output.join("");
        interp.output.clear();
        let result = results
            .iter()
            .filter(|r| **r != acadlisp::Expr::Nil)
            .map(|r| format!("{}", r))
            .collect::<Vec<_>>()
            .join("\n");
        (out, result)
    });
    // NOTE: the OUTBOX (3D scene commands) is NOT drained here. The
    // caller drains it via `drain_outbox_async` AFTER applying the
    // editor-authoring commands, so a build-then-animate script builds
    // the variants/geometry before the choreography (GLDF-VARIANT-SET /
    // ANIMATE-*) tries to drive them.
    (out, result)
}

/// Take the editor-authoring commands collected during the last run.
/// Drains EDITOR_OUTBOX so a re-run starts clean.
fn take_editor_commands() -> Vec<SceneCommand> {
    EDITOR_OUTBOX.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

/// Walk the editor commands and collect every `@file` name they
/// reference — both `EditorAction` byte-refs and `NeedFiles` manifest
/// entries. This is the full set the script needs the user to provide.
fn required_file_names(cmds: &[SceneCommand]) -> std::collections::BTreeSet<String> {
    let mut names = std::collections::BTreeSet::new();
    for cmd in cmds {
        match cmd {
            SceneCommand::NeedFiles { names: declared } => {
                names.extend(declared.iter().cloned());
            }
            SceneCommand::EditorAction { action_json }
            | SceneCommand::WizardApply {
                draft_json: action_json,
            } => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(action_json) {
                    collect_at_file_refs(&v, &mut names);
                }
            }
            _ => {}
        }
    }
    names
}

/// Recursively gather every `{"@file": "<name>"}` string in a JSON value.
fn collect_at_file_refs(
    value: &serde_json::Value,
    out: &mut std::collections::BTreeSet<String>,
) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(name)) = map.get("@file") {
                out.insert(name.clone());
            }
            for v in map.values() {
                collect_at_file_refs(v, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_at_file_refs(v, out);
            }
        }
        _ => {}
    }
}

/// Apply the collected editor commands to the live `GldfState`,
/// resolving `@file` byte-refs against `embedded ∪ picked`. Returns a
/// human-readable summary line for the output pane (errors included).
///
/// `EditorAction`s are batched into one JSON array and run through
/// `apply_script` so a single `MemoryResolver` resolves every `@file`;
/// `WizardApply` drafts are deserialized and applied via `apply_wizard`.
/// Commands run in their original dispatch order by splitting the
/// stream into contiguous runs of the same kind.
fn apply_editor_commands(
    state: &mut crate::state::GldfState,
    cmds: &[SceneCommand],
    picked: &std::collections::HashMap<String, Vec<u8>>,
) -> String {
    use gldf_rs_editor::{apply_script, apply_wizard, MemoryResolver, WizardDraft};

    // Resolver sees both the document's already-embedded files and the
    // freshly dropped ones. Clone+merge avoids borrowing `state` twice.
    let mut files = state.embedded_files.clone();
    for (k, v) in picked {
        files.insert(k.clone(), v.clone());
    }
    let resolver = MemoryResolver::new(&files);

    let mut actions_applied = 0usize;
    let mut wizards_applied = 0usize;
    let mut errors: Vec<String> = Vec::new();

    // Group consecutive EditorActions so each batch is one apply_script
    // call; WizardApply is applied individually in place.
    let mut i = 0;
    while i < cmds.len() {
        match &cmds[i] {
            SceneCommand::EditorAction { .. } => {
                let mut batch = Vec::new();
                while let Some(SceneCommand::EditorAction { action_json }) = cmds.get(i) {
                    match serde_json::from_str::<serde_json::Value>(action_json) {
                        Ok(v) => batch.push(v),
                        Err(e) => errors.push(format!("bad action JSON: {e}")),
                    }
                    i += 1;
                }
                let array = serde_json::Value::Array(batch.clone());
                let json = array.to_string();
                match apply_script(state, &json, &resolver) {
                    Ok(()) => actions_applied += batch.len(),
                    Err(e) => errors.push(e.to_string()),
                }
            }
            SceneCommand::WizardApply { draft_json } => {
                // Resolve `{"@file":"name"}` byte-refs (photometry_bytes /
                // geometry_bytes / …) against the dropped files BEFORE
                // deserializing — otherwise serde sees a map where it
                // expects a Vec<u8> sequence ("invalid type: map").
                match serde_json::from_str::<serde_json::Value>(draft_json) {
                    Ok(mut value) => {
                        if let Err(e) =
                            gldf_rs_editor::expand_file_refs(&mut value, &resolver)
                        {
                            errors.push(format!("wizard @file: {e}"));
                        } else {
                            match serde_json::from_value::<WizardDraft>(value) {
                                Ok(draft) => {
                                    apply_wizard(state, &draft);
                                    if let Some(err) = &state.last_error {
                                        errors.push(format!("wizard: {err}"));
                                    } else {
                                        wizards_applied += 1;
                                    }
                                }
                                Err(e) => errors.push(format!("bad wizard draft: {e}")),
                            }
                        }
                    }
                    Err(e) => errors.push(format!("bad wizard JSON: {e}")),
                }
                i += 1;
            }
            _ => {
                // NeedFiles + any non-editor command: skip.
                i += 1;
            }
        }
    }

    let mut summary = format!(
        "; editor: {actions_applied} action(s)"
    );
    if wizards_applied > 0 {
        summary.push_str(&format!(", {wizards_applied} wizard draft(s)"));
    }
    summary.push_str(" applied");
    if !errors.is_empty() {
        summary.push_str(&format!("\n; {} error(s):", errors.len()));
        for e in &errors {
            summary.push_str(&format!("\n;   {e}"));
        }
    }
    summary
}

/// Open a synchronous multi-file picker and call `on_files` with the
/// dropped files keyed by `file.name()`. MUST be called inside a user
/// gesture (the Run click) so the browser allows the dialog.
///
/// The picker is a transient `<input type=file multiple>` that we click
/// programmatically; its `change` event reads every selected `File` into
/// bytes (via the same `gloo::file` path the wizard/Files page use) and
/// then invokes `on_files`. If the user cancels (no `change` fires),
/// `on_files` is simply never called — the caller's run is abandoned,
/// which matches the "report missing files, don't half-apply" rule.
fn open_file_picker(on_files: impl Fn(std::collections::HashMap<String, Vec<u8>>) + 'static) {
    use wasm_bindgen::JsCast;
    let Some(win) = web_sys::window() else { return };
    let Some(doc) = win.document() else { return };
    let Ok(el) = doc.create_element("input") else {
        return;
    };
    let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() else {
        return;
    };
    input.set_type("file");
    let _ = input.set_attribute("multiple", "multiple");

    let input_for_cb = input.clone();
    let on_files = std::rc::Rc::new(on_files);
    let change = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
        let Some(files) = input_for_cb.files() else {
            return;
        };
        let n = files.length();
        let picked = std::rc::Rc::new(RefCell::new(
            std::collections::HashMap::<String, Vec<u8>>::new(),
        ));
        let remaining = std::rc::Rc::new(std::cell::Cell::new(n));
        if n == 0 {
            on_files(std::collections::HashMap::new());
            return;
        }
        for idx in 0..n {
            let Some(file) = files.get(idx) else {
                remaining.set(remaining.get().saturating_sub(1));
                continue;
            };
            let name = file.name();
            let gloo_file = gloo::file::File::from(file);
            let picked = picked.clone();
            let remaining = remaining.clone();
            let on_files = on_files.clone();
            leptos::task::spawn_local(async move {
                if let Ok(bytes) = gloo::file::futures::read_as_bytes(&gloo_file).await {
                    picked.borrow_mut().insert(name, bytes);
                }
                remaining.set(remaining.get().saturating_sub(1));
                if remaining.get() == 0 {
                    let map = std::mem::take(&mut *picked.borrow_mut());
                    on_files(map);
                }
            });
        }
    }) as Box<dyn FnMut()>);
    input.set_onchange(Some(change.as_ref().unchecked_ref()));
    change.forget();

    // A detached input is fine for opening the native file dialog in
    // every modern browser; clicking it inside the Run gesture is
    // gesture-legal. No need to attach (and hide) it in the DOM.
    let _ = doc; // keep the document handle alive through the closure setup
    input.click();
}

/// Default script shown when the panel is opened OUTSIDE the 3D design
/// view (Welcome / editor pages). Builds a luminaire from a file-drop —
/// no forms, no JSON.
const STARTER_SCRIPT_BUILD: &str = r#"; ═══════════════════════════════════════════════════════════════
; λ Lisp REPL — build a luminaire from a script + a file-drop
; ═══════════════════════════════════════════════════════════════
;
; Hit ▶ Run — that's it. The demo source files are bundled with the
; app, so the script fetches them automatically and builds the whole
; product (no forms, no JSON, no file-drop). If you run your own script
; referencing files that AREN'T bundled, you'll be asked to drop those.
; Open Overview / Files / Variants after the run to see the result.
;
; (Open this panel from the 3D design view instead to get the camera +
;  tilt animation script — as long as you haven't edited this one.)
;
; ── Header: every file this script needs (auto-fetched if bundled) ─
(NEEDS-FILES "middle.ldt" "narrow.ldt" "wide.ldt" "emergency.ldt"
             "sensor.ldt" "example_007.l3d" "spectrum_2700K.spd"
             "product_picture.jpg")

; ── Step 1 — build the core product (one call, fully XSD-valid) ──
; (GLDF-WIZARD manufacturer name author photometry geometry variant sensor emergency)
; Same builder as "+ Create new GLDF": mints the header id + creation
; time, a light source, an emitter bound to the photometry, a variant,
; the L3D geometry, a Sensor, and the emergency beam photometry.
(GLDF-WIZARD "Prolumia" "Lumina Pro" "Holger Trahe"
             "middle.ldt" "example_007.l3d" "RAL6018 — middle"
             "sensor.ldt" "emergency.ldt")

; After GLDF-WIZARD the catalog holds (in mint order):
;   photometry_1 = middle (bound to emitter_1)   photometry_2 = emergency
;   lightsource_1   emitter_1   geometry_1   sensor_1   variant index 0
;
; ── Step 2 — embed the rest of the optical range ───────────────
(GLDF-ADD-PHOTOMETRY "narrow.ldt")     ; → photometry_3
(GLDF-ADD-PHOTOMETRY "wide.ldt")       ; → photometry_4

; ── Step 3 — FOUR light sources, one per CCT (explicit ids) ────
; The wizard already made one (lightsource_1, our 4000K). Add the rest
; so each variant can glow at its own colour temperature in 3D.
(GLDF-ADD-LIGHTSOURCE "ls_3000k" "LED 3000K warm"    3000 18.5)
(GLDF-ADD-LIGHTSOURCE "ls_5000k" "LED 5000K cool"    5000 18.5)
(GLDF-ADD-LIGHTSOURCE "ls_emerg" "LED 3000K emergency" 3000 3.0)

; ── Step 4 — emitters with EXPLICIT ids + lumens (no 0lm gaps) ─
; (GLDF-ADD-EMITTER id photometry light-source leo lumens)
; The explicit id is what the variant binds below — stable, no auto-id
; guessing. The wizard already made emitter_1 (middle/4000K, 0lm) which
; we DON'T use; these 8 cover every LEO.
(GLDF-ADD-EMITTER "em_v0_leo0" "photometry_3" "lightsource_1" "LEO0" 1600) ; v0 narrow 4000K
(GLDF-ADD-EMITTER "em_v0_leo1" "photometry_4" "lightsource_1" "LEO1" 1600) ; v0 wide   4000K
(GLDF-ADD-EMITTER "em_v1_leo0" "photometry_4" "ls_3000k"      "LEO0" 1800) ; v1 wide   3000K
(GLDF-ADD-EMITTER "em_v1_leo1" "photometry_3" "ls_3000k"      "LEO1" 1800) ; v1 narrow 3000K
(GLDF-ADD-EMITTER "em_v2_leo0" "photometry_1" "ls_5000k"      "LEO0" 2200) ; v2 middle 5000K
(GLDF-ADD-EMITTER "em_v2_leo1" "photometry_1" "ls_5000k"      "LEO1" 2200) ; v2 middle 5000K
(GLDF-ADD-EMITTER "em_v3_leo0" "photometry_2" "ls_emerg"      "LEO0" 800)  ; v3 emergency

; ── Step 5 — product-wide marketing + electrical + mechanical ──
; First arg is the scope: -1 / "product" = product default, 0/1/2… = variant.
(GLDF-APPLICATIONS  -1 "Interior: Office" "Interior: Industry/Craft")
(GLDF-MARKET        -1 "Europe")
(GLDF-MARKET        -1 "DACH")          ; second market region
(GLDF-APPROVAL      -1 "ENEC")          ; product-wide approval mark
(GLDF-ELECTRICAL    -1 "II" "IP20")     ; safety class ∈ {0,I,0I,II,III}
(GLDF-MECHANICAL    -1 "IK08" 1.4)      ; IK rating + weight (kg)
(GLDF-SIZE          -1 170 90 55)       ; length × width × height (mm)
(GLDF-EMERGENCY     -1 "Guide Light" 5 100)  ; type, 5 h runtime, 100 lm
(GLDF-PRODUCT-PICTURE "product_picture.jpg")  ; product photo

; ── Sensor: round PIR motion detector ──────────────────────────
(GLDF-ADD-SENSOR-EMITTER "sensor_1")
(GLDF-SENSOR-CHARS 0 "Round" "Passive Infrared" "Motion Detector")

; ── Step 6 — create ALL variants first (so auto ids are stable) ─
; The wizard already made variant_1; add three more → variant_2..4.
; (Renaming BEFORE adding would free variant_1 and the next add would
;  reuse it — so add all, THEN rename.)
(GLDF-ADD-VARIANT "v1")
(GLDF-ADD-VARIANT "v2")
(GLDF-ADD-VARIANT "v3")
(GLDF-RENAME-VARIANT "variant_1" "v_ral6018_yellowgreen")
(GLDF-RENAME-VARIANT "variant_2" "v_ral1023_trafficyellow")
(GLDF-RENAME-VARIANT "variant_3" "v_ral9003_signalwhite")
(GLDF-RENAME-VARIANT "variant_4" "v_ral9005_emergency")

; A GTIN per variant (an unwritten must-have for real catalogues).
(GLDF-VARIANT-GTIN 0 "4260000000041")
(GLDF-VARIANT-GTIN 1 "4260000000042")
(GLDF-VARIANT-GTIN 2 "4260000000043")
(GLDF-VARIANT-GTIN 3 "4260000000044")

; ── Step 7 — per variant: light (2 LEOs) + RAL colour + approval ─
(GLDF-VARIANT-GEOMETRY 0 "geometry_1" "em_v0_leo0" "LEO0" "em_v0_leo1" "LEO1")
(GLDF-HOUSING-COLOR 0 6018 "Yellow Green")
(GLDF-APPROVAL      0 "ENEC")
(GLDF-VARIANT-GEOMETRY 1 "geometry_1" "em_v1_leo0" "LEO0" "em_v1_leo1" "LEO1")
(GLDF-HOUSING-COLOR 1 1023 "Traffic Yellow")
(GLDF-APPROVAL      1 "ENEC")
(GLDF-VARIANT-GEOMETRY 2 "geometry_1" "em_v2_leo0" "LEO0" "em_v2_leo1" "LEO1")
(GLDF-HOUSING-COLOR 2 9003 "Signal White")
(GLDF-APPROVAL      2 "ENEC")
(GLDF-VARIANT-GEOMETRY 3 "geometry_1" "em_v3_leo0" "LEO0")
(GLDF-HOUSING-COLOR 3 9005 "Jet Black")
(GLDF-APPROVAL      3 "CE")

; ── Step 8 — switch to the 3D Design view + play the choreography ─
; (GLDF-SHOW-3D) mounts the Bevy canvas on demand; the SLEEP gives it
; time to load before the first camera move. Then the hero shot: tilt
; the heads, dip under the lamp for each colour reveal, fly back.
(GLDF-SHOW-3D)
(SLEEP 2500)
(RESET-TILTS)
(GLDF-VARIANT-SET 0)
(CAMERA-SET 2.619 2.303 4.968  1.987 1.870 2.657)
(ANIMATE-TILT "l3d://LEO0" 33.0 0.0 0.0  2000.0)
(ANIMATE-TILT "l3d://LEO1" 33.0 0.0 0.0  2000.0)
(SLEEP 500)
(ANIMATE-TILT "l3d://LEO0" -33.0 0.0 0.0  2000.0)
(ANIMATE-TILT "l3d://LEO1" -33.0 0.0 0.0  2000.0)
(ANIMATE-CAMERA 2.338 2.303 3.395  2.068 2.640 2.464  1500.0)
(GLDF-VARIANT-SET 1)
(SLEEP 1500)
(ANIMATE-CAMERA 2.619 2.303 4.968  1.987 1.870 2.657  1500.0)
(ANIMATE-TILT "l3d://LEO0" 33.0 0.0 0.0  2000.0)
(ANIMATE-TILT "l3d://LEO1" 33.0 0.0 0.0  2000.0)
(ANIMATE-CAMERA 2.338 2.303 3.395  2.068 2.640 2.464  1500.0)
(GLDF-VARIANT-SET 2)
(SLEEP 1500)
(ANIMATE-CAMERA 2.619 2.303 4.968  1.987 1.870 2.657  1500.0)
(ANIMATE-TILT "l3d://LEO0" -33.0 0.0 0.0  2000.0)
(ANIMATE-TILT "l3d://LEO1" -33.0 0.0 0.0  2000.0)
(ANIMATE-CAMERA 2.338 2.303 3.395  2.068 2.640 2.464  1500.0)
(GLDF-VARIANT-SET 3)
(SLEEP 1500)
(ANIMATE-CAMERA 2.619 2.303 4.968  1.987 1.870 2.657  1500.0)
(ANIMATE-TILT "l3d://LEO0" 0.0 0.0 0.0  1500.0)
(ANIMATE-TILT "l3d://LEO1" 0.0 0.0 0.0  1500.0)

; ───────────────────────────────────────────────────────────────
; Verb reference
; ── Build-from-scratch (creates header + LS + emitter scaffolding) ─
;   (GLDF-WIZARD manufacturer name author photometry geometry variant
;                sensor emergency)        ; "" or omit to skip a field
; ── Refine an existing product ─────────────────────────────────
;   (GLDF-SET-MANUFACTURER s) (GLDF-SET-AUTHOR s) (GLDF-SET-NAME s)
;   (GLDF-ADD-PHOTOMETRY "x.ldt"|"x.ies")  ; file must be in NEEDS-FILES
;   (GLDF-ADD-SENSOR     "x.sensldt"|"x.sensxml")
;   (GLDF-SET-GEOMETRY   "x.l3d")
;   (GLDF-ADD-SPECTRUM   "x.spd" "lightsource-id")
;   (GLDF-ADD-LIGHTSOURCE "id" "name" cct-kelvin [power-w])
;   (GLDF-ADD-EMITTER    "emitter-id" "photometry" "lightsource" [LEO] [lumens])
;   (GLDF-ADD-SENSOR-EMITTER "sensor-id")
;   (GLDF-ADD-VARIANT    "name")   (GLDF-RENAME-VARIANT "old-id" "new-id")
;   (GLDF-VARIANT-GEOMETRY variant-index "geometry-id" "emitter" "LEO" [...])
;   (GLDF-LUMENS         "emitter-id" lumens)   ; or pass lumens to ADD-EMITTER
; ── Metadata (scope: -1/"product" = product default, 0/1/… = variant) ─
;   (GLDF-HOUSING-COLOR scope ral "name")   ; RAL number + colour name
;   (GLDF-APPLICATIONS  scope "app" [app …]) ; full XSD taxonomy strings
;   (GLDF-MARKET        scope "region")      ; call again for more regions
;   (GLDF-APPROVAL      scope "mark")        ; e.g. "ENEC", "CE"
;   (GLDF-ELECTRICAL    scope "safety" "ip") ; safety ∈ {0,I,0I,II,III}
;   (GLDF-MECHANICAL    scope "IK-rating" weight-kg)
;   (GLDF-SIZE          scope length width [height])     ; mm
;   (GLDF-EMERGENCY     scope "Guide Light" hours flux)  ; type, h, lm
;   (GLDF-VARIANT-GTIN  variant-index "GTIN")
;   (GLDF-SENSOR-CHARS  sensor-index "Round" "Passive Infrared" "Motion Detector")
;   (GLDF-PRODUCT-PICTURE "file.jpg" ["Product Picture"])
; NOTE: the granular verbs add to an EXISTING product. Start every
; from-scratch build with (GLDF-WIZARD …) so the file validates.
"#;

/// Default script shown when the panel is opened FROM the 3D design
/// view. The Trahe ACME hero choreography — camera glides + LEO tilts +
/// variant swaps on the Bevy canvas.
const STARTER_SCRIPT_3D: &str = r#"; ═══════════════════════════════════════════════════════════════
; λ Lisp REPL — 3D design choreography (camera + tilt)
; ═══════════════════════════════════════════════════════════════
;
; Hit ▶ Run to play. ■ Stop interrupts mid-run; ▶ Run again to replay.
;
; ── Signatures (the ones this script uses) ─────────────────────
;   (CAMERA-SET     ex ey ez tx ty tz)         ; snap; eye + look-at target
;   (ANIMATE-CAMERA ex ey ez tx ty tz ms)      ; smooth glide over ms
;   (LUMINAIRE-TILT  joint x-deg y-deg z-deg)  ; snap one joint, target Euler XYZ
;   (ANIMATE-TILT    joint x-deg y-deg z-deg ms) ; smooth rotation over ms
;   (RESET-TILTS)                              ; snap all joints back to L3D defaults
;   (SLEEP ms)                                 ; pause before the next command
;   (GLDF-VARIANT-SET idx)                     ; swap active variant
;
; ── More signatures (room + getters; not used below) ───────────
;   (SET-AMBIENT brightness)                   ; 0 = off; 50 dramatic; 150 neutral
;   (SET-ROOM-SIZE w l h)                      ; metres
;   (SET-WALLS-COLOR r g b)                    ; 0–1 linear sRGB
;   (SET-FLOOR-COLOR r g b)
;   (SET-CEILING-COLOR r g b)
;   (SET-WALLS-REFLECTANCE v)                  ; 0 = matte; 1 = mirror
;   (SET-FLOOR-REFLECTANCE v)
;   (SET-CEILING-REFLECTANCE v)
;   (GET-AMBIENT) (GET-ROOM-SIZE) (GET-CAMERA) (GET-WALLS-COLOR) …
;   (GLDF-VARIANT-COUNT) (GLDF-VARIANT-NAMES) (GLDF-MANUFACTURER)
;
; ── Notes ──────────────────────────────────────────────────────
; * The three angles for ANIMATE-TILT are ONE TARGET (x, y, z) Euler
;   rotation, not three keyframes. The joint animates from its
;   current pose to that target over the duration.
; * Joints are L3D LEO partNames addressed as "l3d://LEO0" /
;   "l3d://LEO1". The L3D loader tags the entire head OBJ with that
;   name, so the WHOLE head tilts, not just the lit disc.
; * Press the C key over the 3D canvas to capture the current camera
;   pose into the panel as a fresh (CAMERA-SET …) line.
;
; ═══════════════════════════════════════════════════════════════
;  Trahe ACME hero choreography (~28s)
;
;  Each variant gets: tilt the heads, dip the camera under the
;  lamp for the colour reveal, then fly back to the hero pose
;  before starting the next variant.
; ═══════════════════════════════════════════════════════════════
(RESET-TILTS)
(GLDF-VARIANT-SET 0)
(CAMERA-SET 2.619 2.303 4.968  1.987 1.870 2.657)

; Variant 0 — yellow-green hero
(ANIMATE-TILT "l3d://LEO0" 33.0 0.0 0.0  2000.0)
(ANIMATE-TILT "l3d://LEO1" 33.0 0.0 0.0  2000.0)
(SLEEP 500)
(ANIMATE-TILT "l3d://LEO0" -33.0 0.0 0.0  2000.0)
(ANIMATE-TILT "l3d://LEO1" -33.0 0.0 0.0  2000.0)

; Dip under the lamp, swap variant, fly back to hero
(ANIMATE-CAMERA 2.338 2.303 3.395  2.068 2.640 2.464  1500.0)
(GLDF-VARIANT-SET 1)
(SLEEP 1500)
(ANIMATE-CAMERA 2.619 2.303 4.968  1.987 1.870 2.657  1500.0)

; Variant 1 — traffic yellow
(ANIMATE-TILT "l3d://LEO0" 33.0 0.0 0.0  2000.0)
(ANIMATE-TILT "l3d://LEO1" 33.0 0.0 0.0  2000.0)

; Dip, swap, back to hero
(ANIMATE-CAMERA 2.338 2.303 3.395  2.068 2.640 2.464  1500.0)
(GLDF-VARIANT-SET 2)
(SLEEP 1500)
(ANIMATE-CAMERA 2.619 2.303 4.968  1.987 1.870 2.657  1500.0)

; Variant 2 — signal white
(ANIMATE-TILT "l3d://LEO0" -33.0 0.0 0.0  2000.0)
(ANIMATE-TILT "l3d://LEO1" -33.0 0.0 0.0  2000.0)

; Final return to hero + zero tilt — clean re-runnable end state
(ANIMATE-CAMERA 2.619 2.303 4.968  1.987 1.870 2.657  1500.0)
(ANIMATE-TILT "l3d://LEO0" 0.0 0.0 0.0  1500.0)
(ANIMATE-TILT "l3d://LEO1" 0.0 0.0 0.0  1500.0)
;(CAMERA-SET 2.338 2.303 3.395  2.068 2.640 2.464)
"#;

/// Floating REPL overlay. Visible when `open` is true.
///
/// `is_3d_view` is a reactive flag the app shell sets true while the
/// active view is the 3D design canvas. It drives which default script
/// the panel shows: the camera/tilt choreography in 3D, the file-drop
/// build script everywhere else — until the user edits the script
/// themselves, after which their text is never overwritten.
#[component]
pub fn LispPanel(open: RwSignal<bool>, #[prop(into)] is_3d_view: Signal<bool>) -> impl IntoView {
    let starter_for = |three_d: bool| {
        if three_d {
            STARTER_SCRIPT_3D
        } else {
            STARTER_SCRIPT_BUILD
        }
    };

    let code = RwSignal::new(starter_for(is_3d_view.get_untracked()).to_string());
    let output = RwSignal::new(String::new());
    let result = RwSignal::new(String::new());
    // True while the OUTBOX drainer is still walking commands. The
    // Run button flips to "■ Stop" while this is set; clicking Stop
    // calls `request_stop()` which aborts the chain on the next tick.
    let running = RwSignal::new(false);

    // Set once the user types into the textarea — after that we NEVER
    // overwrite their script when the view changes. Cleared by "Clear"
    // (which resets to the current view's default). The C-key append is
    // programmatic, not a manual edit, so it does NOT set this.
    let user_edited = RwSignal::new(false);

    // Swap the default script when the active view changes — but only
    // while the script is still one of our untouched defaults. The
    // moment the user edits, this stops firing.
    Effect::new(move |_| {
        let three_d = is_3d_view.get();
        if !user_edited.get_untracked() {
            code.set(starter_for(three_d).to_string());
        }
    });

    // Pull the live editor state so the interpreter can answer
    // GLDF-* queries against the document currently being edited.
    let gldf_ctx = crate::state::use_gldf();

    // Register `window.gldf_lisp_append_camera_set(line)` so the Bevy
    // canvas's C-key handler (in `crates/gldf-bevy/src/lib.rs`) can post
    // a `(CAMERA-SET …)` line into the textarea. We auto-open the panel
    // so the user doesn't have to fish for it — the whole point of the
    // shortcut is "park the camera, press C, see the line ready to
    // paste". The line is appended on its own newline.
    if let Some(win) = web_sys::window() {
        let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |line: String| {
            code.update(|c| {
                if !c.is_empty() && !c.ends_with('\n') {
                    c.push('\n');
                }
                c.push_str(&line);
                c.push('\n');
            });
            open.set(true);
        }) as Box<dyn Fn(String)>);
        let _ = js_sys::Reflect::set(
            &win,
            &wasm_bindgen::JsValue::from_str("gldf_lisp_append_camera_set"),
            cb.as_ref(),
        );
        cb.forget();
    }

    let on_run = move |_| {
        // Toggle: if a previous run is still draining, this click is
        // a Stop. The drain chain notices the flag at the next tick
        // and short-circuits via on_done, which clears `running`.
        if running.get_untracked() {
            request_stop();
            return;
        }
        let src = code.get();
        let product = Some(gldf_ctx.get_untracked().product.clone());
        let on_done: std::rc::Rc<dyn Fn()> = std::rc::Rc::new(move || running.set(false));
        running.set(true);
        // Synchronous engine run: fills OUTBOX (3D scene cmds) AND
        // EDITOR_OUTBOX (document mutations). NEITHER is acted on yet —
        // we apply the editor commands first (below), THEN drain the
        // OUTBOX, so a build-then-animate script has its variants /
        // geometry in place before the choreography drives them.
        let (out, res) = run_lisp(&src, product);
        output.set(out);
        result.set(res);

        let cmds = take_editor_commands();
        if cmds.is_empty() {
            // Pure 3D / query script — just drain the scene commands.
            drain_outbox_async(on_done);
            return;
        }

        // `apply_now` applies the editor commands to the live document,
        // appends a summary, THEN kicks off the OUTBOX drain (so the 3D
        // choreography runs after the build). Takes `on_done` so the
        // drain clears the running flag when it finishes.
        let apply_now = move |picked: std::collections::HashMap<String, Vec<u8>>,
                              cmds: Vec<SceneCommand>,
                              on_done: std::rc::Rc<dyn Fn()>| {
            let mut summary = String::new();
            gldf_ctx.update(|s| {
                summary = apply_editor_commands(s, &cmds, &picked);
            });
            output.update(|o| {
                if !o.is_empty() && !o.ends_with('\n') {
                    o.push('\n');
                }
                o.push_str(&summary);
            });
            // Build done → now play the 3D choreography (if any).
            drain_outbox_async(on_done);
        };

        // Which files does the script reference that we don't already
        // have embedded? Subtract the document's embedded_files.
        let needed = required_file_names(&cmds);
        let embedded: std::collections::BTreeSet<String> =
            gldf_ctx.with_untracked(|s| s.embedded_files.keys().cloned().collect());
        let missing: Vec<String> = needed.difference(&embedded).cloned().collect();

        if missing.is_empty() {
            apply_now(std::collections::HashMap::new(), cmds, on_done);
            return;
        }

        // First try to satisfy the missing files from the BUNDLED demo
        // assets (copied into dist/ via index.html `copy-file`). This is
        // what lets the Prolumia demo run with a single ▶ Run — no manual
        // drop. We fetch each by name; whatever isn't bundled falls back
        // to the file picker.
        let missing_set: std::collections::BTreeSet<String> = missing.into_iter().collect();
        leptos::task::spawn_local(async move {
            let mut picked: std::collections::HashMap<String, Vec<u8>> =
                std::collections::HashMap::new();
            for name in &missing_set {
                if let Ok(bytes) = fetch_bytes(name).await {
                    if !bytes.is_empty() {
                        picked.insert(name.clone(), bytes);
                    }
                }
            }
            let still_missing: Vec<String> = missing_set
                .iter()
                .filter(|n| !picked.contains_key(*n))
                .cloned()
                .collect();

            if still_missing.is_empty() {
                // Everything came from the bundle — run the demo unattended.
                let n = picked.len();
                output.update(|o| {
                    o.push_str(&format!("\n; loaded {n} bundled file(s) automatically"));
                });
                apply_now(picked, cmds.clone(), on_done);
                return;
            }

            // Some files aren't bundled — prompt for just those, merging
            // the bundled ones we already fetched.
            output.update(|o| {
                if !o.is_empty() && !o.ends_with('\n') {
                    o.push('\n');
                }
                o.push_str(&format!(
                    "; drop these {} file(s): {}",
                    still_missing.len(),
                    still_missing.join(", ")
                ));
            });
            let want: std::collections::BTreeSet<String> =
                still_missing.into_iter().collect();
            open_file_picker(move |dropped| {
                let mut all = picked.clone();
                for (k, v) in dropped {
                    all.insert(k, v);
                }
                let unresolved: Vec<String> = want
                    .iter()
                    .filter(|n| !all.contains_key(*n))
                    .cloned()
                    .collect();
                if !unresolved.is_empty() {
                    output.update(|o| {
                        o.push_str(&format!(
                            "\n; aborted — still missing: {}",
                            unresolved.join(", ")
                        ));
                    });
                    return;
                }
                apply_now(all, cmds.clone(), on_done.clone());
            });
        });
    };
    let on_clear = move |_| {
        // Reset to the current view's default script and clear the
        // edited flag, so the context-aware swap resumes. (Empties the
        // output, not the script — a blank textarea is rarely wanted.)
        user_edited.set(false);
        code.set(starter_for(is_3d_view.get_untracked()).to_string());
        output.set(String::new());
        result.set(String::new());
    };
    let on_close = move |_| open.set(false);

    view! {
        {move || open.get().then(|| view! {
            <div
                data-testid="lisp-panel"
                style="position: fixed; top: 80px; right: 24px; width: 480px; max-height: 70vh; \
                       background: var(--bg-secondary, #1e1e1e); border: 1px solid var(--border-color, #444); \
                       border-radius: 8px; box-shadow: 0 8px 24px rgba(0,0,0,0.4); z-index: 9000; \
                       display: flex; flex-direction: column;"
            >
                <div style="display: flex; align-items: center; justify-content: space-between; \
                            padding: 8px 12px; border-bottom: 1px solid var(--border-color, #444); \
                            font-weight: 600;">
                    <span>{ "λ Lisp REPL (acadlisp + gldf-rs)" }</span>
                    <button
                        on:click=on_close
                        data-testid="lisp-panel-close"
                        style="background: none; border: none; color: var(--text-primary, #ddd); \
                               font-size: 16px; cursor: pointer;"
                        title="Close"
                    >{ "✕" }</button>
                </div>
                <textarea
                    data-testid="lisp-panel-source"
                    prop:value=move || code.get()
                    on:input=move |ev| { user_edited.set(true); code.set(crate::components::edit_util::event_value(&ev).unwrap_or_default()); }
                    spellcheck="false"
                    style="flex: 1 1 200px; min-height: 160px; padding: 8px; font-family: var(--font-mono, monospace); \
                           font-size: 12px; background: var(--bg-primary, #111); color: var(--text-primary, #ddd); \
                           border: none; resize: vertical; outline: none;"
                />
                <div style="display: flex; gap: 6px; padding: 6px 8px; border-top: 1px solid var(--border-color, #444);">
                    <button
                        class="btn btn-primary"
                        on:click=on_run
                        data-testid="lisp-panel-run"
                        title=move || if running.get() { "Stop the running script" } else { "Run the script" }
                    >
                        { move || if running.get() { "■ Stop" } else { "▶ Run" } }
                    </button>
                    <button class="btn btn-outline" on:click=on_clear data-testid="lisp-panel-clear">{ "Clear" }</button>
                </div>
                <pre
                    data-testid="lisp-panel-output"
                    style="margin: 0; padding: 8px 12px; max-height: 220px; overflow: auto; \
                           background: var(--bg-tertiary, #0a0a0a); color: var(--text-secondary, #bbb); \
                           font-family: var(--font-mono, monospace); font-size: 11px; border-top: 1px solid var(--border-color, #444);"
                >{ move || output.get() }</pre>
                <div
                    data-testid="lisp-panel-result"
                    style="padding: 6px 12px; background: var(--bg-tertiary, #0a0a0a); color: #9fd0ff; \
                           font-family: var(--font-mono, monospace); font-size: 11px; border-top: 1px solid var(--border-color, #444);"
                >{ move || format!("⇒ {}", result.get()) }</div>
            </div>
        })}
    }
}
