// Registers UInterchangeGldfTranslator with the InterchangeManager so
// .gldf files import natively through UE 5.7's Interchange pipeline.

#include "GldfImporterModule.h"

#include "InterchangeGldfTranslator.h"
#include "InterchangeManager.h"
#include "Logging/LogMacros.h"

// Pull in the Rust C ABI so we can log the linked version at startup.
// PublicIncludePaths in GldfImporter.Build.cs points at
// crates/gldf-unreal/include/.
extern "C" {
#include "gldf_unreal.h"
}

DEFINE_LOG_CATEGORY_STATIC(LogGldfImporter, Log, All);

void FGldfImporterModule::StartupModule()
{
    UE_LOG(LogGldfImporter, Log,
        TEXT("GldfImporter starting (gldf-unreal Rust lib version: %hs)"),
        gldf_unreal_version());

    // Same pattern UE's own InterchangeImport module uses to register
    // its translators — see
    // Engine/Plugins/Interchange/Runtime/Source/Import/Private/InterchangeImportModule.cpp.
    UInterchangeManager& InterchangeManager = UInterchangeManager::GetInterchangeManager();
    InterchangeManager.RegisterTranslator(UInterchangeGldfTranslator::StaticClass());
}

void FGldfImporterModule::ShutdownModule()
{
    // InterchangeManager outlives our module on editor shutdown; UE
    // handles translator deregistration via UObject GC.
}

IMPLEMENT_MODULE(FGldfImporterModule, GldfImporter)
