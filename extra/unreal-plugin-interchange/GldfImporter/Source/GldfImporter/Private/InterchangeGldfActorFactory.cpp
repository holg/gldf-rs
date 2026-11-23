#include "InterchangeGldfActorFactory.h"

#include "GldfLuminaireActor.h"
#include "Engine/StaticMesh.h"

#include "InterchangeActorFactoryNode.h"
#include "InterchangeTextureLightProfileFactoryNode.h"
#include "Nodes/InterchangeBaseNodeContainer.h"
#include "Nodes/InterchangeFactoryBaseNode.h"
#include "Scene/InterchangeActorHelper.h"

#include "Components/StaticMeshComponent.h"
#include "Components/LocalLightComponent.h"
#include "Components/RectLightComponent.h"
#include "Components/PointLightComponent.h"
#include "Engine/StaticMesh.h"
#include "Engine/TextureLightProfile.h"
#include "GameFramework/Actor.h"
#include "InterchangeSourceData.h"
#include "Logging/LogMacros.h"
#include "Misc/Paths.h"

#include UE_INLINE_GENERATED_CPP_BY_NAME(InterchangeGldfActorFactory)

extern "C" {
#include "gldf_unreal.h"
}

DEFINE_LOG_CATEGORY_STATIC(LogGldfActorFactory, Log, All);

UClass* UInterchangeGldfActorFactory::GetFactoryClass() const
{
    return AGldfLuminaireActor::StaticClass();
}

UObject* UInterchangeGldfActorFactory::ProcessActor(
    AActor& SpawnedActor,
    const UInterchangeActorFactoryNode& FactoryNode,
    const UInterchangeBaseNodeContainer& NodeContainer,
    const FImportSceneObjectsParams& Params)
{
    AGldfLuminaireActor* Luminaire = Cast<AGldfLuminaireActor>(&SpawnedActor);
    if (!Luminaire)
    {
        // Shouldn't happen — GetFactoryClass() guarantees the type — but stay
        // defensive: fall back to the base behaviour rather than crash.
        UE_LOG(LogGldfActorFactory, Warning,
            TEXT("ProcessActor: spawned actor is not AGldfLuminaireActor (%s)"),
            *SpawnedActor.GetClass()->GetName());
        return Super::ProcessActor(SpawnedActor, FactoryNode, NodeContainer, Params);
    }

    ApplyBodyMesh(*Luminaire, FactoryNode, NodeContainer);
    ApplyLightAndVariants(*Luminaire, FactoryNode, NodeContainer, Params);

    // The base ImportSceneObject_GameThread applies node custom attributes
    // (local transform, mobility) to whatever we return. Return the root
    // (BodyMesh) so the actor's placement transform lands on the whole
    // luminaire — same contract the stock mesh/light factories follow.
    return Luminaire->GetRootComponent();
}

void UInterchangeGldfActorFactory::ApplyBodyMesh(
    AGldfLuminaireActor& Luminaire,
    const UInterchangeActorFactoryNode& FactoryNode,
    const UInterchangeBaseNodeContainer& NodeContainer) const
{
    if (!Luminaire.BodyMesh)
    {
        return;
    }

    // Standard chain (ActorHelper::FindAssetInstanceFactoryNode): actor factory
    // node → its target scene node → GetCustomAssetInstanceUid → mesh asset node
    // → its factory node → GetCustomReferenceObject (the built UStaticMesh).
    const UInterchangeFactoryBaseNode* MeshFactoryNode =
        UE::Interchange::ActorHelper::FindAssetInstanceFactoryNode(&NodeContainer, &FactoryNode);
    if (!MeshFactoryNode)
    {
        UE_LOG(LogGldfActorFactory, Warning,
            TEXT("ApplyBodyMesh: no mesh factory node for %s"), *FactoryNode.GetUniqueID());
        return;
    }

    FSoftObjectPath ReferenceObject;
    MeshFactoryNode->GetCustomReferenceObject(ReferenceObject);
    UStaticMesh* StaticMesh = Cast<UStaticMesh>(ReferenceObject.TryLoad());
    if (!StaticMesh)
    {
        UE_LOG(LogGldfActorFactory, Warning,
            TEXT("ApplyBodyMesh: mesh factory node has no UStaticMesh reference"));
        return;
    }

    Luminaire.BodyMesh->UnregisterComponent();
    Luminaire.BodyMesh->SetStaticMesh(StaticMesh);
    StaticMesh->CalculateExtendedBounds(); // avoid empty-bounds nav warning (UE-306557)
    Luminaire.BodyMesh->RegisterComponent();

    UE_LOG(LogGldfActorFactory, Log,
        TEXT("ApplyBodyMesh: assigned %s to BodyMesh"), *StaticMesh->GetName());
}

// Resolve an emitter's IES UTextureLightProfile from its photometry id. The
// translator named each IES texture node "\LightIES\Gldf\<DisplayName>\<pid>"
// and the factory node is that prefixed with "Factory_"; from there
// GetCustomReferenceObject yields the built asset. DisplayName is the source
// base filename (same derivation the translator uses), so the factory needs
// no extra plumbing to find these.
static UTextureLightProfile* ResolveIesProfile(
    const UInterchangeBaseNodeContainer& NodeContainer,
    const FString& DisplayName,
    const FString& PhotometryId)
{
    if (PhotometryId.IsEmpty())
    {
        UE_LOG(LogGldfActorFactory, Warning,
            TEXT("ResolveIesProfile: empty PhotometryId (no IES for this emitter)"));
        return nullptr;
    }
    const FString IesNodeUid =
        FString::Printf(TEXT("\\LightIES\\Gldf\\%s\\%s"), *DisplayName, *PhotometryId);
    const FString IesFactoryUid = UInterchangeFactoryBaseNode::BuildFactoryNodeUid(IesNodeUid);
    const UInterchangeTextureLightProfileFactoryNode* IesFactoryNode =
        Cast<UInterchangeTextureLightProfileFactoryNode>(
            NodeContainer.GetFactoryNode(IesFactoryUid));
    if (!IesFactoryNode)
    {
        UE_LOG(LogGldfActorFactory, Warning,
            TEXT("ResolveIesProfile: no IES factory node for uid %s — the texture node "
                 "wasn't built (payload-key/photometry-id mismatch?)"), *IesFactoryUid);
        return nullptr;
    }
    FSoftObjectPath IesRef;
    IesFactoryNode->GetCustomReferenceObject(IesRef);
    UTextureLightProfile* Profile = Cast<UTextureLightProfile>(IesRef.TryLoad());
    if (!Profile)
    {
        UE_LOG(LogGldfActorFactory, Warning,
            TEXT("ResolveIesProfile: factory node %s has no loadable UTextureLightProfile "
                 "(ref=%s) — IES decode/build failed"), *IesFactoryUid, *IesRef.ToString());
    }
    else
    {
        UE_LOG(LogGldfActorFactory, Log,
            TEXT("ResolveIesProfile: resolved %s → %s"), *PhotometryId, *Profile->GetName());
    }
    return Profile;
}

// Read a per-emitter string field (frees the FFI CString).
static FString ReadEmitterString(uint64 Handle, uint32 VIdx, uint32 EIdx, uint32 Field)
{
    FString Out;
    if (char* Raw = gldf_unreal_variant_emitter_string(Handle, VIdx, EIdx, Field))
    {
        Out = FString(UTF8_TO_TCHAR(Raw));
        gldf_unreal_string_free(Raw);
    }
    return Out;
}

void UInterchangeGldfActorFactory::ApplyLightAndVariants(
    AGldfLuminaireActor& Luminaire,
    const UInterchangeActorFactoryNode& /*FactoryNode*/,
    const UInterchangeBaseNodeContainer& NodeContainer,
    const FImportSceneObjectsParams& Params) const
{
    // GLDF source path: available directly off the scene-import params.
    const FString SourcePath = Params.SourceData ? Params.SourceData->GetFilename() : FString();
    if (SourcePath.IsEmpty())
    {
        UE_LOG(LogGldfActorFactory, Warning, TEXT("ApplyLightAndVariants: no source path"));
        return;
    }
    const FTCHARToUTF8 PathUtf8(*SourcePath);
    const FString DisplayName = FPaths::GetBaseFilename(SourcePath);

    constexpr float MetersToCm = 100.0f;

    // ── Open the variant table; build the per-variant per-emitter table ─────
    uint32 VariantCount = 0;
    char* RawErr = nullptr;
    const uint64 Handle = gldf_unreal_variants_open(PathUtf8.Get(), &VariantCount, &RawErr);
    if (Handle == 0)
    {
        const FString Msg = RawErr ? FString(UTF8_TO_TCHAR(RawErr)) : TEXT("(no message)");
        UE_LOG(LogGldfActorFactory, Warning,
            TEXT("ApplyLightAndVariants: variants_open failed: %s"), *Msg);
        if (RawErr) { gldf_unreal_string_free(RawErr); }
        return;
    }

    Luminaire.Variants.Reset(static_cast<int32>(VariantCount));
    {
        auto ReadProduct = [&](uint32 Field) {
            FString Out;
            if (char* Raw = gldf_unreal_product_string(Handle, Field)) { Out = FString(UTF8_TO_TCHAR(Raw)); gldf_unreal_string_free(Raw); }
            return Out;
        };
        Luminaire.Manufacturer = ReadProduct(GLDF_PRODUCT_FIELD_MANUFACTURER);
        Luminaire.ProductName = ReadProduct(GLDF_PRODUCT_FIELD_NAME);
        Luminaire.ProductNumber = ReadProduct(GLDF_PRODUCT_FIELD_NUMBER);
        Luminaire.SourceFile = FPaths::GetCleanFilename(SourcePath);
    }
    // The set of LEO partNames that some variant actually drives with a light
    // (so we skip non-light LEOs like a sensor when building components).
    TSet<FString> LitEmitterNames;
    for (uint32 vi = 0; vi < VariantCount; ++vi)
    {
        FGldfLuminaireVariant V;
        if (char* IdRaw = gldf_unreal_variant_string(Handle, vi, GLDF_VARIANT_FIELD_ID))
        {
            V.VariantId = FString(UTF8_TO_TCHAR(IdRaw));
            gldf_unreal_string_free(IdRaw);
        }
        if (char* NameRaw = gldf_unreal_variant_string(Handle, vi, GLDF_VARIANT_FIELD_NAME))
        {
            V.VariantName = FString(UTF8_TO_TCHAR(NameRaw));
            gldf_unreal_string_free(NameRaw);
        }

        // Mounting (per variant in GLDF): drives AGldfLuminaireActor::SnapToSurface.
        GldfMountingInfo M{};
        if (gldf_unreal_variant_mounting(Handle, vi, &M) == 0)
        {
            switch (M.kind)
            {
            case GLDF_MOUNTING_CEILING:       V.Mounting.Kind = EGldfMountingKind::Ceiling; break;
            case GLDF_MOUNTING_WALL:          V.Mounting.Kind = EGldfMountingKind::Wall; break;
            case GLDF_MOUNTING_GROUND:        V.Mounting.Kind = EGldfMountingKind::Ground; break;
            case GLDF_MOUNTING_WORKING_PLANE: V.Mounting.Kind = EGldfMountingKind::WorkingPlane; break;
            default:                          V.Mounting.Kind = EGldfMountingKind::None; break;
            }
            V.Mounting.bSurfaceMounted = M.surface_mounted != 0;
            V.Mounting.bFreeStanding = M.free_standing != 0;
            V.Mounting.bPoleTop = M.pole_top != 0;
            V.Mounting.bPoleIntegrated = M.pole_integrated != 0;
            V.Mounting.RecessedDepthMm = M.recessed_depth_present ? M.recessed_depth_mm : -1;
            V.Mounting.PendantLengthMm = M.pendant_length_present ? M.pendant_length_mm : -1;
            V.Mounting.MountingHeightMm = M.mounting_height_present ? M.mounting_height_mm : -1;
            V.Mounting.PoleHeightMm = M.pole_height_present ? M.pole_height_mm : -1;
        }

        if (char* Raw = gldf_unreal_variant_string(Handle, vi, GLDF_VARIANT_FIELD_PRODUCT_NUMBER)) { V.ProductNumber = FString(UTF8_TO_TCHAR(Raw)); gldf_unreal_string_free(Raw); }
        if (char* Raw = gldf_unreal_variant_string(Handle, vi, GLDF_VARIANT_FIELD_GTIN)) { V.Gtin = FString(UTF8_TO_TCHAR(Raw)); gldf_unreal_string_free(Raw); }

        const int32 EmitterCount = gldf_unreal_variant_emitter_count(Handle, vi);
        for (int32 ei = 0; ei < EmitterCount; ++ei)
        {
            FGldfEmitterPhotometry P;
            P.EmitterName =
                ReadEmitterString(Handle, vi, ei, GLDF_VARIANT_FIELD_EMITTER_EXTERNAL_NAME);

            GldfVariantInfo Info{};
            if (gldf_unreal_variant_emitter_info(Handle, vi, static_cast<uint32>(ei), &Info) == 0)
            {
                if (Info.lumens_present != 0) { P.Lumens = Info.lumens; }
                if (Info.cct_present != 0)    { P.Cct = Info.cct; }
                if (Info.watts_present != 0)  { V.Watts += static_cast<float>(Info.watts); }
            }

            const FString PhotometryId =
                ReadEmitterString(Handle, vi, ei, GLDF_VARIANT_FIELD_PHOTOMETRY_ID);
            P.IesProfile = ResolveIesProfile(NodeContainer, DisplayName, PhotometryId);

            // Keep the IES text on the actor: the runtime illuminance grid feeds
            // it to the eulumdat core (no editor / Interchange needed in games).
            {
                const uint8* IesPtr = nullptr; uint32 IesLen = 0;
                if (gldf_unreal_variant_emitter_ies(Handle, vi, static_cast<uint32>(ei), &IesPtr, &IesLen) == 0 && IesPtr && IesLen > 0)
                {
                    P.IesBytes.SetNumUninitialized(IesLen);
                    FMemory::Memcpy(P.IesBytes.GetData(), IesPtr, IesLen);
                }
            }

            if (!P.EmitterName.IsEmpty())
            {
                LitEmitterNames.Add(P.EmitterName);
            }
            V.Emitters.Add(MoveTemp(P));
        }

        Luminaire.Variants.Add(MoveTemp(V));
    }
    gldf_unreal_variants_close(Handle);

    // ── Build one light component per LEO ───────────────────────────────────
    USceneComponent* Root = Luminaire.GetRootComponent();
    const int32 LeoCount = gldf_unreal_leo_count(PathUtf8.Get());

    // Gather every LEO's partName up front so we can decide the matching mode.
    TArray<FString> LeoNames;
    LeoNames.Reserve(LeoCount);
    for (int32 li = 0; li < LeoCount; ++li)
    {
        FString PartName;
        if (char* Raw = gldf_unreal_leo_part_name(PathUtf8.Get(), static_cast<uint32>(li)))
        {
            PartName = FString(UTF8_TO_TCHAR(Raw));
            gldf_unreal_string_free(Raw);
        }
        LeoNames.Add(MoveTemp(PartName));
    }

    // STRICT match: a LEO partName equals some emitter's external name (the
    // curated case — e.g. SLV Tria 2's LEO0/LEO1). Then we light exactly the
    // matched LEOs (skipping sensors / unreferenced LEOs).
    //
    // FALLBACK: most real-world GLDFs name the LEO "leo" but the emitter
    // external name something unrelated ("em__---__0001"), so NOTHING matches.
    // Rather than build zero lights, we then light EVERY LEO and assign the
    // variant's emitters POSITIONALLY (LEO i ← emitter i). We name each built
    // component after the emitter's external name it's paired with, so
    // ApplySelectedVariant (which matches component name == variant-emitter
    // name) applies the right IES/lumens/CCT.
    bool bStrictMatch = false;
    for (const FString& Ln : LeoNames)
    {
        if (!Ln.IsEmpty() && LitEmitterNames.Contains(Ln)) { bStrictMatch = true; break; }
    }

    // Ordered emitter external-names from the default variant (index 0), for
    // positional assignment in the fallback path.
    TArray<FString> EmitterNamesV0;
    if (Luminaire.Variants.Num() > 0)
    {
        for (const FGldfEmitterPhotometry& P : Luminaire.Variants[0].Emitters)
        {
            EmitterNamesV0.Add(P.EmitterName);
        }
    }
    int32 FallbackEmitterCursor = 0;

    UE_LOG(LogGldfActorFactory, Log,
        TEXT("ApplyLightAndVariants: %d LEO(s), strict partName match=%d (fallback=%d)"),
        LeoCount, bStrictMatch ? 1 : 0, bStrictMatch ? 0 : 1);

    int32 BuiltLights = 0;
    for (int32 li = 0; li < LeoCount; ++li)
    {
        const FString& PartName = LeoNames[li];

        // Decide whether to light this LEO + which emitter name to tag it with
        // (the key ApplySelectedVariant matches on).
        FString EmitterKeyName;
        if (bStrictMatch)
        {
            if (!LitEmitterNames.Contains(PartName))
            {
                continue; // strict: skip sensors / unreferenced LEOs
            }
            EmitterKeyName = PartName;
        }
        else
        {
            // Fallback: light every LEO, pair it with the next variant emitter.
            EmitterKeyName = EmitterNamesV0.IsValidIndex(FallbackEmitterCursor)
                ? EmitterNamesV0[FallbackEmitterCursor]
                : (EmitterNamesV0.Num() > 0 ? EmitterNamesV0.Last() : PartName);
            ++FallbackEmitterCursor;
        }

        GldfLeoShape Shape{};
        if (gldf_unreal_leo_at(PathUtf8.Get(), static_cast<uint32>(li), &Shape) != 0)
        {
            continue;
        }

        ULocalLightComponent* Light = nullptr;
        const FName CompName(*FString::Printf(TEXT("Emitter_%d_%s"), li,
            PartName.IsEmpty() ? *FString::FromInt(li) : *PartName));
        if (Shape.kind == GLDF_LEO_RECTANGLE)
        {
            URectLightComponent* Rect = NewObject<URectLightComponent>(&Luminaire, CompName);
            Rect->SetSourceWidth(Shape.dim_a * MetersToCm);   // long axis (world +X)
            Rect->SetSourceHeight(Shape.dim_b * MetersToCm);  // short axis (world +Y)
            Light = Rect;
        }
        else
        {
            // Circle or none → point light (emits the IES distribution from the
            // emitter location; closest to a bare luminaire without an aim vector).
            Light = NewObject<UPointLightComponent>(&Luminaire, CompName);
        }

        if (Root)
        {
            Light->SetupAttachment(Root);
        }
        Luminaire.AddInstanceComponent(Light);
        Light->RegisterComponent();

        // Placement + orientation from the LEO's real world axes (Rust now
        // reports them, honouring the geometry·joint·LEO rotation — no more
        // "always down" assumption).
        //
        // UE light-component local frame: a (Rect)LightComponent emits along
        // local +X; for a RectLight the rectangle is in the local YZ plane
        // with SourceWidth → local +Y, SourceHeight → local +Z.
        //
        // L3D LEO frame (Rust axes ax/ay/az = world images of local X/Y/Z):
        // emits along −az; rect width along ax; height along ay.
        //
        // So map: UE local +X ← (−az), +Y ← ax, +Z ← ay. Each Rust axis is in
        // the right-handed Z-up frame; apply the same UE basis flip the mesh
        // uses (negate Y) so the light agrees with the body geometry. An
        // FMatrix's rows are the world images of local X/Y/Z.
        {
            auto FlipY = [](float x, float y, float z) { return FVector(x, -y, z); };
            const FVector EmitDir = FlipY(-Shape.az_x, -Shape.az_y, -Shape.az_z); // local +X
            const FVector WidthDir = FlipY(Shape.ax_x, Shape.ax_y, Shape.ax_z);   // local +Y
            const FVector HeightDir = FlipY(Shape.ay_x, Shape.ay_y, Shape.ay_z);  // local +Z

            const FMatrix RotM(
                FPlane(EmitDir.X,   EmitDir.Y,   EmitDir.Z,   0.f),
                FPlane(WidthDir.X,  WidthDir.Y,  WidthDir.Z,  0.f),
                FPlane(HeightDir.X, HeightDir.Y, HeightDir.Z, 0.f),
                FPlane(0.f, 0.f, 0.f, 1.f));

            // Emitter center: cm in the right-handed Z-up frame; negate Y to
            // match the mesh's UE basis flip (PositionToUEBasis).
            FVector Loc(Shape.pos_x, -Shape.pos_y, Shape.pos_z);

            // (Lights that would shine through their own housing are moved
            // outside it by AGldfLuminaireActor::AdjustLightsOutsideHousing —
            // the mesh has no bounds yet at this point of the import.)
            Light->SetMobility(EComponentMobility::Movable);
            Light->SetRelativeRotation(FQuat(RotM));
            Light->SetRelativeLocation(Loc);
        }

        FGldfEmitterComponent Entry;
        // Tag with the emitter external-name (strict: the matched partName;
        // fallback: the positionally-assigned emitter) so ApplySelectedVariant
        // resolves this light's photometry by name.
        Entry.EmitterName = EmitterKeyName;
        Entry.Light = Light;
        Luminaire.Emitters.Add(MoveTemp(Entry));
        ++BuiltLights;
    }

    UE_LOG(LogGldfActorFactory, Log,
        TEXT("ApplyLightAndVariants: %d variant(s), %d LEO(s) → %d emitter light(s)"),
        Luminaire.Variants.Num(), LeoCount, BuiltLights);

    // Default the dropdown to the first variant (by its display label) and
    // apply it to every emitter light.
    if (Luminaire.Variants.Num() > 0)
    {
        const FGldfLuminaireVariant& First = Luminaire.Variants[0];
        const FString Label = First.VariantName.IsEmpty() ? First.VariantId : First.VariantName;
        Luminaire.SelectedVariantName = FName(*Label);
    }
    Luminaire.ApplySelectedVariant();
}
