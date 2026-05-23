# Testing `gldf-unreal` against UE 5.4

This is the manual acceptance checklist for the GLDF → Datasmith
pipeline. The automated `cargo test -p gldf-unreal` suite covers the
Rust side (coords, OBJ rewrite, XML emission, IES round-trip, bundle
layout); what we still need to verify is that **Unreal Engine 5.4
actually accepts our `.udatasmith` and reproduces the luminaire
correctly.**

Two tracks:

* **Track A — CLI + Datasmith Importer.** Lowest-friction. Run the
  exporter from the terminal, drag the result into UE.
* **Track B — Plugin path (FFI smoke).** Drop the scaffold plugin
  from `extra/unreal-plugin-scaffold/GldfRsImporter` into a UE
  project, link our staticlib, fire the import from inside UE.

Track A is the gate for v0.0.1. Track B is the gate for whoever
ships the downstream plugin.

---

## Pre-flight

```sh
# From workspace root.
cargo build -p gldf-unreal --release
cargo run --release -p gldf-unreal --bin gldf-to-datasmith -- \
    --gldf tests/data/alurays-3000mm.gldf \
    --out  tmp/uetest \
    --variants first \
    --overwrite
```

Expected stdout:

```
Wrote 1 bundle(s):
  variant=var_01               → tmp/uetest/alurays-3000mm__var_01/alurays-3000mm__var_01.udatasmith
    asset: tmp/uetest/alurays-3000mm__var_01/Assets/Geometry/geom_1/luminaire.obj
    asset: tmp/uetest/alurays-3000mm__var_01/Assets/Ies/emitter_photom01.ies
```

If the layout differs (different filenames are fine; missing files
are not), stop and debug the exporter before touching UE.

---

## Track A — CLI + Datasmith Importer (the v0.0.1 gate)

### A.1 — Create a clean test project

1. **UE 5.4 → New Project →** *Games / Blank* (or *Architecture / Blank*).
2. Project name: `GldfImportTest`. Location: anywhere with a few GB
   free. No Starter Content.
3. Wait for the editor to open with an empty level.

### A.2 — Enable the Datasmith Importer plugin (usually pre-enabled)

1. **Edit → Plugins**, search "Datasmith".
2. Confirm **Datasmith Importer** is enabled. If not, tick it and
   restart the editor when prompted.

### A.3 — Import the emitted `.udatasmith`

1. **File → Import Into Level…**.
2. Navigate to `tmp/uetest/alurays-3000mm__var_01/` (the bundle dir
   the CLI wrote).
3. Select `alurays-3000mm__var_01.udatasmith` and click **Open**.
4. In the **Datasmith Import Options** dialog:
   * **Include Geometry**: yes (default).
   * **Include Lights**: yes (default).
   * **Include Materials**: yes (default — we don't author
     materials, but UE will assign a default to the OBJ).
   * Leave **Hierarchy** at its default.
   * Click **Import**.
5. If a "Save Content" dialog appears for the destination asset folder,
   accept the default path (`/Game/<bundle-name>/`).

**Common failure modes here:**

* *"Failed to parse `.udatasmith`"* — schema mismatch with UE 5.4.
  Capture the editor log (`Window → Developer Tools → Output Log`),
  filter for `LogDatasmith`, share the lines. This is the open
  item flagged in `README.md` §"Datasmith schema verification".
* *"No actors imported"* — UE parsed the XML but didn't find any
  recognisable elements. Check that our `<Actor>` tag matches
  what 5.4 expects (we use it; the SDK may want `<ActorMeshActor>`
  or similar in 5.4).
* *"IES profile failed to load"* — UE didn't find the `.ies` at
  the referenced path. Confirm the `.ies` exists on disk; the
  Datasmith bundle uses *relative* paths (`Assets/Ies/…`) which UE
  resolves against the `.udatasmith`'s directory.

### A.4 — Verify the imported scene

In the level's **Outliner**, you should now see:

```
└── alurays-3000mm__var_01    (Actor, layer = GLDF)
    ├── Body                  (StaticMeshActor)
    └── Emitter_photom01      (Spot Light, IES profile assigned)
```

### A.5 — Pass criteria

Tick each. **All must pass for Track A to gate v0.0.1.**

- [ ] **Mesh visible at expected scale.** The luminaire is about 3 m
      long (it's `alurays-3000mm`). In the viewport its bounding box
      should read ~300 cm in the longest dimension (or ~3 m if the
      project's World Unit is metres).
- [ ] **Mesh orientation correct.** Looks like a luminaire, not
      inside-out. If you see back-faces with broken lighting, the
      winding-flip in `mesh.rs` is wrong for this file.
- [ ] **Spotlight present.** Click `Emitter_photom01` in the
      Outliner; the **Details** panel shows a Spot Light Component.
- [ ] **IES profile assigned.** In the Spot Light's Details, the
      *Light Profiles* section shows an IES Texture asset whose
      thumbnail looks like a luminous distribution curve (not a
      uniform disk — that would mean default fallback).
- [ ] **Intensity matches the variant lumens.** Details → Light →
      *Intensity* reads `6856 lm` for `alurays-3000mm` variant
      `var_01` (per the CLI output above). Tolerance: exact match.
- [ ] **Colour temperature correct.** Details → Light → *Light
      Color* uses temperature mode, `4000 K`.

### A.6 — Multi-variant sanity check

Re-run the CLI with `--variants all` against a multi-variant file:

```sh
cargo run --release -p gldf-unreal --bin gldf-to-datasmith -- \
    --gldf tests/data/Freestand_Belviso-2l3d.gldf \
    --out  tmp/uetest-belviso \
    --variants all \
    --overwrite
```

Import each emitted `.udatasmith` into a separate **sublevel** of
the same project. Pass criteria:

- [ ] One bundle per variant emitted (count matches the GLDF's
      variant list).
- [ ] **Geometry is identical** across variants. Visually swapping
      sublevels shouldn't move the luminaire.
- [ ] **Lumens / CCT differ** per variant according to the GLDF's
      `<RatedLuminousFlux>` and the `<FixedLightSource>` colour
      info. Capture the values from the CLI output (it logs the
      written paths; load the `.udatasmith` files to confirm).

### A.7 — Photometric correctness (informal sanity check)

Drop a flat plane below the luminaire at the expected mounting height
(e.g. 2.5 m for a typical ceiling fixture). Trigger **Build → Build
Lighting → Build Lighting Only**. The illuminance pattern on the
plane should resemble the IES candela curve:

- A linear pendant like `alurays-3000mm` produces an elongated
  rectangle of light, brightest under the centre, falling off along
  the long axis.
- An asymmetric floodlight (`FLOODLIGHT_MAX_*_enriched.gldf`) shows
  a directional throw, not a circle.

This is an eyeball check, not a precision measurement.

---

## Track B — Plugin FFI smoke test

After UE 5.4 is installed and Track A passes:

1. Copy `extra/unreal-plugin-scaffold/GldfRsImporter/` into your
   test project's `Plugins/` directory:
   ```sh
   cp -R extra/unreal-plugin-scaffold/GldfRsImporter \
         ~/Documents/Unreal\ Projects/GldfImportTest/Plugins/
   ```
2. Build the release staticlib so the plugin can link it:
   ```sh
   cargo build -p gldf-unreal --release
   ```
3. The plugin's `.Build.cs` references
   `../../crates/gldf-unreal/include/gldf_unreal.h` and
   `../../target/release/libgldf_unreal.a` via relative paths. If
   you copy the plugin somewhere else, edit the paths in
   `GldfRsImporter.Build.cs` to absolute paths to this workspace.
4. Right-click your `.uproject` → **Generate Xcode Project Files**.
5. Open the generated Xcode workspace and **Build** the editor target.
6. Launch the rebuilt editor.
7. In the editor menu bar you should see a new menu item:
   **Tools → GLDF → Import GLDF...** (exact path may vary; check the
   plugin's `Private/GldfRsImporterModule.cpp` for the menu wiring).
8. Click it. The plugin should run `gldf_unreal_export()` on a
   hard-coded test path and report success in the **Output Log**.

### Pass criteria

- [ ] Plugin compiles without link errors. (`Undefined symbol:
      _gldf_unreal_export` means the staticlib path in `.Build.cs`
      is wrong.)
- [ ] Menu item appears.
- [ ] Clicking it logs `gldf_unreal_export returned 0` (or similar)
      and the `.udatasmith` bundle appears at the hard-coded
      output path.

---

## Capturing failures

If anything in Track A fails, capture:

1. **The `.udatasmith` file** (`tmp/uetest/.../*.udatasmith`).
2. **The editor's Output Log**, filtered to `LogDatasmith` +
   `LogImport`. Window → Developer Tools → Output Log → Filters.
3. **A screenshot of the Outliner + the Details panel** of the
   imported actor.

That triple is enough to debug 90% of schema/import issues offline.
