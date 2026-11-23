#include "GldfRuntimeLoader.h"

#include "GldfLuminaireActor.h"

#include "Algo/Reverse.h"
#include "Components/PointLightComponent.h"
#include "Components/RectLightComponent.h"
#include "Components/StaticMeshComponent.h"
#include "Engine/Engine.h"
#include "Engine/StaticMesh.h"
#include "Engine/TextureLightProfile.h"
#include "Engine/World.h"
#include "HAL/FileManager.h"
#include "IESConverter.h"
#include "MeshDescription.h"
#include "Misc/Crc.h"
#include "Misc/Paths.h"
#include "StaticMeshAttributes.h"
#include "StaticMeshOperations.h"
#include "TextureResource.h"
#include "UObject/Package.h"

extern "C" {
#include "gldf_unreal.h"
}

DEFINE_LOG_CATEGORY_STATIC(LogGldfRuntimeLoader, Log, All);

namespace
{
    // Same basis handling as the editor translator (InterchangeGldfTranslator.cpp):
    // the Rust side hands out centimetres in a right-handed Z-up frame; UE is
    // left-handed Z-up, so Y is negated (and polygon winding reversed), V flipped.
    FVector3f ToUE(const FVector3f& V) { return FVector3f(V.X, -V.Y, V.Z); }
    FVector2f UVToUE(const FVector2f& V) { return FVector2f(V.X, 1.0f - V.Y); }

    FString TakeString(char* Raw)
    {
        FString Out;
        if (Raw)
        {
            Out = FString(UTF8_TO_TCHAR(Raw));
            gldf_unreal_string_free(Raw);
        }
        return Out;
    }

    bool BuildMeshDescription(uint64 Handle, const GldfMeshHeader& Header, FMeshDescription& MD)
    {
        const float* Positions = nullptr;
        const float* Normals = nullptr;
        const float* UVs = nullptr;
        const GldfMeshCorner* Corners = nullptr;
        const GldfMeshPolygon* Polygons = nullptr;
        const int32 Borrow = gldf_unreal_mesh_borrow(Handle, &Positions, &Normals, &UVs, &Corners, &Polygons);
        if (Borrow != 0 || !Positions || !Corners || !Polygons)
        {
            UE_LOG(LogGldfRuntimeLoader, Error, TEXT("mesh_borrow failed (%d)"), Borrow);
            return false;
        }

        FStaticMeshAttributes Attributes(MD);
        Attributes.Register();

        const bool bHasNormals = Header.normal_count > 0 && Normals != nullptr;
        const bool bHasUVs = Header.uv_count > 0 && UVs != nullptr;

        TVertexAttributesRef<FVector3f> VertexPositions = Attributes.GetVertexPositions();
        MD.ReserveNewVertices(Header.vertex_count);
        TArray<FVertexID> VertexIDs;
        VertexIDs.Reserve(Header.vertex_count);
        for (uint32 i = 0; i < Header.vertex_count; ++i)
        {
            const FVertexID VID = MD.CreateVertex();
            VertexIDs.Add(VID);
            VertexPositions[VID] = ToUE(FVector3f(Positions[i * 3], Positions[i * 3 + 1], Positions[i * 3 + 2]));
        }

        MD.SetNumUVChannels(1);
        const FPolygonGroupID Group = MD.CreatePolygonGroup();
        Attributes.GetPolygonGroupMaterialSlotNames()[Group] = FName(TEXT("GLDF_Body"));

        TVertexInstanceAttributesRef<FVector3f> InstanceNormals = Attributes.GetVertexInstanceNormals();
        TVertexInstanceAttributesRef<FVector2f> InstanceUVs = Attributes.GetVertexInstanceUVs();
        if (InstanceUVs.GetNumChannels() == 0) { InstanceUVs.SetNumChannels(1); }

        MD.ReserveNewPolygons(Header.polygon_count);
        MD.ReserveNewVertexInstances(Header.corner_count);

        int32 Built = 0;
        for (uint32 p = 0; p < Header.polygon_count; ++p)
        {
            const GldfMeshPolygon& Poly = Polygons[p];
            if (Poly.corner_count < 3) { continue; }

            TArray<FVertexInstanceID, TInlineAllocator<8>> Ids;
            for (uint32 c = 0; c < Poly.corner_count; ++c)
            {
                const GldfMeshCorner& Corner = Corners[Poly.corner_offset + c];
                if (Corner.position_idx >= (uint32)VertexIDs.Num()) { continue; }
                const FVertexInstanceID IID = MD.CreateVertexInstance(VertexIDs[Corner.position_idx]);
                if (bHasNormals && Corner.normal_idx >= 0 && (uint32)Corner.normal_idx < Header.normal_count)
                {
                    const int32 n = Corner.normal_idx;
                    InstanceNormals[IID] = ToUE(FVector3f(Normals[n * 3], Normals[n * 3 + 1], Normals[n * 3 + 2]));
                }
                if (bHasUVs && Corner.uv_idx >= 0 && (uint32)Corner.uv_idx < Header.uv_count)
                {
                    const int32 u = Corner.uv_idx;
                    InstanceUVs.Set(IID, 0, UVToUE(FVector2f(UVs[u * 2], UVs[u * 2 + 1])));
                }
                else
                {
                    InstanceUVs.Set(IID, 0, FVector2f::ZeroVector);
                }
                Ids.Add(IID);
            }
            if (Ids.Num() < 3) { continue; }
            Algo::Reverse(Ids); // Y negation flips orientation
            MD.CreatePolygon(Group, Ids);
            ++Built;
        }
        if (Built == 0)
        {
            UE_LOG(LogGldfRuntimeLoader, Warning, TEXT("mesh has no usable polygons"));
            return false;
        }

        // The editor build pipeline computes normals/tangents; the runtime
        // fast build only copies what is in the description, so fill in here.
        FStaticMeshOperations::ComputeTriangleTangentsAndNormals(MD);
        EComputeNTBsFlags Flags = EComputeNTBsFlags::Tangents;
        if (!bHasNormals) { Flags |= EComputeNTBsFlags::Normals; }
        FStaticMeshOperations::ComputeTangentsAndNormals(MD, Flags);
        return true;
    }
}

static UStaticMesh* BuildMeshFromHandle(uint64 Handle, const GldfMeshHeader& Header, const FString& Name, FString& OutError)
{
    FMeshDescription MD;
    const bool bOk = BuildMeshDescription(Handle, Header, MD);
    gldf_unreal_mesh_close(Handle);
    if (!bOk)
    {
        OutError = TEXT("L3D geometry could not be converted");
        return nullptr;
    }
    const FName MeshName = MakeUniqueObjectName(GetTransientPackage(), UStaticMesh::StaticClass(), FName(*Name));
    UStaticMesh* Mesh = NewObject<UStaticMesh>(GetTransientPackage(), MeshName, RF_Transient);
    Mesh->GetStaticMaterials().Add(FStaticMaterial(nullptr, FName(TEXT("GLDF_Body"))));
    UStaticMesh::FBuildMeshDescriptionsParams Params;
    Params.bFastBuild = true;
    Params.bBuildSimpleCollision = true;
    Params.bAllowCpuAccess = true;
    Params.bMarkPackageDirty = false;
    Params.bCommitMeshDescription = false;
    TArray<const FMeshDescription*> Descs{ &MD };
    if (!Mesh->BuildFromMeshDescriptions(Descs, Params))
    {
        OutError = TEXT("BuildFromMeshDescriptions failed");
        return nullptr;
    }
    return Mesh;
}

/// Build the articulated part tree (joints + per-part meshes) on the actor.
/// Returns the number of nodes built (0 = no structure → caller falls back to
/// the baked single mesh). Fills `OutNodeComponents` (per node) for the
/// light attachment and `OutNodeWorld` (actor-relative transform per node).
static int32 BuildArticulatedParts(AGldfLuminaireActor* Lum, uint64 L3d, uint32 NodeCount,
    TArray<USceneComponent*>& OutNodeComponents, TArray<FTransform>& OutNodeRel)
{
    OutNodeComponents.Init(nullptr, NodeCount);
    OutNodeRel.Init(FTransform::Identity, NodeCount);
    int32 Built = 0;
    for (uint32 i = 0; i < NodeCount; ++i)
    {
        GldfL3dNode N{};
        if (gldf_unreal_l3d_node(L3d, i, &N) != 0) { continue; }
        FString Name;
        if (char* Raw = gldf_unreal_l3d_node_name(L3d, i)) { Name = FString(UTF8_TO_TCHAR(Raw)); gldf_unreal_string_free(Raw); }
        FString Clean; for (TCHAR C : Name) { Clean.AppendChar(FChar::IsAlnum(C) ? C : TEXT('_')); }

        // Rest pose relative to the parent, converted to the UE frame.
        const FVector RelPos = AGldfLuminaireActor::L3dToUePos(FVector(N.local_pos_cm[0], N.local_pos_cm[1], N.local_pos_cm[2]));
        const FQuat RelRot = AGldfLuminaireActor::L3dEulerToUeQuat(FVector(N.local_rot_deg[0], N.local_rot_deg[1], N.local_rot_deg[2]));
        const FTransform Rel(RelRot, RelPos);
        USceneComponent* Parent = (N.parent >= 0 && OutNodeComponents.IsValidIndex(N.parent)) ? OutNodeComponents[N.parent] : nullptr;
        OutNodeRel[i] = (N.parent >= 0) ? Rel * OutNodeRel[N.parent] : Rel;

        if (i == 0)
        {
            // Root part = BodyMesh (actor root). Its own rest pose is folded
            // into the children (the actor origin is the L3D origin).
            OutNodeRel[0] = FTransform::Identity;
            OutNodeComponents[0] = Lum->BodyMesh;
            if (N.has_mesh)
            {
                GldfMeshHeader H{};
                const uint64 Mh = gldf_unreal_l3d_node_mesh_open(L3d, i, &H);
                FString Err;
                if (UStaticMesh* M = Mh ? BuildMeshFromHandle(Mh, H, TEXT("SM_Gldf_") + Clean, Err) : nullptr)
                {
                    Lum->BodyMesh->SetStaticMesh(M);
                }
            }
            ++Built;
            continue;
        }
        if (!Parent) { continue; }

        if (N.is_joint)
        {
            USceneComponent* Pivot = NewObject<USceneComponent>(Lum, MakeUniqueObjectName(Lum, USceneComponent::StaticClass(), FName(*FString::Printf(TEXT("Joint_%s"), *Clean))));
            Pivot->SetupAttachment(Parent);
            Pivot->SetMobility(EComponentMobility::Movable);
            Lum->AddInstanceComponent(Pivot);
            Pivot->RegisterComponent();
            Pivot->SetRelativeLocationAndRotation(RelPos, RelRot);
            FGldfJoint J;
            J.Name = Clean;
            J.Component = Pivot;
            J.AxisMask = N.axis_mask;
            J.MinDeg = FVector(N.axis_min[0], N.axis_min[1], N.axis_min[2]);
            J.MaxDeg = FVector(N.axis_max[0], N.axis_max[1], N.axis_max[2]);
            J.StepDeg = FVector(N.axis_step[0], N.axis_step[1], N.axis_step[2]);
            J.RestLocation = RelPos;
            J.RestRotation = RelRot;
            Lum->Joints.Add(J);
            if (N.has_default_rotation)
            {
                Lum->SetJointAngles(Lum->Joints.Num() - 1, FVector(N.default_rotation_deg[0], N.default_rotation_deg[1], N.default_rotation_deg[2]));
            }
            OutNodeComponents[i] = Pivot;
        }
        else
        {
            UStaticMeshComponent* Part = NewObject<UStaticMeshComponent>(Lum, MakeUniqueObjectName(Lum, UStaticMeshComponent::StaticClass(), FName(*FString::Printf(TEXT("Part_%s"), *Clean))));
            Part->SetupAttachment(Parent);
            Part->SetMobility(EComponentMobility::Movable);
            Lum->AddInstanceComponent(Part);
            Part->RegisterComponent();
            Part->SetRelativeLocationAndRotation(RelPos, RelRot);
            if (N.has_mesh)
            {
                GldfMeshHeader H{};
                const uint64 Mh = gldf_unreal_l3d_node_mesh_open(L3d, i, &H);
                FString Err;
                if (UStaticMesh* M = Mh ? BuildMeshFromHandle(Mh, H, TEXT("SM_Gldf_") + Clean, Err) : nullptr)
                {
                    Part->SetStaticMesh(M);
                }
            }
            Lum->PartMeshes.Add(Part);
            OutNodeComponents[i] = Part;
        }
        ++Built;
    }
    return Built;
}

UStaticMesh* UGldfRuntimeLoader::BuildBodyMesh(const FString& GldfPath, FString& OutError)
{
    const FTCHARToUTF8 PathUtf8(*GldfPath);
    GldfMeshHeader Header{};
    char* RawErr = nullptr;
    const uint64 Handle = gldf_unreal_first_mesh_open(PathUtf8.Get(), &Header, &RawErr);
    if (Handle == 0)
    {
        OutError = TakeString(RawErr);
        if (OutError.IsEmpty()) { OutError = TEXT("no mesh in GLDF"); }
        return nullptr;
    }

    FMeshDescription MD;
    const bool bOk = BuildMeshDescription(Handle, Header, MD);
    gldf_unreal_mesh_close(Handle);
    if (!bOk)
    {
        OutError = TEXT("L3D geometry could not be converted");
        return nullptr;
    }

    const FName MeshName = MakeUniqueObjectName(GetTransientPackage(), UStaticMesh::StaticClass(),
        FName(*FString::Printf(TEXT("SM_Gldf_%s"), *FPaths::GetBaseFilename(GldfPath))));
    UStaticMesh* Mesh = NewObject<UStaticMesh>(GetTransientPackage(), MeshName, RF_Transient);
    // (The three-argument constructor with ImportedMaterialSlotName is editor-only.)
    Mesh->GetStaticMaterials().Add(FStaticMaterial(nullptr, FName(TEXT("GLDF_Body"))));

    UStaticMesh::FBuildMeshDescriptionsParams Params;
    Params.bFastBuild = true;
    Params.bBuildSimpleCollision = true;   // box: enough for picking / snapping traces
    Params.bAllowCpuAccess = true;
    Params.bMarkPackageDirty = false;
    Params.bCommitMeshDescription = false;
    TArray<const FMeshDescription*> Descs{ &MD };
    if (!Mesh->BuildFromMeshDescriptions(Descs, Params))
    {
        OutError = TEXT("BuildFromMeshDescriptions failed");
        return nullptr;
    }
    UE_LOG(LogGldfRuntimeLoader, Log, TEXT("body mesh %s: %u vertices, %u polygons, bounds %s"),
        *Mesh->GetName(), Header.vertex_count, Header.polygon_count, *Mesh->GetBounds().BoxExtent.ToCompactString());
    return Mesh;
}

UTextureLightProfile* UGldfRuntimeLoader::BuildLightProfile(const uint8* IesBytes, uint32 Len, const FString& Name)
{
    if (!IesBytes || Len == 0) { return nullptr; }
    FIESConverter Conv(IesBytes, Len);
    if (!Conv.IsValid())
    {
        UE_LOG(LogGldfRuntimeLoader, Warning, TEXT("IES rejected for %s: %s"), *Name, Conv.GetError());
        return nullptr;
    }
    const uint32 W = Conv.GetWidth(), H = Conv.GetHeight();
    const TArray<uint8>& Raw = Conv.GetRawData(); // RGBA16F
    const int64 Bytes = (int64)W * (int64)H * 8;
    if (W == 0 || H == 0 || Raw.Num() < Bytes)
    {
        UE_LOG(LogGldfRuntimeLoader, Warning, TEXT("IES %s: unexpected raw size %d for %ux%u"), *Name, Raw.Num(), W, H);
        return nullptr;
    }

    const FName TexName = MakeUniqueObjectName(GetTransientPackage(), UTextureLightProfile::StaticClass(),
        FName(*FString::Printf(TEXT("IES_%s"), *Name)));
    UTextureLightProfile* Tex = NewObject<UTextureLightProfile>(GetTransientPackage(), TexName, RF_Transient);

    // Same recipe as UTexture2D::CreateTransient, on the light-profile subclass.
    Tex->SetPlatformData(new FTexturePlatformData());
    FTexturePlatformData* PD = Tex->GetPlatformData();
    PD->SizeX = W;
    PD->SizeY = H;
    PD->SetNumSlices(1);
    PD->PixelFormat = PF_FloatRGBA;
    Tex->bNotOfflineProcessed = true;
    Tex->NeverStream = true;
    Tex->SRGB = false;
    Tex->CompressionSettings = TC_HDR;
    Tex->AddressX = TA_Clamp;
    Tex->AddressY = TA_Clamp;
    Tex->LODGroup = TEXTUREGROUP_IESLightProfile;
    Tex->Brightness = Conv.GetBrightness();
    Tex->TextureMultiplier = Conv.GetMultiplier();

    FTexture2DMipMap* Mip = new FTexture2DMipMap(W, H, 1);
    PD->Mips.Add(Mip);
    Mip->BulkData.Lock(LOCK_READ_WRITE);
    void* Dest = Mip->BulkData.Realloc(Bytes);
    FMemory::Memcpy(Dest, Raw.GetData(), Bytes);
    Mip->BulkData.Unlock();
    Tex->UpdateResource();
    return Tex;
}

bool UGldfRuntimeLoader::PopulateLuminaire(AGldfLuminaireActor* Luminaire, const FString& GldfPath, FString& OutError)
{
    if (!Luminaire)
    {
        OutError = TEXT("no luminaire actor");
        return false;
    }
    if (!FPaths::FileExists(GldfPath))
    {
        OutError = FString::Printf(TEXT("file not found: %s"), *GldfPath);
        return false;
    }
    const FTCHARToUTF8 PathUtf8(*GldfPath);
    constexpr float MetersToCm = 100.0f;

    // ── Body: the articulated part tree when the L3D has one, else one mesh ──
    uint32 L3dNodes = 0, L3dLeos = 0;
    char* L3dErr = nullptr;
    uint64 L3d = gldf_unreal_l3d_open(PathUtf8.Get(), &L3dNodes, &L3dLeos, &L3dErr);
    if (L3dErr) { gldf_unreal_string_free(L3dErr); }
    TArray<USceneComponent*> NodeComps;
    TArray<FTransform> NodeRel;
    bool bArticulated = false;
    if (L3d != 0 && L3dNodes > 1 && Luminaire->BodyMesh)
    {
        bArticulated = BuildArticulatedParts(Luminaire, L3d, L3dNodes, NodeComps, NodeRel) > 1;
        UE_LOG(LogGldfRuntimeLoader, Log, TEXT("%s: %u L3D node(s) → %d joint(s), %d extra part(s)"), *FPaths::GetCleanFilename(GldfPath), L3dNodes, Luminaire->Joints.Num(), Luminaire->PartMeshes.Num());
    }
    if (!bArticulated)
    {
        FString MeshErr;
        if (UStaticMesh* Mesh = BuildBodyMesh(GldfPath, MeshErr))
        {
            if (Luminaire->BodyMesh)
            {
                Luminaire->BodyMesh->SetStaticMesh(Mesh);
            }
        }
        else
        {
            // A luminaire without geometry still lights: keep going with the LEOs.
            UE_LOG(LogGldfRuntimeLoader, Warning, TEXT("%s: no body mesh (%s)"), *GldfPath, *MeshErr);
        }
    }

    // ── Variants / photometry / metadata ────────────────────────────────────
    uint32 VariantCount = 0;
    char* RawErr = nullptr;
    const uint64 Handle = gldf_unreal_variants_open(PathUtf8.Get(), &VariantCount, &RawErr);
    if (Handle == 0)
    {
        OutError = TakeString(RawErr);
        if (OutError.IsEmpty()) { OutError = TEXT("no variants in GLDF"); }
        return false;
    }

    Luminaire->Manufacturer = TakeString(gldf_unreal_product_string(Handle, GLDF_PRODUCT_FIELD_MANUFACTURER));
    Luminaire->ProductName = TakeString(gldf_unreal_product_string(Handle, GLDF_PRODUCT_FIELD_NAME));
    Luminaire->ProductNumber = TakeString(gldf_unreal_product_string(Handle, GLDF_PRODUCT_FIELD_NUMBER));
    Luminaire->SourceFile = FPaths::GetCleanFilename(GldfPath);

    // One transient IES texture per distinct photometry (several emitters /
    // variants usually share one file).
    TMap<uint32, TObjectPtr<UTextureLightProfile>> ProfileCache;
    const FString Base = FPaths::GetBaseFilename(GldfPath);

    TSet<FString> LitEmitterNames;
    Luminaire->Variants.Reset(static_cast<int32>(VariantCount));
    for (uint32 vi = 0; vi < VariantCount; ++vi)
    {
        FGldfLuminaireVariant V;
        V.VariantId = TakeString(gldf_unreal_variant_string(Handle, vi, GLDF_VARIANT_FIELD_ID));
        V.VariantName = TakeString(gldf_unreal_variant_string(Handle, vi, GLDF_VARIANT_FIELD_NAME));
        V.ProductNumber = TakeString(gldf_unreal_variant_string(Handle, vi, GLDF_VARIANT_FIELD_PRODUCT_NUMBER));
        V.Gtin = TakeString(gldf_unreal_variant_string(Handle, vi, GLDF_VARIANT_FIELD_GTIN));

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

        const int32 EmitterCount = gldf_unreal_variant_emitter_count(Handle, vi);
        for (int32 ei = 0; ei < EmitterCount; ++ei)
        {
            FGldfEmitterPhotometry P;
            P.EmitterName = TakeString(gldf_unreal_variant_emitter_string(Handle, vi, ei, GLDF_VARIANT_FIELD_EMITTER_EXTERNAL_NAME));

            GldfVariantInfo Info{};
            if (gldf_unreal_variant_emitter_info(Handle, vi, static_cast<uint32>(ei), &Info) == 0)
            {
                if (Info.lumens_present) { P.Lumens = Info.lumens; }
                if (Info.cct_present)    { P.Cct = Info.cct; }
                if (Info.watts_present)  { V.Watts += static_cast<float>(Info.watts); }
            }

            const uint8* IesPtr = nullptr; uint32 IesLen = 0;
            if (gldf_unreal_variant_emitter_ies(Handle, vi, static_cast<uint32>(ei), &IesPtr, &IesLen) == 0 && IesPtr && IesLen > 0)
            {
                P.IesBytes.SetNumUninitialized(IesLen);
                FMemory::Memcpy(P.IesBytes.GetData(), IesPtr, IesLen);

                const uint32 Key = FCrc::MemCrc32(IesPtr, IesLen);
                if (TObjectPtr<UTextureLightProfile>* Cached = ProfileCache.Find(Key))
                {
                    P.IesProfile = *Cached;
                }
                else
                {
                    const FString PhotometryId = TakeString(gldf_unreal_variant_emitter_string(Handle, vi, ei, GLDF_VARIANT_FIELD_PHOTOMETRY_ID));
                    UTextureLightProfile* Profile = BuildLightProfile(IesPtr, IesLen,
                        PhotometryId.IsEmpty() ? FString::Printf(TEXT("%s_%u_%d"), *Base, vi, ei) : Base + TEXT("_") + PhotometryId);
                    ProfileCache.Add(Key, Profile);
                    P.IesProfile = Profile;
                }
            }

            if (!P.EmitterName.IsEmpty()) { LitEmitterNames.Add(P.EmitterName); }
            V.Emitters.Add(MoveTemp(P));
        }
        Luminaire->Variants.Add(MoveTemp(V));
    }
    gldf_unreal_variants_close(Handle);

    // ── One light per LEO (same pairing rules as the editor factory) ────────
    USceneComponent* Root = Luminaire->GetRootComponent();
    const int32 LeoCount = gldf_unreal_leo_count(PathUtf8.Get());
    TArray<FString> LeoNames;
    for (int32 li = 0; li < LeoCount; ++li)
    {
        LeoNames.Add(TakeString(gldf_unreal_leo_part_name(PathUtf8.Get(), static_cast<uint32>(li))));
    }
    bool bStrictMatch = false;
    for (const FString& Ln : LeoNames)
    {
        if (!Ln.IsEmpty() && LitEmitterNames.Contains(Ln)) { bStrictMatch = true; break; }
    }
    TArray<FString> EmitterNamesV0;
    if (Luminaire->Variants.Num() > 0)
    {
        for (const FGldfEmitterPhotometry& P : Luminaire->Variants[0].Emitters) { EmitterNamesV0.Add(P.EmitterName); }
    }

    // Drop lights a previous population left behind (re-load of the same actor).
    for (FGldfEmitterComponent& Old : Luminaire->Emitters)
    {
        if (Old.Light) { Old.Light->DestroyComponent(); }
    }
    Luminaire->Emitters.Reset();

    int32 Cursor = 0, Built = 0;
    for (int32 li = 0; li < LeoCount; ++li)
    {
        const FString& PartName = LeoNames[li];
        FString KeyName;
        if (bStrictMatch)
        {
            if (!LitEmitterNames.Contains(PartName)) { continue; }
            KeyName = PartName;
        }
        else
        {
            KeyName = EmitterNamesV0.IsValidIndex(Cursor) ? EmitterNamesV0[Cursor]
                : (EmitterNamesV0.Num() > 0 ? EmitterNamesV0.Last() : PartName);
            ++Cursor;
        }

        GldfLeoShape Shape{};
        if (gldf_unreal_leo_at(PathUtf8.Get(), static_cast<uint32>(li), &Shape) != 0) { continue; }

        const FName CompName = MakeUniqueObjectName(Luminaire, ULocalLightComponent::StaticClass(),
            FName(*FString::Printf(TEXT("Emitter_%d_%s"), li, PartName.IsEmpty() ? *FString::FromInt(li) : *PartName)));
        ULocalLightComponent* Light = nullptr;
        if (Shape.kind == GLDF_LEO_RECTANGLE)
        {
            URectLightComponent* Rect = NewObject<URectLightComponent>(Luminaire, CompName);
            Rect->SetSourceWidth(Shape.dim_a * MetersToCm);
            Rect->SetSourceHeight(Shape.dim_b * MetersToCm);
            Light = Rect;
        }
        else
        {
            Light = NewObject<UPointLightComponent>(Luminaire, CompName);
        }
        // Articulated: hang the light under the part that carries this LEO.
        USceneComponent* AttachTo = Root;
        FTransform NodeToActor = FTransform::Identity;
        if (bArticulated && L3d != 0)
        {
            for (uint32 li2 = 0; li2 < L3dLeos; ++li2)
            {
                GldfL3dLeo LeoNode{};
                if (gldf_unreal_l3d_leo(L3d, li2, &LeoNode) == 0 && (int32)li2 == li && NodeComps.IsValidIndex(LeoNode.node) && NodeComps[LeoNode.node])
                {
                    AttachTo = NodeComps[LeoNode.node];
                    NodeToActor = NodeRel[LeoNode.node];
                    break;
                }
            }
        }
        if (AttachTo) { Light->SetupAttachment(AttachTo); }
        Luminaire->AddInstanceComponent(Light);
        Light->RegisterComponent();

        // UE light local +X = emission, +Y = width, +Z = height; L3D LEO emits
        // along -az, width ax, height ay; apply the mesh's Y flip (see factory).
        auto FlipY = [](float x, float y, float z) { return FVector(x, -y, z); };
        const FVector EmitDir = FlipY(-Shape.az_x, -Shape.az_y, -Shape.az_z);
        const FVector WidthDir = FlipY(Shape.ax_x, Shape.ax_y, Shape.ax_z);
        const FVector HeightDir = FlipY(Shape.ay_x, Shape.ay_y, Shape.ay_z);
        const FMatrix RotM(
            FPlane(EmitDir.X, EmitDir.Y, EmitDir.Z, 0.f),
            FPlane(WidthDir.X, WidthDir.Y, WidthDir.Z, 0.f),
            FPlane(HeightDir.X, HeightDir.Y, HeightDir.Z, 0.f),
            FPlane(0.f, 0.f, 0.f, 1.f));
        Light->SetMobility(EComponentMobility::Movable);
        // Pose in actor space (rest pose of all joints) → relative to the node it hangs under.
        const FTransform PoseActor(FQuat(RotM), FVector(Shape.pos_x, -Shape.pos_y, Shape.pos_z));
        const FTransform PoseNode = PoseActor.GetRelativeTransform(NodeToActor);
        Light->SetRelativeLocationAndRotation(PoseNode.GetLocation(), PoseNode.GetRotation());

        FGldfEmitterComponent Entry;
        Entry.EmitterName = KeyName;
        Entry.Light = Light;
        Luminaire->Emitters.Add(MoveTemp(Entry));
        ++Built;
    }

    if (Luminaire->Variants.Num() > 0)
    {
        const FGldfLuminaireVariant& First = Luminaire->Variants[0];
        Luminaire->SelectedVariantName = FName(*(First.VariantName.IsEmpty() ? First.VariantId : First.VariantName));
    }
    // Applies IES / lumens / CCT per emitter, fallback body material, and
    // moves lights that would shine through their own housing.
    Luminaire->ApplySelectedVariant();

    UE_LOG(LogGldfRuntimeLoader, Log, TEXT("%s: %s %s — %d variant(s), %d LEO(s) → %d light(s), %d IES texture(s)"),
        *Luminaire->SourceFile, *Luminaire->Manufacturer, *Luminaire->ProductName,
        Luminaire->Variants.Num(), LeoCount, Built, ProfileCache.Num());

    if (L3d != 0) { gldf_unreal_l3d_close(L3d); }
    if (Built == 0)
    {
        OutError = TEXT("GLDF has no light emitting object");
        return false;
    }
    return true;
}

AGldfLuminaireActor* UGldfRuntimeLoader::SpawnLuminaireFromFile(
    UObject* WorldContextObject, const FString& GldfPath, const FTransform& Transform, FString& OutError)
{
    UWorld* World = GEngine ? GEngine->GetWorldFromContextObject(WorldContextObject, EGetWorldErrorMode::LogAndReturnNull) : nullptr;
    if (!World)
    {
        OutError = TEXT("no world");
        return nullptr;
    }
    FActorSpawnParameters P;
    P.SpawnCollisionHandlingOverride = ESpawnActorCollisionHandlingMethod::AlwaysSpawn;
    AGldfLuminaireActor* Lum = World->SpawnActor<AGldfLuminaireActor>(AGldfLuminaireActor::StaticClass(), Transform, P);
    if (!Lum)
    {
        OutError = TEXT("spawn failed");
        return nullptr;
    }
    if (!PopulateLuminaire(Lum, GldfPath, OutError))
    {
        Lum->Destroy();
        return nullptr;
    }
    return Lum;
}

TArray<FString> UGldfRuntimeLoader::FindGldfFiles(const TArray<FString>& Directories)
{
    TArray<FString> Out;
    for (const FString& Dir : Directories)
    {
        if (Dir.IsEmpty() || !IFileManager::Get().DirectoryExists(*Dir)) { continue; }
        TArray<FString> Found;
        IFileManager::Get().FindFiles(Found, *(Dir / TEXT("*.gldf")), true, false);
        for (const FString& F : Found)
        {
            Out.AddUnique(FPaths::ConvertRelativePathToFull(Dir / F));
        }
    }
    Out.Sort();
    return Out;
}
