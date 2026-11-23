// Illuminance heatmap: a planar grid of receiver points draped over the ground
// around a centre, coloured by horizontal illuminance (lux) from every GLDF
// luminaire in the level. Photometry (IES/LDT interpolation, symmetry, flux
// scaling, inverse-square + cosine law) is computed by the eulumdat core via
// the gldf_unreal FFI, one layer per emitter; UE line traces supply the
// occlusion mask (shadows) before the layers are summed.
//
// Works in editor and in packaged games (runtime module). The demo binds it to
// a key; designers can drop one into a level and press Recompute.
#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "GldfIlluminanceGrid.generated.h"

class UProceduralMeshComponent;
class UMaterialInterface;
class AGldfLuminaireActor;

USTRUCT(BlueprintType)
struct FGldfIlluminanceStats
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance") float MinLux = 0.f;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance") float AvgLux = 0.f;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance") float MaxLux = 0.f;
    /** U0 = Emin / Eavg (EN 12464 / EN 13201 overall uniformity). */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance") float UniformityMinAvg = 0.f;
    /** Ud = Emin / Emax. */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance") float UniformityMinMax = 0.f;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance") int32 Cells = 0;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance") int32 Emitters = 0;
    /** Top of the colour scale actually used (lux). */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance") float ScaleMaxLux = 0.f;
};

/** One measurement point shown as a mini pole with a coloured pad and (via the
 *  HUD) its computed value — the way lighting-design tools show calc points. */
USTRUCT(BlueprintType)
struct FGldfLuxMarker
{
    GENERATED_BODY()
    /** Pad centre (world, at the receiver height). */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance") FVector WorldPos = FVector::ZeroVector;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance") float Lux = 0.f;
};

UCLASS(BlueprintType, Blueprintable, meta = (DisplayName = "GLDF Illuminance Grid"))
class GLDFRUNTIME_API AGldfIlluminanceGrid : public AActor
{
    GENERATED_BODY()

public:
    AGldfIlluminanceGrid();

    /** Heatmap mesh (vertex-coloured, unlit, translucent). */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Illuminance")
    TObjectPtr<UProceduralMeshComponent> Mesh;

    /** Grid side length in cm (square, centred on the actor). */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Illuminance", meta = (ClampMin = "100", ClampMax = "20000"))
    float SizeCm = 3000.f;

    /** Cell size in cm (0.5 m = EN 13201 style spacing for streets). */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Illuminance", meta = (ClampMin = "10", ClampMax = "500"))
    float CellCm = 50.f;

    /** Receiver height above the found ground (0 = road surface, 85 = desk). */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Illuminance", meta = (ClampMin = "0", ClampMax = "300"))
    float ReceiverHeightCm = 0.f;

    /** Shadow traces stop this far short of the emitter, so a fixture's own
     *  housing / glass (e.g. a GLDF emitter placed inside a scene lantern)
     *  does not shadow its own light. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Illuminance", meta = (ClampMin = "5", ClampMax = "300"))
    float EmitterClearanceCm = 70.f;

    /** Trace from each cell to each emitter and drop occluded contributions. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Illuminance")
    bool bOcclusion = true;

    /** Height above the actor from which each cell's ground trace starts (keep
     *  it below awnings / balconies so the grid stays on the street). */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Illuminance", meta = (ClampMin = "50", ClampMax = "2000"))
    float DrapeStartHeightCm = 220.f;

    /** Surfaces higher than this above the grid origin are skipped by the
     *  drape (tables, cabinets, awnings): the measurement plane follows the
     *  floor and steps, not the furniture. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Illuminance", meta = (ClampMin = "0", ClampMax = "1000"))
    float DrapeMaxRiseCm = 120.f;

    /** Follow terrain: trace down per cell to find the ground. Off = flat at the actor's Z. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Illuminance")
    bool bDrapeOnGround = true;

    /** 0 = auto (max lux rounded up); otherwise fixed top of the colour scale. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Illuminance", meta = (ClampMin = "0"))
    float ScaleMaxLux = 0.f;

    /** Draw the continuous colour sheet. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Illuminance")
    bool bShowSheet = true;

    /** Draw measurement poles: a thin post from the ground up to the receiver
     *  height with a colour pad on top, every MarkerSpacingCm. The HUD / a
     *  widget prints each pad's lux value (see Markers). */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Illuminance")
    bool bShowMarkers = true;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Illuminance", meta = (ClampMin = "25", ClampMax = "2000"))
    float MarkerSpacingCm = 200.f;

    /** Pad side length of a measurement pole in cm. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Illuminance", meta = (ClampMin = "5", ClampMax = "100"))
    float MarkerPadCm = 22.f;

    /** Measurement points of the last Recompute() (pad position + lux). */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance")
    TArray<FGldfLuxMarker> Markers;

    /** Heatmap opacity. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Illuminance", meta = (ClampMin = "0.05", ClampMax = "1"))
    float Opacity = 0.75f;

    /** Results of the last Recompute(). */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance")
    FGldfIlluminanceStats Stats;

    /** Lux per cell, row-major [iy * Nx + ix], after occlusion and summing. */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance")
    TArray<float> Lux;

    /** Per cell: a surface was found under it (drape). Cells without one are
     *  neither drawn nor counted (a grid larger than the room). */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance")
    TArray<bool> CellHasSurface;

    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance") int32 Nx = 0;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF|Illuminance") int32 Ny = 0;

    /** Compute the grid from every AGldfLuminaireActor in the world and rebuild the mesh. */
    UFUNCTION(BlueprintCallable, CallInEditor, Category = "GLDF|Illuminance")
    void Recompute();

    /** Lux at a world position (nearest cell), or -1 outside the grid. */
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "GLDF|Illuminance")
    float LuxAt(const FVector& WorldPos) const;

    /** False-colour for a lux value on the current scale (isolux ramp). */
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "GLDF|Illuminance")
    static FLinearColor LuxToColor(float LuxValue, float ScaleMax);

    /** Heatmap material (plugin content). Soft so the class default object does
     *  not root the asset (editing it in place would assert); the cooker still
     *  follows soft references. */
    UPROPERTY(EditAnywhere, Category = "GLDF|Illuminance")
    TSoftObjectPtr<UMaterialInterface> HeatmapMaterial;

private:

    TArray<FVector> CellCentres;   // world space
    FVector GridOrigin, GridU, GridV, GridNormal;
};
