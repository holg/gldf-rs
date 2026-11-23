# GldfImporter — UE 5.7 plugin for GLDF luminaires

Native Unreal Engine importer for **GLDF** (Global Lighting Data Format)
luminaire files, built on Epic's Interchange framework and the Rust
`gldf-unreal` crate (gldf-rs workspace).

Drop a `.gldf` into the level and you get **one draggable luminaire actor**:

- body mesh from the embedded L3D geometry (mm → cm, Nanite-ready),
- **one IES light per emitter** (LEO): a twin-head spot gets two beams, each
  with its own IES profile, lumens and CCT, oriented from the L3D LEO axes,
- all **variants** of the product, hot-swappable from a dropdown on the actor
  (`SelectedVariantName`) — photometry re-applies per emitter,
- the variant's **mounting info** (ceiling / wall / ground, recess depth,
  pendant length, wall mounting height, pole) → `SnapToSurface()` /
  `PlaceFromRay()` place the luminaire *the way it mounts*: recessed
  downlights sink into the ceiling, pendants hang, wall luminaires sit at
  their mounting height with their back on the wall, street-light heads get
  an auto pole. Click a floor and a downlight goes to the ceiling above it.

- an **illuminance heatmap** (`AGldfIlluminanceGrid`): the eulumdat core
  (bundled Rust) interpolates every emitter's IES/LDT, sums E = I(C,γ)·cos i/r²
  on a ground-draped grid with line-trace shadows, and shows min / avg / max,
  U0, Ud with an isolux colour ramp — in the editor (Recompute button) and in
  packaged games.

- **articulated parts**: the L3D part tree is rebuilt as components — one
  static mesh per part, a pivot per `<Joint>` with the manufacturer's axis
  limits — and `AimAt(worldPos)` solves the joint angles within those limits,
  so a gimbal downlight or a twin-head spot turns its real heads (lights ride
  on their part; the heatmap follows). Runtime loader today; the editor
  import still bakes one mesh.

- **runtime loading** (`UGldfRuntimeLoader`): a packaged game builds the same
  luminaire actor straight from a `.gldf` on disk — L3D body → transient
  `UStaticMesh`, every emitter's IES → transient `UTextureLightProfile`,
  variants / mounting / product data as the editor import stores them. No
  editor, no Interchange, no re-cook: users drop their own GLDF files in.

## Modules

| module         | type    | what                                                            |
|----------------|---------|-----------------------------------------------------------------|
| `GldfRuntime`  | Runtime | `AGldfLuminaireActor` + variant / mounting structs, snapping, auto pole, `AGldfIlluminanceGrid`, `UGldfRuntimeLoader` (build a luminaire from a `.gldf` at runtime); links the Rust `gldf_unreal` staticlib (eulumdat photometric core). Ships in packaged games. |
| `GldfImporter` | Editor  | Interchange translator (`.gldf`), scene pipeline (retypes the root actor), actor factory (builds lights + variants), bundled Rust staticlib. |
| `GldfSearch`   | Runtime | gldf-search.de **web panel** (`SGldfWebPanel`: the site in `SWebBrowser`, viewer links become Place actions), REST lookup of a document's file (`/api/doc`), UE-HTTP download, pluggable `UGldfSearchLibrary::PlaceHandler()`. Ships in packaged games. No native search UI, no Rust search lib — the site is the UI. |
| `GldfSearchEditor` | Editor | *Window ▸ Tools ▸ GLDF Search* tab (the web panel), binds the place handler to an Interchange import (assets under `/Game/GLDF` + actor snapped where the viewport looks). |

## Install

Copy (or symlink) `GldfImporter/` into `<Project>/Plugins/`. The Rust
staticlib + header (`gldf_unreal`, geometry + eulumdat photometry) are
bundled under `ThirdParty/GldfUnreal/` for Mac, Linux and Win64, so a C++
project builds without Rust installed. Binary-only evaluation builds:
`../package_eval_mac.sh [days]`.

**Packaging a game that uses the heatmap:** the heatmap material
(`/GldfImporter/M_GldfHeatmap`) is only soft-referenced from code, and the
cooker does not follow soft references into plugin content. Add to your
project's `DefaultGame.ini`:

```ini
[/Script/UnrealEd.ProjectPackagingSettings]
+DirectoriesToAlwaysCook=(Path="/GldfImporter")
```

Otherwise the packaged grid renders with the engine's default grey material.

## Workflows

- **Editor:** drag a `.gldf` onto the viewport → *Import Into Level*. Or use
  *GLDF Search* and press Place. Select the actor → Details ▸ GLDF: variant
  dropdown, aim buttons (Tilt / Spin / Reset), mounting info.
- **Blueprint / C++ (runtime):** `AGldfLuminaireActor::PlaceFromRay(Start,
  Dir)` snaps an existing luminaire onto whatever the ray hits;
  `ApplySelectedVariant()` after changing `SelectedVariantName`;
  `Emitters[i].Light` are the live `ULocalLightComponent`s.
- **Runtime `.gldf` (Blueprint / C++):**
  `UGldfRuntimeLoader::SpawnLuminaireFromFile(World, Path, Transform, Err)`
  then `PlaceFromRay()`; `PopulateLuminaire()` fills an existing actor;
  `FindGldfFiles(Dirs)` lists a folder. Emitters without rated lumens get
  their flux from the IES via the eulumdat core (`ApplySelectedVariant`).
- **Python (editor):** `Content/Python/gldf_place.py` — `place_gldf(path)`.
- **Illuminance:** drop a *GLDF Illuminance Grid* actor (or spawn one at
  runtime), set size / cell / receiver height, call `Recompute()`; read
  `Stats` and `Lux[]`, or `LuxAt(worldPos)`. `bShowMarkers` adds measurement
  poles (post from the ground to the receiver height, colour pad on top) and
  fills `Markers` (pad position + lux) for value labels; `bShowSheet` is the
  continuous false-colour sheet. The auto colour scale tops out at the 95th
  percentile. Each emitter's IES text travels on the luminaire actor
  (`FGldfEmitterPhotometry::IesBytes`), so no source file is needed after import.
- **Joints (Blueprint / C++):** `Joints[i]` (name, axis mask, min/max/step,
  `AnglesDeg`), `SetJointAngles(i, deg)`, `AimAt(worldPos)` (falls back to
  rotating the lights for baked single-mesh luminaires), `HasJoints()`.
  Angles use the L3D convention (degrees, X·Y·Z); the actor converts to UE.
- **Height adjustment:** `HeightOverrideCm` on the actor overrides the pole
  height (ground / pole-top luminaires, bollards) or the wall mounting height;
  `ReSnap()` re-runs the last snap (or finds the ground / wall itself).
  Ceiling luminaires dropped on a floor with no ceiling above are hung on a
  virtual ceiling (`DefaultCeilingHeightCm`, 2.8 m).
- **Search (runtime or editor):** host `SGldfWebPanel` anywhere (a tab, a game
  window); a click on a result's viewer link downloads the `.gldf` to
  `<Project>/Saved/GldfSearch/` and calls `PlaceHandler()` (bind it to
  `UGldfRuntimeLoader::SpawnLuminaireFromFile` in a game; the editor module
  binds Interchange). `DownloadAndPlace(docHex, variant)` / `PlaceFromUrl(url)`
  do the same without the panel.

## Rebuilding the Rust layer

```sh
# in the gldf-rs workspace
cargo build -p gldf-unreal --release
cargo run -p gldf-unreal --bin gen-header            # → crates/gldf-unreal/include/gldf_unreal.h
extra/unreal-plugin-interchange/build_thirdparty_libs.sh   # Mac + Linux + Win64 → ThirdParty/GldfUnreal/
```

After any change to `crates/gldf-unreal` the staticlib in `ThirdParty/` must
be refreshed, or UE links the stale lib ("Undefined symbols").

## Tests

- Rust FFI: `cargo test -p gldf-unreal` (mesh, LEO positions, L3D structure /
  joints, variants, mounting, photometry).
- Editor snapping smoke test: `../tests/run_snap_smoke.sh` (headless editor
  run in a host project, 7 placement cases on ceiling / wall / floor).

## Data-quality notes (real-world GLDFs)

- Most street-light GLDFs declare `<PoleTop/>` **without** `poleHeight` →
  the actor's `DefaultPoleHeightCm` (6 m) is used. `<PoleIntegrated/>` means
  the model *is* the bollard/pole and is stood on the ground.
- Wall `mountingHeight="0"` means unknown; some files carry unit slips
  (1 400 000 mm). Only 0.2–20 m is trusted.
- LEO `partName` ≠ emitter `EmitterObjectExternalName` in most files → the
  factory falls back to positional LEO↔emitter pairing (logged).
- Some L3Ds write the LEO position in mm instead of m (the emitter lands
  500 m from a 60 cm body — `tests/data/FLOODLIGHT_…_enriched.gldf`). The
  Rust side reports the file as-is; the actor pulls such a runaway emitter
  back onto the body and logs a warning (`AdjustLightsOutsideHousing`).
- Beyond that the importer is intentionally free of heuristics: a correct
  GLDF imports correctly; broken files are the publisher's problem.

## Known gaps

- Runtime-loaded luminaires are transient (not saved with the level); the
  editor import remains the way to author a level.
- Geometry is variant-invariant (variant 0's L3D); only photometry swaps.
- The editor (Interchange) import produces one baked static mesh without
  joints; articulated parts are built by the runtime loader only.
- Wall "back" direction of L3D models is assumed to be −Y (UE); tune
  `WallBackDirLocal` per actor if a manufacturer modelled it otherwise.

## License

Part of the gldf-rs workspace (AGPL-3.0-or-later); commercial licensing of
the plugin is available from the author.
