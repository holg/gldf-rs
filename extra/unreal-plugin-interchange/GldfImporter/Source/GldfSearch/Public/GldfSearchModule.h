// GldfSearch module (Runtime) — "search gldf-search.de → download → place".
// Links the gldf-search-cabi Rust staticlib for the query side and uses UE's
// HTTP module to fetch the chosen .gldf; placement is delegated through
// UGldfSearchLibrary::PlaceHandler() (Interchange in the editor, the runtime
// loader in games). Ships in packaged builds.

#pragma once

#include "CoreMinimal.h"
#include "Modules/ModuleManager.h"

class FGldfSearchModule : public IModuleInterface
{
public:
    virtual void StartupModule() override;
    virtual void ShutdownModule() override;
};
