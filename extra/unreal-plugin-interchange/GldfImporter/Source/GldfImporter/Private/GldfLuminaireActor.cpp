#include "GldfLuminaireActor.h"

#include "Components/StaticMeshComponent.h"
#include "Components/LocalLightComponent.h"
#include "Engine/TextureLightProfile.h"

#include UE_INLINE_GENERATED_CPP_BY_NAME(GldfLuminaireActor)

AGldfLuminaireActor::AGldfLuminaireActor()
{
    PrimaryActorTick.bCanEverTick = false;

    // Body mesh is the root: dragging the actor drags everything.
    BodyMesh = CreateDefaultSubobject<UStaticMeshComponent>(TEXT("BodyMesh"));
    SetRootComponent(BodyMesh);

    // Emitter lights are created by the import (UInterchangeGldfActorFactory),
    // one per LEO, as instanced components — the LEO shape decides Rect vs
    // Point and the count varies per product, so we don't create them here.
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

void AGldfLuminaireActor::ApplySelectedVariant()
{
    const int32 Index = ResolveSelectedVariantIndex();
    if (!Variants.IsValidIndex(Index))
    {
        return;
    }
    const FGldfLuminaireVariant& V = Variants[Index];

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
        Light->SetIntensity(Photo->Lumens > 0 ? static_cast<float>(Photo->Lumens) : 1000.f);

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
