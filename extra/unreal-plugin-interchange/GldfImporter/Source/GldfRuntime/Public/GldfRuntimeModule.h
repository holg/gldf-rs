// GldfRuntime module — runtime half of the GLDF plugin (AGldfLuminaireActor).
// Loaded in editor AND packaged games; registers nothing, it only has to
// exist so the luminaire class is available at cook + runtime.

#pragma once

#include "CoreMinimal.h"
#include "Modules/ModuleManager.h"

class FGldfRuntimeModule : public IModuleInterface
{
public:
    virtual void StartupModule() override;
    virtual void ShutdownModule() override;
};
