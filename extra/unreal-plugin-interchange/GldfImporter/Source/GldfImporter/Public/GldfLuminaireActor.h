// A single placed GLDF luminaire: body mesh + one light per emitter, all
// components of ONE actor, so the user drags one object into the scene
// rather than hunting for a root among separate mesh/light actors.
//
// Multi-emitter aware (Phase 3c.2): a luminaire can have several light-
// emitting objects (LEOs) — e.g. a twin-head spotlight with LEO0 + LEO1,
// each its own beam at its own position. We create one light component per
// emitter, keyed by the LEO partName, and a variant carries a per-emitter
// photometry table so switching variants re-applies the right IES /
// intensity / colour temperature to each head.
//
// The GLDF Interchange import (see UInterchangeGldfActorFactory) spawns
// this actor, assigns its StaticMesh, builds the emitter lights, and
// records the available variants.

#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "GldfLuminaireActor.generated.h"

class UStaticMeshComponent;
class ULightComponent;
class ULocalLightComponent;
class UTextureLightProfile;

/** Photometry for ONE emitter within a variant. The variant holds one of
 *  these per LEO partName, so a twin-head luminaire can give each head its
 *  own IES + flux. */
USTRUCT(BlueprintType)
struct FGldfEmitterPhotometry
{
    GENERATED_BODY()

    /** The LEO partName this applies to (e.g. "LEO0"). Matches the
     *  partName on the emitter component the factory created. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    FString EmitterName;

    /** IES profile for this emitter in this variant. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF")
    TObjectPtr<UTextureLightProfile> IesProfile = nullptr;

    /** Luminous flux in lumens (0 if the GLDF didn't specify it). */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    int32 Lumens = 0;

    /** Correlated colour temperature in Kelvin (0 if unspecified). */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    int32 Cct = 0;
};

/** One resolved GLDF variant: the per-emitter photometry table, cached on
 *  the actor so the variant dropdown can hot-swap without re-reading the
 *  source file. */
USTRUCT(BlueprintType)
struct FGldfLuminaireVariant
{
    GENERATED_BODY()

    /** The GLDF variant id (e.g. "var_01"). Stable key, not shown in the
     *  dropdown. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    FString VariantId;

    /** Human display name (GLDF Variant.Name, e.g. "NEW TRIA 2 - narrow").
     *  This is what the variant dropdown shows; falls back to the id if the
     *  GLDF didn't name the variant. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    FString VariantName;

    /** One entry per emitter (LEO). For a single-emitter luminaire this
     *  has one element; a twin-head spotlight has two. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF")
    TArray<FGldfEmitterPhotometry> Emitters;
};

/** One emitter light on the luminaire, paired with the LEO partName that
 *  identifies it across variants. The component is either a Rect or a
 *  Point light (per the LEO shape); we keep it as the common base so the
 *  actor code is shape-agnostic. */
USTRUCT(BlueprintType)
struct FGldfEmitterComponent
{
    GENERATED_BODY()

    /** The LEO partName (e.g. "LEO0") — matched against a variant's
     *  FGldfEmitterPhotometry::EmitterName to find this emitter's data. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    FString EmitterName;

    /** The light component (a URectLightComponent or UPointLightComponent),
     *  held as its local-light base. Attached under the body mesh. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    TObjectPtr<ULocalLightComponent> Light = nullptr;
};

UCLASS(BlueprintType, Blueprintable,
       meta = (DisplayName = "GLDF Luminaire"))
class GLDFIMPORTER_API AGldfLuminaireActor : public AActor
{
    GENERATED_BODY()

public:
    AGldfLuminaireActor();

    /** Luminaire body. Root component — moving the actor moves the body and
     *  every emitter light beneath it. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    TObjectPtr<UStaticMeshComponent> BodyMesh;

    /** One light per emitter (LEO). Built by the import to match each LEO's
     *  shape + world position. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    TArray<FGldfEmitterComponent> Emitters;

    /** All variants found in the source GLDF. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF")
    TArray<FGldfLuminaireVariant> Variants;

    /** The active variant, chosen by NAME from a dropdown in the Details
     *  panel. The `GetOptions` meta makes UE render this FName as a combo
     *  box populated by GetVariantNameOptions() (the variant display names)
     *  — so the user picks "NEW TRIA 2 - narrow", not an index. Changing it
     *  re-applies that variant's per-emitter photometry to every light. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF",
              meta = (GetOptions = "GetVariantNameOptions"))
    FName SelectedVariantName;

    /** The variant display names, for the SelectedVariantName dropdown.
     *  Order matches Variants. */
    UFUNCTION()
    TArray<FString> GetVariantNameOptions() const;

    /** Index of the variant whose VariantName == SelectedVariantName, or 0
     *  if unset / not found (so there's always a sane default). */
    int32 ResolveSelectedVariantIndex() const;

    /** Apply the selected variant to every emitter light — matching each
     *  emitter component's name to the variant's per-emitter entry, setting
     *  IES, intensity (lumens), and colour temperature. Safe to call with no
     *  selection or unmatched emitters; it no-ops the parts it can't
     *  resolve. */
    UFUNCTION(BlueprintCallable, Category = "GLDF")
    void ApplySelectedVariant();

    // ── Aim helpers ─────────────────────────────────────────────────────
    // One-click orientation buttons in the Details panel (CallInEditor),
    // mainly for aiming spots / pot lights without wrestling the rotation
    // gizmo. All rotate the WHOLE actor (body + emitter lights together).

    /** Back to the as-designed orientation (identity — for a downlight
     *  that's "pointing straight down"). */
    UFUNCTION(CallInEditor, BlueprintCallable, Category = "GLDF|Aim")
    void ResetAim();

    /** Tilt the beam 15° (local pitch). Click repeatedly for more. */
    UFUNCTION(CallInEditor, BlueprintCallable, Category = "GLDF|Aim")
    void Tilt15();

    /** Tilt the beam back -15° (local pitch). */
    UFUNCTION(CallInEditor, BlueprintCallable, Category = "GLDF|Aim")
    void TiltBack15();

    /** Spin 45° around the vertical axis (aim the tilt direction). */
    UFUNCTION(CallInEditor, BlueprintCallable, Category = "GLDF|Aim")
    void Spin45();

#if WITH_EDITOR
    /** Re-apply the variant when SelectedVariantName is edited in the Details
     *  panel — gives the live hot-swap. */
    virtual void PostEditChangeProperty(FPropertyChangedEvent& PropertyChangedEvent) override;
#endif
};
