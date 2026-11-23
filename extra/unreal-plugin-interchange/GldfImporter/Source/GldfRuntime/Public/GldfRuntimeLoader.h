// Runtime GLDF loader: build a complete AGldfLuminaireActor from a .gldf file
// WITHOUT the editor / Interchange — in a packaged game, from Blueprint or C++.
//
// The editor import (UInterchangeGldfTranslator + UInterchangeGldfActorFactory)
// produces assets (UStaticMesh, UTextureLightProfile) through the Interchange
// pipeline. Here the same Rust FFI (gldf_unreal) feeds transient objects
// instead: the L3D body becomes a UStaticMesh via BuildFromMeshDescriptions
// (fast build), every emitter's IES text becomes a UTextureLightProfile via the
// engine's IESFile converter, and the variants / mounting / product metadata
// land on the actor exactly as the factory would store them — so snapping,
// variant switching, the illuminance heatmap and the HUD work unchanged.
#pragma once

#include "CoreMinimal.h"
#include "Kismet/BlueprintFunctionLibrary.h"
#include "GldfRuntimeLoader.generated.h"

class AGldfLuminaireActor;
class UStaticMesh;
class UTextureLightProfile;

UCLASS()
class GLDFRUNTIME_API UGldfRuntimeLoader : public UBlueprintFunctionLibrary
{
    GENERATED_BODY()

public:
    /** Spawn a luminaire at `Transform` and fill it from `GldfPath` (absolute
     *  path to a .gldf on disk). Returns nullptr and sets `OutError` when the
     *  file cannot be read or has no usable geometry / photometry. The caller
     *  usually follows up with PlaceFromRay() to snap it onto a surface. */
    UFUNCTION(BlueprintCallable, Category = "GLDF|Runtime",
              meta = (WorldContext = "WorldContextObject"))
    static AGldfLuminaireActor* SpawnLuminaireFromFile(
        UObject* WorldContextObject, const FString& GldfPath,
        const FTransform& Transform, FString& OutError);

    /** Fill an existing (freshly spawned, empty) luminaire from a .gldf: body
     *  mesh, one light per LEO, variants with per-emitter photometry, mounting
     *  info, product strings. Applies the first variant. */
    UFUNCTION(BlueprintCallable, Category = "GLDF|Runtime")
    static bool PopulateLuminaire(AGldfLuminaireActor* Luminaire, const FString& GldfPath, FString& OutError);

    /** Transient UStaticMesh from the GLDF's first L3D mesh (positions in cm,
     *  UE basis, one material slot "GLDF_Body"). nullptr + OutError on failure. */
    static UStaticMesh* BuildBodyMesh(const FString& GldfPath, FString& OutError);

    /** Transient UTextureLightProfile from IES (LM-63) text. nullptr if the
     *  engine's IES converter rejects the data. */
    static UTextureLightProfile* BuildLightProfile(const uint8* IesBytes, uint32 Len, const FString& Name);

    /** All *.gldf files directly inside the given directories (non-recursive),
     *  absolute paths, sorted, duplicates removed. Missing directories are skipped. */
    UFUNCTION(BlueprintCallable, Category = "GLDF|Runtime")
    static TArray<FString> FindGldfFiles(const TArray<FString>& Directories);
};
