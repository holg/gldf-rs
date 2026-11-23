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
class UMaterialInterface;

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

    /** The emitter's photometry as IES (LM-63) text, variant-resolved (lumens
     *  patched). Kept on the actor so the eulumdat core can compute
     *  illuminance at runtime (AGldfIlluminanceGrid), independent of the
     *  editor-only import. A few KB per emitter. Editable (advanced) so editor
     *  duplicate / copy-paste (text export) carries it along. */
    UPROPERTY(EditAnywhere, AdvancedDisplay, Category = "GLDF")
    TArray<uint8> IesBytes;
};

/** Primary mounting surface declared by the GLDF variant's <Mountings>. */
UENUM(BlueprintType)
enum class EGldfMountingKind : uint8
{
    None         UMETA(DisplayName = "Not specified"),
    Ceiling,
    Wall,
    Ground,
    WorkingPlane UMETA(DisplayName = "Working plane"),
};

/** The GLDF <Mountings> of one variant, flattened. Drives SnapToSurface():
 *  how deep a recessed body sinks into the ceiling, how far a pendant hangs,
 *  how high a wall luminaire sits above the floor, how tall the pole is.
 *  Lengths are millimetres as in the GLDF; -1 means "not specified". */
USTRUCT(BlueprintType)
struct FGldfMountingInfo
{
    GENERATED_BODY()

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting")
    EGldfMountingKind Kind = EGldfMountingKind::None;

    /** Ceiling/Wall <SurfaceMounted/>. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting")
    bool bSurfaceMounted = false;

    /** Ground/WorkingPlane <FreeStanding/>. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting")
    bool bFreeStanding = false;

    /** Ground <PoleTop/> declared (street / path luminaire head). */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting")
    bool bPoleTop = false;

    /** Ground <PoleIntegrated/> declared (luminaire IS the pole top). */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting")
    bool bPoleIntegrated = false;

    /** Ceiling/Wall <Recessed recessedDepth>: how far the body sinks INTO the
     *  mounting surface (SLV Tria 2: 11). -1 = not recessed. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting")
    int32 RecessedDepthMm = -1;

    /** Ceiling <Pendant pendantLength>: how far below the ceiling the body
     *  hangs. -1 = not a pendant. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting")
    int32 PendantLengthMm = -1;

    /** Wall mountingHeight: mounting point above the floor. -1 = unknown. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting")
    int32 MountingHeightMm = -1;

    /** Ground <PoleTop>/<PoleIntegrated> poleHeight. -1 = no pole / unknown. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting")
    int32 PoleHeightMm = -1;
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

    /** This variant's <Mountings> (GLDF declares mountings per variant). */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF")
    FGldfMountingInfo Mounting;

    /** Variant product number (article number) and GTIN, when the GLDF has them. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    FString ProductNumber;
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    FString Gtin;

    /** Rated input power of this variant in W (sum over emitters; 0 if unknown). */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    float Watts = 0.f;
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

/** One articulation of the L3D part tree (`<Joint>`): a pivot between a
 *  parent part and its child parts, rotating about its local X/Y/Z within
 *  the manufacturer's limits. Angles follow the L3D convention (degrees,
 *  applied X then Y then Z, right-handed Z-up); the actor converts to UE. */
USTRUCT(BlueprintType)
struct FGldfJoint
{
    GENERATED_BODY()

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Joints")
    FString Name;

    /** The pivot. Child parts and their emitter lights hang under it. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Joints")
    TObjectPtr<USceneComponent> Component = nullptr;

    /** Bit 0/1/2 = rotates about local X/Y/Z. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Joints")
    uint8 AxisMask = 0;

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Joints") FVector MinDeg = FVector::ZeroVector;
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Joints") FVector MaxDeg = FVector::ZeroVector;
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Joints") FVector StepDeg = FVector::ZeroVector;

    /** Current rotation about the joint axes (L3D convention). */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Joints")
    FVector AnglesDeg = FVector::ZeroVector;

    /** Rest pose relative to the parent part, already in the UE frame. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Joints") FVector RestLocation = FVector::ZeroVector;
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Joints") FQuat RestRotation = FQuat::Identity;
};

UCLASS(BlueprintType, Blueprintable,
       meta = (DisplayName = "GLDF Luminaire"))
class GLDFRUNTIME_API AGldfLuminaireActor : public AActor
{
    GENERATED_BODY()

public:
    AGldfLuminaireActor();

    /** Product identity from the GLDF header / metadata (for the HUD / specs). */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    FString Manufacturer;
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    FString ProductName;
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    FString ProductNumber;
    /** Source file name (no path), e.g. "SLV - Tria 2.gldf". */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    FString SourceFile;

    /** Luminaire body. Root component — moving the actor moves the body and
     *  every emitter light beneath it. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    TObjectPtr<UStaticMeshComponent> BodyMesh;

    /** One light per emitter (LEO). Built by the import to match each LEO's
     *  shape + world position. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF")
    TArray<FGldfEmitterComponent> Emitters;

    // ── Articulated parts (L3D joints) ──────────────────────────────────
    /** Joints of the L3D part tree (empty for a baked single-mesh import). */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Joints")
    TArray<FGldfJoint> Joints;

    /** Parts other than the root (BodyMesh), one static mesh each, attached
     *  under their joint. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Joints")
    TArray<TObjectPtr<UStaticMeshComponent>> PartMeshes;

    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "GLDF|Joints")
    bool HasJoints() const { return Joints.Num() > 0; }

    /** Rotate joint `Index` to `AnglesDeg` (clamped to its limits; axes the
     *  joint does not have are ignored). Moves every child part and light. */
    UFUNCTION(BlueprintCallable, Category = "GLDF|Joints")
    void SetJointAngles(int32 Index, FVector AnglesDeg);

    /** Point the beam(s) at a world position. With joints: solves the joint
     *  angles within their limits (pan/tilt heads move with their geometry).
     *  Without: rotates the emitter lights only. Returns false if nothing
     *  could be aimed. */
    UFUNCTION(BlueprintCallable, Category = "GLDF|Joints")
    bool AimAt(const FVector& WorldTarget);

    /** Convert the UE-frame <-> L3D-frame rotation (mirror of the Y axis). */
    static FQuat L3dToUeQuat(const FQuat& Q) { return FQuat(-Q.X, Q.Y, -Q.Z, Q.W); }
    static FVector L3dToUePos(const FVector& P) { return FVector(P.X, -P.Y, P.Z); }
    /** L3D Euler (deg, X then Y then Z) as a UE-frame quaternion. */
    static FQuat L3dEulerToUeQuat(const FVector& Deg);

    /** Re-point Joints[i].Component / PartMeshes after a template spawn or load. */
    UFUNCTION(BlueprintCallable, Category = "GLDF|Joints")
    void RebindPartComponents();

    /** Union of all part bounds in actor space (what snapping measures). */
    FBox ActorSpaceBodyBox() const;

    /** Auto-generated pole for ground/pole-top luminaires (street lights):
     *  a thin cylinder from the ground hit up to the luminaire base, sized
     *  from the GLDF poleHeight. Hidden unless SnapToSurface() put the
     *  luminaire on a pole. */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Mounting")
    TObjectPtr<UStaticMeshComponent> PoleMesh;

    /** Build the pole when the active variant is pole-mounted and the
     *  luminaire is snapped to the ground. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting")
    bool bAutoPole = true;

    /** Pole height used when the GLDF declares <PoleTop/> / <PoleIntegrated/>
     *  but no poleHeight (most street-light GLDFs). 6 m is a typical
     *  residential street. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting",
              meta = (ClampMin = "50.0", ClampMax = "2000.0"))
    float DefaultPoleHeightCm = 600.0f;

    /** User override for the mounting height in cm: pole height for ground /
     *  pole-top luminaires (bollards, street lights), height above the floor
     *  for wall luminaires. 0 = as declared in the GLDF (or DefaultPoleHeightCm).
     *  Call ReSnap() after changing it. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting", meta = (ClampMin = "0", ClampMax = "2000"))
    float HeightOverrideCm = 0.f;

    /** Re-run the last SnapToSurface() (same hit point) — after changing the
     *  variant, HeightOverrideCm or DefaultPoleHeightCm. Without a stored snap
     *  it finds the ground below (or the wall behind) the luminaire first. */
    UFUNCTION(BlueprintCallable, Category = "GLDF|Mounting")
    bool ReSnap();

    /** The hit the luminaire was last snapped to (kept so ReSnap() can repeat it). */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Mounting") bool bHasLastSnap = false;
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Mounting") FVector LastSnapLocation = FVector::ZeroVector;
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Mounting") FVector LastSnapNormal = FVector::UpVector;
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "GLDF|Mounting") FVector LastSnapView = FVector::ForwardVector;

    /** True when the luminaire currently stands on its auto pole. */
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "GLDF|Mounting")
    bool IsOnPole() const;

    /** Where a ceiling luminaire goes when it is dropped on a floor with no
     *  ceiling above (outdoors): hung at this height above the hit, emitting
     *  down, as if on a virtual ceiling. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting", meta = (ClampMin = "50.0", ClampMax = "2000.0"))
    float DefaultCeilingHeightCm = 280.0f;

    /** Pole diameter in cm for the auto pole. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting",
              meta = (ClampMin = "1.0", ClampMax = "60.0"))
    float PoleDiameterCm = 10.0f;

    /** Fallback body material for L3D parts that carry no material (otherwise
     *  UE shows its grid checker). Plugin content, soft-referenced. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF")
    TSoftObjectPtr<UMaterialInterface> DefaultBodyMaterial;

    /** Give body slots without a real material the DefaultBodyMaterial. */
    UFUNCTION(BlueprintCallable, Category = "GLDF")
    void EnsureBodyMaterials();

    /** Point each Emitters[i].Light at THIS actor's i-th emitter light. Editor
     *  duplicates / copy-paste keep references to the source actor's instanced
     *  components, which do not survive save+load (null in game). */
    UFUNCTION(BlueprintCallable, Category = "GLDF")
    void RebindEmitterComponents();

    /** Some L3Ds put a LEO on the far face and emit THROUGH the housing (PiL
     *  SPILLO: LEO on top, 17 cm body below, emission down) — the body would
     *  shadow its own light. Moves such lights just outside the body along
     *  their emission axis. Idempotent. */
    UFUNCTION(BlueprintCallable, Category = "GLDF")
    void AdjustLightsOutsideHousing();

    virtual void PostInitializeComponents() override;

    /** Actor-local direction that faces INTO the wall for a wall-mounted
     *  luminaire. L3D models luminaires in their mounted orientation with the
     *  wall behind them; after the importer's Y flip that is -Y in UE. Edit
     *  per luminaire if a manufacturer modelled it differently. */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "GLDF|Mounting")
    FVector WallBackDirLocal = FVector(0.0f, -1.0f, 0.0f);

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

    // ── Placement (surface snapping) ────────────────────────────────────
    // Editor "Place" and the runtime drag-drop share these. The hit SURFACE
    // (ceiling / wall / floor, from the hit normal) decides the frame; the
    // variant's GLDF mounting info decides the offsets (recess depth, pendant
    // length, wall mounting height, pole height). Convenience redirects: a
    // ceiling luminaire dropped on the FLOOR goes to the ceiling above the
    // click; a wall luminaire dropped on the floor looks for the wall ahead.

    /** The active variant's mounting info (variant 0 when none selected). */
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "GLDF|Mounting")
    FGldfMountingInfo GetActiveMounting() const;

    /** Trace `RayStart + RayDir * MaxDistance` through the world (ignoring
     *  this actor) and snap to what it hits. Returns false when nothing was
     *  hit — the caller decides where to park the luminaire then. */
    UFUNCTION(BlueprintCallable, Category = "GLDF|Mounting")
    bool PlaceFromRay(const FVector& RayStart, const FVector& RayDir, float MaxDistance = 100000.0f);

    /** Snap onto a surface hit: `HitLocation` + `HitNormal` from a trace,
     *  `ViewDirection` = where the user is looking (used to pick the wall for
     *  wall luminaires dropped on a floor). Returns true if the luminaire was
     *  moved. */
    UFUNCTION(BlueprintCallable, Category = "GLDF|Mounting")
    bool SnapToSurface(const FVector& HitLocation, const FVector& HitNormal, const FVector& ViewDirection);

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
