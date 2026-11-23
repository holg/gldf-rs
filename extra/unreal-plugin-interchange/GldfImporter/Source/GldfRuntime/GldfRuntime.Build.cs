// Build rules for the GldfRuntime module — the part of the GLDF plugin that
// ships INSIDE a packaged game.
//
// Holds AGldfLuminaireActor (+ variant / mounting structs, surface snapping)
// and AGldfIlluminanceGrid (the eulumdat-powered illuminance heatmap). Links
// the Rust `gldf_unreal` staticlib (bundled under ThirdParty/GldfUnreal) so
// the photometric core is available at runtime; GldfImporter (editor) gets it
// transitively. Keep this module free of UnrealEd / Interchange.

using System.IO;
using UnrealBuildTool;

public class GldfRuntime : ModuleRules
{
    public GldfRuntime(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = ModuleRules.PCHUsageMode.UseExplicitOrSharedPCHs;

        PublicDependencyModuleNames.AddRange(new string[] {
            "Core",
            "CoreUObject",
            "Engine",
            "ProceduralMeshComponent",  // heatmap mesh
        });
        PrivateDependencyModuleNames.AddRange(new string[] {
            "MeshDescription",          // runtime .gldf loader: L3D → UStaticMesh
            "StaticMeshDescription",
            "IESFile",                  // IES text → UTextureLightProfile at runtime
            "RHI",
            "RenderCore",
        });

        // ── bundled Rust staticlib + header (plugin-local ThirdParty) ──────
        string ThirdParty = Path.Combine(PluginDirectory, "ThirdParty", "GldfUnreal");
        string CrateInclude = Path.Combine(ThirdParty, "include");
        string PlatformDir; string LibName;
        if (Target.Platform == UnrealTargetPlatform.Mac) { PlatformDir = "Mac"; LibName = "libgldf_unreal.a"; }
        else if (Target.Platform == UnrealTargetPlatform.Linux) { PlatformDir = "Linux"; LibName = "libgldf_unreal.a"; }
        else if (Target.Platform == UnrealTargetPlatform.Win64) { PlatformDir = "Win64"; LibName = "gldf_unreal.lib"; }
        else { throw new BuildException("GldfRuntime: unsupported platform " + Target.Platform.ToString()); }

        string LibPath = Path.Combine(ThirdParty, "lib", PlatformDir, LibName);
        if (!File.Exists(LibPath))
        {
            throw new BuildException(
                "GldfRuntime: bundled Rust staticlib not found at " + LibPath + ".\n"
                + "All platforms cross-build from macOS — run\n"
                + "  extra/unreal-plugin-interchange/build_thirdparty_libs.sh\n"
                + "in the gldf-rs workspace to rebuild and refresh ThirdParty/.");
        }
        PublicIncludePaths.Add(CrateInclude);
        PublicAdditionalLibraries.Add(LibPath);

        if (Target.Platform == UnrealTargetPlatform.Mac)
        {
            PublicFrameworks.AddRange(new string[] { "CoreFoundation", "Security" });
        }
        else if (Target.Platform == UnrealTargetPlatform.Linux)
        {
            PublicSystemLibraries.AddRange(new string[] { "pthread", "dl", "m" });
        }
    }
}
