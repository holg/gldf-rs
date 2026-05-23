// Phase 1 GLDF Interchange translator: emits one
// UInterchangeTextureLightProfileNode for the first non-emergency
// emitter of the first variant. The IES bytes themselves are fetched
// lazily by UE via GetLightProfilePayloadData().
//
// The actual GLDF parsing lives in Rust (crates/gldf-unreal). This
// file is just the C++ shim that turns FFI calls into Interchange
// nodes + payload data.

#include "InterchangeGldfTranslator.h"

#include "IESConverter.h"
#include "Logging/LogMacros.h"
#include "Misc/Paths.h"
#include "Nodes/InterchangeBaseNodeContainer.h"
#include "InterchangeTextureLightProfileNode.h"

#include UE_INLINE_GENERATED_CPP_BY_NAME(InterchangeGldfTranslator)

extern "C" {
#include "gldf_unreal.h"
}

DEFINE_LOG_CATEGORY_STATIC(LogGldfTranslator, Log, All);

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

    return true;
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
