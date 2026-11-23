// GldfSearchEditor — the editor half of GLDF Search: the Window > Tools tab
// (hosting the runtime web panel) and the Interchange import that places a
// downloaded .gldf as assets + actor. Everything a packaged game needs is in
// the Runtime module GldfSearch.
using UnrealBuildTool;

public class GldfSearchEditor : ModuleRules
{
    public GldfSearchEditor(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = ModuleRules.PCHUsageMode.UseExplicitOrSharedPCHs;

        PublicDependencyModuleNames.AddRange(new string[] { "Core", "CoreUObject", "Engine" });
        PrivateDependencyModuleNames.AddRange(new string[] {
            "GldfSearch",
            "GldfRuntime",
            "GldfImporter",            // the Interchange translator/factory/pipeline
            "InterchangeCore",
            "InterchangeEngine",
            "UnrealEd",
            "Slate",
            "SlateCore",
            "InputCore",
            "WorkspaceMenuStructure",  // Window > Tools menu category for the tab
        });
    }
}
