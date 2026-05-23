// Copyright (c) gldf-rs project. Licensed under the workspace AGPL-3.0+
// (or any dual-licence applied to crates/gldf-unreal). Untested scaffold.

#pragma once

#include "CoreMinimal.h"
#include "Modules/ModuleManager.h"

class FGldfRsImporterModule : public IModuleInterface
{
public:
    // IModuleInterface
    virtual void StartupModule() override;
    virtual void ShutdownModule() override;

private:
    void RegisterMenus();
    void OnImportGldfClicked();
};
