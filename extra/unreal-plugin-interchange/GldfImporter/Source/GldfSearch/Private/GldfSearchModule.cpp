#include "GldfSearchModule.h"

#include "SGldfSearchPanel.h"
#include "SGldfWebPanel.h"

#include "Modules/ModuleManager.h"
#include "Framework/Docking/TabManager.h"
#include "Framework/Application/SlateApplication.h"
#include "Widgets/Docking/SDockTab.h"
#include "WorkspaceMenuStructure.h"
#include "WorkspaceMenuStructureModule.h"

#define LOCTEXT_NAMESPACE "GldfSearch"

// Evaluation builds: see GldfImporterModule.cpp — same generated-header gate.
#if __has_include("GldfEvalExpiry.h")
#include "GldfEvalExpiry.h"
#endif
#ifndef GLDF_EVAL_EXPIRY_UNIX
#define GLDF_EVAL_EXPIRY_UNIX 0
#endif

static const FName GldfSearchTabId("GldfSearch");
static const FName GldfWebSearchTabId("GldfWebSearch");

static TSharedRef<SDockTab> SpawnGldfSearchTab(const FSpawnTabArgs& /*Args*/)
{
    return SNew(SDockTab)
        .TabRole(ETabRole::NomadTab)
        [
            SNew(SGldfSearchPanel)
        ];
}

static TSharedRef<SDockTab> SpawnGldfWebSearchTab(const FSpawnTabArgs& /*Args*/)
{
    return SNew(SDockTab)
        .TabRole(ETabRole::NomadTab)
        [
            SNew(SGldfWebPanel)
        ];
}

void FGldfSearchModule::StartupModule()
{
#if GLDF_EVAL_EXPIRY_UNIX
    if (FDateTime::UtcNow() > FDateTime::FromUnixTimestamp(GLDF_EVAL_EXPIRY_UNIX))
    {
        // Expired: register no tabs. GldfImporterModule logs the details.
        return;
    }
#endif

    // Window > Tools > GLDF Search.
    FGlobalTabmanager::Get()
        ->RegisterNomadTabSpawner(GldfSearchTabId, FOnSpawnTab::CreateStatic(&SpawnGldfSearchTab))
        .SetDisplayName(LOCTEXT("TabTitle", "GLDF Search"))
        .SetTooltipText(LOCTEXT("TabTip", "Search gldf-search.de and place luminaires"))
        .SetGroup(WorkspaceMenu::GetMenuStructure().GetToolsCategory());

    // CEF spike — Window > Tools > GLDF Search (Web). Hosts the deployed
    // gldf-search-leptos UI; exists to answer "does UE's bundled CEF render
    // the WASM app?" before committing to a webview migration.
    FGlobalTabmanager::Get()
        ->RegisterNomadTabSpawner(GldfWebSearchTabId, FOnSpawnTab::CreateStatic(&SpawnGldfWebSearchTab))
        .SetDisplayName(LOCTEXT("WebTabTitle", "GLDF Search (Web)"))
        .SetTooltipText(LOCTEXT("WebTabTip", "CEF spike: gldf-search.de in an embedded webview"))
        .SetGroup(WorkspaceMenu::GetMenuStructure().GetToolsCategory());
}

void FGldfSearchModule::ShutdownModule()
{
    if (FSlateApplication::IsInitialized())
    {
        FGlobalTabmanager::Get()->UnregisterNomadTabSpawner(GldfSearchTabId);
        FGlobalTabmanager::Get()->UnregisterNomadTabSpawner(GldfWebSearchTabId);
    }
}

#undef LOCTEXT_NAMESPACE

IMPLEMENT_MODULE(FGldfSearchModule, GldfSearch)
