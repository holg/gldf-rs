# GldfRsImporter — UE 5.4 scaffold

Drop-in Unreal Engine 5.4 editor plugin that exposes
**File → Import GLDF…**. The menu action picks a `.gldf` and an output
directory, then calls `gldf_unreal_export()` from the Rust staticlib
`libgldf_unreal.a`. The resulting `.udatasmith` is opened by the
**Datasmith Importer** as a separate step.

> **Status:** untested. The plugin compiles in principle, but the
> exact UE 5.4 API names (`FToolMenuSection`, `FDesktopPlatformModule`,
> menu-path strings, etc.) haven't been verified against a live build.
> Expect to fix one or two compile errors on the first build attempt.

## Layout

```
GldfRsImporter/
├── GldfRsImporter.uplugin
├── README.md                                     ← this file
└── Source/
    └── GldfRsImporter/
        ├── GldfRsImporter.Build.cs               ← links libgldf_unreal.a
        ├── Public/
        │   └── GldfRsImporterModule.h
        └── Private/
            └── GldfRsImporterModule.cpp          ← editor menu + FFI call
```

## How to use

1. **Build the Rust staticlib once.**

   ```sh
   cd /path/to/gldf-rs                    # this workspace
   cargo build -p gldf-unreal --release
   # → target/release/libgldf_unreal.a
   # → crates/gldf-unreal/include/gldf_unreal.h
   ```

2. **Copy the plugin into a UE 5.4 project's `Plugins/` directory.**

   ```sh
   # Replace ~/Documents/Unreal Projects/MyProject with your project root.
   mkdir -p "~/Documents/Unreal Projects/MyProject/Plugins"
   cp -R extra/unreal-plugin-scaffold/GldfRsImporter \
         "~/Documents/Unreal Projects/MyProject/Plugins/"
   ```

3. **Tell the plugin where the gldf-rs workspace lives.** The
   `.Build.cs` walks up four directories from itself to find the
   workspace by default — that only works if the plugin sits inside
   `extra/unreal-plugin-scaffold/` of the gldf-rs repo. If you copied
   it elsewhere, set:

   ```sh
   export GLDF_RS_WORKSPACE=/absolute/path/to/gldf-rs
   ```

   …before running the UE editor, or edit `WorkspaceRoot` directly in
   `GldfRsImporter.Build.cs`.

4. **Generate project files and build the editor.**

   ```sh
   # macOS: right-click <MyProject>.uproject → Generate Xcode Project Files.
   # Or from the terminal:
   #   /Users/Shared/Epic\ Games/UE_5.4/Engine/Build/BatchFiles/Mac/GenerateProjectFiles.sh \
   #     -project="/absolute/path/MyProject.uproject" -game
   ```

   Open the generated `.xcworkspace`, build the editor target.
   First build will take a while as it compiles the plugin module.

5. **Use it.** Launch the editor, **File → Import GLDF…**. Pick a
   `.gldf` (try `tests/data/alurays-3000mm.gldf` from the workspace),
   pick an output dir, wait for the toast. Then **File → Import
   Into Level…** on the emitted `.udatasmith`.

## Troubleshooting first build

* `'gldf_unreal.h' file not found` — the `PublicIncludePaths` setup
  in `.Build.cs` couldn't find the header. Check the path it tried
  to use (BuildException prints it). Set `GLDF_RS_WORKSPACE`.
* `Undefined symbol: _gldf_unreal_export` — the staticlib path is
  wrong. Re-check `target/release/libgldf_unreal.a` exists and the
  path in `.Build.cs` resolves.
* `Could not find MainFrame.MainMenu.File` log warning — UE 5.4
  changed the menu hook point. Try `LevelEditor.MainMenu.File`
  instead in `RegisterMenus()`. Some versions also use
  `MainFrame.NomadMainMenu.File`.
* Slate / IModuleInterface compile errors — usually means a missing
  dependency in `.Build.cs`. Common additions: `EditorStyle`,
  `EditorFramework`, `LevelEditor`.

## License

Inherits the workspace AGPL-3.0-or-later (until the dual-licence
question flagged in `crates/gldf-unreal/README.md` is resolved).
Commercial UE shipping linked against this plugin needs that
question answered first.
