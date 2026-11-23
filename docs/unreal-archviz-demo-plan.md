# Unreal ArchViz demo — two packaged executables (Bistro outdoor / Sponza indoor)

Goal: two standalone demo builds that show GLDF luminaires lighting real
ArchViz scenes, as the marketing vehicle for selling the GLDF importer as an
ArchViz plugin (Fab) later.

- `GldfDemo_Bistro`  — Amazon Lumberyard Bistro **Exterior**, night scene, street/facade luminaires
- `GldfDemo_Sponza`  — Sponza atrium, **Interior**, downlights / spots / wall washers

Status 2026-10-26 (evening): P0 module split DONE + builds; mounting-aware
surface snapping implemented (Rust FFI + runtime actor + editor Place flow),
compiled AND verified by an editor smoke test in the Interior map
(`extra/unreal-plugin-interchange/tests/run_snap_smoke.sh`, 7/7 PASS: ceiling
recessed, pendant, floor→ceiling redirect, free-standing, pole, wall at
mountingHeight, floor→wall redirect). Licence question settled by the
user (sole author of every crate involved) — no blocker.

## The benchmark: Obscura (obscura.lighting)

Obscura® is a commercial "real-time lighting environment simulation software"
**built on Unreal Engine**, aimed at lighting designers and cities (urban,
landscape, architectural + interior lighting), with a paid Partner Program for
manufacturers ("Integration of selected luminaires", brand showrooms). Its
published capabilities: real-time 3D environment, "Adjust distributions,
intensities, optics, colors and CCT, positions" with instant rendering, a
"Brand-Independent Library", "Photometric Evaluation" (illuminance, uniformity,
gradients), real-time measurements, report layout editor, 30-day trial, LIT
Design Award 2023, Lux Futurum 2024. Not published anywhere: GLDF/IES/LDT
support, how luminaires are placed, pricing.

Where we are better, and must stay better:

| Obscura                                        | Us                                                                 |
|------------------------------------------------|--------------------------------------------------------------------|
| Manufacturers pay to be "integrated"           | ANY GLDF, from any manufacturer, drag it in — gldf-search.de has thousands |
| Closed library, curated per partner            | Open: own files, gldf-search, DIALux/Relux exports                 |
| Placement = move a light, set position         | Placement = **the GLDF knows how it mounts**: recessed downlights sink `recessedDepth` into the ceiling, pendants hang `pendantLength`, wall luminaires sit at `mountingHeight` with their back on the wall, street lights get a pole of `poleHeight` — one click on floor/ceiling/wall, no numbers typed |
| Per-fixture photometry                         | Per-EMITTER photometry (twin-head spots, LEO ≠ LEO), variants hot-swappable |
| Standalone app                                 | Plugin inside the user's own UE project → ArchViz pipeline, not a silo |

Still to match / beat: illuminance measurement (lux at a point / grid, false
colour) and report export. Both are v2 items below; the data for them (IES on
every light, lumens, CCT) is already on the actor.

## Done today (2026-10-26)

- `GldfRuntime` module (Runtime) holds `AGldfLuminaireActor`; `GldfImporter` +
  `GldfSearch` depend on it. `Config/DefaultEngine.ini` in the plugin carries
  CoreRedirects for assets saved before the move. Editor builds: three dylibs.
- Rust: `gldf_unreal_variant_mounting()` + `GldfMountingInfo` (kind, recess,
  pendant, wall height, pole height, surface/free-standing flags) per variant;
  5 FFI tests on real fixtures (Tria 2 ceiling-recessed 11 mm, Philips pendant
  500 mm, floodlight wall 20 m, Belviso free-standing, alurays none). Header +
  Mac staticlib refreshed in ThirdParty (Linux/Win64 libs still OLD — run
  `build_thirdparty_libs.sh` before shipping).
- Actor: `FGldfMountingInfo` per variant, `GetActiveMounting()`,
  `PlaceFromRay()`, `SnapToSurface()`, auto `PoleMesh` (engine cylinder, hard
  ref so it cooks), `WallBackDirLocal` tunable (L3D wall convention assumed
  -Y in UE — verify on a real wall luminaire). Rules: ceiling → body top flush
  (−recess / +pendant); wall → back face on wall, origin at mountingHeight
  above the floor found below; floor → stand on it, or pole top for ground
  luminaires; floor click redirects to the ceiling above (ceiling kind) or the
  wall ahead (wall kind); emitters always kept ≥ 0.5 cm on the room side.
- Editor `PlaceFromUrl` (GLDF Search panels) now calls `PlaceFromRay`.
- Engine `Apple_SDK.json` MaxVersion raised 26.9 → 27.9 (Xcode 27.1); backup
  `Apple_SDK.json.orig-26.9` next to it. Re-apply after any engine update.
- Xcode 27.1 ships without the Metal toolchain → editor exits at startup
  ("cannot execute tool 'metal'"). Installed via
  `xcodebuild -downloadComponent MetalToolchain` (839 MB).
- Smoke test harness: `tests/snap_smoke.py` + `tests/run_snap_smoke.sh`
  (UnrealEditor-Cmd -ExecutePythonScript, needs a real RHI — `-nullrhi`
  crashes in viewport placement during spawn_actor_from_class).

## Next (in order)
1. Interactive check with REAL imports (the smoke test uses a cube body):
   open `tmp/GldfArchTest`, GLDF Search → place SLV Tria 2 looking at the
   ceiling and at the floor; a wall luminaire at a wall (verifies the
   `WallBackDirLocal` assumption on a real L3D); a street light on the ground.
2. Runtime drag-drop in the demo game module: cursor trace → `PlaceFromRay`
   on a luminaire spawned from a pre-imported asset (Interchange is editor-only
   in our setup; v1 demo picks from the cooked luminaire set).
3. Then the project/scenes work below.

## Hard prerequisites (do these first)

### P0 — the plugin has NO runtime module
`GldfImporter.uplugin` declares both modules (`GldfImporter`, `GldfSearch`)
as `"Type": "Editor"`. Editor modules are not compiled into a packaged game,
so `AGldfLuminaireActor` (currently in `GldfImporter`) would not exist in the
.exe and every cooked map containing a luminaire would fail to load.

Fix: split the plugin into

| module         | type    | contents                                                        |
|----------------|---------|-----------------------------------------------------------------|
| `GldfRuntime`  | Runtime | `AGldfLuminaireActor` + `FGldfEmitterPhotometry/Variant/Component`, `ApplySelectedVariant`, aim helpers. Deps: Core, CoreUObject, Engine only. |
| `GldfImporter` | Editor  | Interchange translator / pipeline / actor factory, Rust staticlib link. Depends on `GldfRuntime`. |
| `GldfSearch`   | Editor  | unchanged                                                       |

`GldfLuminaireActor.cpp` is already runtime-clean (includes only
StaticMeshComponent, LocalLightComponent, TextureLightProfile; the
`PostEditChangeProperty` is behind `WITH_EDITOR`). The `GLDFIMPORTER_API`
macro becomes `GLDFRUNTIME_API`. Rebuild `tmp/GldfArchTest` editor to verify,
then re-import one GLDF (existing cooked actors in `Content/GLDF` reference
the old class path → re-import or add a `[CoreRedirects]` ActiveClassRedirect
`/Script/GldfImporter.GldfLuminaireActor` → `/Script/GldfRuntime.GldfLuminaireActor`).

### P1 — a `.exe` needs a Windows UE 5.7 install
UE cannot package Win64 from macOS. Only the Rust ThirdParty staticlibs are
already cross-built for Win64 (`ThirdParty/*/lib/Win64/*.lib`, via
`build_thirdparty_libs.sh`); the plugin C++ + game compile on the Windows box
with the project. Plan: develop + package a Mac `.app` here first (same
content, same code), then run the identical UAT command on a Windows machine
(or VM / cloud Windows with UE 5.7) for the two `.exe` builds.

### P2 — scene assets on disk + licences

| scene  | source on disk                                                   | licence   | notes |
|--------|------------------------------------------------------------------|-----------|-------|
| Bistro | `~/Downloads/Bistro_v5_2/BistroExterior.fbx` (120 MB, 2.8 M tris) + `Textures/*.dds` + `san_giuseppe_bridge_4k.hdr`; 1.6 GB total | CC BY 4.0 | README: becomes a night scene when rendered without analytic lights / env map — exactly what we want (our luminaires are the only light). DirectX-style normals = UE convention, no green flip. Specular tex packs Occlusion/Roughness/Metal in R/G/B → one master material. |
| Sponza | Crytek Sponza glTF: `../eulumdat-rs/assets/sponza/Sponza.gltf` (50 MB); FBX variant in `../_to_sort/Fyrox/examples/data/sponza/Sponza.fbx` | CC BY 3.0 | Quick start. Optional upgrade: Intel Sponza (2022, CC BY 4.0, PBR, ~7 GB) looks far more ArchViz-grade — decide after the Crytek version works. |

Attribution is mandatory for both (credits screen in the demo + README +
Fab listing text).

### P3 — licence of the plugin itself
Plugin README: "Inherits the gldf-rs workspace's AGPL-3.0-or-later … dual-licence
question still applies for commercial UE shipping." A paid Fab plugin that
statically links gldf-rs / l3d-rs / eulumdat-rs needs an explicit commercial
licence grant from the (sole) author of those crates — a one-line decision,
but it must be written down before the first sale. The eval-expiry mechanism
(`package_eval_mac.sh`) already exists for time-limited trials.

## Scene fixtures carry the GLDF photometry (2026-10-26 night, user request)
Bistro's own period fixtures — 26 lamp-post glass heads (`StreetLight_Glass_*`,
6–7 m) and 5 bracket lanterns (`Lantern_Wind_*`) — now get the GLDF luminaire
placed AT the fixture's light source with the GLDF body hidden (`hide_body` in
the place script: body + auto pole off), so the historic lantern remains the
visible source and the GLDF supplies the distribution. Products: Siteco
"Laterne iQ", LEDVANCE "Urban Lantern" (heads, round-robin), Ansell "Latina
Half Lantern" (wall). Generic pole/wall sites are only used when a scene has
no fixtures. The L key now also zeroes `EmissiveStrength` on scene materials
with an emissive texture (lanterns, string lights) so "lights off" is dark;
`-GldfLightsOff` renders that state for checks.
Environment controls (2026-10-27, user: "moon much too bright"): arrows
up/down scale the moon (directional light, sky light follows at ½), left/right
scale the haze (height-fog density), 0 = both off (GLDF light only); values in
the HUD. Default moon lowered to 0.3 lx (real full moon ≈ 0.25), sky 0.35, haze 0.002 (0.006 looked washed out with 64 lights in volumetric fog).
Fixture coverage: 28 post heads (cap raised to 40), 5 bracket lanterns, 4 small
wall lights, 25 façade spotlights (`Spotlight_Glass_Emissive`; PiL TYK+ 15,
ESYLUX SUN AFL, Targetti JEDI; aimed 35° down-and-out from the wall behind
them), 2 bollards → 64 GLDF luminaires. Left emissive-only: string lights and
shop signs (not luminaires).
Picking (2026-10-27, user: "massive problems to select"): a direct trace hit
on the luminaire actor wins; otherwise the luminaire whose EMITTER is closest
to the aim ray (≤ 4°) or to the hit point (≤ 2.5 m) is selected — so clicking
the scene lantern that hosts a hidden GLDF body works, as does a tiny far
head. Tab cycles luminaires nearest-first. The HUD draws a ring + name +
distance at the selected emitter. `-GldfPick` picks once for automated checks.
The small emissive wall lights (`building_01_bottom_*`, ~36 cm) are fixtures
too (Ansell lantern assigned).
Controls (2026-10-27): left click select · right click place copy · **N / B
next / previous GLDF product** (replaces the selected luminaire with another
product from the level, same place / mounting / body visibility) · V variant ·
Del remove copy · L all luminaires + scene emissives on/off · G heatmap · H
help. Specs panel on selection: manufacturer, product, source file, product
k/n, variant k/n, article / GTIN, emitters, lm, W, lm/W, CCT, mounting with
recess / pendant / pole, body size. Product identity comes from the GLDF
header / ProductMetaData via `gldf_unreal_product_string` and lives on the
actor (`Manufacturer`, `ProductName`, `ProductNumber`, `SourceFile`; per
variant `ProductNumber`, `Gtin`, `Watts`). "Change the underlying GLDF" at
runtime = swap among products already cooked into the level; importing a NEW
.gldf in a packaged build remains the v2 runtime-import item.
SCALE BUG (found 2026-10-27 via the SPILLO "tiny bulb"): since commit 539dd1e
(2026-07-02) the Rust mesh payload bakes the L3D unit scale and returns CM,
but `InterchangeGldfTranslator::PositionToUEBasis` still multiplied by 0.1
(the June fix for the then-mm payload) → every luminaire BODY imported 10×
too small (lights were right: `position_cm`). Fixed: translator applies the
Y flip only. All GLDF assets must be re-imported.
"Tiny bulb acting as a spotlight" (user, 2026-10-27): PiL SPILLO is a 17 cm
`<PoleIntegrated/>` HEAD modelled top-down with the LEO at the top; standing it
on the ground put the emitter at ankle height. Rule now: PoleIntegrated with a
body < 60 cm tall = head → auto pole (bollard role uses a 1 m pole via
`DefaultPoleHeightCm`); taller PoleIntegrated bodies still stand.
Self-shadowing housings: when the body AABB extends ≥ 2 cm in front of a LEO
along its emission axis (LEO on the far face, emitting through the housing),
`AGldfLuminaireActor::AdjustLightsOutsideHousing()` (PostInitializeComponents +
ApplySelectedVariant, idempotent) moves the light just outside the body —
otherwise VSM draws a dark disc under the luminaire. Cannot be done in the
factory: the static mesh has no bounds yet while the actor factory runs.
Heatmap gotcha: with emitters inside scene lanterns every shadow trace ended
in the lantern glass → 100 % "shadowed". `AGldfIlluminanceGrid::EmitterClearanceCm`
(70) stops the trace short of the emitter. Per-layer diagnostics (`layer N:
direct max / after occlusion max / cells shadowed`) are logged on Recompute.
Fixture heads default to the product's brightest variant (Siteco Laterne iQ
ships 18 identically named dimming variants, 537–2967 lm).
Gotchas: editor `duplicate_actor` = text export → non-editable UPROPERTYs
(the IES byte array) were dropped on copies → `IesBytes` is now EditAnywhere +
AdvancedDisplay. Automated game frames drifted when a hand touched the mouse →
`-GldfShot` runs ignore look/move input.
Open: packaged Shipping app runs but writes no log/screenshot when launched
from a shell (window focus / sandbox?) — verify by double-clicking the .app.

## STATUS 2026-10-26 late: Bistro v1 DONE on Mac
- Opening view = the Vespa (PlayerStart framed by the place script; pawn
  snapped to it at BeginPlay + 0.5 s, 40 cm above the start to clear furniture).
- Colours correct, road intact, 10 GLDF luminaires, heatmap (G) works.
- `Scripts/package.sh Bistro Mac Shipping` → `Dist/GldfDemo_Bistro_Mac/`
  (2.7 GB staged folder: `GldfDemo-Mac-Shipping.app` + `GldfDemo/Content/Paks`).
  Cook ≈ 18 min first time. Win64 `.exe`: same script on a Windows UE 5.7 box.
- Verification loop: `Scripts/run_game_shots.sh <name> "-"|"x,y,z,p,y" map delay [-GldfHeatmap]`.

## Bistro build-out (started 2026-10-26)

Project lives OUTSIDE gldf-rs: `../gldf-unreal-demo/GldfDemo/` (C++ game
module `GldfDemo`, plugin symlinked at `Plugins/GldfImporter`, `.gitignore`
excludes `Content/Scenes/` — reproducible via scripts). Not a git repo yet.

- `Source/GldfDemo/` — `AGldfDemoGameMode` (engine `ADefaultPawn` = free fly)
  + `AGldfDemoPlayerController`: LMB pick luminaire, RMB place a copy where
  you look (template spawn → `PlaceFromRay`), V next variant, Del remove
  placed copy, L all luminaires on/off.
- `Scripts/import_bistro.py` + `run_import_bistro.sh` — Interchange scene
  import of `BistroExterior.fbx` into `/Game/Maps/Bistro`, removes the FBX
  lights/cameras, adds moon (0.3 lx, 9000 K), sky light, height fog, manual
  exposure PPV, PlayerStart at street centre, saves, quits.
- `Scripts/fetch_bistro_gldfs.py` — pulls real GLDFs from gldf-search.de
  (`POST /api/search` with `mounting_place`/`mounting_type` filters, docs via
  `/api/doc/<hex>`, files via `/gldfs/<path>`) into `SourceGldf/Bistro/` +
  `manifest.json`. Current set: 4 pole-top street lights (PiL KYRO+, LEDVANCE
  Streetlight Flex, Thorn AVN, Siteco SL 21), 1 bollard (PiL SPILLO,
  PoleIntegrated), 3 wall (PiL MIMIK, SLV BIG THEO, SG Piano).
- `Scripts/place_bistro_luminaires.py` + `run_place_bistro.sh` — scene
  analysis by traces (open-sky ground = street, facades at 2.8 m), imports
  each GLDF with the GLDF per-translator pipeline list as OverridePipelines,
  snaps via `SnapToSurface`, duplicates per site, saves, quits.
- STATUS 2026-10-26 evening: Bistro level BUILT — 1296 mesh actors, 625 PNG
  textures, 131 material instances (0 unmatched), night setup, 10 luminaires
  placed (6 pole heads @6 m, 2 wall @3.2 m, 2 bollards). Verified via logs;
  first game-mode screenshot pending (`Scripts/run_game_shots.sh`).
- REGRESSION FOUND+FIXED: `InterchangeGldfPipeline.cpp` hard-coded
  `/Script/GldfImporter.GldfLuminaireActor`; after the module split every
  import silently produced a plain `Actor`. Now `StaticClass()->GetPathName()`.
- Interchange gotcha: a second scene import of the same source is a RE-import
  keyed on the `SceneImport_<name>` asset — it updates tracked actors instead of
  spawning; with those actors deleted nothing appears. Delete `/Game/GLDF/<set>`
  before re-importing (the place script does).
- Interchange DDS translator rejects BC1/BC3 → convert with Pillow
  (`convert_bistro_textures.py`), import PNG, rebuild materials by name
  (`bistro_materials.py`: 2 masters, ORM packing, masked where BaseColor has alpha).
- Bistro texture lessons: normals are BC5/ATI2 two-channel → reconstruct Z in
  conversion or every wall looks like a golf ball; Specular R (documented as
  AO) is ALL ZERO → never wire it as AO or all indirect/sky light dies;
  the `*_Wet*` / `*Leaves_BLENDSHADER` sheets are NOT overlays — they are the
  road surface (hiding them opened a hole to the sky: "the street looks
  cringe"). Keep every static mesh visible; the grey-sheet look came from the
  broken materials. Luminaire L3D parts without MTL get UE's grid checker →
  actor assigns plugin `M_GldfLuminaireBody` (dark painted metal) as fallback.
  `unreal.Rotator(roll, pitch, yaw)` — not (pitch, yaw, roll); the first moon
  pointed at the sky. UE default light attenuation is 10 m → actor now sizes
  reach from lumens (0.3 lx isophote, 5–60 m).
- Scripted UE materials: a TextureSampleParameter's DEFAULT texture must match
  the sampler type (Masks ↔ TC_MASKS, Color ↔ sRGB). One mismatch ("Sampler
  type is Linear Color, should be Color for WhiteSquareTexture") fails the
  master → every instance renders the grey default and the whole scene looks
  like beige clay (user: "I know the Vespa is blue!"). Now: ORM sampler =
  Masks with a generated 4×4 `T_DefaultORM` (AO 1, rough 0.8, metal 0).
  Interchange scene imports also leave per-COMPONENT material overrides on
  every actor (its own texture-less materials) → assign instances per
  component by slot name, not only on the mesh assets (fixed; colours back).
  `MaterialExpressionVertexColor`'s colour pin is "" (unnamed), not "RGB";
  unlit overlays need `EyeAdaptationInverse` to survive night exposure.
- Night look: moon 1 lx @9000 K pitch −35 yaw 60, sky light 0.5, fog 0.006,
  auto-exposure histogram EV 0–7 bias +0.5 (`tune_bistro_night.py`).
  Verification loop: `run_game_shots.sh <name> "x,y,z,pitch,yaw"` runs the
  real game (`-GldfCam`/`-GldfShot` command line) and writes Saved/Shots/*.png.
- Three downloaded GLDFs (Siteco SL21, SLV BIG THEO, SG Piano) ship an L3D with
  an EMPTY structure (tool "rolf") — gldf-search `has_3d` only means "L3D
  present". Fetch script now requires a real .obj inside the L3D.
- Data lessons: `<PoleTop/>` almost never carries poleHeight → actor
  `DefaultPoleHeightCm` (6 m); `<PoleIntegrated/>` = the model IS the bollard →
  stands on the ground, no auto pole; wall `mountingHeight` is "0" for unknown
  and sometimes 1400000 (SLV) → only 0.2–20 m trusted.

## eulumdat core in the runtime → illuminance heatmap (started 2026-10-26 night)

User priority: "integrate the eulumdat core completely so we can calc the
heatmap". Design (implemented, first frames pending):

- Rust `crates/gldf-unreal/src/ffi_photometry.rs`: `gldf_unreal_ldc_open(bytes,len)`
  parses IES or LDT text through eulumdat (`IesParser::parse` / `Eulumdat::parse`);
  `gldf_unreal_ldc_intensity_cd(h, C, γ)` = `Eulumdat::sample` (symmetry-aware,
  interpolated) × flux/1000 → absolute cd; `gldf_unreal_illuminance_grid(poses,
  n, grid, cell_offsets, out)` = direct illuminance E = I(C,γ)·cos i / r² per
  EMITTER LAYER on a planar grid (metres, any consistent frame). Tests in
  `tests/photometry_ffi.rs` (round trip of importer IES bytes, inverse square,
  two-emitter layering).
- The Rust staticlib now links into **GldfRuntime** (runtime module; GldfImporter
  inherits it) so games have the photometric core.
- `FGldfEmitterPhotometry.IesBytes` — the factory stores each emitter's
  variant-resolved IES text on the actor (a few KB) → no editor needed later.
- `AGldfIlluminanceGrid` (GldfRuntime): square grid (SizeCm/CellCm) draped on
  the ground by traces, receiver height, per-emitter layers from the FFI,
  **occlusion by UE line trace** per (cell, emitter), summed → `Lux[]`, stats
  (min/avg/max, U0 = Emin/Eavg, Ud = Emin/Emax), isolux colour ramp on a
  `ProceduralMeshComponent` (vertex colours) with plugin material
  `/GldfImporter/M_GldfHeatmap` (unlit translucent; made by
  `Scripts/make_plugin_materials.py`). `Recompute()` is CallInEditor.
- Demo: **G** toggles the heatmap around the player; HUD shows a legend,
  stats and "here: N lx"; `-GldfHeatmap` on the command line for automated
  shots.
- Photometric frame mapping: UE light local +X = nadir (γ=0), +Y = LEO width =
  C0, +Z = LEO height = C90. C-plane handedness under UE's left-handed frame is
  UNVERIFIED for asymmetric distributions (street lights!) — validate against
  eulumdat's own isolux for one luminaire before trusting street-light lobes.
- Not yet: indirect light (eulumdat has radiosity for rooms — Sponza candidate),
  point-and-click lux probe, export of the grid (CSV / eulumdat AreaResult).

## Project layout

One UE project, two maps, packaged twice. A combined single exe with a scene
menu is then a one-line config change later.

```
<new repo or tmp/>/GldfDemo/
  GldfDemo.uproject            (EngineAssociation 5.7, C++ project, game module "GldfDemo")
  Plugins/GldfImporter -> symlink to gldf-rs/extra/unreal-plugin-interchange/GldfImporter
  Content/Maps/Bistro.umap     Exterior, night, HDRI off, ~8-12 GLDF luminaires
  Content/Maps/Sponza.umap     Interior, ~10-15 GLDF luminaires
  Content/Scenes/Bistro/...    imported FBX meshes + master material + textures
  Content/Scenes/Sponza/...
  Content/GLDF/...             imported luminaire assets (via our importer, in editor)
  Content/UI/...               HUD widget, credits
  Source/GldfDemo/             fly pawn, demo game mode, luminaire HUD glue
  Config/DefaultGame.ini       GameDefaultMap per build (swapped by the package script)
```

Do NOT grow `tmp/GldfArchTest` into the demo — it is the ArchvisProject template
with Datasmith/CAD/C4D/AxF/Lidar plugins enabled, all irrelevant and all
bloat for packaging. Fresh project, only `GldfImporter` + `Interchange` enabled.
Content is multi-GB → separate repo with Git LFS, or keep it out of git and
script the import (FBX/glTF paths are deterministic).

## Demo feature set

### v1 (what the two exes ship)
- Pre-placed luminaires: real GLDFs downloaded from gldf-search.de, imported in
  the editor with the existing importer (drag Import Into Level — the
  pipeline-stack path that is known to shine). Street/bollard/facade for Bistro;
  recessed downlights, track spots, pendants for Sponza.
- Fly camera (WASD + mouse), F11 fullscreen, F12 screenshot.
- HUD: list of placed luminaires (manufacturer / product / variant), click to
  fly-to, **variant dropdown** (`ApplySelectedVariant` is already
  BlueprintCallable → works at runtime), dim slider, IES on/off toggle,
  "all lights off" to show the scene is lit only by GLDF photometry.
- Bistro: night only (remove sun + sky from the FBX import; keep emissive signs).
- Credits panel: Bistro CC BY 4.0, Sponza CC BY 3.0/4.0, gldf-search.de source list.
- Lumen GI + hardware RT off by default (Mac/Metal + mid-range Windows GPUs).

### v2 (the actual product pitch — runtime import)
- Drag a `.gldf` onto the running exe → luminaire appears at the cursor.
- `InterchangeEngine` is a **Runtime** module in 5.7, so the manager exists in
  game; what blocks us is our own code: `GldfImporter` links `UnrealEd` (private
  dep) and the factory builds the static mesh via editor build paths. Runtime
  path: `UStaticMesh::BuildFromMeshDescriptions` (runtime-safe) + transient
  `UTextureLightProfile` filled from the Rust IES bytes (IESFile is a Runtime
  module, so `FIESConverter` is available). This is what makes the plugin
  worth paying for vs. "yet another FBX+IES workflow"; do it after v1 ships.

## Packaging commands

Mac (verify here):
```sh
UE="/Users/Shared/Epic Games/UE_5.7"
"$UE/Engine/Build/BatchFiles/RunUAT.sh" BuildCookRun \
  -project="$PWD/GldfDemo/GldfDemo.uproject" -noP4 \
  -platform=Mac -clientconfig=Shipping -cook -allmaps=false \
  -map=/Game/Maps/Bistro -build -stage -pak -archive \
  -archivedirectory="$PWD/tmp/dist/GldfDemo_Bistro_Mac"
```
Same with `-map=/Game/Maps/Sponza`; a wrapper script sets `GameDefaultMap`
in `Config/DefaultGame.ini` before each run. Windows: identical command with
`RunUAT.bat … -platform=Win64`.

## Order of work
1. P0 module split + rebuild GldfArchTest editor (half day, Mac, in gldf-rs).
2. Fresh `GldfDemo` project, plugin symlinked, Bistro Exterior FBX + Sponza glTF
   imported, master materials, night lighting (1–2 days, mostly editor work).
3. Pick + import 20–25 real GLDFs, place them (1 day; LEO data quality of
   public GLDFs is poor — pick products that import cleanly, do not add
   heuristics to the importer).
4. Game module: fly pawn, HUD, variant/dim controls, credits (1–2 days).
5. Package Mac `.app` for both maps; fix cook errors (half day).
6. Windows machine: clone, package both `.exe` (half day once a box exists).
7. v2 runtime import (separate milestone, multi-day).


## 2026-10-27 (evening) — runtime GLDF loading + clickable menu

- **Users load their own `.gldf` in the packaged demo** (`UGldfRuntimeLoader`,
  GldfRuntime module): L3D → `UStaticMesh::BuildFromMeshDescriptions`
  (fast build, box collision, normals/tangents via `FStaticMeshOperations`),
  IES text → `UTextureLightProfile` through the engine's `FIESConverter`
  (RGBA16F platform data, same recipe as `UTexture2D::CreateTransient`),
  lights/variants/mounting/product strings mirrored from the actor factory.
  Verified in-game with four fixtures (floodlight wall @ 20 m, Belviso floor
  stand, Philips pendant lying on the pavement outdoors, Tria 2 on a wall).
- Demo: `M` menu (pure Slate, `SGldfDemoMenu`): product + variant dropdowns,
  "Your own GLDF files" list (`~/Documents/GldfDemo`, `<dist>/GLDF`,
  `<project>/UserGldf`, `-GldfDir=`) with load/rescan, place/next/delete,
  lights + heatmap checkboxes, moon/haze log sliders, env off, help.
  Cursor visible by default, mouse-look only while RMB is held, rays go
  through the cursor ("where you point"); `P` = place copy (RMB freed).
  Automation: `-GldfLoad=<file>`, `-GldfLoadNext` (menu/O path).
- Fixes found on the way: LEO positions in mm (floodlight fixture) → actor
  clamps runaway emitters onto the body (`tests/leo_ffi.rs` documents it);
  emitters without rated lumens take the flux from the IES (eulumdat core).
- `package.sh` copies `UserGldf/*.gldf` + README into `<dist>/GLDF/`.

## 2026-10-27 (night) — real menu bar, open dialog, gldf-search in the game, measurement poles

- User asks: "the heatmap is hard to see… mini poles on the floor… adjust the
  height… calced value (like Obscura)", "move and place gldf files, adjust the
  bollard height", "why no REAL menu", "choose from wherever we want",
  "integrate gldf-search" → "we already have the gldf-search plugin!".
- Plugin: `AGldfIlluminanceGrid` measurement poles (`bShowMarkers`,
  `MarkerSpacingCm`, `Markers[]`), 95th-percentile auto scale; actor
  `HeightOverrideCm` + `ReSnap()` (+ last-snap memory, ground multi-trace past
  scene fixtures, `DefaultCeilingHeightCm` virtual ceiling outdoors).
- **GldfSearch split**: `GldfSearch` is now a Runtime module (cabi search +
  HTTP download + `SGldfSearchPanel` public, `PlaceHandler()` delegate);
  `GldfSearchEditor` (new) holds the Window▸Tools tab, the Interchange
  placement and the CEF web panel. Verified in-game: search "bollard" → 10
  hits → download → runtime loader → placed (`-GldfSearchTest=`).
- Demo: Slate **menu bar** (`SGldfDemoMenuBar`, FMenuBarBuilder: File /
  Luminaire / Analysis / View / Help with product, variant, measurement
  height, moon, haze submenus); **native Open dialog** (Mac NSOpenPanel in
  `GldfDemoFileDialogMac.mm`, Windows GetOpenFileName in
  `GldfDemoFileDialog.cpp`, Cmd/Ctrl+O); **search window** (F) hosting the
  plugin panel; **move mode** (X, follows the cursor, click drops, Esc
  cancels); turn 45°; pole / mounting height slider and +/- keys;
  measurement height slider + PgUp/PgDn; poles/sheet toggles; HUD lux labels
  above each pole (nearest 90, overlap-free). Automation: `-GldfHeatmapHeight=`,
  `-GldfSearch`, `-GldfSearchTest=<query>`, `-GldfHeightTest=<cm>`.
- Known: `SGldfSearchPanel::Refresh` runs the cabi HTTP calls blocking on
  the game thread (short hitch per search). A WorkingPlane luminaire placed
  outdoors stands on the pavement (as the file says).

## 2026-10-28 — your own building (USD/USDZ) + web-only gldf-search

- User direction: move away from baked scenes; their scan tool exports IFC and
  USDZ. UE 5.7: USDStage is a Runtime module (Win64/Mac/Linux) → runtime
  `.usdz` loading works in a packaged app; there is NO IFC importer in 5.7
  (Datasmith IFC gone). IFC = later, via web-ifc core or scan-side export.
- Demo: File ▸ Open scene (USD/USDZ)… (`AGldfDemoPlayerController::LoadScene`):
  spawns `AUsdStageActor` 600 m from the Bistro (SceneOrigin), opens the stage
  first, applies `PhysicsCollisionAPI` to every Mesh/Cube/… prim (collisionEnabled
  defaults true, approximation none → complex-as-simple through the plugin's
  own path), then loads; on `OnStageLoaded` flies the player into the bounds,
  sets exposure bias 0 (scans are white / baked), re-centres the heatmap
  (now on the pawn, not the lagging camera). Verified with the user's Apple
  RoomPlan exports in ~/Downloads (parametric walls/floor/furniture, Y-up,
  metersPerUnit 1): 24 geometry prims with collision, luminaires snap to its
  walls/floor, heatmap works inside. Automation `-GldfScene=<file>`.
- Snapping additions: Ground-kind luminaire aimed at a wall → the floor in
  front of it; auto colour scale = 95th percentile of LIT cells only.
- **gldf-search: native dropped** ("dead end, web is easier to maintain"):
  GldfSearch (Runtime) = `SGldfWebPanel` (site in SWebBrowser; viewer links →
  Place) + `UGldfSearchLibrary` (REST `/api/doc/<hex>` → `/gldfs/<path>` URL,
  UE-HTTP download, `PlaceHandler()`); SGldfSearchPanel, the cabi staticlib,
  ThirdParty/GldfSearch and the gldf-search step in build_thirdparty_libs.sh
  are gone. GldfSearchEditor = Window ▸ Tools tab (web panel) + Interchange
  placement. Demo F window hosts the web panel; verified `-GldfSearchTest=<hex>`
  REST round trip and that the page loads in-game (document state completed).
- **Mac WKWebView is a native overlay** (Apple backend paints into a texture
  only for 3D widgets): UE screenshots cannot show it, so the in-game panel
  must be checked by eye. CEF (Windows/Linux) renders off-screen into Slate.

## 2026-10-28 (later) — synthetic ceiling, cooked heatmap material

- RoomPlan exports have no ceiling: `AddCeilingIfMissing()` builds one from the
  floor footprint at the wall-top height (engine cube, body material) and
  attaches it to the stage; snapping redirects added: ceiling luminaire on a
  wall hit → ceiling above; ground luminaire on a wall hit → floor below.
- **Packaged heatmap was grey ("sand")**: `/GldfImporter/M_GldfHeatmap` is only
  soft-referenced, so the cooker skipped it (manifest proved it). Fixed with
  `+DirectoriesToAlwaysCook=(Path="/GldfImporter")` (+ `/Engine/BasicShapes`)
  in the demo's DefaultGame.ini; README tells plugin users to do the same.
- Apple options reviewed for the user's scan tool: RoomPlan parametric/mesh
  exports carry no ceiling/textures; ARKit scene reconstruction (ARMeshAnchor,
  classified incl. ceiling) can run alongside RoomPlan since iOS 17 and should
  be exported as a second layer; textures via camera projection later.
- Drape rule: `DrapeMaxRiseCm` (120) — the measurement plane looks past
  furniture tops (RoomPlan Storage boxes are 1.7 m; pads were landing near the
  ceiling) down to the floor; cells with no surface are skipped and not counted.
- **"Open does nothing" in the packaged app (2026-10-28):** Shipping started
  fullscreen; a free-standing modal NSOpenPanel opens behind the fullscreen
  space while the game thread waits on it. Fix: the panel runs as a SHEET on
  the game window (`beginSheetModalForWindow` + `runModalForWindow`), and
  `DefaultGameUserSettings.ini` defaults the app to a 1920×1080 window
  (Alt+Enter for fullscreen). `-GldfOpenTest` + the `listwin` Swift helper
  (scratchpad) verified the panel window appears for the demo process.
- Dialog, final form (2026-10-29): `GldfDemoFileDialog::OpenFilesAsync` — a
  non-blocking sheet on the game window, callback on the game thread. A
  blocking modal froze the game; a free panel was dismissed by the engine after
  ~6 s and would sit below UE's fullscreen window (CGShieldingWindowLevel()+1).
- **ROOT CAUSE of "Open does nothing" + black gldf-search in the packaged app
  (2026-10-29): App Sandbox.** UE signs Mac packages with sandbox entitlements
  by default (Development: Sandbox.Server; Shipping: Sandbox.NoNet = no
  network). Sandboxed without user-selected-files / network-client
  entitlements: no NSOpenPanel, WebKit black, ~/Documents unreadable. Fixed via
  XcodeProjectSettings → Unrestricted.entitlements. Shipping also cannot log on
  an installed engine; the Development package logs to
  ~/Library/Logs/GldfDemo and is the better demo/diagnostic build.
- Verified the non-sandboxed Development package (`Dist/GldfDemo_Bistro_Mac/GldfDemo.app`):
  Open sheet appears, a file was picked, gldf-search.de loads in the web panel
  (document state completed). The picked file `Lichtdaten/product-omaz885.gldf`
  is a 2022 "GLDF Edit" rc.1 file (FormatVersion 0.0.0) that gldf-rs rejects
  ("missing field @fileId", some rc.1 reference element); 19 of the 20 files
  in that folder load. Legacy rc.1 support is a gldf-rs question, not a demo one.
- **Aim mode (R)** (user: "some handle to adjust the lighting position / orbiting
  of the light output"): the selected luminaire's emitter lights follow the
  point under the cursor (shortest-arc rotation of each light component, roll
  kept so C-planes stay put; the housing stays mounted — gimbal spots stay in
  the ceiling, pole heads keep their pole); HUD draws beam lines + target ring
  + distance; click sets, Esc restores; heatmap refreshes. `-GldfAimTest`.
  Note: body geometry does not tilt with the beam (no L3D joint animation yet).

## 2026-10-29 — animated L3D joints

- User: "make the l3d joints animated! this is what obscura does". Rust:
  `mesh_payload::collect_l3d_structure` (nodes = geometries + joints, parent
  links, authored local pose, axis limits, DefaultRotation, per-part mesh in
  node space in cm, LEO → node) + `ffi_l3d.rs` (`gldf_unreal_l3d_open/node/
  node_name/node_mesh_open/leo/close`); l3d-rs `Axis` fields made pub. Test
  `tests/l3d_ffi.rs`: Tria 2 = base + 2 × (Z ±180° connector joint → X ±45°
  head joint → head with LEO).
- UE actor: `FGldfJoint` (pivot USceneComponent, axis mask, limits, angles,
  rest pose), `PartMeshes`, `SetJointAngles` (L3D Euler X·Y·Z → UE via the
  Y-mirror: q' = (−x, y, −z, w), p' = (x, −y, z)), `AimAt` = coordinate
  descent per joint/axis within limits (36-sample scan + 12 bisections,
  3 sweeps; lights below the joint only), fallback = rotate lights.
  `ActorSpaceBodyBox` for snapping; `RebindPartComponents` after template
  spawns. Runtime loader builds the hierarchy (`BuildArticulatedParts`),
  lights attach to their LEO's node with the pose relative to that node.
  Editor Interchange import still bakes one mesh (no joints) — TODO.
- Demo aim mode calls `AimAt`; Esc restores joint angles. Verified: Tria 2 on
  the scan ceiling, both heads solve toward the target.

## 2026-10-29 (evening) — one app, three scenes

- User: "we do not want separate demos anymore, but implement sponza into it,
  and also some small stylized office sample, so the weaker gpu can render it".
- Maps: `/Game/Maps/Bistro` (as before), `/Game/Maps/Sponza` (Khronos glTF of
  Crytek Sponza, Interchange scene import in 41 s, ONE mesh → collision set to
  complex-as-simple or its convex hull = the building), `/Game/Maps/Office`
  (procedural: 114 engine cubes/cylinders/spheres, one flat master material +
  14 colour instances, PPV GI/reflections = screen space, no moon).
- Scripts: `run_editor_py.sh <script> [name]` (generic marker runner),
  `scene_common.py` (fresh_level, trace, bounds, environment(), player_start),
  `import_sponza.py`, `build_office.py` (writes Saved/office_sites.json),
  `place_scene.py` ($GLDF_SCENE=office|sponza; GLDFs staged in
  SourceGldf/Office + SourceGldf/Sponza). Office: 6 Philips SP542P pendants,
  6 Zumtobel P-EVO R100L downlights, SLV Tria 2 over the meeting table,
  Belviso floor lamp. Sponza: Siteco lanterns on 3.8 m poles in the courtyard,
  Ansell Latina lanterns on the arcade walls, Targetti JEDI aimed at the vaults
  (sites from traces).
- App: File ▸ Built-in scene (radio list) → OpenLevel, remembered in
  GameUserSettings ([GldfDemo] LastScene); packaged default map = Office (light),
  first BeginPlay of the session opens the remembered scene (default Bistro);
  explicit /Game/Maps/… on the command line or -GldfStartScene= wins. Per-scene
  credits; exposure restored to the level's own PPV bias; View ▸ Render quality
  (scalability 0–3). `package.sh [Mac|Win64] [Development|Shipping]` cooks all
  three maps into Dist/GldfDemo_<Platform>.
