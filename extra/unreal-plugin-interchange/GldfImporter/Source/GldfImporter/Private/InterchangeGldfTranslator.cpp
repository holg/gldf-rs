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
#include "InterchangeTextureLightProfileNode.h"
#include "InterchangeMeshNode.h"

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
    /** Pull the IES bytes for the first non-emergency emitter from the
     *  GLDF at SourcePath. Returns an empty TArray on any failure
     *  (logs the cause). */
    static TArray<uint8> FetchFirstIesBytes(const FString& SourcePath)
    {
        TArray<uint8> Result;

        const FTCHARToUTF8 PathUtf8(*SourcePath);
        uint8* RawBuf = nullptr;
        uintptr_t RawLen = 0;
        char* RawErr = nullptr;

        const int32 Code = gldf_unreal_first_ies_bytes(
            PathUtf8.Get(),
            &RawBuf,
            &RawLen,
            &RawErr);

        if (Code != 0)
        {
            const FString Msg = RawErr ? FString(UTF8_TO_TCHAR(RawErr)) : TEXT("(no message)");
            UE_LOG(LogGldfTranslator, Error,
                TEXT("gldf_unreal_first_ies_bytes failed (code %d): %s"),
                Code, *Msg);
            if (RawErr) { gldf_unreal_string_free(RawErr); }
            return Result;
        }

        if (RawBuf && RawLen > 0)
        {
            Result.SetNumUninitialized(static_cast<int32>(RawLen));
            FMemory::Memcpy(Result.GetData(), RawBuf, RawLen);
        }
        if (RawBuf) { gldf_unreal_bytes_free(RawBuf, RawLen); }
        return Result;
    }

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

    // Sanity-check that Rust can at least find an emitter. We don't keep
    // the bytes here — the actual payload is fetched lazily by UE via
    // GetLightProfilePayloadData() during factory materialization, which
    // calls FetchFirstIesBytes again. Two reads per import is fine for
    // Phase 1; Phase 2 will cache via a translator-side member.
    const TArray<uint8> Bytes = UE::GldfImporter::Private::FetchFirstIesBytes(SourcePath);
    if (Bytes.Num() == 0)
    {
        UE_LOG(LogGldfTranslator, Warning,
            TEXT("Translate: no IES bytes resolved from %s; skipping"), *SourcePath);
        return false;
    }

    // Emit one light-profile node. Pattern lifted from
    // Engine/Plugins/Interchange/Runtime/Source/Import/Private/Gltf/InterchangeGltfTranslator.cpp
    // (around line 692 in 5.7).
    UInterchangeTextureLightProfileNode* IesNode =
        NewObject<UInterchangeTextureLightProfileNode>(&BaseNodeContainer);

    const FString DisplayName = FPaths::GetBaseFilename(SourcePath);
    const FString NodeUid = FString::Printf(TEXT("\\LightIES\\Gldf\\%s\\first"), *DisplayName);

    BaseNodeContainer.SetupNode(
        IesNode,
        NodeUid,
        DisplayName,
        EInterchangeNodeContainerType::TranslatedAsset);

    // The payload key tells our GetLightProfilePayloadData callback
    // which emitter to fetch. In Phase 1 there's only one ("first"),
    // so the key is just a sentinel — Phase 2+ will encode variant
    // and emitter indices here.
    IesNode->SetPayLoadKey(TEXT("first"));

    UE_LOG(LogGldfTranslator, Log,
        TEXT("Translate: emitted IES node %s (%d bytes available)"),
        *NodeUid, Bytes.Num());

    // ── Phase 2: emit the luminaire mesh node ────────────────────────────
    const uint64 Handle = EnsureMeshHandle();
    if (Handle != 0 && MeshHeader.vertex_count > 0 && MeshHeader.polygon_count > 0)
    {
        UInterchangeMeshNode* MeshNode =
            NewObject<UInterchangeMeshNode>(&BaseNodeContainer);

        const FString MeshNodeUid =
            FString::Printf(TEXT("\\Mesh\\Gldf\\%s\\body"), *DisplayName);

        BaseNodeContainer.SetupNode(
            MeshNode,
            MeshNodeUid,
            DisplayName,
            EInterchangeNodeContainerType::TranslatedAsset);

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
    else
    {
        UE_LOG(LogGldfTranslator, Warning,
            TEXT("Translate: no mesh emitted (handle=%llu, verts=%u, polys=%u)"),
            Handle, MeshHeader.vertex_count, MeshHeader.polygon_count);
    }

    return true;
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
UInterchangeGldfTranslator::GetLightProfilePayloadData(const FString& /*PayloadKey*/,
                                                       TOptional<FString>& /*AlternateTexturePath*/) const
{
    const FString SourcePath = GetSourceData() ? GetSourceData()->GetFilename() : FString();
    if (SourcePath.IsEmpty())
    {
        return {};
    }

    const TArray<uint8> Bytes = UE::GldfImporter::Private::FetchFirstIesBytes(SourcePath);
    if (Bytes.Num() == 0)
    {
        return {};
    }
    return GetLightProfilePayloadData(Bytes.GetData(), static_cast<uint32>(Bytes.Num()));
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
