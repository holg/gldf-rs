// GLDF Interchange translator.
//
// Phase 1: emits a UInterchangeTextureLightProfileNode (→ UTextureLightProfile).
// Phase 2: also emits a UInterchangeMeshNode (→ UStaticMesh) for the body,
//          with geometry pulled from the Rust gldf-unreal mesh handle API.
//
// The actual GLDF parsing lives in Rust (crates/gldf-unreal). This file
// is just the C++ shim that turns FFI calls into Interchange nodes +
// payload data.

#include "InterchangeGldfTranslator.h"

#include "IESConverter.h"
#include "Logging/LogMacros.h"
#include "Misc/Paths.h"
#include "Nodes/InterchangeBaseNodeContainer.h"
#include "InterchangeSceneNode.h"
#include "InterchangeTextureLightProfileNode.h"
#include "InterchangeMeshNode.h"
#include "InterchangeLightNode.h"

// Mesh-description building (Phase 2).
#include "MeshDescription.h"
#include "StaticMeshAttributes.h"
#include "StaticMeshOperations.h"
#include "Algo/Reverse.h"

#include UE_INLINE_GENERATED_CPP_BY_NAME(InterchangeGldfTranslator)

extern "C" {
#include "gldf_unreal.h"
}

DEFINE_LOG_CATEGORY_STATIC(LogGldfTranslator, Log, All);

// Payload key shared between Translate() (where we set it on the mesh
// node) and GetMeshPayloadData() (where UE hands it back). Phase 2 ships
// one mesh per import so a constant sentinel is enough; Phase 3 will
// encode variant + part indices.
static const TCHAR* GGldfMeshPayloadKey = TEXT("mesh:first");

#define LOCTEXT_NAMESPACE "InterchangeGldfTranslator"

namespace UE::GldfImporter::Private
{
    // (Phase 1's FetchFirstIesBytes was removed in Phase 3b — IES bytes
    // now come from the variant table, keyed by variant index, in
    // GetLightProfilePayloadData.)

    // L3D source frame (right-handed Z-up, mm) → UE basis. Mirrors
    // InterchangeOBJTranslator.cpp's PositionToUEBasis/UVToUEBasis: negate
    // Y on positions + normals (handedness flip), flip V on UVs. Units
    // (mm→cm) are left to UE's import settings, same as the OBJ path.
    static FVector3f PositionToUEBasis(const FVector3f& V)
    {
        return FVector3f(V.X, -V.Y, V.Z);
    }
    static FVector2f UVToUEBasis(const FVector2f& V)
    {
        return FVector2f(V.X, 1.0f - V.Y);
    }

    /// Build an FMeshDescription from a live Rust mesh handle + its
    /// already-captured header. Returns an empty description on failure.
    ///
    /// Because PositionToUEBasis negates Y (det < 0, orientation flips),
    /// we reverse each polygon's corner order so triangles stay
    /// front-facing in UE.
    static FMeshDescription BuildMeshDescription(uint64 Handle, const GldfMeshHeader& Header)
    {
        FMeshDescription MeshDescription;

        const float* Positions = nullptr;
        const float* Normals = nullptr;
        const float* UVs = nullptr;
        const GldfMeshCorner* Corners = nullptr;
        const GldfMeshPolygon* Polygons = nullptr;

        const int32 BorrowResult = gldf_unreal_mesh_borrow(
            Handle, &Positions, &Normals, &UVs, &Corners, &Polygons);
        if (BorrowResult != 0 || Positions == nullptr || Corners == nullptr || Polygons == nullptr)
        {
            UE_LOG(LogGldfTranslator, Error, TEXT("BuildMeshDescription: borrow failed (%d)"), BorrowResult);
            return MeshDescription;
        }

        FStaticMeshAttributes Attributes(MeshDescription);
        Attributes.Register();

        const bool bHasNormals = (Header.normal_count > 0) && (Normals != nullptr);
        const bool bHasUVs = (Header.uv_count > 0) && (UVs != nullptr);

        // 1. Vertices (positions). One MeshDescription vertex per source
        //    OBJ position; corners reference these by index.
        TVertexAttributesRef<FVector3f> VertexPositions = Attributes.GetVertexPositions();
        MeshDescription.ReserveNewVertices(Header.vertex_count);
        TArray<FVertexID> VertexIDs;
        VertexIDs.Reserve(Header.vertex_count);
        for (uint32 i = 0; i < Header.vertex_count; ++i)
        {
            const FVertexID VID = MeshDescription.CreateVertex();
            VertexIDs.Add(VID);
            const FVector3f Pos(
                Positions[i * 3 + 0],
                Positions[i * 3 + 1],
                Positions[i * 3 + 2]);
            VertexPositions[VID] = PositionToUEBasis(Pos);
        }

        // 2. One UV channel always (UE StaticMesh wants >=1). If the
        //    source has none, we leave the per-instance UVs at (0,0).
        MeshDescription.SetNumUVChannels(1);

        // 3. One polygon group (single material slot in v0).
        const FPolygonGroupID PolyGroup = MeshDescription.CreatePolygonGroup();
        Attributes.GetPolygonGroupMaterialSlotNames()[PolyGroup] = FName(TEXT("GLDF_Body"));

        TVertexInstanceAttributesRef<FVector3f> InstanceNormals = Attributes.GetVertexInstanceNormals();
        TVertexInstanceAttributesRef<FVector2f> InstanceUVs = Attributes.GetVertexInstanceUVs();
        // Guard: ensure the per-instance UV ref has a channel even if
        // SetNumUVChannels above didn't propagate (matches the belt-and-
        // braces pattern in InterchangeOBJTranslator.cpp:478).
        if (InstanceUVs.GetNumChannels() == 0)
        {
            InstanceUVs.SetNumChannels(1);
        }

        MeshDescription.ReserveNewPolygons(Header.polygon_count);
        MeshDescription.ReserveNewVertexInstances(Header.corner_count);

        for (uint32 p = 0; p < Header.polygon_count; ++p)
        {
            const GldfMeshPolygon& Poly = Polygons[p];
            if (Poly.corner_count < 3)
            {
                continue;
            }

            // Build vertex instances, then reverse for the Y-flip winding fix.
            TArray<FVertexInstanceID, TInlineAllocator<8>> InstanceIDs;
            InstanceIDs.Reserve(Poly.corner_count);

            for (uint32 c = 0; c < Poly.corner_count; ++c)
            {
                const GldfMeshCorner& Corner = Corners[Poly.corner_offset + c];
                if (Corner.position_idx >= (uint32)VertexIDs.Num())
                {
                    continue;
                }
                const FVertexInstanceID IID =
                    MeshDescription.CreateVertexInstance(VertexIDs[Corner.position_idx]);

                if (bHasNormals && Corner.normal_idx >= 0
                    && (uint32)Corner.normal_idx < Header.normal_count)
                {
                    const int32 n = Corner.normal_idx;
                    const FVector3f Nrm(
                        Normals[n * 3 + 0],
                        Normals[n * 3 + 1],
                        Normals[n * 3 + 2]);
                    InstanceNormals[IID] = PositionToUEBasis(Nrm);
                }

                if (bHasUVs && Corner.uv_idx >= 0
                    && (uint32)Corner.uv_idx < Header.uv_count)
                {
                    const int32 u = Corner.uv_idx;
                    InstanceUVs.Set(IID, 0, UVToUEBasis(FVector2f(UVs[u * 2 + 0], UVs[u * 2 + 1])));
                }
                else
                {
                    InstanceUVs.Set(IID, 0, FVector2f(0.0f, 0.0f));
                }

                InstanceIDs.Add(IID);
            }

            if (InstanceIDs.Num() < 3)
            {
                continue;
            }
            // Reverse winding to compensate for the Y-negation.
            Algo::Reverse(InstanceIDs);
            MeshDescription.CreatePolygon(PolyGroup, InstanceIDs);
        }

        return MeshDescription;
    }
}

// ─── UInterchangeGldfTranslator ────────────────────────────────────────────

TArray<FString> UInterchangeGldfTranslator::GetSupportedFormats() const
{
    return TArray<FString>{ TEXT("gldf;GLDF Global Lighting Data Format") };
}

bool UInterchangeGldfTranslator::Translate(UInterchangeBaseNodeContainer& BaseNodeContainer) const
{
    const FString SourcePath = GetSourceData() ? GetSourceData()->GetFilename() : FString();
    if (SourcePath.IsEmpty())
    {
        UE_LOG(LogGldfTranslator, Error, TEXT("Translate: no source path"));
        return false;
    }

    const FString DisplayName = FPaths::GetBaseFilename(SourcePath);

    // ── Resolve all variants (Phase 3a FFI) ──────────────────────────────
    const uint64 VarHandle = EnsureVariantTable();
    if (VarHandle == 0 || Variants.Num() == 0)
    {
        UE_LOG(LogGldfTranslator, Warning,
            TEXT("Translate: no variants resolved from %s; nothing to import"),
            *SourcePath);
        return false;
    }

    // ── Emit one IES light-profile node per DISTINCT photometry ──────────
    // Variants that share a photometry id share one texture node. Each
    // node's payload key is the Rust variant index whose IES bytes we'll
    // serve in GetLightProfilePayloadData.
    TMap<FString, FString> PhotometryToIesUid; // photometry id → node uid
    for (FGldfTranslatorVariant& V : Variants)
    {
        if (const FString* Existing = PhotometryToIesUid.Find(V.PhotometryId))
        {
            V.IesNodeUid = *Existing;
            continue;
        }

        UInterchangeTextureLightProfileNode* IesNode =
            NewObject<UInterchangeTextureLightProfileNode>(&BaseNodeContainer);
        const FString IesUid =
            FString::Printf(TEXT("\\LightIES\\Gldf\\%s\\%s"), *DisplayName, *V.PhotometryId);
        const FString IesLabel = FString::Printf(TEXT("%s_%s"), *DisplayName, *V.PhotometryId);
        BaseNodeContainer.SetupNode(
            IesNode, IesUid, IesLabel, EInterchangeNodeContainerType::TranslatedAsset);
        // Payload key = the Rust variant index (as text). Our
        // GetLightProfilePayloadData parses it and fetches that variant's
        // IES bytes via gldf_unreal_variant_ies.
        IesNode->SetPayLoadKey(FString::FromInt(V.RustIndex));

        PhotometryToIesUid.Add(V.PhotometryId, IesUid);
        V.IesNodeUid = IesUid;

        UE_LOG(LogGldfTranslator, Log,
            TEXT("Translate: emitted IES node %s (photometry=%s, variant idx=%d)"),
            *IesUid, *V.PhotometryId, V.RustIndex);
    }

    // ── Emit the luminaire mesh node (Phase 2) ───────────────────────────
    FString MeshNodeUid;
    const uint64 MeshH = EnsureMeshHandle();
    if (MeshH != 0 && MeshHeader.vertex_count > 0 && MeshHeader.polygon_count > 0)
    {
        UInterchangeMeshNode* MeshNode =
            NewObject<UInterchangeMeshNode>(&BaseNodeContainer);
        MeshNodeUid = FString::Printf(TEXT("\\Mesh\\Gldf\\%s\\body"), *DisplayName);
        BaseNodeContainer.SetupNode(
            MeshNode, MeshNodeUid, DisplayName, EInterchangeNodeContainerType::TranslatedAsset);

        MeshNode->SetPayLoadKey(GGldfMeshPayloadKey, EInterchangeMeshPayLoadType::STATIC);
        MeshNode->SetCustomVertexCount(static_cast<int32>(MeshHeader.vertex_count));
        MeshNode->SetCustomPolygonCount(static_cast<int32>(MeshHeader.polygon_count));
        MeshNode->SetCustomHasVertexNormal(MeshHeader.normal_count > 0);
        MeshNode->SetCustomHasVertexBinormal(false);
        MeshNode->SetCustomHasVertexTangent(false);
        MeshNode->SetCustomHasSmoothGroup(false);
        MeshNode->SetCustomHasVertexColor(false);

        UE_LOG(LogGldfTranslator, Log,
            TEXT("Translate: emitted mesh node %s (%u verts, %u polys, normals=%d)"),
            *MeshNodeUid, MeshHeader.vertex_count, MeshHeader.polygon_count,
            MeshHeader.normal_count > 0 ? 1 : 0);
    }

    // ── Emit a light asset node for the DEFAULT variant (index 0) ────────
    // The L3D LEO shape decides the UE light type: a rectangular emitter
    // (linear strip / panel) → RectLight sized to the strip; a round or
    // point emitter → PointLight. Phase 3c's component hot-swaps which
    // variant the light shows; here we wire the default so the import
    // "shines" immediately.
    //
    // LEO dimensions are in METRES (unlike OBJ mm); ×100 → UE cm.
    const FGldfTranslatorVariant& Default = Variants[0];

    GldfLeoShape Shape{};
    {
        const FTCHARToUTF8 PathUtf8(*SourcePath);
        gldf_unreal_first_leo_shape(PathUtf8.Get(), &Shape);
    }
    constexpr float MetersToCm = 100.0f;

    const FString LightNodeUid =
        FString::Printf(TEXT("\\Light\\Gldf\\%s\\emitter"), *DisplayName);

    // Common light setters live on UInterchangeLightNode; keep a base
    // pointer regardless of the concrete type we create.
    UInterchangeLightNode* LightNode = nullptr;
    const TCHAR* LightTypeName = TEXT("Point");

    if (Shape.kind == GLDF_LEO_RECTANGLE)
    {
        UInterchangeRectLightNode* RectNode =
            NewObject<UInterchangeRectLightNode>(&BaseNodeContainer);
        BaseNodeContainer.SetupNode(
            RectNode, LightNodeUid, DisplayName,
            EInterchangeNodeContainerType::TranslatedAsset);
        RectNode->SetCustomSourceWidth(Shape.dim_a * MetersToCm);
        RectNode->SetCustomSourceHeight(Shape.dim_b * MetersToCm);
        LightNode = RectNode;
        LightTypeName = TEXT("Rect");
    }
    else
    {
        // Circle or no shape → point light. (A spot would need an aim
        // direction we don't yet derive; a point emits the IES
        // distribution from the emitter location, which is closer to a
        // bare luminaire's behaviour.)
        UInterchangePointLightNode* PointNode =
            NewObject<UInterchangePointLightNode>(&BaseNodeContainer);
        BaseNodeContainer.SetupNode(
            PointNode, LightNodeUid, DisplayName,
            EInterchangeNodeContainerType::TranslatedAsset);
        LightNode = PointNode;
        LightTypeName = (Shape.kind == GLDF_LEO_CIRCLE) ? TEXT("Point(circle)") : TEXT("Point");
    }

    LightNode->SetCustomIESTexture(Default.IesNodeUid);
    LightNode->SetCustomUseIESBrightness(true);
    if (Default.bHasLumens)
    {
        LightNode->SetCustomIntensity(static_cast<float>(Default.Lumens));
        LightNode->SetCustomIntensityUnits(EInterchangeLightUnits::Lumens);
    }
    if (Default.bHasCct)
    {
        LightNode->SetCustomUseTemperature(true);
        LightNode->SetCustomTemperature(static_cast<float>(Default.Cct));
    }

    UE_LOG(LogGldfTranslator, Log,
        TEXT("Translate: emitted %s light %s (default '%s', %d lm, %dK, LEO kind=%d %.3fx%.3f m, IES=%s)"),
        LightTypeName, *LightNodeUid, *Default.VariantId,
        Default.bHasLumens ? Default.Lumens : -1,
        Default.bHasCct ? Default.Cct : -1,
        Shape.kind, Shape.dim_a, Shape.dim_b,
        *Default.IesNodeUid);

    // ── Scene tree: root actor → mesh actor + light actor ────────────────
    // Pattern from InterchangeOBJTranslator.cpp:1270 (root + per-mesh
    // actor scene nodes) and the glTF light wiring.
    UInterchangeSceneNode* RootNode =
        NewObject<UInterchangeSceneNode>(&BaseNodeContainer);
    const FString RootUid = FString::Printf(TEXT("\\Scene\\Gldf\\%s\\root"), *DisplayName);
    BaseNodeContainer.SetupNode(
        RootNode, RootUid, DisplayName, EInterchangeNodeContainerType::TranslatedScene);
    RootNode->SetCustomLocalTransform(&BaseNodeContainer, FTransform::Identity);

    if (!MeshNodeUid.IsEmpty())
    {
        UInterchangeSceneNode* MeshActorNode =
            NewObject<UInterchangeSceneNode>(&BaseNodeContainer);
        const FString MeshActorUid =
            FString::Printf(TEXT("\\Scene\\Gldf\\%s\\body"), *DisplayName);
        BaseNodeContainer.SetupNode(
            MeshActorNode, MeshActorUid, DisplayName,
            EInterchangeNodeContainerType::TranslatedScene, RootUid);
        MeshActorNode->SetCustomLocalTransform(&BaseNodeContainer, FTransform::Identity);
        MeshActorNode->SetCustomAssetInstanceUid(MeshNodeUid);
    }

    UInterchangeSceneNode* LightActorNode =
        NewObject<UInterchangeSceneNode>(&BaseNodeContainer);
    const FString LightActorUid =
        FString::Printf(TEXT("\\Scene\\Gldf\\%s\\light"), *DisplayName);
    BaseNodeContainer.SetupNode(
        LightActorNode, LightActorUid, DisplayName,
        EInterchangeNodeContainerType::TranslatedScene, RootUid);
    LightActorNode->SetCustomLocalTransform(&BaseNodeContainer, FTransform::Identity);
    LightActorNode->SetCustomAssetInstanceUid(LightNodeUid);

    // ── Diagnostic: dump what we put in the container ────────────────────
    // Confirms (in the editor log) that our scene nodes exist at hand-off.
    // If actors don't spawn despite this, the level/scenes pipeline isn't
    // in the active import stack — a dialog/config issue, not our wiring.
    {
        int32 SceneCount = 0, AssetCount = 0, OtherCount = 0;
        BaseNodeContainer.IterateNodes(
            [&](const FString& Uid, UInterchangeBaseNode* Node)
            {
                const EInterchangeNodeContainerType T = Node->GetNodeContainerType();
                if (T == EInterchangeNodeContainerType::TranslatedScene) { ++SceneCount; }
                else if (T == EInterchangeNodeContainerType::TranslatedAsset) { ++AssetCount; }
                else { ++OtherCount; }
                UE_LOG(LogGldfTranslator, Log,
                    TEXT("  node[%d] uid=%s class=%s"),
                    (int32)T, *Uid, *Node->GetClass()->GetName());
            });
        UE_LOG(LogGldfTranslator, Log,
            TEXT("Translate: container holds %d scene + %d asset + %d other nodes"),
            SceneCount, AssetCount, OtherCount);
    }

    return true;
}

uint64 UInterchangeGldfTranslator::EnsureVariantTable() const
{
    if (VariantHandle != 0)
    {
        return VariantHandle;
    }
    if (bVariantHandleTried)
    {
        return 0;
    }
    bVariantHandleTried = true;

    const FString SourcePath = GetSourceData() ? GetSourceData()->GetFilename() : FString();
    if (SourcePath.IsEmpty())
    {
        return 0;
    }

    const FTCHARToUTF8 PathUtf8(*SourcePath);
    uint32 Count = 0;
    char* RawErr = nullptr;
    const uint64 Handle = gldf_unreal_variants_open(PathUtf8.Get(), &Count, &RawErr);
    if (Handle == 0)
    {
        const FString Msg = RawErr ? FString(UTF8_TO_TCHAR(RawErr)) : TEXT("(no message)");
        UE_LOG(LogGldfTranslator, Warning,
            TEXT("EnsureVariantTable: gldf_unreal_variants_open failed: %s"), *Msg);
        if (RawErr) { gldf_unreal_string_free(RawErr); }
        return 0;
    }

    Variants.Reset();
    for (uint32 i = 0; i < Count; ++i)
    {
        GldfVariantInfo Info{};
        if (gldf_unreal_variant_info(Handle, i, &Info) != 0)
        {
            continue;
        }
        FGldfTranslatorVariant V;
        V.RustIndex = static_cast<int32>(i);
        V.Lumens = Info.lumens;
        V.bHasLumens = Info.lumens_present != 0;
        V.Cct = Info.cct;
        V.bHasCct = Info.cct_present != 0;

        if (char* IdRaw = gldf_unreal_variant_string(Handle, i, GLDF_VARIANT_FIELD_ID))
        {
            V.VariantId = FString(UTF8_TO_TCHAR(IdRaw));
            gldf_unreal_string_free(IdRaw);
        }
        if (char* PidRaw = gldf_unreal_variant_string(Handle, i, GLDF_VARIANT_FIELD_PHOTOMETRY_ID))
        {
            V.PhotometryId = FString(UTF8_TO_TCHAR(PidRaw));
            gldf_unreal_string_free(PidRaw);
        }
        Variants.Add(MoveTemp(V));
    }

    VariantHandle = Handle;
    return VariantHandle;
}

uint64 UInterchangeGldfTranslator::EnsureMeshHandle() const
{
    if (MeshHandle != 0)
    {
        return MeshHandle;
    }
    if (bMeshHandleTried)
    {
        return 0; // already failed once; don't spam retries
    }
    bMeshHandleTried = true;

    const FString SourcePath = GetSourceData() ? GetSourceData()->GetFilename() : FString();
    if (SourcePath.IsEmpty())
    {
        return 0;
    }

    const FTCHARToUTF8 PathUtf8(*SourcePath);
    GldfMeshHeader Header{};
    char* RawErr = nullptr;

    const uint64 Handle = gldf_unreal_first_mesh_open(PathUtf8.Get(), &Header, &RawErr);
    if (Handle == 0)
    {
        const FString Msg = RawErr ? FString(UTF8_TO_TCHAR(RawErr)) : TEXT("(no message)");
        UE_LOG(LogGldfTranslator, Warning,
            TEXT("EnsureMeshHandle: gldf_unreal_first_mesh_open failed: %s"), *Msg);
        if (RawErr) { gldf_unreal_string_free(RawErr); }
        return 0;
    }

    MeshHandle = Handle;
    MeshHeader = Header;
    return MeshHandle;
}

void UInterchangeGldfTranslator::ReleaseSource()
{
    if (MeshHandle != 0)
    {
        gldf_unreal_mesh_close(MeshHandle);
        MeshHandle = 0;
    }
    MeshHeader = GldfMeshHeader{};
    bMeshHandleTried = false;

    if (VariantHandle != 0)
    {
        gldf_unreal_variants_close(VariantHandle);
        VariantHandle = 0;
    }
    bVariantHandleTried = false;
    Variants.Reset();

    Super::ReleaseSource();
}

TOptional<UE::Interchange::FMeshPayloadData>
UInterchangeGldfTranslator::GetMeshPayloadData(
    const FInterchangeMeshPayLoadKey& /*PayLoadKey*/,
    const UE::Interchange::FAttributeStorage& /*PayloadAttributes*/) const
{
    const uint64 Handle = EnsureMeshHandle();
    if (Handle == 0)
    {
        UE_LOG(LogGldfTranslator, Error, TEXT("GetMeshPayloadData: no mesh handle"));
        return {};
    }

    UE::Interchange::FMeshPayloadData Payload;
    Payload.MeshDescription =
        UE::GldfImporter::Private::BuildMeshDescription(Handle, MeshHeader);

    if (Payload.MeshDescription.Vertices().Num() == 0)
    {
        UE_LOG(LogGldfTranslator, Error, TEXT("GetMeshPayloadData: empty mesh description"));
        return {};
    }

    // Recompute normals/tangents that the source didn't provide, and
    // catch NaNs. ComputeTangentsAndNormals is a no-op for data we
    // already filled.
    FStaticMeshOperations::ValidateAndFixData(Payload.MeshDescription, TEXT("GLDF"));

    return Payload;
}

TOptional<UE::Interchange::FImportLightProfile>
UInterchangeGldfTranslator::GetLightProfilePayloadData(const FString& PayloadKey,
                                                       TOptional<FString>& /*AlternateTexturePath*/) const
{
    // PayloadKey is the Rust variant index (set in Translate when emitting
    // the IES texture node). Fetch that variant's IES bytes from the
    // variant-table handle and feed them through FIESConverter.
    const uint64 Handle = EnsureVariantTable();
    if (Handle == 0)
    {
        UE_LOG(LogGldfTranslator, Error, TEXT("GetLightProfilePayloadData: no variant table"));
        return {};
    }

    int32 VariantIdx = 0;
    if (!PayloadKey.IsEmpty())
    {
        VariantIdx = FCString::Atoi(*PayloadKey);
    }

    const uint8* IesPtr = nullptr;
    uint32 IesLen = 0;
    if (gldf_unreal_variant_ies(Handle, static_cast<uint32>(VariantIdx), &IesPtr, &IesLen) != 0
        || IesPtr == nullptr || IesLen == 0)
    {
        UE_LOG(LogGldfTranslator, Error,
            TEXT("GetLightProfilePayloadData: no IES for variant idx %d"), VariantIdx);
        return {};
    }
    return GetLightProfilePayloadData(IesPtr, IesLen);
}

TOptional<UE::Interchange::FImportLightProfile>
UInterchangeGldfTranslator::GetLightProfilePayloadData(const uint8* Buffer, uint32 BufferLength) const
{
    // Same pattern as InterchangeIESTranslator::GetLightProfilePayloadData
    // (5.7: Engine/Plugins/Interchange/Runtime/Source/Import/Private/Texture/InterchangeIESTranslator.cpp).
    UE::Interchange::FImportLightProfile Payload;

    Payload.SourceDataBuffer.SetNum(BufferLength);
    FPlatformMemory::Memcpy(Payload.SourceDataBuffer.GetData(), Buffer, BufferLength);

    FIESConverter IESConverter(Payload.SourceDataBuffer.GetData(), Payload.SourceDataBuffer.Num());
    if (!IESConverter.IsValid())
    {
        UE_LOG(LogGldfTranslator, Error,
            TEXT("IES conversion failed: %s"),
            *FString(IESConverter.GetError()));
        return {};
    }

    Payload.Init2DWithParams(
        IESConverter.GetWidth(),
        IESConverter.GetHeight(),
        TSF_RGBA16F,
        false);
    Payload.CompressionSettings = TC_HDR;
    Payload.Brightness = IESConverter.GetBrightness();
    Payload.TextureMultiplier = IESConverter.GetMultiplier();

    const TArray<uint8>& RawData = IESConverter.GetRawData();
    FPlatformMemory::Memcpy(Payload.RawData.GetData(), RawData.GetData(), Payload.RawData.GetSize());

    return Payload;
}

#undef LOCTEXT_NAMESPACE
