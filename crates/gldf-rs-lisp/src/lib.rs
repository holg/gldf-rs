//! AutoLISP scripting engine for GLDF luminaire data.
//!
//! Mirrors the `bimifc-lisp` pattern verbatim — see that crate for the
//! reference embedding. This one trades `IfcModel` for
//! [`gldf_rs::GldfProduct`] and ships function modules that query the
//! GLDF product tree, manipulate the active variant, drive 3D-scene
//! updates (camera moves + per-joint luminaire tilt) and trigger the
//! lib's export paths.
//!
//! ```text
//!     +-----------+    register_fn(...)    +-------------+
//!     |  caller   | --------------------> |  Interpreter |
//!     +-----------+                       +-------------+
//!           |                                    | user_data
//!           | (gldf-load ...)                    v
//!           |                              +------------+
//!           +----------------------------> |  GldfState |
//!                                          +------------+
//!                                              | product / variant idx
//!                                              v
//!                                       gldf_rs::GldfProduct
//! ```
//!
//! The Leptos REPL panel and the Bevy `#[wasm_bindgen]` joint-rotation
//! companion are out of scope for this crate — they live in
//! `gldf-rs-wasm` and `gldf-bevy` respectively. This crate stays
//! `no-DOM, no-wasm-bindgen` so it can be reused from the CLI binary
//! (`gldf-lisp`), tests, and any future TUI / Python frontend.

mod bevy_functions;
mod editor_functions;
mod export_functions;
mod gldf_functions;

use acadlisp::Interpreter;
use gldf_rs::GldfProduct;

/// Engine-side handle for "what should happen to the 3D scene when a
/// script calls `(camera-set ...)` or `(luminaire-tilt ...)`?".
///
/// Headless callers (the CLI binary, tests) leave this `None` — the
/// camera/tilt functions then record their intent into [`GldfState::log`]
/// and return so the rest of the script keeps running. The WASM panel
/// will fill it in with a closure that calls the `gldf-bevy`
/// `#[wasm_bindgen]` bridge.
pub type SceneBridge = Box<dyn FnMut(SceneCommand) + Send>;

/// Side-effect commands the script can request on the 3D scene.
///
/// All angles are degrees, all distances metres. Joint paths use the
/// `"l3d://<partName>"` format — see the SLV hero-shot memory for the
/// rationale (memory: `project-lisp-engine-camera-tilt-demo`).
#[derive(Debug, Clone, PartialEq)]
pub enum SceneCommand {
    /// Snap the camera to a pose this frame (no interpolation).
    CameraSet {
        /// Camera world position (x, y, z) in metres.
        eye: [f32; 3],
        /// Look-at target in metres.
        target: [f32; 3],
    },
    /// Snap one L3D joint to a target Euler rotation (no interpolation).
    LuminaireTilt {
        /// Joint path matching an L3D structure-tree partName.
        joint_path: String,
        /// XYZ Euler rotation in degrees.
        euler_deg: [f32; 3],
    },
    /// Animate the camera from its current pose to a target pose over
    /// `duration_ms` milliseconds. Eases via smoothstep on the Bevy
    /// side. Multiple animations chain — the "from" pose is captured
    /// when the bridge receives the command.
    CameraAnimate {
        /// Destination eye position in metres.
        eye: [f32; 3],
        /// Look-at target in metres.
        target: [f32; 3],
        /// Animation duration in milliseconds.
        duration_ms: f32,
    },
    /// Animate one joint from its current tilt to a target Euler-deg
    /// rotation. Same easing + chaining semantics as `CameraAnimate`.
    LuminaireAnimate {
        /// Joint path (`"l3d://<partName>"`).
        joint_path: String,
        /// Target XYZ Euler rotation in degrees.
        euler_deg: [f32; 3],
        /// Animation duration in milliseconds.
        duration_ms: f32,
    },
    /// Switch the active variant in the surrounding editor. The WASM
    /// panel forwards this to the Leptos `Scope` signal so the Bevy
    /// canvas swaps housing colour + emitter config + L3D as if the
    /// user had clicked the variant chip.
    SetActiveVariant {
        /// Zero-based variant index.
        index: usize,
    },
    /// Pause script execution for `duration_ms` milliseconds before
    /// processing the next command. Useful between
    /// [`SceneCommand::SetActiveVariant`]s so the canvas has time to
    /// reload the new L3D before the next swap. The Lisp interpreter
    /// itself is synchronous; the WASM panel implements the wait by
    /// draining the dispatched-command log via setTimeout.
    Sleep {
        /// Wait duration in milliseconds.
        duration_ms: f32,
    },
    /// Snap every joint back to the rotation the L3D structure XML
    /// gave it at load time (the `LightEmitterBaseRotation`). Clears
    /// all pending snap tilts AND in-flight tilt animations so the
    /// luminaire returns to its as-loaded "defaults" pose. Useful at
    /// the start of a choreography to make scripts re-runnable without
    /// leaving the luminaire in whatever state the previous run left.
    ResetTilts,
    /// Switch the editor to the 3D Design view (`NavItem::BevyViewer`),
    /// which mounts the Bevy canvas on demand. Emitted by `(GLDF-SHOW-3D)`
    /// so a build-then-animate demo can navigate to the canvas before
    /// running the camera/tilt choreography. The WASM panel routes this
    /// to `window.gldf_nav_3d()`; headless callers ignore it.
    ShowView3d,
    /// Set the global ambient/fill brightness. Bevy's
    /// `GlobalAmbientLight.brightness` resource. 50 = dim/dramatic
    /// (housing dark when LEDs tilt away); 150 = neutral demo default;
    /// 300+ = "indoor on a sunny day" (walls glow). 0 leaves only the
    /// luminaire's own light, useful for hero shots.
    SetAmbient { brightness: f32 },
    /// Resize the surrounding room. Walls/ceiling/floor are rebuilt
    /// from these dimensions; all other scene contents (the luminaire
    /// model + lights) are preserved.
    SetRoomSize {
        /// Width in metres (along world X).
        width: f32,
        /// Length in metres (along world Z).
        length: f32,
        /// Height in metres (along world Y).
        height: f32,
    },
    /// Set the base colour of a room surface. Linear sRGB, 0–1 per
    /// channel.
    SetSurfaceColor {
        /// Which surface to recolour.
        surface: RoomSurface,
        /// Red component, 0–1.
        r: f32,
        /// Green component, 0–1.
        g: f32,
        /// Blue component, 0–1.
        b: f32,
    },
    /// Set the **reflectance** of a room surface — the fraction of
    /// incident light that bounces off. Implemented as a neutral grey
    /// `base_color = (v, v, v)`, since for matte surfaces the bounced
    /// energy IS the base albedo. Use [`SceneCommand::SetSurfaceColor`]
    /// for a tinted surface, this one for a neutral plaster/concrete
    /// look you can tune by overall lightness.
    SetSurfaceReflectance {
        /// Which surface to retune.
        surface: RoomSurface,
        /// Reflectance, 0–1.
        value: f32,
    },
    /// Set the **emissive** colour of a room surface — light that the
    /// surface emits ON ITS OWN, independent of incident light. Useful
    /// for backlit signage panels or "the wall IS the colour" hero
    /// shots. Set to `(0, 0, 0)` to disable emission and return the
    /// surface to a normal lit material.
    SetSurfaceEmissive {
        /// Which surface to glow.
        surface: RoomSurface,
        /// Red, 0–1 linear sRGB.
        r: f32,
        /// Green, 0–1.
        g: f32,
        /// Blue, 0–1.
        b: f32,
    },

    // ── Editor-authoring commands ────────────────────────────────────
    //
    // These let a script BUILD / REFACTOR the GLDF product (not just
    // the 3D scene). The engine stays pure: it carries opaque JSON the
    // verb layer produced and never depends on `gldf-rs-editor`. The
    // WASM panel interprets these — resolving `{"@file":"name"}` byte
    // refs against dropped files and dispatching through the editor's
    // existing `apply_action_json` / `apply_wizard` paths.
    /// Apply one editor action, encoded as `{"action":"…","data":…}`
    /// JSON (the `#[serde(tag,content)]` `GldfAction` wire format).
    /// `data` may contain `{"@file":"name"}` byte-references the panel
    /// resolves against dropped/embedded files before dispatching.
    EditorAction {
        /// Serialized `GldfAction` (single object).
        action_json: String,
    },
    /// Apply a whole `WizardDraft` (the New-GLDF flow), encoded as JSON.
    /// Byte fields may use `{"@file":"name"}` refs.
    WizardApply {
        /// Serialized `WizardDraft`.
        draft_json: String,
    },
    /// Declare the files a script needs (the header block). The panel
    /// reads these pre-run to know what to prompt the user to drop.
    /// A no-op at run time (consumed before execution); logged so
    /// headless callers can introspect the manifest.
    NeedFiles {
        /// Filenames the script references (e.g. `"middle.ldt"`).
        names: Vec<String>,
    },
}

/// Tag identifying which room surface a [`SceneCommand::SetSurfaceColor`]
/// or [`SceneCommand::SetSurfaceReflectance`] targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomSurface {
    /// All four walls together. Treated as one material in the
    /// existing room builder, so they always change in unison.
    Walls,
    /// The floor plane.
    Floor,
    /// The ceiling plane.
    Ceiling,
}

/// State stored inside [`acadlisp::Interpreter::user_data`].
///
/// Direct mirror of `bimifc_lisp::IfcState`. The `bridge` field is the
/// novel bit — IFC is read-only at script time, but a luminaire viewer
/// needs writable scene state, so we hand the panel a slot to deposit
/// side effects.
pub struct GldfState {
    /// Currently loaded GLDF product. `None` until `(gldf-load …)` runs.
    pub product: Option<GldfProduct>,
    /// Active variant index. Defaults to 0; `(gldf-variant-set i)`
    /// changes it; functions that report "the current variant" read
    /// from here.
    pub active_variant: usize,
    /// Optional handle to the 3D scene. See [`SceneBridge`].
    pub bridge: Option<SceneBridge>,
    /// In-process log of [`SceneCommand`]s that have been requested.
    /// The CLI / tests inspect this; the WASM panel can clear it
    /// between runs.
    pub log: Vec<SceneCommand>,
}

impl GldfState {
    /// Empty state — no product loaded, no bridge.
    pub fn new() -> Self {
        Self {
            product: None,
            active_variant: 0,
            bridge: None,
            log: Vec::new(),
        }
    }

    /// Dispatch a [`SceneCommand`]. Calls the bridge if set; always
    /// records into [`Self::log`] so callers that don't wire a bridge
    /// can still introspect what a script would have done.
    pub fn dispatch(&mut self, cmd: SceneCommand) {
        self.log.push(cmd.clone());
        if let Some(b) = self.bridge.as_mut() {
            b(cmd);
        }
    }
}

impl Default for GldfState {
    fn default() -> Self {
        Self::new()
    }
}

/// Build an interpreter with every gldf-rs function registered.
///
/// Mirrors `bimifc_lisp::create_ifc_interpreter`. The returned
/// interpreter has an empty [`GldfState`] in its `user_data` slot;
/// supply a real product via `(gldf-load ...)` or by mutating
/// `interp.user_data` before running.
pub fn create_gldf_interpreter() -> Interpreter {
    let mut interp = Interpreter::new();
    interp.user_data = Box::new(GldfState::new());
    gldf_functions::register_all(&mut interp);
    bevy_functions::register_all(&mut interp);
    export_functions::register_all(&mut interp);
    editor_functions::register_all(&mut interp);
    interp
}

// -----------------------------------------------------------------------------
// Helpers shared by the function modules
// -----------------------------------------------------------------------------

pub(crate) fn state_mut<'a>(
    ctx: &'a mut acadlisp::ForeignContext<'_>,
) -> Option<&'a mut GldfState> {
    ctx.user_data.downcast_mut::<GldfState>()
}

pub(crate) fn state_ref<'a>(
    ctx: &'a acadlisp::ForeignContext<'_>,
) -> Option<&'a GldfState> {
    ctx.user_data.downcast_ref::<GldfState>()
}

pub(crate) fn expr_to_string(e: &acadlisp::Expr) -> Option<String> {
    match e {
        acadlisp::Expr::String(s) | acadlisp::Expr::Symbol(s) => Some(s.clone()),
        _ => None,
    }
}

pub(crate) fn expr_to_f32(e: &acadlisp::Expr) -> Option<f32> {
    match e {
        acadlisp::Expr::Integer(i) => Some(*i as f32),
        acadlisp::Expr::Real(r) => Some(*r as f32),
        _ => None,
    }
}

pub(crate) fn expr_to_usize(e: &acadlisp::Expr) -> Option<usize> {
    match e {
        acadlisp::Expr::Integer(i) if *i >= 0 => Some(*i as usize),
        acadlisp::Expr::Real(r) if *r >= 0.0 => Some(*r as usize),
        _ => None,
    }
}

/// Build an "argument-error" expression — modelled on bimifc's
/// convention of returning `Nil` plus pushing an explanation onto
/// `ctx.output` so scripts can keep running while the REPL surfaces
/// the diagnostic.
pub(crate) fn arg_error(
    output: &mut Vec<String>,
    name: &str,
    expected: &str,
    got: &[acadlisp::Expr],
) -> acadlisp::Expr {
    output.push(format!(
        "({name}) expected {expected}, got {} arg(s): {:?}\n",
        got.len(),
        got
    ));
    acadlisp::Expr::Nil
}
