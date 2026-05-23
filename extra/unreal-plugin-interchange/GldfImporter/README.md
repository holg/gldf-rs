# GldfImporter — UE 5.7 Interchange plugin

Native UE 5.7 importer for GLDF luminaire files, built on Epic's
**Interchange** framework. Phase 1 imports the IES photometric profile
of the first non-emergency emitter of the first variant as a
`UTextureLightProfile` asset. Phase 2 (mesh), 3 (actor + spotlight +
metadata), and 4 (variants) follow.

See the full plan: `/Users/htr/.claude/plans/interchange-gldf-translator.md`.

> **Status: Phase 1, untested.** First UE build will need to compile.
> Expect at least one round of "fix missing module dep" or "fix the
> Interchange API call" on the first build attempt.

## What lands in UE after Phase 1

Drop a `.gldf` on the viewport (or Content Browser):
- A **`UTextureLightProfile`** asset appears in the Content Browser
  with the IES bytes of the first emitter (variant-resolved lumens
  and watts already patched by the Rust gldf-unreal pipeline).

No mesh yet. No actor yet. Phase 2 + 3 add those.

## How to test

1. **Build the Rust staticlib once** in the gldf-rs workspace:
   ```sh
   cd /Users/htr/Documents/develeop/rust/gldf-rs
   cargo build -p gldf-unreal --release
   # → target/release/libgldf_unreal.a
   # → crates/gldf-unreal/include/gldf_unreal.h
   ```

2. **Copy the plugin into a UE 5.7 project's `Plugins/`** directory:
   ```sh
   mkdir -p ~/Documents/Unreal\ Projects/GldfImportTest/Plugins
   cp -R extra/unreal-plugin-interchange/GldfImporter \
         ~/Documents/Unreal\ Projects/GldfImportTest/Plugins/
   ```

3. **Tell the plugin where the gldf-rs workspace lives.** The
   `.Build.cs` walks up four directories from itself to find the
   workspace by default — only correct if the plugin sits inside
   `extra/unreal-plugin-interchange/` in the gldf-rs repo. If you
   copied it elsewhere:
   ```sh
   export GLDF_RS_WORKSPACE=/Users/htr/Documents/develeop/rust/gldf-rs
   ```
   before launching UE, or edit `WorkspaceRoot` directly in
   `GldfImporter.Build.cs`.

4. **Generate project files and build the editor.**
   ```sh
   # macOS: right-click <MyProject>.uproject → Generate Xcode Project Files
   # Or:
   "/Users/Shared/Epic Games/UE_5.7/Engine/Build/BatchFiles/Mac/GenerateProjectFiles.sh" \
       -project="/absolute/path/MyProject.uproject" -game
   ```
   Open the generated `.xcworkspace`, build the editor target.

5. **Launch the editor**, open the project.

6. **Drag** `tests/data/alurays-3000mm.gldf` from Finder into the
   Content Browser **or** the viewport. The Interchange Import
   Options dialog should appear (different from the Datasmith
   dialog — Interchange's is simpler).

7. **Click Import.** A `UTextureLightProfile` asset should land in
   the Content Browser. Double-click it; the asset editor should
   show the IES candela curve as a texture preview.

## Pass criteria (Phase 1)

- [ ] Plugin compiles without link errors.
  - `Undefined symbol _gldf_unreal_first_ies_bytes` ⇒ the staticlib
    in `target/release/libgldf_unreal.a` doesn't have the new
    symbol yet. Re-run `cargo build -p gldf-unreal --release`.
- [ ] `.gldf` is recognized by Interchange (file extension shows up
      in the Open / Import dialogs).
- [ ] Import completes without errors.
  - If UE hangs or logs `LogFileManager: Error: Requested read of N
    bytes when 0 bytes remain`, **kill the editor immediately**.
    That was the bug that affected the old Datasmith path; if it
    repeats here it means our Interchange wiring is somehow
    short-circuiting to a different importer. Capture the log at
    `~/Library/Logs/Unreal Engine/<ProjectName>Editor/<Project>.log`.
- [ ] A `UTextureLightProfile` asset appears in the Content Browser.
- [ ] Opening the asset shows the IES candela curve.

## Troubleshooting

- **Missing `'CoreMinimal.h'`** in your IDE → benign; clang in the
  IDE doesn't see UE's headers. UBT will resolve them during build.
- **`Undefined symbol` for any `gldf_unreal_*` function** → the
  staticlib path in `.Build.cs` is wrong, or the staticlib is out
  of date. Re-run `cargo build -p gldf-unreal --release` and
  rebuild the editor.
- **No Interchange Import dialog when dragging `.gldf`** → either
  the plugin didn't load (check `~/Library/Logs/Unreal Engine/.../
  *.log` for "GldfImporter starting"), or the file extension
  registration didn't happen (check that `GetSupportedFormats()`
  returns `{"gldf;…"}`).

## License

Inherits the gldf-rs workspace's `AGPL-3.0-or-later`. The dual-licence
question flagged in `crates/gldf-unreal/README.md` still applies for
commercial UE shipping.
