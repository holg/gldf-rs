// gldf-search.de → download → place. The UI is the web site itself
// (SGldfWebPanel); this library is the glue: resolve a document's .gldf URL
// over the REST API, fetch the file, hand it to whoever places luminaires in
// the current host (editor: Interchange import; game: UGldfRuntimeLoader).
#pragma once

#include "CoreMinimal.h"
#include "Kismet/BlueprintFunctionLibrary.h"
#include "GldfSearchLibrary.generated.h"

/** Called when a .gldf has been downloaded: `LocalPath` is the file on disk,
 *  `DisplayName` the human name (manufacturer_product). */
DECLARE_DELEGATE_TwoParams(FOnGldfDownloaded, const FString& /*LocalPath*/, const FString& /*DisplayName*/);

UCLASS()
class GLDFSEARCH_API UGldfSearchLibrary : public UBlueprintFunctionLibrary
{
    GENERATED_BODY()

public:
    /** How a downloaded .gldf becomes a luminaire in the world. The editor
     *  module binds the Interchange import (assets + actor); a game binds
     *  UGldfRuntimeLoader::SpawnLuminaireFromFile + PlaceFromRay. Unbound →
     *  the file is only downloaded and logged. */
    static FOnGldfDownloaded& PlaceHandler();

    /** Folder the downloads land in (<Project>/Saved/GldfSearch). */
    static FString DownloadDir();

    /** Fetch `Url` via UE HTTP into DownloadDir()/<name>.gldf and call `OnDone`
     *  on the game thread. Returns true if the request started. */
    static bool DownloadGldf(const FString& Url, const FString& DisplayName, FOnGldfDownloaded OnDone);

    /** Fetch a .gldf from `Url` and hand it to PlaceHandler(). The web panel's
     *  GLDF viewer links already carry this URL. Returns true if started. */
    UFUNCTION(BlueprintCallable, Category = "GLDF Search", meta = (AdvancedDisplay = "DisplayName"))
    static bool PlaceFromUrl(const FString& Url, const FString& DisplayName = TEXT(""));

    /** Resolve a gldf-search document (hex id) + variant to its .gldf through
     *  GET {BaseUrl}/api/doc/{DocId} (variants[i].source_index → source_paths),
     *  download it and hand it to PlaceHandler(). The web panel's LDC viewer
     *  links carry doc + variant. Asynchronous; returns true if started. */
    UFUNCTION(BlueprintCallable, Category = "GLDF Search", meta = (AdvancedDisplay = "DisplayName,BaseUrl"))
    static bool DownloadAndPlace(const FString& DocId, int32 Variant, const FString& DisplayName = TEXT(""),
        const FString& BaseUrl = TEXT("https://gldf-search.de"));
};
