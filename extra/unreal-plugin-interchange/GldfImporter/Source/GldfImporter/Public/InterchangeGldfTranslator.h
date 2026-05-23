// Interchange translator for GLDF (Global Lighting Data Format).
//
// Phase 1: emits a UInterchangeTextureLightProfileNode for the first
// non-emergency emitter's IES (→ UTextureLightProfile asset).
// Phase 2: also emits a UInterchangeMeshNode for the luminaire body
// (→ UStaticMesh asset), with mesh data pulled from the Rust
// gldf-unreal staticlib's mesh handle API.
//
// Later phases add the variant-aware Blueprint (Phase 3).

#pragma once

#include "CoreMinimal.h"
#include "UObject/Object.h"
#include "UObject/ObjectMacros.h"

#include "InterchangeTranslatorBase.h"
#include "Nodes/InterchangeBaseNodeContainer.h"
#include "Texture/InterchangeTextureLightProfilePayloadData.h"
#include "Texture/InterchangeTextureLightProfilePayloadInterface.h"
#include "Mesh/InterchangeMeshPayload.h"
#include "Mesh/InterchangeMeshPayloadInterface.h"

// Rust C ABI — for the GldfMeshHeader type used in the cached member
// below. PublicIncludePaths in GldfImporter.Build.cs points at
// crates/gldf-unreal/include/.
extern "C" {
#include "gldf_unreal.h"
}

#include "InterchangeGldfTranslator.generated.h"

UCLASS(MinimalAPI, BlueprintType)
class UInterchangeGldfTranslator
    : public UInterchangeTranslatorBase
    , public IInterchangeTextureLightProfilePayloadInterface
    , public IInterchangeMeshPayloadInterface
{
    GENERATED_BODY()

public:
    /** File extensions this translator registers for. */
    virtual TArray<FString> GetSupportedFormats() const override;

    /** GLDF is a *scene* import (mesh + light + actor in later phases),
     *  not a pure asset import. UE's file-picker dialog uses this to
     *  decide whether to offer our extension in scene-import vs
     *  asset-import contexts. glTF + FBX also return Scenes; the IES
     *  translator defaults to Assets and is correspondingly only
     *  offered from asset-import flows. */
    virtual EInterchangeTranslatorType GetTranslatorType() const override
    {
        return EInterchangeTranslatorType::Scenes;
    }

    /** What kinds of UE assets we currently produce. Phase 1 emits a
     *  UTextureLightProfile; Phase 2 adds a UStaticMesh. The actor side
     *  (Phase 3) isn't an asset type — it's covered via the Scenes
     *  translator type above. */
    virtual EInterchangeTranslatorAssetType GetSupportedAssetTypes() const override
    {
        return EInterchangeTranslatorAssetType::Textures
             | EInterchangeTranslatorAssetType::Meshes;
    }

    /** Walks the GLDF, emits an IES light-profile node + (Phase 2) a
     *  mesh node for the luminaire body. */
    virtual bool Translate(UInterchangeBaseNodeContainer& BaseNodeContainer) const override;

    /** Release any cached Rust mesh handle. Called by UE when the
     *  translator is done with the source. */
    virtual void ReleaseSource() override;

    // IInterchangeTextureLightProfilePayloadInterface ────────────────────

    /** Fetch the raw IES bytes for the node registered in Translate().
     *  Reads the GLDF via the Rust FFI to obtain the variant-resolved
     *  IES. PayloadKey identifies which emitter (Phase 1: always
     *  "first"). */
    virtual TOptional<UE::Interchange::FImportLightProfile>
        GetLightProfilePayloadData(const FString& PayloadKey,
                                   TOptional<FString>& AlternateTexturePath) const override;

    virtual TOptional<UE::Interchange::FImportLightProfile>
        GetLightProfilePayloadData(const uint8* Buffer, uint32 BufferLength) const override;

    // IInterchangeMeshPayloadInterface ───────────────────────────────────

    /** Build the FMeshDescription for the luminaire body. Pulls the
     *  L3D OBJ arrays from the Rust gldf-unreal mesh handle and applies
     *  UE's coordinate basis (PositionToUEBasis / UVToUEBasis) exactly
     *  like UE's own InterchangeOBJTranslator. */
    virtual TOptional<UE::Interchange::FMeshPayloadData>
        GetMeshPayloadData(const FInterchangeMeshPayLoadKey& PayLoadKey,
                           const UE::Interchange::FAttributeStorage& PayloadAttributes) const override;

private:
    /** Lazily-opened Rust mesh handle for the current source file. Opened
     *  on first need (in Translate or GetMeshPayloadData), released in
     *  ReleaseSource(). 0 = not open. `mutable` because the payload
     *  callbacks are const but need to memoize the handle. */
    mutable uint64 MeshHandle = 0;

    /** Header (counts) captured when MeshHandle was opened. Valid only
     *  while MeshHandle != 0. */
    mutable GldfMeshHeader MeshHeader = {};

    /** True once we've tried to open the mesh handle for this source,
     *  whether or not it succeeded — so we don't retry a failing open on
     *  every payload callback. */
    mutable bool bMeshHandleTried = false;

    /** Open (or return the cached) Rust mesh handle for our SourceData's
     *  file, capturing its header into MeshHeader. Returns 0 on failure
     *  (logs the cause once). */
    uint64 EnsureMeshHandle() const;
};
