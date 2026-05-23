# gldf-unreal

GLDF → Unreal Engine Datasmith exporter.

Converts a [GLDF](https://gldf.io) file (luminaire 3D model + IES/LDT
photometry + per-variant lumens/watts overrides) into an Epic Datasmith
bundle (`.udatasmith` + companion OBJ/IES assets) that **Unreal Engine
5.4+** imports natively via its built-in Datasmith Importer.

> Status: **v0.0.1 — first pass complete.** CLI, Rust library API, FFI
> surface, and committed C header all work end to end against the demo
> GLDFs in `tests/data/`. Schema verification against UE 5.4 itself is
> the next blocker before tagging.

## Quick start (CLI)

```sh
cargo install --path crates/gldf-unreal --bin gldf-to-datasmith
gldf-to-datasmith \
    --gldf path/to/luminaire.gldf \
    --out  ./out \
    --variants first \
    --overwrite
```

Then in UE 5.4: **File → Import Into Level…** → pick the emitted
`.udatasmith` file.

## Bundle layout (one per variant)

```
out/
└── <product_id>__<variant_id>/
    ├── <product_id>__<variant_id>.udatasmith       # Datasmith 0.24 XML
    └── Assets/
        ├── Geometry/   luminaire_body.obj  +  .mtl  +  textures/
        ├── Ies/        emitter_<photometry_id>.ies (lm/W patched)
        └── Manifest.json (Phase 5+; not emitted yet)
```

## Library API

```rust
use gldf_unreal::{Exporter, ExportOptions, UnitSystem, VariantSelector};

let opts = ExportOptions {
    out_dir: "./out".into(),
    bundle_name: "MyLamp".into(),
    units: UnitSystem::Cm,
    variants: VariantSelector::First,
    embed_textures: false,
    apply_mounting: false,
    overwrite: true,
};
let report = Exporter::from_path("MyLamp.gldf".as_ref())?.export(&opts)?;
for b in &report.bundles {
    println!("variant={} → {}", b.variant_id, b.udatasmith_path.display());
}
```

## Coordinate conversion

L3D source: Z-up, **right-handed**, millimetres. Unreal: Z-up,
**left-handed**, centimetres (or metres if you select `UnitSystem::M`).

The transform is a Y-negating uniform scale: `M = diag(s, -s, s, 1)`, where
`s` is the mm→target scale factor. Triangle winding flips because
`det(M) < 0`, so `mesh::rewrite_obj_to_ue` reverses index order on every
face and negates the Y component of vertex normals.

## Why a separate FFI surface (and not just UniFFI)?

The workspace already has `crates/gldf-rs-ffi/` which uses **UniFFI** to
generate idiomatic bindings for Swift, Kotlin, Python, and C# from a
single Rust source. UniFFI is excellent for managed-language consumers
(`bindings/csharp/` ships on NuGet). It is **not** a clean fit for an
Unreal Engine **C++** plugin: UniFFI's "scaffolding" C header is full of
runtime types (`RustBuffer`, `ForeignBytes`, error-handling structs)
that an Unreal `.Build.cs` author would have to reverse-engineer, and
the UniFFI runtime would have to be dragged into the plugin binary.

So `gldf-unreal` ships a **deliberately tiny, hand-curated C ABI**
(4 functions, one POD struct — see below) that maps 1:1 to what an UE
editor menu action needs to call. It links the same gldf-rs-lib core as
`gldf-rs-ffi`; nothing is parsed or computed twice.

### What is reused from `gldf-rs-ffi` (and what isn't, and why)

| Concern | `gldf-rs-ffi` provides | What `gldf-unreal` uses | Why |
|---|---|---|---|
| GLDF parsing | `GldfEngine`, `gldf_to_json`, etc. (UniFFI) | `gldf_rs::GldfProduct::load_gldf_from_buf_all` (lib-level) | We need the full `FileBufGldf` (parsed product + raw bundled file bytes) to look up LDT bytes by photometry id. The UniFFI surface flattens those into DTOs. |
| L3D geometry | `parse_l3d`, `L3dScene`, `L3dScenePart` | `l3d_rs::from_buffer` (lib-level) | We need the raw OBJ bytes (`L3dAsset.content`) for the text-level rewrite; the UniFFI structs expose them too, but going through the lib avoids a UniFFI round-trip. |
| Photometry | `parse_eulumdat` (returns flat `EulumdatData`) | `gldf_rs::photometry::*` + `eulumdat::Eulumdat::parse` | We need the typed `Eulumdat` back for re-serialization via `export_photometry`; the UniFFI DTO discards the internal model. |
| Variant resolution | (none in UniFFI yet) | `gldf_rs::resolve_variant_photometries` | Lib-level only; the photometry submodule split landed in gldf-rs 0.4.0. |
| C# / Swift / Kotlin / Python bindings | Generated automatically | Not exposed (yet) | A `#[uniffi::export] fn gldf_to_datasmith(...)` wrapper in `gldf-rs-ffi` that calls `gldf_unreal::export_gldf_to_datasmith` is a ~30-line follow-up if/when those consumers want the Datasmith path. Not blocking v0.0.1. |

The `gldf-unreal` C ABI (`gldf_unreal_export` and three companions) is
*additive* to `gldf-rs-ffi`, not a replacement.

## C ABI for the UE plugin

The library compiles as `staticlib` and `cdylib` and ships a committed
header at `include/gldf_unreal.h`. Regenerate it after changing `src/ffi.rs`:

```sh
cargo run -p gldf-unreal --bin gen-header
```

### Calling from C / C++

```c
#include "gldf_unreal.h"

GldfUnrealOpts opts = {
    .units_cm = 1, .embed_textures = 0, .apply_mounting = 0,
    .overwrite = 1, .variants_csv = NULL,
};
char *err = NULL;
int32_t code = gldf_unreal_export(
    "luminaire.gldf", "/tmp/out", &opts, &err);
if (code != 0) {
    fprintf(stderr, "export failed (%d): %s\n", code, err ? err : "?");
    gldf_unreal_string_free(err);
}
```

### Error codes (stable contract)

| Code | Meaning                                |
|------|----------------------------------------|
| 0    | Success                                |
| 1    | Filesystem I/O failure                 |
| 2    | GLDF parsing failed                    |
| 3    | Requested variant id unknown           |
| 4    | GLDF has no L3D geometry               |
| 5    | LDT/IES photometry processing failed   |
| 6    | Output exists and `--overwrite` is off |
| 7    | XML serialization failed               |
| 99   | Internal exporter bug                  |

### String ownership

* All `*const char` inputs are caller-owned, NUL-terminated UTF-8. The
  library copies what it needs before returning.
* `out_err` and the return of `gldf_unreal_last_report_json()` are
  Rust-owned heap strings. The C caller MUST free them via
  `gldf_unreal_string_free()`. Calling `free()` instead will corrupt the
  allocator.

### Wiring into an UE5 plugin

In your plugin's `Source/GldfRsImporter/GldfRsImporter.Build.cs`:

```csharp
using System.IO;
using UnrealBuildTool;

public class GldfRsImporter : ModuleRules
{
    public GldfRsImporter(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
        PublicDependencyModuleNames.AddRange(new string[] {
            "Core", "CoreUObject", "Engine", "DatasmithCore", "UnrealEd",
        });

        // Where you vendor the gldf-unreal artefacts. Recommended layout:
        //   ThirdParty/gldf_unreal/
        //     include/gldf_unreal.h
        //     lib/Win64/gldf_unreal.lib
        //     lib/Mac/libgldf_unreal.a
        //     lib/Linux/libgldf_unreal.a
        string ThirdParty = Path.Combine(ModuleDirectory,
            "..", "..", "ThirdParty", "gldf_unreal");

        PublicIncludePaths.Add(Path.Combine(ThirdParty, "include"));

        if (Target.Platform == UnrealTargetPlatform.Win64) {
            PublicAdditionalLibraries.Add(
                Path.Combine(ThirdParty, "lib", "Win64", "gldf_unreal.lib"));
        } else if (Target.Platform == UnrealTargetPlatform.Mac) {
            PublicAdditionalLibraries.Add(
                Path.Combine(ThirdParty, "lib", "Mac", "libgldf_unreal.a"));
        } else if (Target.Platform == UnrealTargetPlatform.Linux) {
            PublicAdditionalLibraries.Add(
                Path.Combine(ThirdParty, "lib", "Linux", "libgldf_unreal.a"));
        }
    }
}
```

## Out of scope (v0.0.1)

- The UE5 plugin itself (`.uplugin`, `Source/GldfRsImporter/`, the
  `.Build.cs`, Blueprint nodes). Separate downstream repo.
- Datasmith `VariantSet` emission. v0.0.1 emits **one `.udatasmith` per
  variant**.
- `.udsmesh` binary mesh writing — external OBJ references instead.
- Material reconstruction beyond L3D MTL passthrough.
- Reverse direction (`.udatasmith → .gldf`).
- UE 4.27.

## Datasmith schema verification (open item)

The XML element names (`<DatasmithUnrealScene>`, `<StaticMesh>`,
`<ActorLight type="Spot">`, `<Ies file="…">`, etc.) target Datasmith 0.24
/ UE 5.4 based on public Epic documentation. **They MUST be verified
against a `.udatasmith` exported by UE 5.4 itself** (e.g. via the
Datasmith Exporter plugin for 3ds Max or Revit) before v0.0.1 is tagged
for general use. The recommended check:

1. Export an empty UE 5.4 scene with one point light + one IES profile +
   one static mesh via the Datasmith Exporter.
2. Commit it as `tests/data/datasmith/reference.udatasmith`.
3. Add a test (or extend `export_smoke`) that diffs element names /
   nesting between our emitted file and the reference.

## Licence

Inherits the workspace licence (**AGPL-3.0-or-later**). A UE C++ plugin
that statically links `libgldf_unreal.a` is a derivative work and AGPL
propagates to the linking project. Before this crate gets a
non-`publish = false` release, a decision is needed:

- **(a)** Dual-licence `gldf-unreal` as `Apache-2.0 OR AGPL-3.0-or-later`
  for downstream UE adoption (recommended), or
- **(b)** Keep AGPL and document loudly that commercial UE integration
  requires a separate licence agreement.

This is flagged for the maintainer in the plan; v0.0.1 ships with
`publish = false` until the decision lands.
