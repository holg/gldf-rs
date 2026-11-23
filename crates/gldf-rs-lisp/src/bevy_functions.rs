//! Camera + per-joint luminaire-tilt functions.
//!
//! These produce side effects on a 3D scene. The crate stays
//! `no-wasm-bindgen` though — calls don't go straight to Bevy, they
//! land in [`crate::GldfState::dispatch`], which forwards to whatever
//! bridge the host registered (the WASM Leptos panel) and ALWAYS
//! records the request into [`crate::GldfState::log`].
//!
//! That gives the headless CLI binary and the integration tests a
//! "what would have happened?" trace without needing a real Bevy
//! canvas. See `tests/smoke.rs` for the assertion pattern.

use acadlisp::{Expr, ForeignContext, Interpreter};

use crate::{arg_error, expr_to_f32, expr_to_string, state_mut, SceneCommand};

pub fn register_all(interp: &mut Interpreter) {
    interp.register_fn("CAMERA-SET", camera_set);
    interp.register_fn("LUMINAIRE-TILT", luminaire_tilt);
    interp.register_fn("ANIMATE-CAMERA", animate_camera);
    interp.register_fn("ANIMATE-TILT", animate_tilt);
    interp.register_fn("SLEEP", sleep);
    interp.register_fn("RESET-TILTS", reset_tilts);
    interp.register_fn("GLDF-SHOW-3D", show_view_3d);
    interp.register_fn("SET-AMBIENT", set_ambient);
    interp.register_fn("SET-ROOM-SIZE", set_room_size);
    interp.register_fn("SET-WALLS-COLOR", set_walls_color);
    interp.register_fn("SET-FLOOR-COLOR", set_floor_color);
    interp.register_fn("SET-CEILING-COLOR", set_ceiling_color);
    interp.register_fn("SET-WALLS-REFLECTANCE", set_walls_reflectance);
    interp.register_fn("SET-FLOOR-REFLECTANCE", set_floor_reflectance);
    interp.register_fn("SET-CEILING-REFLECTANCE", set_ceiling_reflectance);
    interp.register_fn("SET-WALLS-EMISSIVE", set_walls_emissive);
    interp.register_fn("SET-FLOOR-EMISSIVE", set_floor_emissive);
    interp.register_fn("SET-CEILING-EMISSIVE", set_ceiling_emissive);
}

/// `(RESET-TILTS)` — clear every joint tilt and snap each joint back
/// to the rotation the L3D structure XML gave it at load time. Cancels
/// any in-flight animations too. Useful at the top of a choreography
/// so re-running leaves the luminaire in its as-loaded pose, not
/// wherever the last run finished.
///
/// Always returns `T`. See [`SceneCommand::ResetTilts`].
fn reset_tilts(ctx: &mut ForeignContext, _args: &[Expr]) -> Expr {
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::ResetTilts);
    }
    Expr::T
}

/// `(GLDF-SHOW-3D)` — switch the editor to the 3D Design view so the
/// Bevy canvas mounts before a build-then-animate demo runs its
/// camera/tilt choreography. Goes through the drained OUTBOX (not
/// immediate) so it fires in dispatch order; follow it with a `(SLEEP
/// …)` to give the canvas time to load before the first ANIMATE-*.
/// See [`SceneCommand::ShowView3d`].
fn show_view_3d(ctx: &mut ForeignContext, _args: &[Expr]) -> Expr {
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::ShowView3d);
    }
    Expr::T
}

/// `(SLEEP <duration-ms>)` — pause subsequent command dispatch on the
/// JS bridge side for `duration-ms` milliseconds. Useful between
/// `GLDF-VARIANT-SET` calls so the Bevy canvas has time to reload
/// before the next swap. The Lisp interpreter is synchronous; the
/// pause is enforced inside the WASM panel's setTimeout drainer.
///
/// Returns `T` on success, `Nil` on arg-shape error. See
/// [`SceneCommand::Sleep`].
fn sleep(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let duration_ms = match args.first().and_then(expr_to_f32) {
        Some(d) if d.is_finite() && d >= 0.0 => d,
        _ => return arg_error(ctx.output, "SLEEP", "<duration-ms>", args),
    };
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::Sleep { duration_ms });
    }
    Expr::T
}

/// `(CAMERA-SET eye-x eye-y eye-z target-x target-y target-z)` — move
/// the camera. All values are metres in world space; target is the
/// look-at point.
///
/// Returns `T` on success, `Nil` on arg-shape error. See [`SceneCommand::CameraSet`].
fn camera_set(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    if args.len() != 6 {
        return arg_error(
            ctx.output,
            "CAMERA-SET",
            "<eye-x> <eye-y> <eye-z> <target-x> <target-y> <target-z>",
            args,
        );
    }
    let nums: Vec<f32> = args.iter().filter_map(expr_to_f32).collect();
    if nums.len() != 6 {
        return arg_error(ctx.output, "CAMERA-SET", "six numeric args", args);
    }
    let eye = [nums[0], nums[1], nums[2]];
    let target = [nums[3], nums[4], nums[5]];
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::CameraSet { eye, target });
    }
    Expr::T
}

/// `(LUMINAIRE-TILT joint-path-string degrees-x degrees-y degrees-z)` —
/// rotate one L3D joint by Euler XYZ in degrees.
///
/// The SLV hero-shot acceptance test uses this to tilt the inner-ring
/// gimbal of the Tria 2 — see memory `project-lisp-engine-camera-tilt-demo`.
/// Joint paths use the `"l3d://<partName>"` form.
fn luminaire_tilt(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    if args.len() != 4 {
        return arg_error(
            ctx.output,
            "LUMINAIRE-TILT",
            "<joint-path-string> <deg-x> <deg-y> <deg-z>",
            args,
        );
    }
    let joint_path = match expr_to_string(&args[0]) {
        Some(p) => p,
        None => return arg_error(ctx.output, "LUMINAIRE-TILT", "string joint path", args),
    };
    let nums: Vec<f32> = args[1..].iter().filter_map(expr_to_f32).collect();
    if nums.len() != 3 {
        return arg_error(
            ctx.output,
            "LUMINAIRE-TILT",
            "three numeric rotation args",
            args,
        );
    }
    let euler_deg = [nums[0], nums[1], nums[2]];
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::LuminaireTilt {
            joint_path,
            euler_deg,
        });
    }
    Expr::T
}

/// `(ANIMATE-CAMERA eye-x eye-y eye-z target-x target-y target-z duration-ms)` —
/// smoothly fly the camera from its current pose to the given pose.
///
/// Animations chain: calling this twice with different targets produces
/// a multi-leg trajectory. Each leg eases via smoothstep on the Bevy
/// side. The "from" is whatever the camera was at the moment this
/// command reaches the bridge.
fn animate_camera(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    if args.len() != 7 {
        return arg_error(
            ctx.output,
            "ANIMATE-CAMERA",
            "<ex> <ey> <ez> <tx> <ty> <tz> <duration-ms>",
            args,
        );
    }
    let nums: Vec<f32> = args.iter().filter_map(expr_to_f32).collect();
    if nums.len() != 7 {
        return arg_error(ctx.output, "ANIMATE-CAMERA", "seven numeric args", args);
    }
    let eye = [nums[0], nums[1], nums[2]];
    let target = [nums[3], nums[4], nums[5]];
    let duration_ms = nums[6].max(1.0);
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::CameraAnimate {
            eye,
            target,
            duration_ms,
        });
    }
    Expr::T
}

/// `(ANIMATE-TILT joint-path deg-x deg-y deg-z duration-ms)` — smoothly
/// rotate one joint from its current tilt to a target. Same semantics
/// as `ANIMATE-CAMERA` (smoothstep, chain-able).
fn animate_tilt(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    if args.len() != 5 {
        return arg_error(
            ctx.output,
            "ANIMATE-TILT",
            "<joint-path> <dx> <dy> <dz> <duration-ms>",
            args,
        );
    }
    let joint_path = match expr_to_string(&args[0]) {
        Some(p) => p,
        None => return arg_error(ctx.output, "ANIMATE-TILT", "string joint path", args),
    };
    let nums: Vec<f32> = args[1..].iter().filter_map(expr_to_f32).collect();
    if nums.len() != 4 {
        return arg_error(ctx.output, "ANIMATE-TILT", "four numeric args", args);
    }
    let euler_deg = [nums[0], nums[1], nums[2]];
    let duration_ms = nums[3].max(1.0);
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::LuminaireAnimate {
            joint_path,
            euler_deg,
            duration_ms,
        });
    }
    Expr::T
}

/// `(SET-AMBIENT brightness)` — set the global ambient/fill light
/// strength. 0 = scene lit only by the luminaire's own light;
/// ~50 = dramatic (hero shots); ~150 = neutral demo default;
/// 300+ = "indoor on a sunny day" (walls glow).
fn set_ambient(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    let brightness = match args.first().and_then(expr_to_f32) {
        Some(b) if b.is_finite() && b >= 0.0 => b,
        _ => return arg_error(ctx.output, "SET-AMBIENT", "<brightness>", args),
    };
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::SetAmbient { brightness });
    }
    Expr::T
}

/// `(SET-ROOM-SIZE width length height)` — resize the surrounding
/// room. All three are metres. The luminaire stays in place; only
/// the walls/floor/ceiling are rebuilt at the new dimensions.
fn set_room_size(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    if args.len() != 3 {
        return arg_error(
            ctx.output,
            "SET-ROOM-SIZE",
            "<width> <length> <height>",
            args,
        );
    }
    let nums: Vec<f32> = args.iter().filter_map(expr_to_f32).collect();
    if nums.len() != 3 || !nums.iter().all(|n| n.is_finite() && *n > 0.0) {
        return arg_error(ctx.output, "SET-ROOM-SIZE", "three positive numbers", args);
    }
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::SetRoomSize {
            width: nums[0],
            length: nums[1],
            height: nums[2],
        });
    }
    Expr::T
}

/// Parse three 0–1 colour channels from `args`, dispatching a
/// `SetSurfaceColor` for the given surface. Pulled out of three
/// near-identical setter wrappers so we have one validator.
fn dispatch_color(
    ctx: &mut ForeignContext,
    fn_name: &str,
    surface: crate::RoomSurface,
    args: &[Expr],
) -> Expr {
    if args.len() != 3 {
        return arg_error(ctx.output, fn_name, "<r> <g> <b>", args);
    }
    let nums: Vec<f32> = args.iter().filter_map(expr_to_f32).collect();
    if nums.len() != 3
        || !nums
            .iter()
            .all(|n| n.is_finite() && (0.0..=1.0).contains(n))
    {
        return arg_error(ctx.output, fn_name, "three numbers in [0, 1]", args);
    }
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::SetSurfaceColor {
            surface,
            r: nums[0],
            g: nums[1],
            b: nums[2],
        });
    }
    Expr::T
}

/// `(SET-WALLS-COLOR r g b)` — recolour all four walls. 0–1 per channel.
fn set_walls_color(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    dispatch_color(ctx, "SET-WALLS-COLOR", crate::RoomSurface::Walls, args)
}

/// `(SET-FLOOR-COLOR r g b)` — recolour the floor. 0–1 per channel.
fn set_floor_color(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    dispatch_color(ctx, "SET-FLOOR-COLOR", crate::RoomSurface::Floor, args)
}

/// `(SET-CEILING-COLOR r g b)` — recolour the ceiling. 0–1 per channel.
fn set_ceiling_color(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    dispatch_color(ctx, "SET-CEILING-COLOR", crate::RoomSurface::Ceiling, args)
}

/// Common reflectance parser. 0 = matte (roughness 1.0), 1 = mirror
/// (roughness 0.0). The Bevy side does the roughness inversion.
fn dispatch_reflectance(
    ctx: &mut ForeignContext,
    fn_name: &str,
    surface: crate::RoomSurface,
    args: &[Expr],
) -> Expr {
    let value = match args.first().and_then(expr_to_f32) {
        Some(v) if v.is_finite() && (0.0..=1.0).contains(&v) => v,
        _ => return arg_error(ctx.output, fn_name, "<reflectance in [0, 1]>", args),
    };
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::SetSurfaceReflectance { surface, value });
    }
    Expr::T
}

/// `(SET-WALLS-REFLECTANCE value)` — 0 = matte, 1 = mirror.
fn set_walls_reflectance(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    dispatch_reflectance(
        ctx,
        "SET-WALLS-REFLECTANCE",
        crate::RoomSurface::Walls,
        args,
    )
}

/// `(SET-FLOOR-REFLECTANCE value)` — 0 = matte, 1 = mirror.
fn set_floor_reflectance(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    dispatch_reflectance(
        ctx,
        "SET-FLOOR-REFLECTANCE",
        crate::RoomSurface::Floor,
        args,
    )
}

/// `(SET-CEILING-REFLECTANCE value)` — 0 = matte, 1 = mirror.
fn set_ceiling_reflectance(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    dispatch_reflectance(
        ctx,
        "SET-CEILING-REFLECTANCE",
        crate::RoomSurface::Ceiling,
        args,
    )
}

/// Parse three 0–1 colour channels for `SET-*-EMISSIVE`. Mirrors
/// `dispatch_color` but emits a `SetSurfaceEmissive` command.
fn dispatch_emissive(
    ctx: &mut ForeignContext,
    fn_name: &str,
    surface: crate::RoomSurface,
    args: &[Expr],
) -> Expr {
    if args.len() != 3 {
        return arg_error(ctx.output, fn_name, "<r> <g> <b>", args);
    }
    let nums: Vec<f32> = args.iter().filter_map(expr_to_f32).collect();
    if nums.len() != 3 || !nums.iter().all(|n| n.is_finite() && *n >= 0.0) {
        return arg_error(ctx.output, fn_name, "three non-negative numbers", args);
    }
    if let Some(state) = state_mut(ctx) {
        state.dispatch(SceneCommand::SetSurfaceEmissive {
            surface,
            r: nums[0],
            g: nums[1],
            b: nums[2],
        });
    }
    Expr::T
}

/// `(SET-WALLS-EMISSIVE r g b)` — wall glow; 0 0 0 disables.
fn set_walls_emissive(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    dispatch_emissive(ctx, "SET-WALLS-EMISSIVE", crate::RoomSurface::Walls, args)
}

/// `(SET-FLOOR-EMISSIVE r g b)` — floor glow; 0 0 0 disables.
fn set_floor_emissive(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    dispatch_emissive(ctx, "SET-FLOOR-EMISSIVE", crate::RoomSurface::Floor, args)
}

/// `(SET-CEILING-EMISSIVE r g b)` — ceiling glow; 0 0 0 disables.
fn set_ceiling_emissive(ctx: &mut ForeignContext, args: &[Expr]) -> Expr {
    dispatch_emissive(
        ctx,
        "SET-CEILING-EMISSIVE",
        crate::RoomSurface::Ceiling,
        args,
    )
}
