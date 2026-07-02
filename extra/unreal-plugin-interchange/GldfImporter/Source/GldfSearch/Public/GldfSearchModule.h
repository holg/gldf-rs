// GldfSearch module — the in-editor "search gldf-search.de → download → place"
// demo flow on top of the GldfImporter Interchange plugin. Links the
// gldf-search-cabi Rust staticlib for the query side; uses UE's HTTP module to
// fetch the chosen .gldf and the Interchange manager to import it.

#pragma once

#include "CoreMinimal.h"
#include "Modules/ModuleManager.h"

class FGldfSearchModule : public IModuleInterface
{
public:
    virtual void StartupModule() override;
    virtual void ShutdownModule() override;
};
