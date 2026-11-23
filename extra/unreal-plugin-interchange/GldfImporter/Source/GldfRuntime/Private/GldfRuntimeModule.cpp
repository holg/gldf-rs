#include "GldfRuntimeModule.h"

#include "Logging/LogMacros.h"

DEFINE_LOG_CATEGORY_STATIC(LogGldfRuntime, Log, All);

void FGldfRuntimeModule::StartupModule()
{
    UE_LOG(LogGldfRuntime, Log, TEXT("GldfRuntime starting (AGldfLuminaireActor available)"));
}

void FGldfRuntimeModule::ShutdownModule()
{
}

IMPLEMENT_MODULE(FGldfRuntimeModule, GldfRuntime)
