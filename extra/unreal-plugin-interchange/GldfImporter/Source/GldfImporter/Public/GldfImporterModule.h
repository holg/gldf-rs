// GldfImporter module — registers UInterchangeGldfTranslator with UE's
// Interchange framework so .gldf files import natively.
//
// Phase 1: IES only.

#pragma once

#include "CoreMinimal.h"
#include "Modules/ModuleManager.h"

class FGldfImporterModule : public IModuleInterface
{
public:
    // IModuleInterface
    virtual void StartupModule() override;
    virtual void ShutdownModule() override;

private:
    /** Inject the GLDF per-translator pipeline into the Interchange "Scene"
     *  stack so .gldf scene imports retype the root to AGldfLuminaireActor.
     *  Done in code (not ini) for reliability + log feedback. */
    void RegisterGldfScenePipeline();
};
