#include "GldfIlluminanceGrid.h"
#include "GldfLuminaireActor.h"
#include "ProceduralMeshComponent.h"
#include "Components/LocalLightComponent.h"
#include "Engine/World.h"
#include "EngineUtils.h"
#include "Materials/MaterialInterface.h"

extern "C" {
#include "gldf_unreal.h"
}

DEFINE_LOG_CATEGORY_STATIC(LogGldfIlluminance, Log, All);

AGldfIlluminanceGrid::AGldfIlluminanceGrid()
{
    PrimaryActorTick.bCanEverTick = false;
    Mesh = CreateDefaultSubobject<UProceduralMeshComponent>(TEXT("Heatmap"));
    SetRootComponent(Mesh);
    Mesh->SetCollisionEnabled(ECollisionEnabled::NoCollision);
    Mesh->SetCastShadow(false);
    Mesh->bUseAsyncCooking = false;

    // Vertex-colour heatmap material, shipped with the plugin content.
    HeatmapMaterial = TSoftObjectPtr<UMaterialInterface>(FSoftObjectPath(TEXT("/GldfImporter/M_GldfHeatmap.M_GldfHeatmap")));
}

FLinearColor AGldfIlluminanceGrid::LuxToColor(float LuxValue, float ScaleMax)
{
    // Isolux ramp: 0 → deep blue → cyan → green → yellow → red at ScaleMax.
    const float T = ScaleMax > 0.f ? FMath::Clamp(LuxValue / ScaleMax, 0.f, 1.f) : 0.f;
    static const FLinearColor Stops[] = {
        FLinearColor(0.02f, 0.02f, 0.25f), FLinearColor(0.0f, 0.35f, 0.9f), FLinearColor(0.0f, 0.85f, 0.85f),
        FLinearColor(0.1f, 0.8f, 0.1f), FLinearColor(0.95f, 0.9f, 0.05f), FLinearColor(0.95f, 0.1f, 0.05f) };
    constexpr int32 N = UE_ARRAY_COUNT(Stops);
    const float F = T * (N - 1);
    const int32 I = FMath::Clamp(FMath::FloorToInt(F), 0, N - 2);
    return FMath::Lerp(Stops[I], Stops[I + 1], F - I);
}

float AGldfIlluminanceGrid::LuxAt(const FVector& WorldPos) const
{
    if (Nx <= 0 || Ny <= 0 || Lux.Num() != Nx * Ny) { return -1.f; }
    const FVector Rel = WorldPos - GridOrigin;
    const float U = FVector::DotProduct(Rel, GridU.GetSafeNormal()) / GridU.Size();
    const float V = FVector::DotProduct(Rel, GridV.GetSafeNormal()) / GridV.Size();
    const int32 IX = FMath::FloorToInt(U), IY = FMath::FloorToInt(V);
    if (IX < 0 || IY < 0 || IX >= Nx || IY >= Ny) { return -1.f; }
    return Lux[IY * Nx + IX];
}

void AGldfIlluminanceGrid::Recompute()
{
    UWorld* World = GetWorld();
    if (!World) { return; }
    constexpr float CmToM = 0.01f;

    // ── Grid geometry (world, cm) ───────────────────────────────────────
    Nx = Ny = FMath::Max(1, FMath::RoundToInt(SizeCm / CellCm));
    const FVector Centre = GetActorLocation();
    GridOrigin = Centre - FVector(SizeCm * 0.5f, SizeCm * 0.5f, 0.f);
    GridU = FVector(CellCm, 0.f, 0.f);
    GridV = FVector(0.f, CellCm, 0.f);
    GridNormal = FVector::UpVector;

    FCollisionQueryParams Q(TEXT("GldfIlluminance"), true);
    Q.AddIgnoredActor(this);
    for (TActorIterator<AGldfLuminaireActor> It(World); It; ++It)
    {
        Q.AddIgnoredActor(*It); // luminaire bodies don't shade the receiver plane for the drape trace
    }

    CellCentres.SetNum(Nx * Ny);
    CellHasSurface.Init(true, Nx * Ny);
    TArray<double> Offsets; Offsets.SetNum(Nx * Ny);
    for (int32 IY = 0; IY < Ny; ++IY)
    {
        for (int32 IX = 0; IX < Nx; ++IX)
        {
            FVector P = GridOrigin + GridU * (IX + 0.5f) + GridV * (IY + 0.5f);
            if (bDrapeOnGround)
            {
                FHitResult Hit;
                // Start just above head height so the grid stays on the street
                // under awnings / balconies instead of draping onto their tops,
                // and look past furniture tops (anything higher than
                // DrapeMaxRiseCm above the origin) down to the floor.
                FVector From = P + FVector(0, 0, DrapeStartHeightCm);
                bool bFound = false;
                for (int32 Step = 0; Step < 6; ++Step)
                {
                    if (!World->LineTraceSingleByChannel(Hit, From, P - FVector(0, 0, 2000.f), ECC_Visibility, Q)) { break; }
                    if (Hit.ImpactPoint.Z <= GridOrigin.Z + DrapeMaxRiseCm)
                    {
                        P.Z = Hit.ImpactPoint.Z;
                        bFound = true;
                        break;
                    }
                    From = Hit.ImpactPoint - FVector(0, 0, 2.f);
                }
                if (!bFound)
                {
                    CellHasSurface[IY * Nx + IX] = false; // outside the room / off the map / only furniture
                }
            }
            P.Z += ReceiverHeightCm;
            CellCentres[IY * Nx + IX] = P;
            Offsets[IY * Nx + IX] = (P.Z - GridOrigin.Z) * CmToM;
        }
    }

    {
        float MinZ = FLT_MAX, MaxZ = -FLT_MAX; int32 WithSurface = 0;
        TMap<FString, int32> HitActors;
        for (int32 Ci = 0; Ci < Nx * Ny; ++Ci)
        {
            if (!CellHasSurface[Ci]) { continue; }
            ++WithSurface; MinZ = FMath::Min(MinZ, (float)CellCentres[Ci].Z); MaxZ = FMath::Max(MaxZ, (float)CellCentres[Ci].Z);
        }
        UE_LOG(LogGldfIlluminance, Log, TEXT("drape: origin z %.0f, %d/%d cells with a surface, receiver z %.0f..%.0f"), GridOrigin.Z, WithSurface, Nx * Ny, MinZ, MaxZ);
    }

    // ── Emitters: open photometry handles from the stored IES bytes ─────
    struct FEmitterRef { uint64 Ldc; FVector Pos; ULocalLightComponent* Light; };
    TArray<FEmitterRef> Refs;
    TArray<GldfEmitterPose> Poses;
    int32 SkipNoVariant = 0, SkipNoLight = 0, SkipHidden = 0, SkipNoPhoto = 0, SkipNoBytes = 0, SkipParse = 0, Actors = 0;
    for (TActorIterator<AGldfLuminaireActor> It(World); It; ++It)
    {
        AGldfLuminaireActor* Lum = *It;
        ++Actors;
        const int32 VI = Lum->ResolveSelectedVariantIndex();
        if (!Lum->Variants.IsValidIndex(VI)) { ++SkipNoVariant; continue; }
        const FGldfLuminaireVariant& V = Lum->Variants[VI];
        if (Lum->Emitters.Num() == 0) { ++SkipNoLight; }
        for (const FGldfEmitterComponent& E : Lum->Emitters)
        {
            if (!E.Light) { ++SkipNoLight; continue; }
            if (!E.Light->IsVisible()) { ++SkipHidden; continue; }
            const FGldfEmitterPhotometry* Photo = V.Emitters.FindByPredicate(
                [&E](const FGldfEmitterPhotometry& P) { return P.EmitterName == E.EmitterName; });
            if (!Photo && V.Emitters.Num() > 0) { Photo = &V.Emitters[0]; }
            if (!Photo) { ++SkipNoPhoto; continue; }
            if (Photo->IesBytes.Num() == 0) { ++SkipNoBytes; continue; }
            const uint64 Ldc = gldf_unreal_ldc_open(Photo->IesBytes.GetData(), Photo->IesBytes.Num());
            if (Ldc == 0)
            {
                ++SkipParse;
                UE_LOG(LogGldfIlluminance, Warning, TEXT("%s/%s: photometry did not parse (%d bytes)"), *Lum->GetName(), *E.EmitterName, Photo->IesBytes.Num());
                continue;
            }
            // UE light frame: +X = emission (photometric nadir), +Y = LEO width (C0), +Z = LEO height (C90).
            const FTransform T = E.Light->GetComponentTransform();
            const FVector Pos = T.GetLocation();
            const FVector Nadir = T.GetUnitAxis(EAxis::X), C0 = T.GetUnitAxis(EAxis::Y), C90 = T.GetUnitAxis(EAxis::Z);
            GldfEmitterPose Pose{};
            Pose.ldc = Ldc;
            Pose.pos_x = Pos.X * CmToM; Pose.pos_y = Pos.Y * CmToM; Pose.pos_z = Pos.Z * CmToM;
            Pose.nadir_x = Nadir.X; Pose.nadir_y = Nadir.Y; Pose.nadir_z = Nadir.Z;
            Pose.c0_x = C0.X; Pose.c0_y = C0.Y; Pose.c0_z = C0.Z;
            Pose.c90_x = C90.X; Pose.c90_y = C90.Y; Pose.c90_z = C90.Z;
            Pose.factor = 1.0;
            Poses.Add(Pose);
            Refs.Add({ Ldc, Pos, E.Light });
            UE_LOG(LogGldfIlluminance, Log, TEXT("  emitter %s/%s owner=%s at %s flux %.0f lm"), *Lum->GetActorNameOrLabel(), *E.EmitterName,
                *GetNameSafe(E.Light->GetOwner()), *Pos.ToCompactString(), gldf_unreal_ldc_flux_lm(Ldc));
        }
    }

    UE_LOG(LogGldfIlluminance, Log, TEXT("emitters: %d actor(s) → %d used; skipped: no variant %d, no light %d, hidden %d, no photometry %d, no IES bytes %d, parse %d"),
        Actors, Poses.Num(), SkipNoVariant, SkipNoLight, SkipHidden, SkipNoPhoto, SkipNoBytes, SkipParse);
    Lux.Init(0.f, Nx * Ny);
    Stats = FGldfIlluminanceStats();
    Stats.Cells = Nx * Ny;
    Stats.Emitters = Poses.Num();

    if (Poses.Num() > 0)
    {
        GldfGridSpec Spec{};
        Spec.origin_x = GridOrigin.X * CmToM; Spec.origin_y = GridOrigin.Y * CmToM; Spec.origin_z = GridOrigin.Z * CmToM;
        Spec.u_x = GridU.X * CmToM; Spec.u_y = GridU.Y * CmToM; Spec.u_z = 0.0;
        Spec.v_x = GridV.X * CmToM; Spec.v_y = GridV.Y * CmToM; Spec.v_z = 0.0;
        Spec.normal_x = 0.0; Spec.normal_y = 0.0; Spec.normal_z = 1.0;
        Spec.nx = Nx; Spec.ny = Ny;

        TArray<double> Layers; Layers.SetNumZeroed(Poses.Num() * Nx * Ny);
        const int32 Rc = gldf_unreal_illuminance_grid(Poses.GetData(), Poses.Num(), &Spec, Offsets.GetData(), Layers.GetData());
        if (Rc != 0)
        {
            UE_LOG(LogGldfIlluminance, Warning, TEXT("illuminance_grid failed rc=%d"), Rc);
        }

        // Occlusion + sum.
        FCollisionQueryParams VisQ(TEXT("GldfIlluminanceVis"), true);
        VisQ.AddIgnoredActor(this);
        for (int32 Ei = 0; Ei < Poses.Num(); ++Ei)
        {
            const double* Layer = Layers.GetData() + Ei * Nx * Ny;
            double LayerMax = 0.0, LayerUsed = 0.0; int32 Occluded = 0;
            for (int32 Ci = 0; Ci < Nx * Ny; ++Ci) { LayerMax = FMath::Max(LayerMax, Layer[Ci]); }
            for (int32 Ci = 0; Ci < Nx * Ny; ++Ci)
            {
                const double E = Layer[Ci];
                if (E <= 0.0) { continue; }
                if (bOcclusion)
                {
                    FHitResult Hit;
                    const FVector From = CellCentres[Ci] + FVector(0, 0, 1.f);
                    // Stop short of the emitter so its own housing / the scene
                    // fixture it sits in does not block it.
                    const FVector ToEmitter = Refs[Ei].Pos - From;
                    const float Dist = ToEmitter.Size();
                    if (Dist <= EmitterClearanceCm) { Lux[Ci] += static_cast<float>(E); LayerUsed = FMath::Max(LayerUsed, E); continue; }
                    const FVector To = From + ToEmitter.GetSafeNormal() * (Dist - EmitterClearanceCm);
                    if (World->LineTraceSingleByChannel(Hit, From, To, ECC_Visibility, VisQ)
                        && !Hit.GetActor()->IsA<AGldfLuminaireActor>())
                    {
                        ++Occluded;
                        continue;
                    }
                }
                Lux[Ci] += static_cast<float>(E);
                LayerUsed = FMath::Max(LayerUsed, E);
            }
            UE_LOG(LogGldfIlluminance, Log, TEXT("  layer %d %s: direct max %.2f lx, after occlusion max %.2f lx, %d cell(s) shadowed"),
                Ei, *GetNameSafe(Refs[Ei].Light ? Refs[Ei].Light->GetOwner() : nullptr), LayerMax, LayerUsed, Occluded);
        }
        for (const FEmitterRef& R : Refs) { gldf_unreal_ldc_close(R.Ldc); }

        float Sum = 0.f; int32 Counted = 0; Stats.MinLux = FLT_MAX; Stats.MaxLux = 0.f;
        for (int32 Ci = 0; Ci < Lux.Num(); ++Ci)
        {
            if (!CellHasSurface[Ci]) { continue; }
            const float L = Lux[Ci];
            Sum += L; ++Counted; Stats.MinLux = FMath::Min(Stats.MinLux, L); Stats.MaxLux = FMath::Max(Stats.MaxLux, L);
        }
        Stats.Cells = Counted;
        Stats.AvgLux = Counted > 0 ? Sum / Counted : 0.f;
        if (Counted == 0) { Stats.MinLux = 0.f; }
        Stats.UniformityMinAvg = Stats.AvgLux > 0.f ? Stats.MinLux / Stats.AvgLux : 0.f;
        Stats.UniformityMinMax = Stats.MaxLux > 0.f ? Stats.MinLux / Stats.MaxLux : 0.f;
    }
    else
    {
        Stats.MinLux = 0.f;
    }

    // Colour scale: auto = the 95th percentile (one hot spot right under a
    // lantern must not squash the whole street into the bottom colour),
    // rounded up to a 1/1.5/2/…/10 step.
    float ScaleMax = ScaleMaxLux;
    if (ScaleMax <= 0.f)
    {
        // ...over the LIT cells only: a grid that is mostly outside a small
        // room (or beyond the lights' reach) must not collapse the scale to 1 lx.
        TArray<float> Sorted;
        Sorted.Reserve(Lux.Num());
        for (int32 Ci = 0; Ci < Lux.Num(); ++Ci) { if (CellHasSurface[Ci] && Lux[Ci] > 0.05f) { Sorted.Add(Lux[Ci]); } }
        Sorted.Sort();
        const float P95 = Sorted.Num() > 0 ? Sorted[FMath::Clamp(FMath::FloorToInt(Sorted.Num() * 0.95f), 0, Sorted.Num() - 1)] : 0.f;
        const float M = FMath::Max(P95, 1.f);
        const float P = FMath::Pow(10.f, FMath::FloorToFloat(FMath::LogX(10.f, M)));
        const float N = M / P;
        static const float Steps[] = { 1.f, 1.5f, 2.f, 2.5f, 3.f, 4.f, 5.f, 6.f, 8.f, 10.f };
        float Step = 10.f;
        for (float S : Steps) { if (N <= S) { Step = S; break; } }
        ScaleMax = Step * P;
    }
    Stats.ScaleMaxLux = ScaleMax;

    // ── Mesh: one quad per cell (4 verts, flat colour) ──────────────────
    TArray<FVector> Verts; TArray<int32> Tris; TArray<FVector> Normals; TArray<FVector2D> UVs; TArray<FLinearColor> Colors; TArray<FProcMeshTangent> Tangents;
    Verts.Reserve(Nx * Ny * 4); Tris.Reserve(Nx * Ny * 6); Colors.Reserve(Nx * Ny * 4);
    const FTransform Inv = GetActorTransform().Inverse();
    const float Lift = 3.f;
    for (int32 IY = 0; IY < Ny; ++IY)
    {
        for (int32 IX = 0; IX < Nx; ++IX)
        {
            const int32 Ci = IY * Nx + IX;
            if (!CellHasSurface[Ci]) { continue; }
            const FVector C = CellCentres[Ci];
            const float H = CellCm * 0.5f;
            FLinearColor Col = LuxToColor(Lux[Ci], ScaleMax); Col.A = Opacity;
            const int32 Base = Verts.Num();
            const FVector Corners[4] = { C + FVector(-H, -H, Lift), C + FVector(H, -H, Lift), C + FVector(H, H, Lift), C + FVector(-H, H, Lift) };
            for (const FVector& W : Corners)
            {
                Verts.Add(Inv.TransformPosition(W)); Normals.Add(FVector::UpVector); UVs.Add(FVector2D::ZeroVector);
                Colors.Add(Col); Tangents.Add(FProcMeshTangent(1, 0, 0));
            }
            Tris.Append({ Base, Base + 2, Base + 1, Base, Base + 3, Base + 2 });
        }
    }
    Mesh->ClearAllMeshSections();
    UMaterialInterface* HeatMat = HeatmapMaterial.LoadSynchronous();
    if (!HeatMat) { UE_LOG(LogGldfIlluminance, Warning, TEXT("heatmap material %s not found"), *HeatmapMaterial.ToString()); }
    if (bShowSheet)
    {
        Mesh->CreateMeshSection_LinearColor(0, Verts, Tris, Normals, UVs, Colors, Tangents, false);
        if (HeatMat) { Mesh->SetMaterial(0, HeatMat); }
    }

    // ── Measurement poles: post from the ground to the pad at receiver height ──
    Markers.Reset();
    if (bShowMarkers)
    {
        TArray<FVector> MV; TArray<int32> MT; TArray<FVector> MN; TArray<FVector2D> MUV; TArray<FLinearColor> MC; TArray<FProcMeshTangent> MTan;
        auto AddBox = [&](const FVector& Centre, const FVector& HalfExt, const FLinearColor& Col)
        {
            const int32 B = MV.Num();
            for (int32 c = 0; c < 8; ++c)
            {
                const FVector W = Centre + FVector((c & 1) ? HalfExt.X : -HalfExt.X, (c & 2) ? HalfExt.Y : -HalfExt.Y, (c & 4) ? HalfExt.Z : -HalfExt.Z);
                MV.Add(Inv.TransformPosition(W)); MN.Add(FVector::UpVector); MUV.Add(FVector2D::ZeroVector); MC.Add(Col); MTan.Add(FProcMeshTangent(1, 0, 0));
            }
            // 6 faces, outward winding (UE: clockwise seen from outside).
            static const int32 F[6][4] = { {0,2,3,1}, {4,5,7,6}, {0,1,5,4}, {2,6,7,3}, {0,4,6,2}, {1,3,7,5} };
            for (const int32* Q4 : F)
            {
                MT.Append({ B + Q4[0], B + Q4[1], B + Q4[2], B + Q4[0], B + Q4[2], B + Q4[3] });
            }
        };
        const int32 Step = FMath::Max(1, FMath::RoundToInt(MarkerSpacingCm / CellCm));
        const int32 Start = (Nx % Step) / 2 + Step / 2; // centre the pattern
        const float PadH = MarkerPadCm * 0.5f, PostH = 1.5f;
        const FLinearColor PostCol(0.12f, 0.12f, 0.12f, 1.f);
        for (int32 IY = Start; IY < Ny; IY += Step)
        {
            for (int32 IX = Start; IX < Nx; IX += Step)
            {
                const int32 Ci = IY * Nx + IX;
                if (!CellHasSurface[Ci]) { continue; }
                const FVector Pad = CellCentres[Ci] + FVector(0, 0, Lift + 1.f);
                const float GroundZ = CellCentres[Ci].Z - ReceiverHeightCm;
                // Post: always at least a stub so the point reads as a marker on the ground too.
                const float PostTop = Pad.Z - 1.f, PostBottom = FMath::Min(GroundZ, PostTop - 12.f);
                AddBox(FVector(Pad.X, Pad.Y, (PostTop + PostBottom) * 0.5f), FVector(PostH, PostH, (PostTop - PostBottom) * 0.5f), PostCol);
                FLinearColor Col = LuxToColor(Lux[Ci], ScaleMax); Col.A = 1.f;
                AddBox(Pad, FVector(PadH, PadH, 1.5f), Col);
                FGldfLuxMarker M; M.WorldPos = Pad; M.Lux = Lux[Ci];
                Markers.Add(M);
            }
        }
        Mesh->CreateMeshSection_LinearColor(1, MV, MT, MN, MUV, MC, MTan, false);
        if (HeatMat) { Mesh->SetMaterial(1, HeatMat); }
    }

    UE_LOG(LogGldfIlluminance, Log, TEXT("Illuminance grid %dx%d @%.0f cm: %d emitter(s), min %.1f avg %.1f max %.1f lx, U0 %.2f, scale %.0f lx"),
        Nx, Ny, CellCm, Stats.Emitters, Stats.MinLux, Stats.AvgLux, Stats.MaxLux, Stats.UniformityMinAvg, ScaleMax);
}
