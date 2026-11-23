// GldfSearch module (Runtime) — gldf-search.de inside Unreal, web based:
// SGldfWebPanel hosts the site in SWebBrowser (WKWebView on Mac, CEF on
// Windows/Linux) and turns its viewer links into Place actions; the library
// resolves a document's .gldf URL over the REST API, downloads it with UE
// HTTP and hands the file to a pluggable place handler (Interchange import in
// the editor, the runtime loader in games). No Rust search lib, no native
// facet UI — the site is the UI, so there is one search front end to maintain.
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
            "HTTP",         // download the .gldf, GET /api/doc, URL encode/decode
            "Json",         // /api/doc response
            "WebBrowser",   // SWebBrowser (the gldf-search.de panel)
            "Slate",
            "SlateCore",
            "GldfRuntime",  // AGldfLuminaireActor
        });
    }
}
