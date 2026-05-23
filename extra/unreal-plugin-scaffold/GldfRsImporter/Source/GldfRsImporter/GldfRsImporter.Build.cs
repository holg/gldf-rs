// Untested against UE 5.4 — first build will need verification.
//
// This module links the Rust-produced `libgldf_unreal.a` and exposes a
// single editor menu entry that calls `gldf_unreal_export()`.
//
// Layout assumption: this plugin lives at
//   <UEProject>/Plugins/GldfRsImporter/
// and `gldf-rs` workspace is reachable via relative paths. If you move
// the plugin somewhere else, set GLDF_RS_WORKSPACE to an absolute path
// or edit the WorkspaceRoot helper below.

using System;
using System.IO;
using UnrealBuildTool;

public class GldfRsImporter : ModuleRules
{
    public GldfRsImporter(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = ModuleRules.PCHUsageMode.UseExplicitOrSharedPCHs;

        PublicDependencyModuleNames.AddRange(new string[] {
            "Core",
            "CoreUObject",
            "Engine",
        });

        PrivateDependencyModuleNames.AddRange(new string[] {
            "Slate",
            "SlateCore",
            "UnrealEd",          // editor-only: FToolMenus, FUICommandList
            "ToolMenus",
            "Projects",          // for IPluginManager (locate our own dir)
            "DesktopPlatform",   // open-file dialog
        });

        // ── locate the gldf-rs workspace ───────────────────────────────────
        // Two ways:
        //   1. Env var GLDF_RS_WORKSPACE → absolute path. Most robust.
        //   2. Walk up from this .Build.cs: typical layout when the plugin
        //      sits inside `extra/unreal-plugin-scaffold/GldfRsImporter/`
        //      is `../../../../` to reach the workspace root.
        string WorkspaceRoot = Environment.GetEnvironmentVariable("GLDF_RS_WORKSPACE");
        if (string.IsNullOrEmpty(WorkspaceRoot))
        {
            // ModuleDirectory == .../GldfRsImporter/Source/GldfRsImporter/
            WorkspaceRoot = Path.GetFullPath(
                Path.Combine(ModuleDirectory, "..", "..", "..", "..")
            );
        }

        string CrateInclude = Path.Combine(WorkspaceRoot, "crates", "gldf-unreal", "include");
        string CrateLibDir  = Path.Combine(WorkspaceRoot, "target", "release");

        if (!Directory.Exists(CrateInclude))
        {
            throw new BuildException(
                "GldfRsImporter: could not find gldf-unreal include dir at " + CrateInclude
                + ". Set GLDF_RS_WORKSPACE to the absolute path of the gldf-rs workspace."
            );
        }
        if (!Directory.Exists(CrateLibDir))
        {
            throw new BuildException(
                "GldfRsImporter: could not find Rust release output at " + CrateLibDir
                + ". Run `cargo build -p gldf-unreal --release` first."
            );
        }

        PublicIncludePaths.Add(CrateInclude);

        // Static-link the Rust staticlib. Path varies by platform.
        string LibName;
        if (Target.Platform == UnrealTargetPlatform.Mac)
        {
            LibName = "libgldf_unreal.a";
        }
        else if (Target.Platform == UnrealTargetPlatform.Linux)
        {
            LibName = "libgldf_unreal.a";
        }
        else if (Target.Platform == UnrealTargetPlatform.Win64)
        {
            LibName = "gldf_unreal.lib";
        }
        else
        {
            throw new BuildException(
                "GldfRsImporter: unsupported platform " + Target.Platform.ToString()
            );
        }

        string LibPath = Path.Combine(CrateLibDir, LibName);
        if (!File.Exists(LibPath))
        {
            throw new BuildException(
                "GldfRsImporter: " + LibPath + " not found. Run "
                + "`cargo build -p gldf-unreal --release` from " + WorkspaceRoot + "."
            );
        }
        PublicAdditionalLibraries.Add(LibPath);

        // The Rust staticlib pulls in standard C++/system libs through its
        // bundled `cdylib` deps. On macOS we may need to link CoreFoundation
        // (used transitively by some Rust crates). On Linux: pthread, dl, m.
        if (Target.Platform == UnrealTargetPlatform.Mac)
        {
            PublicFrameworks.AddRange(new string[] {
                "CoreFoundation",
                "Security",
            });
        }
        else if (Target.Platform == UnrealTargetPlatform.Linux)
        {
            PublicSystemLibraries.AddRange(new string[] {
                "pthread", "dl", "m",
            });
        }
    }
}
