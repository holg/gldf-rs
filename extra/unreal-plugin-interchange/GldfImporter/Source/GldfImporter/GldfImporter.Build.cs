// Build rules for the GldfImporter UE 5.7 plugin.
//
// Links the Rust-produced `libgldf_unreal.a` (release build) and pulls
// in the Interchange runtime modules our translator subclasses.
//
// Layout assumption: this plugin lives at
//   <UEProject>/Plugins/GldfImporter/
// and the gldf-rs workspace is reachable via relative paths. If the
// plugin is copied elsewhere, set the env var GLDF_RS_WORKSPACE to an
// absolute path to the gldf-rs root.

using System;
using System.IO;
using UnrealBuildTool;

public class GldfImporter : ModuleRules
{
    public GldfImporter(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = ModuleRules.PCHUsageMode.UseExplicitOrSharedPCHs;

        // Public deps — anything our public headers reference.
        PublicDependencyModuleNames.AddRange(new string[] {
            "Core",
            "CoreUObject",
            "Engine",
            "InterchangeCore",       // UInterchangeTranslatorBase
            "InterchangeEngine",     // UInterchangeManager
            "InterchangeNodes",      // UInterchangeTextureLightProfileNode + UInterchangeBaseNodeContainer
            "InterchangeImport",     // IInterchangeTextureLightProfilePayloadInterface, payload data types
        });

        // Private deps — implementation-only.
        PrivateDependencyModuleNames.AddRange(new string[] {
            "Slate",
            "SlateCore",
            "UnrealEd",
            "Projects",              // for IPluginManager (locate our own dir)
        });

        // ── locate the gldf-rs workspace ───────────────────────────────────
        string WorkspaceRoot = Environment.GetEnvironmentVariable("GLDF_RS_WORKSPACE");
        if (string.IsNullOrEmpty(WorkspaceRoot))
        {
            // ModuleDirectory == .../GldfImporter/Source/GldfImporter/
            // Walk up: Source/ → GldfImporter/ → unreal-plugin-interchange/ → extra/ → workspace root
            WorkspaceRoot = Path.GetFullPath(
                Path.Combine(ModuleDirectory, "..", "..", "..", "..")
            );
        }

        string CrateInclude = Path.Combine(WorkspaceRoot, "crates", "gldf-unreal", "include");
        string CrateLibDir  = Path.Combine(WorkspaceRoot, "target", "release");

        if (!Directory.Exists(CrateInclude))
        {
            throw new BuildException(
                "GldfImporter: could not find gldf-unreal include dir at " + CrateInclude
                + ". Set GLDF_RS_WORKSPACE to the absolute path of the gldf-rs workspace."
            );
        }
        if (!Directory.Exists(CrateLibDir))
        {
            throw new BuildException(
                "GldfImporter: could not find Rust release output at " + CrateLibDir
                + ". Run `cargo build -p gldf-unreal --release` from " + WorkspaceRoot + " first."
            );
        }

        PublicIncludePaths.Add(CrateInclude);

        string LibName;
        if (Target.Platform == UnrealTargetPlatform.Mac || Target.Platform == UnrealTargetPlatform.Linux)
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
                "GldfImporter: unsupported platform " + Target.Platform.ToString()
            );
        }

        string LibPath = Path.Combine(CrateLibDir, LibName);
        if (!File.Exists(LibPath))
        {
            throw new BuildException(
                "GldfImporter: " + LibPath + " not found. Run "
                + "`cargo build -p gldf-unreal --release` from " + WorkspaceRoot + " first."
            );
        }
        PublicAdditionalLibraries.Add(LibPath);

        // Transitive system libs the Rust staticlib needs.
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
