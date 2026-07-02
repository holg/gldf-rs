// Custom Interchange pipeline for GLDF scene imports.
//
// Runs AFTER UInterchangeGenericLevelPipeline in the scene pipeline stack
// (so the generic pipeline has already built actor factory nodes from our
// translator's scene nodes). Its job: retype the GLDF root actor factory
// node to AGldfLuminaireActor, so the placed luminaire is a single custom
// actor — the body mesh + emitter light attach beneath it and move as one,
// and the actor carries the variant dropdown (Phase 3c).
//
// We do NOT do factory-graph surgery beyond setting the root's actor class:
// the generic pipeline already parents the mesh + light factory nodes under
// the root via the parent UIDs our translator set. Retyping the root is the
// minimal, robust change.

#pragma once

#include "CoreMinimal.h"
#include "InterchangePipelineBase.h"
#include "InterchangeGldfPipeline.generated.h"

class UInterchangeBaseNodeContainer;
class UInterchangeSourceData;

UCLASS(BlueprintType, editinlinenew,
       meta = (DisplayName = "GLDF Luminaire Pipeline"))
class GLDFIMPORTER_API UInterchangeGldfPipeline : public UInterchangePipelineBase
{
    GENERATED_BODY()

protected:
    virtual void ExecutePipeline(
        UInterchangeBaseNodeContainer* BaseNodeContainer,
        const TArray<UInterchangeSourceData*>& SourceDatas,
        const FString& ContentBasePath) override;

    // Editor-only pipeline; nothing to do at runtime.
    virtual bool CanExecuteOnAnyThread(EInterchangePipelineTask PipelineTask) override
    {
        return false;
    }
};
