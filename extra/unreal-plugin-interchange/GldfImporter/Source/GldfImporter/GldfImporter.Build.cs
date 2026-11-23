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
            "GldfRuntime",             // AGldfLuminaireActor lives in the runtime module
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

        // The Rust staticlib + header come from GldfRuntime (public dependency),
        // which links them so the photometric core is also available in games.
    }
}
