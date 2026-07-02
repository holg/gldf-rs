// Build rules for the GldfSearch UE 5.7 editor module.
//
// Links the Rust-produced `libgldf_search_cabi.a` (the POD C-ABI over the
// gldf-search query engine) and wires the demo flow: search gldf-search.de →
// pick a hit → resolve its download URL → fetch the .gldf via UE's HTTP module
// → import it through the GldfImporter Interchange plugin (placing one
// AGldfLuminaireActor).
//
// The search cabi lives in a DIFFERENT workspace than the importer (the
// gldf-search repo, not gldf-rs), but the built staticlib ships bundled in
// this plugin's ThirdParty/GldfSearch/ — refresh it (all platforms, from
// macOS) via extra/unreal-plugin-interchange/build_thirdparty_libs.sh.

using System.IO;
using UnrealBuildTool;

public class GldfSearch : ModuleRules
{
    public GldfSearch(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = ModuleRules.PCHUsageMode.UseExplicitOrSharedPCHs;

        PublicDependencyModuleNames.AddRange(new string[] {
            "Core",
            "CoreUObject",
            "Engine",
        });

        PrivateDependencyModuleNames.AddRange(new string[] {
            "HTTP",                // FHttpModule — download the .gldf
            "InterchangeCore",     // UInterchangeSourceData
            "InterchangeEngine",   // UInterchangeManager::ImportScene
            "UnrealEd",            // editor-only import context
            "GldfImporter",        // the Interchange translator/factory/actor
            // Slate UI (the search panel) — native, no Python.
            "Slate",
            "SlateCore",
            "InputCore",                 // text-commit / key types
            "WorkspaceMenuStructure",    // Window > Tools menu category for the tab
            // CEF spike: SGldfWebPanel embeds the gldf-search-leptos web UI.
            // If UE's bundled CEF renders the WASM app, the Slate panel +
            // search cabi can eventually be replaced by this webview.
            "WebBrowser",
        });

        // ── bundled gldf-search-cabi staticlib + header (plugin-local) ─────
        // Self-contained, like GldfImporter: ThirdParty/GldfSearch/ travels
        // with the plugin. No env var, no sibling gldf-search workspace.
        //   ThirdParty/GldfSearch/include/gldf_search.h
        //   ThirdParty/GldfSearch/lib/{Mac,Linux,Win64}/<lib>
        string ThirdParty = Path.Combine(PluginDirectory, "ThirdParty", "GldfSearch");
        string CrateInclude = Path.Combine(ThirdParty, "include");

        string PlatformDir;
        string LibName;
        if (Target.Platform == UnrealTargetPlatform.Mac)
        {
            PlatformDir = "Mac"; LibName = "libgldf_search_cabi.a";
        }
        else if (Target.Platform == UnrealTargetPlatform.Linux)
        {
            PlatformDir = "Linux"; LibName = "libgldf_search_cabi.a";
        }
        else if (Target.Platform == UnrealTargetPlatform.Win64)
        {
            PlatformDir = "Win64"; LibName = "gldf_search_cabi.lib";
        }
        else
        {
            throw new BuildException("GldfSearch: unsupported platform " + Target.Platform.ToString());
        }

        string LibPath = Path.Combine(ThirdParty, "lib", PlatformDir, LibName);
        if (!File.Exists(LibPath))
        {
            throw new BuildException(
                "GldfSearch: bundled Rust staticlib not found at " + LibPath + ".\n"
                + "All platforms cross-build from macOS — run\n"
                + "  extra/unreal-plugin-interchange/build_thirdparty_libs.sh\n"
                + "in the gldf-rs workspace to rebuild and refresh ThirdParty/."
            );
        }

        PublicIncludePaths.Add(CrateInclude);
        PublicAdditionalLibraries.Add(LibPath);

        // Transitive system libs the Rust staticlib needs (ureq + rustls — no
        // OpenSSL; rustls uses the platform RNG/cert store via these).
        if (Target.Platform == UnrealTargetPlatform.Mac)
        {
            PublicFrameworks.AddRange(new string[] {
                "CoreFoundation",
                "Security",
                "SystemConfiguration", // ureq/rustls proxy + network reachability
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
