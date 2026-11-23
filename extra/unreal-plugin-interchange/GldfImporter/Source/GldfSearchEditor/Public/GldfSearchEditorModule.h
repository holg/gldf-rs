#pragma once

#include "CoreMinimal.h"
#include "Modules/ModuleManager.h"

class FGldfSearchEditorModule : public IModuleInterface
{
public:
    virtual void StartupModule() override;
    virtual void ShutdownModule() override;

    /** Import a downloaded .gldf through Interchange (assets under /Game/GLDF)
     *  and place the resulting luminaire where the level viewport looks. */
    static void ImportAndPlace(const FString& LocalPath, const FString& DisplayName);
};
