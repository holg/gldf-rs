// Interchange translator for GLDF (Global Lighting Data Format).
//
// Phase 1 implements ONLY the texture light profile path: walks the GLDF
// via the Rust gldf-unreal staticlib, finds the first non-emergency
// emitter's IES bytes, emits one UInterchangeTextureLightProfileNode.
// UE's built-in UInterchangeTextureFactory materializes the node into a
// UTextureLightProfile asset — no custom factory needed.
//
// Later phases will add mesh + actor + variant support.

#pragma once

#include "CoreMinimal.h"
#include "UObject/Object.h"
#include "UObject/ObjectMacros.h"

#include "InterchangeTranslatorBase.h"
#include "Nodes/InterchangeBaseNodeContainer.h"
#include "Texture/InterchangeTextureLightProfilePayloadData.h"
#include "Texture/InterchangeTextureLightProfilePayloadInterface.h"

#include "InterchangeGldfTranslator.generated.h"

UCLASS(MinimalAPI, BlueprintType)
class UInterchangeGldfTranslator
    : public UInterchangeTranslatorBase
    , public IInterchangeTextureLightProfilePayloadInterface
{
    GENERATED_BODY()

public:
    /** File extensions this translator registers for. */
    virtual TArray<FString> GetSupportedFormats() const override;

    /** GLDF imports produce a Texture (IES profile) in Phase 1.
     *  Later phases add Meshes + Scenes; the bitmask will expand. */
    virtual EInterchangeTranslatorAssetType GetSupportedAssetTypes() const override
    {
        return EInterchangeTranslatorAssetType::Textures;
    }

    /** First-pass translation: walks the GLDF, emits one
     *  UInterchangeTextureLightProfileNode pointing at the first
     *  non-emergency emitter's IES. */
    virtual bool Translate(UInterchangeBaseNodeContainer& BaseNodeContainer) const override;

    // IInterchangeTextureLightProfilePayloadInterface ────────────────────

    /** Called by UInterchangeTextureFactory to fetch the raw IES bytes
     *  for the node we registered in Translate(). Reads the GLDF again
     *  via the Rust gldf-unreal FFI to obtain the variant-resolved IES.
     *
     *  The PayloadKey we set in Translate() identifies which emitter to
     *  fetch (currently always "first"; future phases will key by
     *  variant + emitter index). */
    virtual TOptional<UE::Interchange::FImportLightProfile>
        GetLightProfilePayloadData(const FString& PayloadKey,
                                   TOptional<FString>& AlternateTexturePath) const override;

    virtual TOptional<UE::Interchange::FImportLightProfile>
        GetLightProfilePayloadData(const uint8* Buffer, uint32 BufferLength) const override;
};
