// Blueprint-callable demo flow: search gldf-search.de, then download + place a
// chosen result as a GLDF luminaire.
//
// The query side calls the gldf-search-cabi Rust staticlib
// (gldf_engine_from_remote / gldf_engine_search / gldf_engine_download_url).
// The download is UE-native (FHttpModule). The import reuses the GldfImporter
// Interchange plugin (UInterchangeManager::ImportScene), which spawns one
// AGldfLuminaireActor — the same actor a drag-import produces.
//
// Editor-only: Interchange import is editor-only, so this whole flow is too.

#pragma once

#include "CoreMinimal.h"
#include "Kismet/BlueprintFunctionLibrary.h"
#include "GldfSearchLibrary.generated.h"

/** One search hit, flattened for Blueprint/UMG display. Carries the keys
 *  needed to resolve the download URL: the parent doc id (hex) + variant. */
USTRUCT(BlueprintType)
struct FGldfSearchHit
{
    GENERATED_BODY()

    /** Parent doc id (lowercase 64-char hex). */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search")
    FString DocId;

    /** Variant index within the parent doc (selects the right source file). */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search")
    int32 Variant = 0;

    /** Manufacturer display name (resolved via a doc lookup). */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search")
    FString Manufacturer;

    /** Product / family name (resolved via a doc lookup). */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search")
    FString Product;

    /** Variant display name. */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search")
    FString VariantName;
};

/** Full search filter, mirroring the cabi `GldfFilter` (and the WASM
 *  `Filter`). Enum-id list fields hold the canonical ids the facet panel
 *  collects; the three tristates use -1 = unset, 0 = false, 1 = true (maps
 *  to cabi GldfTriState). A range with Min > Max (or both 0) is treated as
 *  unset. Empty arrays / empty text = "don't filter on that field". */
USTRUCT(BlueprintType)
struct FGldfSearchFilter
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") FString Text;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") FString Locale;

    /** Manufacturer display names (string-list filter, not ids). */
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") TArray<FString> Manufacturer;

    // Enum-id lists (canonical ids; resolve labels via the enum tables).
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") TArray<int32> Applications;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") TArray<int32> Labels;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") TArray<int32> Adjustability;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") TArray<int32> IpCode;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") TArray<int32> SafetyClass;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") TArray<int32> IkRating;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") TArray<int32> ProductForm;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") TArray<int32> LightDistribution;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") TArray<int32> LampType;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") TArray<int32> ControlGearInterfaces;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") TArray<int32> EmergencyLightingType;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") TArray<int32> MountingPlace;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") TArray<int32> MountingType;

    // Numeric ranges. bHas* gates each; for ranges only one end may be set.
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") bool bHasFluxMin = false;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") float FluxMin = 0.f;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") bool bHasFluxMax = false;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") float FluxMax = 0.f;

    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") bool bHasPowerMin = false;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") float PowerMin = 0.f;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") bool bHasPowerMax = false;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") float PowerMax = 0.f;

    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") bool bHasEfficacyMin = false;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") float EfficacyMin = 0.f;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") bool bHasEfficacyMax = false;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") float EfficacyMax = 0.f;

    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") bool bHasBeamMin = false;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") float BeamMin = 0.f;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") bool bHasBeamMax = false;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") float BeamMax = 0.f;

    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") bool bHasCctMin = false;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") int32 CctMin = 0;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") bool bHasCctMax = false;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") int32 CctMax = 0;

    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") bool bHasDepthMin = false;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") int32 DepthMin = 0;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") bool bHasDepthMax = false;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") int32 DepthMax = 0;

    /** Minimum CRI Ra; -1 = no minimum. */
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") int32 CriMin = -1;

    /** Tristates: -1 unset, 0 false ("missing"), 1 true ("present"). */
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") int32 HasPhotometry = -1;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") int32 Has3d = -1;
    UPROPERTY(BlueprintReadWrite, Category = "GLDF Filter") int32 HasEmergencyLighting = -1;
};

/** One facet value + its result count (the cabi GldfFacetValue). */
USTRUCT(BlueprintType)
struct FGldfFacetValue
{
    GENERATED_BODY()
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") FString Value;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") int32 Count = 0;
    /** The canonical id this value maps to (via gldf_facet_value_from_str),
     *  so the UI can toggle it into the matching filter list. -1 if the
     *  field isn't id-keyed (e.g. manufacturer) or didn't resolve. */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") int32 Id = -1;
};

/** One facet field + its values (the cabi GldfFacet). */
USTRUCT(BlueprintType)
struct FGldfFacet
{
    GENERATED_BODY()
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") FString Field;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") TArray<FGldfFacetValue> Values;
};

/** One numeric axis' reachable bounds; bHasMin/Max false = axis disabled. */
USTRUCT(BlueprintType)
struct FGldfRange
{
    GENERATED_BODY()
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") bool bHasMin = false;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") float Min = 0.f;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") bool bHasMax = false;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") float Max = 0.f;
};

/** Per-axis reachable bounds (the cabi GldfRangeBounds), for slider rails. */
USTRUCT(BlueprintType)
struct FGldfRangeBounds
{
    GENERATED_BODY()
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") FGldfRange Flux;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") FGldfRange Power;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") FGldfRange Efficacy;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") FGldfRange Cct;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") FGldfRange Beam;
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") FGldfRange RecessedDepth;
    /** False when the engine couldn't compute bounds (e.g. remote host
     *  without the /api/range_bounds endpoint — returns a 405). The UI
     *  should then disable the range sliders. */
    UPROPERTY(BlueprintReadOnly, Category = "GLDF Search") bool bValid = false;
};

UCLASS()
class UGldfSearchLibrary : public UBlueprintFunctionLibrary
{
    GENERATED_BODY()

public:
    /** Free-text search against a gldf-search deployment (default
     *  https://gldf-search.de). Returns up to `Limit` hits, each enriched
     *  with manufacturer/product/variant names for display. Blocking — runs
     *  the cabi query (which does the HTTP to the search API) inline; fine for
     *  an editor button. Logs + returns empty on failure. */
    UFUNCTION(BlueprintCallable, Category = "GLDF Search",
              meta = (AdvancedDisplay = "BaseUrl,Limit"))
    static TArray<FGldfSearchHit> Search(
        const FString& Query,
        const FString& BaseUrl = TEXT("https://gldf-search.de"),
        int32 Limit = 20);

    /** Full-filter search (text + facets + ranges + tristates). Same
     *  enrichment + semantics as Search(); the WASM-parity panel uses this. */
    UFUNCTION(BlueprintCallable, Category = "GLDF Search",
              meta = (AdvancedDisplay = "BaseUrl,Offset,Limit"))
    static TArray<FGldfSearchHit> SearchFiltered(
        const FGldfSearchFilter& Filter,
        const FString& BaseUrl = TEXT("https://gldf-search.de"),
        int32 Offset = 0,
        int32 Limit = 50);

    /** Facet counts for the current filter (all fields). Drives the
     *  checklists' value/count rows; each value's Id is resolved back via
     *  the enum tables so a click can toggle it into the filter. */
    UFUNCTION(BlueprintCallable, Category = "GLDF Search",
              meta = (AdvancedDisplay = "BaseUrl"))
    static TArray<FGldfFacet> Facets(
        const FGldfSearchFilter& Filter,
        const FString& BaseUrl = TEXT("https://gldf-search.de"));

    /** Reachable numeric bounds for the current filter (slider rails).
     *  bValid=false when the engine can't compute them (remote 405). */
    UFUNCTION(BlueprintCallable, Category = "GLDF Search",
              meta = (AdvancedDisplay = "BaseUrl"))
    static FGldfRangeBounds RangeBounds(
        const FGldfSearchFilter& Filter,
        const FString& BaseUrl = TEXT("https://gldf-search.de"));

    /** Resolve a hit's downloadable .gldf URL (cabi), fetch it via UE HTTP,
     *  write a temp .gldf, and import it through Interchange — placing one
     *  AGldfLuminaireActor in the current editor world. Asynchronous download;
     *  the import + placement happen in the HTTP completion callback. Returns
     *  true if the request was started. */
    UFUNCTION(BlueprintCallable, Category = "GLDF Search",
              meta = (AdvancedDisplay = "DisplayName,BaseUrl"))
    static bool DownloadAndPlace(
        const FString& DocId,
        int32 Variant,
        const FString& DisplayName = TEXT(""),
        const FString& BaseUrl = TEXT("https://gldf-search.de"));

    /** Fetch a .gldf directly from `Url` (no cabi resolution), write a temp
     *  file, and import it through Interchange — placing one
     *  AGldfLuminaireActor. Used by the web panel, whose result links already
     *  carry the downloadable URL. Returns true if the request was started. */
    UFUNCTION(BlueprintCallable, Category = "GLDF Search",
              meta = (AdvancedDisplay = "DisplayName"))
    static bool PlaceFromUrl(
        const FString& Url,
        const FString& DisplayName = TEXT(""));
};
