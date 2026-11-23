#include "GldfSearchEditorModule.h"

#include "GldfSearchLibrary.h"
#include "SGldfWebPanel.h"
#include "GldfLuminaireActor.h"
#include "InterchangeGldfPipeline.h"
#include "InterchangeGldfTranslator.h"

#include "Editor.h"
#include "Framework/Application/SlateApplication.h"
#include "Framework/Docking/TabManager.h"
#include "InterchangeManager.h"
#include "InterchangeProjectSettings.h"
#include "InterchangeSourceData.h"
#include "LevelEditorViewport.h"
#include "Modules/ModuleManager.h"
#include "Widgets/Docking/SDockTab.h"
#include "WorkspaceMenuStructure.h"
#include "WorkspaceMenuStructureModule.h"

#define LOCTEXT_NAMESPACE "GldfSearch"

DEFINE_LOG_CATEGORY_STATIC(LogGldfSearch, Log, All);

// Evaluation builds: see GldfImporterModule.cpp — same generated-header gate.
#if __has_include("GldfEvalExpiry.h")
#include "GldfEvalExpiry.h"
#endif
#ifndef GLDF_EVAL_EXPIRY_UNIX
#define GLDF_EVAL_EXPIRY_UNIX 0
#endif

static const FName GldfSearchTabId("GldfSearch");

static TSharedRef<SDockTab> SpawnGldfSearchTab(const FSpawnTabArgs&)
{
    return SNew(SDockTab).TabRole(ETabRole::NomadTab)[ SNew(SGldfWebPanel) ];
}

void FGldfSearchEditorModule::ImportAndPlace(const FString& LocalPath, const FString& /*DisplayName*/)
{
    // 4. Import via Interchange — spawns the AGldfLuminaireActor (the
    //    same path a drag-import / the Python helper uses). Capture the
    //    actor via the scene-object-done delegate.
    UInterchangeManager& Mgr = UInterchangeManager::GetInterchangeManager();
    UInterchangeSourceData* Source = UInterchangeManager::CreateSourceData(LocalPath);

    FImportAssetParameters Params;
    Params.bIsAutomated = true;  // no modal dialog

    // CRITICAL: a programmatic ImportScene does NOT apply our
    // PerTranslatorPipelines entry the way the interactive "Import Into
    // Level" dialog does — it would run the default Scene stack, which
    // lacks UInterchangeGldfPipeline, so the root never gets retyped to
    // AGldfLuminaireActor and no emitter lights are built (→ dark).
    // Fix: pass the SAME pipeline list explicitly via OverridePipelines
    // — the Scene stack's base pipelines + our pipeline appended last
    // (mirrors FGldfImporterModule::RegisterGldfScenePipeline).
    if (const UInterchangeProjectSettings* Settings = GetDefault<UInterchangeProjectSettings>())
    {
        const FInterchangePipelineStack* SceneStack =
            Settings->SceneImportSettings.PipelineStacks.Find(FName(TEXT("Scene")));
        if (SceneStack)
        {
            // Prefer the GLDF per-translator list if present (base + ours);
            // else fall back to base + append ours.
            const TSoftClassPtr<UInterchangeTranslatorBase> GldfTranslatorClass(
                UInterchangeGldfTranslator::StaticClass());
            const FInterchangeTranslatorPipelines* PerGldf =
                SceneStack->PerTranslatorPipelines.FindByPredicate(
                    [&](const FInterchangeTranslatorPipelines& E)
                    { return E.Translator == GldfTranslatorClass; });
            if (PerGldf)
            {
                Params.OverridePipelines = PerGldf->Pipelines;
            }
            else
            {
                Params.OverridePipelines = SceneStack->Pipelines;
                Params.OverridePipelines.Add(
                    FSoftObjectPath(UInterchangeGldfPipeline::StaticClass()->GetDefaultObject()));
            }
        }
    }
    // Last-resort: at least our pipeline, so the root gets retyped.
    if (Params.OverridePipelines.Num() == 0)
    {
        Params.OverridePipelines.Add(
            FSoftObjectPath(UInterchangeGldfPipeline::StaticClass()->GetDefaultObject()));
    }
    Params.OnSceneObjectDoneNative.BindLambda(
        [](UObject* Obj)
        {
            if (AGldfLuminaireActor* Lum = Cast<AGldfLuminaireActor>(Obj))
            {
                // ImportScene spawns at the world origin, which in a
                // real level is usually below the floor / outside the
                // building. Place the luminaire where the user is
                // looking instead: the actor snaps itself to the hit
                // surface honouring its GLDF mounting (recess depth,
                // pendant length, wall height, pole); if the camera ray
                // hits nothing, park it 4 m ahead.
                if (GCurrentLevelEditingViewportClient)
                {
                    const FVector Start = GCurrentLevelEditingViewportClient->GetViewLocation();
                    const FVector Dir = GCurrentLevelEditingViewportClient->GetViewRotation().Vector();
                    if (!Lum->PlaceFromRay(Start, Dir))
                    {
                        Lum->SetActorLocation(Start + Dir * 400.0f);
                    }
                }
                if (GEditor)
                {
                    GEditor->SelectNone(false, true);
                    GEditor->SelectActor(Lum, true, true);
                }
                UE_LOG(LogGldfSearch, Log, TEXT("PlaceFromUrl: placed %s at %s"),
                    *Lum->GetName(), *Lum->GetActorLocation().ToCompactString());
            }
        });

    const bool bStarted = Mgr.ImportScene(TEXT("/Game/GLDF"), Source, Params);
    UE_LOG(LogGldfSearch, Log,
        TEXT("PlaceFromUrl: ImportScene started=%d for %s"), bStarted ? 1 : 0, *LocalPath);
}

void FGldfSearchEditorModule::StartupModule()
{
#if GLDF_EVAL_EXPIRY_UNIX
    if (FDateTime::UtcNow() > FDateTime::FromUnixTimestamp(GLDF_EVAL_EXPIRY_UNIX))
    {
        return; // expired: no tabs. GldfImporterModule logs the details.
    }
#endif
    // In the editor a downloaded .gldf becomes real assets + an actor.
    UGldfSearchLibrary::PlaceHandler().BindStatic(&FGldfSearchEditorModule::ImportAndPlace);

    // Window > Tools > GLDF Search — the gldf-search.de web UI, Place → Interchange import.
    FGlobalTabmanager::Get()
        ->RegisterNomadTabSpawner(GldfSearchTabId, FOnSpawnTab::CreateStatic(&SpawnGldfSearchTab))
        .SetDisplayName(LOCTEXT("TabTitle", "GLDF Search"))
        .SetTooltipText(LOCTEXT("TabTip", "Search gldf-search.de and place luminaires in the level"))
        .SetGroup(WorkspaceMenu::GetMenuStructure().GetToolsCategory());
}

void FGldfSearchEditorModule::ShutdownModule()
{
    UGldfSearchLibrary::PlaceHandler().Unbind();
    if (FSlateApplication::IsInitialized())
    {
        FGlobalTabmanager::Get()->UnregisterNomadTabSpawner(GldfSearchTabId);
    }
}

#undef LOCTEXT_NAMESPACE

IMPLEMENT_MODULE(FGldfSearchEditorModule, GldfSearchEditor)
