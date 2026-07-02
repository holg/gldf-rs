// Registers UInterchangeGldfTranslator with the InterchangeManager so
// .gldf files import natively through UE 5.7's Interchange pipeline.

#include "GldfImporterModule.h"

#include "InterchangeGldfTranslator.h"
#include "InterchangeGldfPipeline.h"
#include "InterchangeGldfActorFactory.h"
#include "InterchangeManager.h"
#include "InterchangeProjectSettings.h"
#include "Logging/LogMacros.h"
#include "UObject/SoftObjectPath.h"

// Pull in the Rust C ABI so we can log the linked version at startup.
// PublicIncludePaths in GldfImporter.Build.cs points at
// crates/gldf-unreal/include/.
extern "C" {
#include "gldf_unreal.h"
}

// Evaluation builds: package_eval_mac.sh drops a GldfEvalExpiry.h with
// GLDF_EVAL_EXPIRY_UNIX into Private/ before building and removes it after.
// Normal dev builds have no header → no expiry code compiled at all.
#if __has_include("GldfEvalExpiry.h")
#include "GldfEvalExpiry.h"
#endif
#ifndef GLDF_EVAL_EXPIRY_UNIX
#define GLDF_EVAL_EXPIRY_UNIX 0
#endif

DEFINE_LOG_CATEGORY_STATIC(LogGldfImporter, Log, All);

void FGldfImporterModule::StartupModule()
{
#if GLDF_EVAL_EXPIRY_UNIX
    const FDateTime Expiry = FDateTime::FromUnixTimestamp(GLDF_EVAL_EXPIRY_UNIX);
    if (FDateTime::UtcNow() > Expiry)
    {
        UE_LOG(LogGldfImporter, Error,
            TEXT("GldfImporter evaluation build expired on %s — .gldf import disabled. "
                 "Contact the gldf-rs project for a current build."),
            *Expiry.ToString());
        return; // nothing registered: no translator, no factory, no pipeline
    }
    UE_LOG(LogGldfImporter, Warning,
        TEXT("GldfImporter EVALUATION build — expires %s (%d day(s) left)"),
        *Expiry.ToString(), FMath::Max(0, (int32)(Expiry - FDateTime::UtcNow()).GetDays()));
#endif

    UE_LOG(LogGldfImporter, Log,
        TEXT("GldfImporter starting (gldf-unreal Rust lib version: %hs)"),
        gldf_unreal_version());

    // Same pattern UE's own InterchangeImport module uses to register
    // its translators — see
    // Engine/Plugins/Interchange/Runtime/Source/Import/Private/InterchangeImportModule.cpp.
    UInterchangeManager& InterchangeManager = UInterchangeManager::GetInterchangeManager();
    InterchangeManager.RegisterTranslator(UInterchangeGldfTranslator::StaticClass());

    // Register our actor factory (GetFactoryClass() == AGldfLuminaireActor) so
    // any actor factory node whose object class is AGldfLuminaireActor — which
    // our pipeline produces by retyping the root — is built by us, filling the
    // body mesh + emitter light + variant table on a single self-contained
    // luminaire actor. Most-derived IsChildOf wins, so this beats the stock
    // AStaticMeshActor factory for that node.
    InterchangeManager.RegisterFactory(UInterchangeGldfActorFactory::StaticClass());

    RegisterGldfScenePipeline();
}

void FGldfImporterModule::RegisterGldfScenePipeline()
{
    // Register our pipeline programmatically rather than via ini: a single
    // malformed token in a SceneImportSettings= override silently discards
    // the whole struct (falling back to the engine default with no GLDF
    // entry, and no error logged). Mutating the settings object here is
    // verifiable via the log below and immune to ini-parsing quirks.
    //
    // What we do: in the default "Scene" pipeline stack, add a
    // PerTranslatorPipelines entry for UInterchangeGldfTranslator whose
    // pipeline list = the stack's normal pipelines + our
    // UInterchangeGldfPipeline (which retypes the GLDF root actor to
    // AGldfLuminaireActor). PerTranslatorPipelines REPLACES the stack list
    // for a matching translator, so we must include the defaults too.
    UInterchangeProjectSettings* Settings = GetMutableDefault<UInterchangeProjectSettings>();
    if (!Settings)
    {
        UE_LOG(LogGldfImporter, Warning,
            TEXT("RegisterGldfScenePipeline: no InterchangeProjectSettings"));
        return;
    }

    const FName SceneStackName(TEXT("Scene"));
    FInterchangePipelineStack* SceneStack =
        Settings->SceneImportSettings.PipelineStacks.Find(SceneStackName);
    if (!SceneStack)
    {
        UE_LOG(LogGldfImporter, Warning,
            TEXT("RegisterGldfScenePipeline: no 'Scene' pipeline stack to extend"));
        return;
    }

    // Don't double-register if StartupModule runs twice.
    const TSoftClassPtr<UInterchangeTranslatorBase> GldfTranslatorClass(
        UInterchangeGldfTranslator::StaticClass());
    for (const FInterchangeTranslatorPipelines& Existing : SceneStack->PerTranslatorPipelines)
    {
        if (Existing.Translator == GldfTranslatorClass)
        {
            return; // already registered
        }
    }

    FInterchangeTranslatorPipelines GldfEntry;
    GldfEntry.Translator = GldfTranslatorClass;
    // Start from the stack's normal pipelines (scene assets + level), then
    // append ours so it runs last (after the generic level pipeline built
    // the actor factory nodes we retype).
    GldfEntry.Pipelines = SceneStack->Pipelines;
    GldfEntry.Pipelines.Add(FSoftObjectPath(UInterchangeGldfPipeline::StaticClass()->GetDefaultObject()));

    SceneStack->PerTranslatorPipelines.Add(MoveTemp(GldfEntry));

    UE_LOG(LogGldfImporter, Log,
        TEXT("RegisterGldfScenePipeline: GLDF translator → Scene stack + UInterchangeGldfPipeline "
             "(%d base pipelines + ours)"),
        SceneStack->Pipelines.Num());
}

void FGldfImporterModule::ShutdownModule()
{
    // InterchangeManager outlives our module on editor shutdown; UE
    // handles translator deregistration via UObject GC.
}

IMPLEMENT_MODULE(FGldfImporterModule, GldfImporter)
