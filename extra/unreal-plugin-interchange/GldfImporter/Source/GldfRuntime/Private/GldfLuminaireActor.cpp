#include "GldfLuminaireActor.h"

#include "Components/StaticMeshComponent.h"
#include "Components/LocalLightComponent.h"
#include "Components/ActorComponent.h"
#include "Engine/CollisionProfile.h"
#include "Engine/HitResult.h"
#include "Engine/StaticMesh.h"
#include "Engine/TextureLightProfile.h"
#include "Materials/MaterialInterface.h"
#include "Engine/World.h"
#include "UObject/ConstructorHelpers.h"

#include UE_INLINE_GENERATED_CPP_BY_NAME(GldfLuminaireActor)

extern "C" {
#include "gldf_unreal.h"
}

AGldfLuminaireActor::AGldfLuminaireActor()
{
    PrimaryActorTick.bCanEverTick = false;

    // Body mesh is the root: dragging the actor drags everything.
    BodyMesh = CreateDefaultSubobject<UStaticMeshComponent>(TEXT("BodyMesh"));
    SetRootComponent(BodyMesh);

    // Emitter lights are created by the import (UInterchangeGldfActorFactory),
    // one per LEO, as instanced components — the LEO shape decides Rect vs
    // Point and the count varies per product, so we don't create them here.

    // Auto pole for pole-top luminaires: engine cylinder, hidden until
    // SnapToSurface() puts the luminaire on the ground. Hard-referenced from
    // the CDO so it is cooked into packaged builds.
    PoleMesh = CreateDefaultSubobject<UStaticMeshComponent>(TEXT("Pole"));
    PoleMesh->SetupAttachment(BodyMesh);
    PoleMesh->SetCollisionProfileName(UCollisionProfile::NoCollision_ProfileName);
    PoleMesh->SetVisibility(false);
    PoleMesh->SetHiddenInGame(true);
    static ConstructorHelpers::FObjectFinder<UStaticMesh> Cylinder(TEXT("/Engine/BasicShapes/Cylinder.Cylinder"));
    if (Cylinder.Succeeded())
    {
        PoleMesh->SetStaticMesh(Cylinder.Object);
    }
    DefaultBodyMaterial = TSoftObjectPtr<UMaterialInterface>(FSoftObjectPath(TEXT("/GldfImporter/M_GldfLuminaireBody.M_GldfLuminaireBody")));
}

FQuat AGldfLuminaireActor::L3dEulerToUeQuat(const FVector& Deg)
{
    // L3D: R = Rx · Ry · Rz (Rz applied first). UE's A*B applies B first.
    const FQuat Qx(FVector::XAxisVector, FMath::DegreesToRadians(Deg.X));
    const FQuat Qy(FVector::YAxisVector, FMath::DegreesToRadians(Deg.Y));
    const FQuat Qz(FVector::ZAxisVector, FMath::DegreesToRadians(Deg.Z));
    return L3dToUeQuat(Qx * Qy * Qz);
}

void AGldfLuminaireActor::SetJointAngles(int32 Index, FVector AnglesDeg)
{
    if (!Joints.IsValidIndex(Index)) { return; }
    FGldfJoint& J = Joints[Index];
    FVector A = FVector::ZeroVector;
    for (int32 k = 0; k < 3; ++k)
    {
        if (!(J.AxisMask & (1 << k))) { continue; }
        const float Lo = J.MinDeg[k], Hi = J.MaxDeg[k];
        A[k] = (Hi > Lo) ? FMath::Clamp(static_cast<float>(AnglesDeg[k]), Lo, Hi) : AnglesDeg[k];
    }
    J.AnglesDeg = A;
    if (J.Component)
    {
        // local = rest · delta (delta first when applied to a point).
        J.Component->SetRelativeLocationAndRotation(J.RestLocation, J.RestRotation * L3dEulerToUeQuat(A));
    }
}

void AGldfLuminaireActor::RebindPartComponents()
{
    for (FGldfJoint& J : Joints)
    {
        if (J.Component && J.Component->GetOwner() == this) { continue; }
        const FString Want = TEXT("Joint_") + J.Name;
        for (UActorComponent* C : GetComponents())
        {
            if (C->GetName().StartsWith(Want) && Cast<USceneComponent>(C) && !Cast<UStaticMeshComponent>(C) && !Cast<ULightComponent>(C))
            {
                J.Component = Cast<USceneComponent>(C);
                break;
            }
        }
    }
    for (TObjectPtr<UStaticMeshComponent>& P : PartMeshes)
    {
        if (P && P->GetOwner() == this) { continue; }
        const FString Want = P ? P->GetName() : FString();
        for (UActorComponent* C : GetComponents())
        {
            if (UStaticMeshComponent* M = Cast<UStaticMeshComponent>(C))
            {
                if (M != BodyMesh && M != PoleMesh && (Want.IsEmpty() || M->GetName() == Want) && !PartMeshes.Contains(M)) { P = M; break; }
            }
        }
    }
}

FBox AGldfLuminaireActor::ActorSpaceBodyBox() const
{
    FBox Box(ForceInit);
    const FTransform ActorInv = GetActorTransform().Inverse();
    auto Add = [&](const UStaticMeshComponent* C)
    {
        if (!C || !C->GetStaticMesh()) { return; }
        const FBox Local = C->GetStaticMesh()->GetBounds().GetBox();
        const FTransform ToActor = C->GetComponentTransform() * ActorInv;
        for (int32 i = 0; i < 8; ++i)
        {
            const FVector Corner((i & 1) ? Local.Max.X : Local.Min.X, (i & 2) ? Local.Max.Y : Local.Min.Y, (i & 4) ? Local.Max.Z : Local.Min.Z);
            Box += ToActor.TransformPosition(Corner);
        }
    };
    Add(BodyMesh);
    for (const TObjectPtr<UStaticMeshComponent>& P : PartMeshes) { Add(P); }
    if (!Box.IsValid) { Box = FBox(FVector::ZeroVector, FVector::ZeroVector); }
    return Box;
}

namespace
{
    // Angle (rad) between a light's beam and the direction to the target.
    float BeamError(const ULocalLightComponent* L, const FVector& Target)
    {
        const FVector Fwd = L->GetComponentQuat().GetAxisX();
        const FVector To = (Target - L->GetComponentLocation()).GetSafeNormal();
        return FMath::Acos(FMath::Clamp(FVector::DotProduct(Fwd, To), -1.f, 1.f));
    }
}

bool AGldfLuminaireActor::AimAt(const FVector& WorldTarget)
{
    TArray<ULocalLightComponent*> Lights;
    for (const FGldfEmitterComponent& E : Emitters) { if (E.Light) { Lights.Add(E.Light); } }
    if (Lights.Num() == 0) { return false; }

    if (Joints.Num() == 0)
    {
        // Baked single-mesh luminaire: aim the lights themselves (keep roll).
        for (ULocalLightComponent* L : Lights)
        {
            const FVector Cur = L->GetComponentQuat().GetAxisX();
            const FVector Want = (WorldTarget - L->GetComponentLocation()).GetSafeNormal();
            if (Want.IsNearlyZero()) { continue; }
            L->SetWorldRotation(FQuat::FindBetweenNormals(Cur, Want) * L->GetComponentQuat());
        }
        return true;
    }

    // Coordinate descent over the joints: each joint only looks at the lights
    // below it, each axis is searched within its limits (coarse scan, then
    // refinement). Three sweeps settle a pan/tilt chain.
    bool bAny = false;
    for (int32 Sweep = 0; Sweep < 3; ++Sweep)
    {
        for (int32 Ji = 0; Ji < Joints.Num(); ++Ji)
        {
            FGldfJoint& J = Joints[Ji];
            if (!J.Component || J.AxisMask == 0) { continue; }
            TArray<ULocalLightComponent*> Below;
            for (ULocalLightComponent* L : Lights) { if (L->IsAttachedTo(J.Component)) { Below.Add(L); } }
            if (Below.Num() == 0) { continue; }
            auto Err = [&]() { float E = 0.f; for (ULocalLightComponent* L : Below) { E += BeamError(L, WorldTarget); } return E / Below.Num(); };
            for (int32 k = 0; k < 3; ++k)
            {
                if (!(J.AxisMask & (1 << k))) { continue; }
                float Lo = J.MinDeg[k], Hi = J.MaxDeg[k];
                if (Hi <= Lo) { Lo = -180.f; Hi = 180.f; }
                FVector Angles = J.AnglesDeg;
                float BestA = Angles[k], BestE = FLT_MAX;
                const int32 N = 36;
                for (int32 i = 0; i <= N; ++i)
                {
                    Angles[k] = FMath::Lerp(Lo, Hi, i / float(N));
                    SetJointAngles(Ji, Angles);
                    const float E = Err();
                    if (E < BestE) { BestE = E; BestA = Angles[k]; }
                }
                float Span = (Hi - Lo) / N;
                for (int32 r = 0; r < 12; ++r)
                {
                    for (float Cand : { BestA - Span, BestA + Span })
                    {
                        Angles[k] = FMath::Clamp(Cand, Lo, Hi);
                        SetJointAngles(Ji, Angles);
                        const float E = Err();
                        if (E < BestE) { BestE = E; BestA = Angles[k]; }
                    }
                    Span *= 0.5f;
                }
                Angles[k] = BestA;
                SetJointAngles(Ji, Angles);
                bAny = true;
            }
        }
    }
    return bAny;
}

void AGldfLuminaireActor::EnsureBodyMaterials()
{
    UMaterialInterface* Fallback = DefaultBodyMaterial.LoadSynchronous();
    if (!Fallback) { return; }
    TArray<UStaticMeshComponent*> Comps = { BodyMesh.Get(), PoleMesh.Get() };
    for (const TObjectPtr<UStaticMeshComponent>& P : PartMeshes) { Comps.Add(P); }
    for (UStaticMeshComponent* Comp : Comps)
    {
        if (!Comp || !Comp->GetStaticMesh()) { continue; }
        for (int32 i = 0; i < Comp->GetNumMaterials(); ++i)
        {
            UMaterialInterface* M = Comp->GetMaterial(i);
            if (!M || M->GetName() == TEXT("WorldGridMaterial") || M->GetName() == TEXT("DefaultMaterial"))
            {
                Comp->SetMaterial(i, Fallback);
            }
        }
    }
}

TArray<FString> AGldfLuminaireActor::GetVariantNameOptions() const
{
    TArray<FString> Options;
    Options.Reserve(Variants.Num());
    for (const FGldfLuminaireVariant& V : Variants)
    {
        // Prefer the display name; fall back to the id so the dropdown is
        // never blank.
        Options.Add(V.VariantName.IsEmpty() ? V.VariantId : V.VariantName);
    }
    return Options;
}

int32 AGldfLuminaireActor::ResolveSelectedVariantIndex() const
{
    if (SelectedVariantName.IsNone())
    {
        return 0;
    }
    const FString Target = SelectedVariantName.ToString();
    const int32 Found = Variants.IndexOfByPredicate(
        [&Target](const FGldfLuminaireVariant& V)
        {
            const FString Label = V.VariantName.IsEmpty() ? V.VariantId : V.VariantName;
            return Label == Target;
        });
    return Found != INDEX_NONE ? Found : 0;
}

void AGldfLuminaireActor::RebindEmitterComponents()
{
    TArray<ULocalLightComponent*> Lights;
    for (UActorComponent* C : GetComponents())
    {
        if (ULocalLightComponent* L = Cast<ULocalLightComponent>(C)) { Lights.Add(L); }
    }
    // Factory names them Emitter_<index>_<partName>; sort to restore the order.
    Lights.Sort([](const ULocalLightComponent& A, const ULocalLightComponent& B) { return A.GetName() < B.GetName(); });
    int32 Rebound = 0;
    for (int32 i = 0; i < Emitters.Num(); ++i)
    {
        ULocalLightComponent* L = Emitters[i].Light;
        if (!L || L->GetOwner() != this)
        {
            if (Lights.IsValidIndex(i)) { Emitters[i].Light = Lights[i]; ++Rebound; }
        }
    }
    if (Rebound > 0)
    {
        UE_LOG(LogTemp, Log, TEXT("GldfLuminaire %s: rebound %d emitter light(s)"), *GetName(), Rebound);
    }
}

void AGldfLuminaireActor::AdjustLightsOutsideHousing()
{
    if (!BodyMesh || !BodyMesh->GetStaticMesh()) { return; }
    if (Joints.Num() > 0) { return; } // articulated parts: the L3D places the LEO on the head, leave it
    const FBox Box = ActorSpaceBodyBox();
    if (!Box.IsValid || Box.GetSize().IsNearlyZero()) { return; }
    // Runaway LEO: some publishers write the LEO position in mm where L3D
    // wants metres, which puts the emitter hundreds of metres from a 50 cm
    // body. Spec-correct files never trip this (the LEO sits on or in the
    // body); a clearly broken one is pulled back to the body with a warning
    // instead of lighting a spot half a kilometre away.
    const float Runaway = FMath::Max(200.f, 10.f * static_cast<float>(Box.GetSize().Size()));
    for (const FGldfEmitterComponent& E : Emitters)
    {
        ULocalLightComponent* L = E.Light;
        if (!L) { continue; }
        FVector Loc = L->GetRelativeLocation();
        const FVector Dir = L->GetRelativeRotation().RotateVector(FVector::ForwardVector); // emission = local +X
        const FVector Nearest = Box.GetClosestPointTo(Loc);
        if (FVector::Dist(Nearest, Loc) > Runaway)
        {
            UE_LOG(LogTemp, Warning, TEXT("GldfLuminaire %s: emitter '%s' at %s is %.0f m from its %.0f cm body (LEO position not in metres?) — moved onto the body"),
                *GetName(), *E.EmitterName, *Loc.ToCompactString(), FVector::Dist(Nearest, Loc) / 100.f, Box.GetSize().Size());
            Loc = Nearest;
            L->SetRelativeLocation(Loc);
        }
        float Ahead = 0.f;
        for (int32 c = 0; c < 8; ++c)
        {
            const FVector Corner((c & 1) ? Box.Max.X : Box.Min.X, (c & 2) ? Box.Max.Y : Box.Min.Y, (c & 4) ? Box.Max.Z : Box.Min.Z);
            Ahead = FMath::Max(Ahead, static_cast<float>(FVector::DotProduct(Corner - Loc, Dir)));
        }
        // Only a clear "housing in front" case (2–100 cm): a LEO on the exit face
        // reads ~0 and stays; a second run reads ≤ 0 (already outside) and stays.
        if (Ahead >= 2.f && Ahead <= 100.f)
        {
            L->SetRelativeLocation(Loc + Dir * (Ahead + 0.5f));
            UE_LOG(LogTemp, Log, TEXT("GldfLuminaire %s: emitter '%s' emitted through its housing — moved %.1f cm out"), *GetName(), *E.EmitterName, Ahead + 0.5f);
        }
    }
}

bool AGldfLuminaireActor::IsOnPole() const
{
    return PoleMesh && PoleMesh->IsVisible() && !PoleMesh->bHiddenInGame;
}

bool AGldfLuminaireActor::ReSnap()
{
    if (bHasLastSnap)
    {
        return SnapToSurface(LastSnapLocation, LastSnapNormal, LastSnapView);
    }
    UWorld* World = GetWorld();
    if (!World) { return false; }
    FCollisionQueryParams Q(TEXT("GldfReSnap"), true);
    Q.AddIgnoredActor(this);
    FHitResult Hit;
    const FGldfMountingInfo Mount = GetActiveMounting();
    if (Mount.Kind == EGldfMountingKind::Wall)
    {
        // The wall is behind the back face.
        const FVector Back = GetActorRotation().RotateVector(WallBackDirLocal).GetSafeNormal();
        if (World->LineTraceSingleByChannel(Hit, GetActorLocation(), GetActorLocation() + Back * 300.f, ECC_Visibility, Q))
        {
            return SnapToSurface(Hit.ImpactPoint, Hit.ImpactNormal, GetActorForwardVector());
        }
    }
    // Ground straight below. A pole-top head often sits inside a scene fixture
    // (lantern glass, rings, bracket arm): for those take the LOWEST surface
    // under the actor — the street — not the first thing the trace touches.
    const bool bPoleHead = Mount.Kind == EGldfMountingKind::Ground && (Mount.bPoleTop || Mount.PoleHeightMm > 0 || HeightOverrideCm > 0.f);
    FHitResult Ground;
    bool bFound = false;
    // Pole heads: five rays (centre + 60 cm to each side, since the post
    // itself may cover the ground mesh), each followed down through every
    // surface; the lowest hit of all is the street.
    const FVector Offsets[5] = { FVector::ZeroVector, FVector(60, 0, 0), FVector(-60, 0, 0), FVector(0, 60, 0), FVector(0, -60, 0) };
    for (int32 Ray = 0; Ray < (bPoleHead ? 5 : 1); ++Ray)
    {
        FVector From = GetActorLocation() + Offsets[Ray];
        for (int32 Step = 0; Step < 64; ++Step)
        {
            if (!World->LineTraceSingleByChannel(Hit, From, From - FVector(0, 0, 5000.f), ECC_Visibility, Q)) { break; }
            if (!bFound || Hit.ImpactPoint.Z < Ground.ImpactPoint.Z) { Ground = Hit; }
            bFound = true;
            if (!bPoleHead) { break; }
            UE_LOG(LogTemp, Verbose, TEXT("GldfLuminaire ReSnap %s: ray %d step %d hit %s at z=%.0f"), *GetName(), Ray, Step,
                *GetNameSafe(Hit.GetActor()), Hit.ImpactPoint.Z);
            From = Hit.ImpactPoint - FVector(0, 0, 5.f);
        }
    }
    if (bFound && bPoleHead)
    {
        // Put the pole under the head, on the ground level found.
        Ground.ImpactPoint.X = GetActorLocation().X;
        Ground.ImpactPoint.Y = GetActorLocation().Y;
        Ground.ImpactNormal = FVector::UpVector;
    }
    if (bFound)
    {
        return SnapToSurface(Ground.ImpactPoint, Ground.ImpactNormal, GetActorForwardVector());
    }
    return false;
}

void AGldfLuminaireActor::PostInitializeComponents()
{
    Super::PostInitializeComponents();
    RebindEmitterComponents();
    RebindPartComponents();
    AdjustLightsOutsideHousing();
}

void AGldfLuminaireActor::ApplySelectedVariant()
{
    RebindEmitterComponents();
    RebindPartComponents();
    AdjustLightsOutsideHousing();
    EnsureBodyMaterials();
    const int32 Index = ResolveSelectedVariantIndex();
    if (!Variants.IsValidIndex(Index))
    {
        return;
    }
    FGldfLuminaireVariant& V = Variants[Index];

    // No rated flux in the GLDF (demo / incomplete files): integrate the
    // emitter's own IES with the eulumdat core and use that, once.
    for (FGldfEmitterPhotometry& P : V.Emitters)
    {
        if (P.Lumens > 0 || P.IesBytes.Num() == 0) { continue; }
        const uint64 Ldc = gldf_unreal_ldc_open(P.IesBytes.GetData(), static_cast<uintptr_t>(P.IesBytes.Num()));
        if (Ldc != 0)
        {
            const double Flux = gldf_unreal_ldc_flux_lm(Ldc);
            gldf_unreal_ldc_close(Ldc);
            if (Flux > 0.0)
            {
                P.Lumens = FMath::RoundToInt32(static_cast<float>(Flux));
                UE_LOG(LogTemp, Log, TEXT("GldfLuminaire %s: emitter '%s' has no rated lumens — %d lm from its photometry"), *GetName(), *P.EmitterName, P.Lumens);
            }
        }
    }

    // For each emitter component, find the matching per-emitter photometry
    // in this variant (by LEO partName) and apply it.
    for (const FGldfEmitterComponent& Emitter : Emitters)
    {
        ULocalLightComponent* Light = Emitter.Light;
        if (!Light)
        {
            continue;
        }

        const FGldfEmitterPhotometry* Photo = V.Emitters.FindByPredicate(
            [&Emitter](const FGldfEmitterPhotometry& P)
            {
                return P.EmitterName == Emitter.EmitterName;
            });
        // If this variant has no per-emitter entry by name, fall back to the
        // first emitter so a single-emitter luminaire (the common case) still
        // gets lit even when the emitter-name pairing didn't line up.
        if (!Photo && V.Emitters.Num() > 0)
        {
            Photo = &V.Emitters[0];
        }
        if (!Photo)
        {
            UE_LOG(LogTemp, Warning,
                TEXT("GldfLuminaire: emitter '%s' has no photometry in variant '%s' — light stays default"),
                *Emitter.EmitterName, *V.VariantName);
            continue;
        }

        // IES only makes sense with a real profile; turning on IES-brightness
        // WITHOUT a texture can leave the light with no usable intensity.
        Light->SetIESTexture(Photo->IesProfile);
        Light->SetUseIESBrightness(Photo->IesProfile != nullptr);

        // Intensity: prefer the GLDF lumens; if absent, apply a sane default so
        // the luminaire ALWAYS shines (a dark placed light reads as "broken").
        Light->SetIntensityUnits(ELightUnits::Lumens);
        const float Lumens = Photo->Lumens > 0 ? static_cast<float>(Photo->Lumens) : 1000.f;
        Light->SetIntensity(Lumens);

        // Reach: UE's default attenuation radius is 10 m, which clips a street
        // light at 6 m or a 25 klm floodlight. Size it from the flux: the
        // distance where an isotropic source of this flux drops to ~0.3 lux
        // (I ≈ Φ/4π cd, r = sqrt(I/E)), clamped to 5–60 m.
        const float ReachCm = FMath::Clamp(FMath::Sqrt(Lumens / (4.f * PI * 0.3f)) * 100.f, 500.f, 6000.f);
        Light->SetAttenuationRadius(ReachCm);

        UE_LOG(LogTemp, Log,
            TEXT("GldfLuminaire: emitter '%s' ← variant '%s' (IES=%s, %d lm, %dK)"),
            *Emitter.EmitterName, *V.VariantName,
            Photo->IesProfile ? TEXT("yes") : TEXT("none"),
            Photo->Lumens, Photo->Cct);

        if (Photo->Cct > 0)
        {
            Light->SetUseTemperature(true);
            Light->SetTemperature(static_cast<float>(Photo->Cct));
        }

        Light->MarkRenderStateDirty();
    }
}

#if WITH_EDITOR
void AGldfLuminaireActor::PostEditChangeProperty(FPropertyChangedEvent& PropertyChangedEvent)
{
    const FName Changed = PropertyChangedEvent.GetPropertyName();
    if (Changed == GET_MEMBER_NAME_CHECKED(AGldfLuminaireActor, SelectedVariantName)
        || Changed == GET_MEMBER_NAME_CHECKED(AGldfLuminaireActor, Variants))
    {
        ApplySelectedVariant();
    }
    Super::PostEditChangeProperty(PropertyChangedEvent);
}
#endif

// ── Aim helpers ─────────────────────────────────────────────────────────

void AGldfLuminaireActor::ResetAim()
{
    Modify();
    SetActorRotation(FRotator::ZeroRotator);
}

void AGldfLuminaireActor::Tilt15()
{
    Modify();
    AddActorLocalRotation(FRotator(15.0f, 0.0f, 0.0f));
}

void AGldfLuminaireActor::TiltBack15()
{
    Modify();
    AddActorLocalRotation(FRotator(-15.0f, 0.0f, 0.0f));
}

void AGldfLuminaireActor::Spin45()
{
    // World yaw (not local), so the spin always turns the tilt direction
    // around the vertical axis even after prior tilts.
    Modify();
    AddActorWorldRotation(FRotator(0.0f, 45.0f, 0.0f));
}

// ── Placement (surface snapping) ────────────────────────────────────────

FGldfMountingInfo AGldfLuminaireActor::GetActiveMounting() const
{
    const int32 Index = ResolveSelectedVariantIndex();
    if (Variants.IsValidIndex(Index))
    {
        return Variants[Index].Mounting;
    }
    return FGldfMountingInfo();
}

namespace
{
    constexpr float MmToCm = 0.1f;
    // How far an emitter must stay below a ceiling / in front of a wall so a
    // recessed body never pushes its light plane into the slab (a light
    // behind the surface renders dark — reads as "broken").
    constexpr float EmitterClearanceCm = 0.5f;
    // Reach for the convenience redirects (floor click → ceiling above,
    // floor click → wall ahead).
    constexpr float RedirectReachCm = 2000.0f;

    // Wall mountingHeight sanity: real-world data has "0" for unknown and the
    // odd unit slip (SLV BIG THEO: 1400000 = 1.4 km). Only trust 0.2–20 m.
    bool UsableWallHeight(int32 Mm) { return Mm >= 200 && Mm <= 20000; }

    enum class ESurface : uint8 { Ceiling, Wall, Floor };

    ESurface ClassifySurface(const FVector& Normal)
    {
        if (Normal.Z < -0.7f) { return ESurface::Ceiling; }
        if (Normal.Z >  0.7f) { return ESurface::Floor; }
        return ESurface::Wall;
    }

    const TCHAR* SurfaceName(ESurface S)
    {
        switch (S)
        {
        case ESurface::Ceiling: return TEXT("ceiling");
        case ESurface::Wall:    return TEXT("wall");
        default:                return TEXT("floor");
        }
    }

    // Body bounds in actor space (BodyMesh is the root with identity relative
    // transform, so mesh-local == actor-local). A luminaire without geometry
    // gets a point box so every offset below degrades to "origin on surface".
    FBox LocalBodyBox(const UStaticMeshComponent* Body)
    {
        if (Body && Body->GetStaticMesh())
        {
            return Body->GetStaticMesh()->GetBounds().GetBox();
        }
        return FBox(FVector::ZeroVector, FVector::ZeroVector);
    }

    // Largest extent of the box along a (unit) local direction — distance
    // from the origin to the box face that direction points at.
    float ExtentAlong(const FBox& Box, const FVector& Dir)
    {
        float Best = 0.0f;
        for (int32 i = 0; i < 8; ++i)
        {
            const FVector Corner(
                (i & 1) ? Box.Max.X : Box.Min.X,
                (i & 2) ? Box.Max.Y : Box.Min.Y,
                (i & 4) ? Box.Max.Z : Box.Min.Z);
            Best = FMath::Max(Best, static_cast<float>(FVector::DotProduct(Corner, Dir)));
        }
        return Best;
    }
}

bool AGldfLuminaireActor::PlaceFromRay(const FVector& RayStart, const FVector& RayDir, float MaxDistance)
{
    UWorld* World = GetWorld();
    if (!World)
    {
        return false;
    }
    FCollisionQueryParams QueryParams(TEXT("GldfPlaceTrace"), /*bTraceComplex*/ true);
    QueryParams.AddIgnoredActor(this);
    FHitResult Hit;
    const FVector Dir = RayDir.GetSafeNormal();
    if (!World->LineTraceSingleByChannel(Hit, RayStart, RayStart + Dir * MaxDistance, ECC_Visibility, QueryParams))
    {
        return false;
    }
    return SnapToSurface(Hit.ImpactPoint, Hit.ImpactNormal, Dir);
}

bool AGldfLuminaireActor::SnapToSurface(const FVector& HitLocation, const FVector& HitNormal, const FVector& ViewDirection)
{
    UWorld* World = GetWorld();
    const FGldfMountingInfo Mount = GetActiveMounting();
    const FBox Box = PartMeshes.Num() > 0 ? ActorSpaceBodyBox() : LocalBodyBox(BodyMesh);
    const FVector Normal = HitNormal.GetSafeNormal(UE_SMALL_NUMBER, FVector::UpVector);
    const ESurface Surface = ClassifySurface(Normal);

    FCollisionQueryParams QueryParams(TEXT("GldfSnapTrace"), /*bTraceComplex*/ true);
    QueryParams.AddIgnoredActor(this);

    // ── Convenience redirects from a floor click ────────────────────────
    if (World && Surface == ESurface::Floor)
    {
        FHitResult Redirect;
        if (Mount.Kind == EGldfMountingKind::Ceiling)
        {
            // Downlight dropped on the floor → the ceiling straight above.
            if (World->LineTraceSingleByChannel(Redirect, HitLocation + FVector(0, 0, 1),
                    HitLocation + FVector(0, 0, RedirectReachCm), ECC_Visibility, QueryParams)
                && ClassifySurface(Redirect.ImpactNormal) == ESurface::Ceiling)
            {
                UE_LOG(LogTemp, Log, TEXT("GldfLuminaire: floor click → ceiling %.0f cm above"),
                    Redirect.ImpactPoint.Z - HitLocation.Z);
                return SnapToSurface(Redirect.ImpactPoint, Redirect.ImpactNormal, ViewDirection);
            }
        }
        else if (Mount.Kind == EGldfMountingKind::Wall)
        {
            // Wall luminaire dropped on the floor → the wall the user looks at.
            FVector Ahead = FVector(ViewDirection.X, ViewDirection.Y, 0.0f).GetSafeNormal();
            if (Ahead.IsNearlyZero()) { Ahead = GetActorForwardVector(); }
            const float ProbeZ = UsableWallHeight(Mount.MountingHeightMm) ? Mount.MountingHeightMm * MmToCm : 150.0f;
            const FVector Probe = HitLocation + FVector(0, 0, ProbeZ);
            if (World->LineTraceSingleByChannel(Redirect, Probe, Probe + Ahead * RedirectReachCm,
                    ECC_Visibility, QueryParams)
                && ClassifySurface(Redirect.ImpactNormal) == ESurface::Wall)
            {
                UE_LOG(LogTemp, Log, TEXT("GldfLuminaire: floor click → wall %.0f cm ahead"),
                    Redirect.Distance);
                return SnapToSurface(Redirect.ImpactPoint, Redirect.ImpactNormal, ViewDirection);
            }
        }
    }

    // Ceiling luminaire aimed at a WALL → the ceiling above the point in front
    // of it (a downlight does not hang on a wall when there is a ceiling).
    if (World && Surface == ESurface::Wall && Mount.Kind == EGldfMountingKind::Ceiling)
    {
        FHitResult Up;
        const FVector InFront = HitLocation + Normal * 40.0f;
        if (World->LineTraceSingleByChannel(Up, InFront, InFront + FVector(0, 0, RedirectReachCm), ECC_Visibility, QueryParams)
            && ClassifySurface(Up.ImpactNormal) == ESurface::Ceiling)
        {
            UE_LOG(LogTemp, Log, TEXT("GldfLuminaire: wall click → ceiling %.0f cm above"), Up.ImpactPoint.Z - InFront.Z);
            return SnapToSurface(Up.ImpactPoint, Up.ImpactNormal, ViewDirection);
        }
    }

    // Free-standing / pole luminaire aimed at a WALL → the floor in front of it
    // (a floor lamp does not hang on a wall).
    if (World && Surface == ESurface::Wall && Mount.Kind == EGldfMountingKind::Ground)
    {
        FHitResult Floor;
        const FVector InFront = HitLocation + Normal * 40.0f;
        if (World->LineTraceSingleByChannel(Floor, InFront, InFront - FVector(0, 0, RedirectReachCm * 2.0f), ECC_Visibility, QueryParams)
            && ClassifySurface(Floor.ImpactNormal) == ESurface::Floor)
        {
            UE_LOG(LogTemp, Log, TEXT("GldfLuminaire: wall click → floor %.0f cm below"), InFront.Z - Floor.ImpactPoint.Z);
            return SnapToSurface(Floor.ImpactPoint, Floor.ImpactNormal, ViewDirection);
        }
    }

    // ── Rotation ────────────────────────────────────────────────────────
    // Ceiling/floor: keep the as-designed orientation (emission down) plus
    // whatever yaw the user already gave it. Wall: yaw so WallBackDirLocal
    // points into the wall.
    FRotator NewRot(0.0f, GetActorRotation().Yaw, 0.0f);
    if (Surface == ESurface::Wall)
    {
        const FVector BackLocal = FVector(WallBackDirLocal.X, WallBackDirLocal.Y, 0.0f).GetSafeNormal(
            UE_SMALL_NUMBER, FVector(0, -1, 0));
        const FVector BackWorld = FVector(-Normal.X, -Normal.Y, 0.0f).GetSafeNormal();
        const float YawDeg = FMath::RadiansToDegrees(
            FMath::Atan2(BackWorld.Y, BackWorld.X) - FMath::Atan2(BackLocal.Y, BackLocal.X));
        NewRot = FRotator(0.0f, YawDeg, 0.0f);
    }
    const FQuat Q = NewRot.Quaternion();

    // ── Location ────────────────────────────────────────────────────────
    FVector Origin = HitLocation;
    bool bOnPole = false;
    float PoleCm = 0.0f;

    switch (Surface)
    {
    case ESurface::Ceiling:
    {
        // Body top flush with the ceiling; recess sinks it in, pendant hangs
        // it down. Centre the footprint on the click.
        float TopZ = Box.Max.Z;
        if (Mount.RecessedDepthMm > 0)       { TopZ -= Mount.RecessedDepthMm * MmToCm; }
        else if (Mount.PendantLengthMm > 0)  { TopZ += Mount.PendantLengthMm * MmToCm; }
        const FVector Centre = Q.RotateVector(FVector(Box.GetCenter().X, Box.GetCenter().Y, 0.0f));
        Origin = FVector(HitLocation.X - Centre.X, HitLocation.Y - Centre.Y, HitLocation.Z - TopZ);
        break;
    }
    case ESurface::Wall:
    {
        // Back face against the wall (recess sinks it in); mounting point at
        // the GLDF mounting height above the floor below the click.
        const FVector BackLocal = FVector(WallBackDirLocal.X, WallBackDirLocal.Y, 0.0f).GetSafeNormal(
            UE_SMALL_NUMBER, FVector(0, -1, 0));
        float BackExtent = ExtentAlong(Box, BackLocal);
        if (Mount.RecessedDepthMm > 0) { BackExtent -= Mount.RecessedDepthMm * MmToCm; }
        Origin = HitLocation + Normal * BackExtent;
        const float WallHeightCm = HeightOverrideCm > 0.f ? HeightOverrideCm
            : (UsableWallHeight(Mount.MountingHeightMm) ? Mount.MountingHeightMm * MmToCm : 0.f);
        if (World && WallHeightCm > 0.f)
        {
            FHitResult Floor;
            if (World->LineTraceSingleByChannel(Floor, HitLocation + Normal * 5.0f,
                    HitLocation + Normal * 5.0f - FVector(0, 0, RedirectReachCm * 2.0f),
                    ECC_Visibility, QueryParams))
            {
                Origin.Z = Floor.ImpactPoint.Z + WallHeightCm;
            }
        }
        break;
    }
    case ESurface::Floor:
    default:
    {
        const FVector Centre = Q.RotateVector(FVector(Box.GetCenter().X, Box.GetCenter().Y, 0.0f));
        Origin = FVector(HitLocation.X - Centre.X, HitLocation.Y - Centre.Y, HitLocation.Z);
        // <PoleTop/>: the GLDF geometry is the HEAD only → put it on a pole.
        // <PoleIntegrated/>: the geometry IS the pole/bollard → stand it on the
        // ground like a free-standing luminaire (unless an explicit poleHeight
        // says the model is a head after all).
        // A <PoleIntegrated/> model that is only a few decimetres tall is a HEAD
        // (the pole is "integrated" in the product system, not in the L3D) —
        // standing it on the ground puts the emitter at ankle height.
        const float BodyHeightCm = Box.Max.Z - Box.Min.Z;
        const bool bIntegratedHeadOnly = Mount.bPoleIntegrated && BodyHeightCm < 60.f;
        const bool bWantsPole = Mount.Kind == EGldfMountingKind::Ground
            && (Mount.PoleHeightMm > 0 || (Mount.bPoleTop && !Mount.bPoleIntegrated) || bIntegratedHeadOnly);
        if (bWantsPole)
        {
            // Street light: the head's origin is the pole-top attachment. GLDFs
            // mostly declare <PoleTop/> without a height → DefaultPoleHeightCm.
            PoleCm = HeightOverrideCm > 0.f ? HeightOverrideCm
                : (Mount.PoleHeightMm > 0 ? Mount.PoleHeightMm * MmToCm : DefaultPoleHeightCm);
            Origin.Z = HitLocation.Z + PoleCm;
            bOnPole = bAutoPole;
        }
        else if (Mount.Kind == EGldfMountingKind::Wall && (HeightOverrideCm > 0.f || UsableWallHeight(Mount.MountingHeightMm)))
        {
            // No wall found ahead: at least hang it at its mounting height.
            Origin.Z = HitLocation.Z + (HeightOverrideCm > 0.f ? HeightOverrideCm : Mount.MountingHeightMm * MmToCm);
        }
        else if (Mount.Kind == EGldfMountingKind::Ceiling && Surface == ESurface::Floor)
        {
            // No ceiling above (outdoors): a downlight lying on the pavement would
            // shine into the ground. Hang it on a virtual ceiling instead.
            const float CeilCm = HeightOverrideCm > 0.f ? HeightOverrideCm : DefaultCeilingHeightCm;
            float TopZ = Box.Max.Z;
            if (Mount.PendantLengthMm > 0) { TopZ += Mount.PendantLengthMm * MmToCm; }
            Origin.Z = HitLocation.Z + CeilCm - TopZ;
            UE_LOG(LogTemp, Log, TEXT("GldfLuminaire: no ceiling above the floor click — hung %.0f cm up"), CeilCm);
        }
        else
        {
            // Stand on the surface (free-standing, bollard, desk luminaire).
            Origin.Z = HitLocation.Z - Box.Min.Z;
        }
        break;
    }
    }

    // ── Emitter clearance ───────────────────────────────────────────────
    // A recessed body is allowed to sink into the slab, its emitters are not:
    // keep every light at least EmitterClearanceCm on the room side.
    if (Surface == ESurface::Ceiling || Surface == ESurface::Wall)
    {
        float WorstPenetration = 0.0f;
        for (const FGldfEmitterComponent& E : Emitters)
        {
            if (!E.Light) { continue; }
            const FVector EmitterWorld = Origin + Q.RotateVector(E.Light->GetRelativeLocation());
            // Signed distance in front of the surface (positive = room side).
            const float InFront = FVector::DotProduct(EmitterWorld - HitLocation, Normal);
            WorstPenetration = FMath::Max(WorstPenetration, EmitterClearanceCm - InFront);
        }
        if (WorstPenetration > 0.0f)
        {
            Origin += Normal * WorstPenetration;
            UE_LOG(LogTemp, Log, TEXT("GldfLuminaire: lifted %.1f cm off the %s so emitters stay in the room"),
                WorstPenetration, SurfaceName(Surface));
        }
    }

    Modify();
    SetActorLocationAndRotation(Origin, Q, false, nullptr, ETeleportType::TeleportPhysics);
    bHasLastSnap = true;
    LastSnapLocation = HitLocation;
    LastSnapNormal = Normal;
    LastSnapView = ViewDirection;

    // ── Pole ────────────────────────────────────────────────────────────
    if (PoleMesh)
    {
        if (bOnPole && PoleMesh->GetStaticMesh())
        {
            // Engine cylinder is 100 cm tall, 100 cm wide, centred on its origin.
            const float Scale = PoleDiameterCm / 100.0f;
            PoleMesh->SetRelativeScale3D(FVector(Scale, Scale, PoleCm / 100.0f));
            PoleMesh->SetRelativeLocation(FVector(0.0f, 0.0f, -PoleCm * 0.5f));
            PoleMesh->SetRelativeRotation(FRotator::ZeroRotator);
            PoleMesh->SetVisibility(true);
            PoleMesh->SetHiddenInGame(false);
        }
        else
        {
            PoleMesh->SetVisibility(false);
            PoleMesh->SetHiddenInGame(true);
        }
    }

    UE_LOG(LogTemp, Log,
        TEXT("GldfLuminaire: snapped to %s at %s (mount=%d recess=%d pendant=%d height=%d pole=%d)"),
        SurfaceName(Surface), *Origin.ToCompactString(), static_cast<int32>(Mount.Kind),
        Mount.RecessedDepthMm, Mount.PendantLengthMm, Mount.MountingHeightMm, Mount.PoleHeightMm);
    return true;
}
