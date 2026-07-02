// Build rules for the GldfImporter UE 5.7 plugin.
//
// Links the Rust-produced `gldf_unreal` staticlib + its C header, both
// bundled INSIDE the plugin under ThirdParty/GldfUnreal/ so the plugin is
// self-contained — no env vars, no external workspace, drop-in portable.
//
//   ThirdParty/GldfUnreal/include/gldf_unreal.h
//   ThirdParty/GldfUnreal/lib/{Mac,Linux,Win64}/<lib>
//
// Mac + Linux (x86_64) + Win64 (x86_64 MSVC) libs are prebuilt + shipped —
// all cross-built from macOS via extra/unreal-plugin-interchange/
// build_thirdparty_libs.sh (Win64 uses clang-cl + rust-lld + an xwin-style
// sysroot; a pure C-ABI staticlib needs no Windows linker, so this works).

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
            "InterchangeCore",         // UInterchangeTranslatorBase + UInterchangePipelineBase
            "InterchangeEngine",       // UInterchangeManager
            "InterchangeNodes",        // UInterchangeTextureLightProfileNode + UInterchangeMeshNode
            "InterchangeFactoryNodes", // UInterchangeActorFactoryNode (pipeline retypes root, Phase 3b.2)
            "InterchangeImport",       // payload interfaces + FMeshPayloadData
            "InterchangePipelines",    // UInterchangeGenericLevelPipeline (stack ordering)
            "MeshDescription",         // FMeshDescription (mesh payload, Phase 2)
            "StaticMeshDescription",   // FStaticMeshAttributes (Phase 2)
        });

        // Private deps — implementation-only.
        PrivateDependencyModuleNames.AddRange(new string[] {
            "Slate",
            "SlateCore",
            "UnrealEd",
            "Projects",              // for IPluginManager (locate our own dir)
            "IESFile",               // FIESConverter (used in GetLightProfilePayloadData)
        });

        // ── bundled Rust staticlib + header (plugin-local ThirdParty) ──────
        // PluginDirectory == .../GldfImporter/ ; ModuleDirectory is
        // .../GldfImporter/Source/GldfImporter/. Use the plugin dir so this
        // works no matter where the plugin is dropped (incl. via symlink).
        string ThirdParty = Path.Combine(PluginDirectory, "ThirdParty", "GldfUnreal");
        string CrateInclude = Path.Combine(ThirdParty, "include");

        // Per-platform lib subdir + name.
        string PlatformDir;
        string LibName;
        if (Target.Platform == UnrealTargetPlatform.Mac)
        {
            PlatformDir = "Mac"; LibName = "libgldf_unreal.a";
        }
        else if (Target.Platform == UnrealTargetPlatform.Linux)
        {
            PlatformDir = "Linux"; LibName = "libgldf_unreal.a";
        }
        else if (Target.Platform == UnrealTargetPlatform.Win64)
        {
            PlatformDir = "Win64"; LibName = "gldf_unreal.lib";
        }
        else
        {
            throw new BuildException(
                "GldfImporter: unsupported platform " + Target.Platform.ToString()
            );
        }

        string LibPath = Path.Combine(ThirdParty, "lib", PlatformDir, LibName);
        if (!File.Exists(LibPath))
        {
            throw new BuildException(
                "GldfImporter: bundled Rust staticlib not found at " + LibPath + ".\n"
                + "All platforms cross-build from macOS — run\n"
                + "  extra/unreal-plugin-interchange/build_thirdparty_libs.sh\n"
                + "in the gldf-rs workspace to rebuild and refresh ThirdParty/."
            );
        }

        PublicIncludePaths.Add(CrateInclude);
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
