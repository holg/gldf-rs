#include "GldfSearchLibrary.h"

#include "Dom/JsonObject.h"
#include "GenericPlatform/GenericPlatformHttp.h"
#include "HttpModule.h"
#include "Interfaces/IHttpRequest.h"
#include "Interfaces/IHttpResponse.h"
#include "Misc/FileHelper.h"
#include "Misc/Paths.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

DEFINE_LOG_CATEGORY_STATIC(LogGldfSearch, Log, All);

FOnGldfDownloaded& UGldfSearchLibrary::PlaceHandler()
{
    static FOnGldfDownloaded Handler;
    return Handler;
}

FString UGldfSearchLibrary::DownloadDir()
{
    return FPaths::Combine(FPaths::ProjectSavedDir(), TEXT("GldfSearch"));
}

bool UGldfSearchLibrary::PlaceFromUrl(const FString& Url, const FString& DisplayName)
{
    return DownloadGldf(Url, DisplayName, FOnGldfDownloaded::CreateLambda([](const FString& Path, const FString& Name)
    {
        if (PlaceHandler().IsBound())
        {
            PlaceHandler().Execute(Path, Name);
        }
        else
        {
            UE_LOG(LogGldfSearch, Warning, TEXT("PlaceFromUrl: downloaded %s but no place handler is bound (UGldfSearchLibrary::PlaceHandler)"), *Path);
        }
    }));
}

bool UGldfSearchLibrary::DownloadAndPlace(const FString& DocId, int32 Variant, const FString& DisplayName, const FString& BaseUrl)
{
    const FString Base = BaseUrl.TrimEnd().TrimChar(TEXT('/'));
    const FString DocUrl = Base + TEXT("/api/doc/") + DocId;
    UE_LOG(LogGldfSearch, Log, TEXT("DownloadAndPlace: GET %s (variant %d)"), *DocUrl, Variant);

    TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
    Request->SetURL(DocUrl);
    Request->SetVerb(TEXT("GET"));
    Request->OnProcessRequestComplete().BindLambda(
        [Base, DocId, Variant, DisplayName](FHttpRequestPtr, FHttpResponsePtr Response, bool bOk)
        {
            if (!bOk || !Response.IsValid() || Response->GetResponseCode() != 200)
            {
                UE_LOG(LogGldfSearch, Error, TEXT("DownloadAndPlace: /api/doc failed (ok=%d, code=%d) for %s"),
                    bOk ? 1 : 0, Response.IsValid() ? Response->GetResponseCode() : -1, *DocId);
                return;
            }
            TSharedPtr<FJsonObject> Doc;
            const TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Response->GetContentAsString());
            if (!FJsonSerializer::Deserialize(Reader, Doc) || !Doc.IsValid())
            {
                UE_LOG(LogGldfSearch, Error, TEXT("DownloadAndPlace: /api/doc returned no JSON object for %s"), *DocId);
                return;
            }

            // Which file: variants[Variant].source_index → source_paths[idx] ({Path} or {Url}); fall back to "source".
            int32 SourceIndex = 0;
            const TArray<TSharedPtr<FJsonValue>>* Variants = nullptr;
            if (Doc->TryGetArrayField(TEXT("variants"), Variants) && Variants && Variants->Num() > 0)
            {
                const int32 VI = FMath::Clamp(Variant, 0, Variants->Num() - 1);
                const TSharedPtr<FJsonObject>* VObj = nullptr;
                if ((*Variants)[VI].IsValid() && (*Variants)[VI]->TryGetObject(VObj) && VObj)
                {
                    (*VObj)->TryGetNumberField(TEXT("source_index"), SourceIndex);
                }
            }
            TSharedPtr<FJsonObject> Source;
            const TArray<TSharedPtr<FJsonValue>>* Paths = nullptr;
            if (Doc->TryGetArrayField(TEXT("source_paths"), Paths) && Paths && Paths->IsValidIndex(SourceIndex))
            {
                const TSharedPtr<FJsonObject>* SObj = nullptr;
                if ((*Paths)[SourceIndex].IsValid() && (*Paths)[SourceIndex]->TryGetObject(SObj) && SObj) { Source = *SObj; }
            }
            if (!Source.IsValid())
            {
                const TSharedPtr<FJsonObject>* SObj = nullptr;
                if (Doc->TryGetObjectField(TEXT("source"), SObj) && SObj) { Source = *SObj; }
            }
            FString Url;
            FString RelPath;
            if (Source.IsValid() && Source->TryGetStringField(TEXT("Url"), Url) && !Url.IsEmpty())
            {
                // remote source: use as is
            }
            else if (Source.IsValid() && Source->TryGetStringField(TEXT("Path"), RelPath) && !RelPath.IsEmpty())
            {
                TArray<FString> Segs;
                RelPath.ParseIntoArray(Segs, TEXT("/"));
                Url = Base + TEXT("/gldfs");
                for (const FString& Seg : Segs) { Url += TEXT("/") + FGenericPlatformHttp::UrlEncode(Seg); }
            }
            if (Url.IsEmpty())
            {
                UE_LOG(LogGldfSearch, Warning, TEXT("DownloadAndPlace: document %s has no downloadable source"), *DocId);
                return;
            }

            FString Name = DisplayName;
            if (Name.IsEmpty())
            {
                FString Mfr, Product;
                Doc->TryGetStringField(TEXT("manufacturer"), Mfr);
                Doc->TryGetStringField(TEXT("product"), Product);
                Name = Mfr.IsEmpty() ? Product : (Product.IsEmpty() ? Mfr : Mfr + TEXT("_") + Product);
                if (Name.IsEmpty()) { Name = DocId.Left(16); }
            }
            PlaceFromUrl(Url, FString::Printf(TEXT("%s_v%d"), *Name, Variant));
        });
    return Request->ProcessRequest();
}

bool UGldfSearchLibrary::DownloadGldf(const FString& Url, const FString& DisplayName, FOnGldfDownloaded OnDone)
{
    // Sanitised base name for the file (→ the imported asset/actor name).
    FString SafeName = DisplayName.IsEmpty() ? FPaths::GetBaseFilename(Url) : DisplayName;
    {
        FString Clean;
        Clean.Reserve(SafeName.Len());
        for (TCHAR C : SafeName) { Clean.AppendChar(FChar::IsAlnum(C) ? C : TEXT('_')); }
        SafeName = Clean.IsEmpty() ? TEXT("Gldf") : Clean.Left(64);
    }
    UE_LOG(LogGldfSearch, Log, TEXT("DownloadGldf: fetching %s"), *Url);

    TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
    Request->SetURL(Url);
    Request->SetVerb(TEXT("GET"));
    Request->OnProcessRequestComplete().BindLambda(
        [Url, SafeName, OnDone](FHttpRequestPtr, FHttpResponsePtr Response, bool bOk)
        {
            if (!bOk || !Response.IsValid() || Response->GetResponseCode() != 200)
            {
                UE_LOG(LogGldfSearch, Error, TEXT("DownloadGldf: HTTP failed (ok=%d, code=%d) for %s"),
                    bOk ? 1 : 0, Response.IsValid() ? Response->GetResponseCode() : -1, *Url);
                return;
            }
            const TArray<uint8>& Bytes = Response->GetContent();
            if (Bytes.Num() == 0)
            {
                UE_LOG(LogGldfSearch, Error, TEXT("DownloadGldf: empty .gldf from %s"), *Url);
                return;
            }
            // The importer / runtime loader re-open the file by path, so it
            // must persist (Saved/ is writable in packaged games too).
            const FString Path = FPaths::Combine(DownloadDir(), SafeName + TEXT(".gldf"));
            if (!FFileHelper::SaveArrayToFile(Bytes, *Path))
            {
                UE_LOG(LogGldfSearch, Error, TEXT("DownloadGldf: could not write %s"), *Path);
                return;
            }
            OnDone.ExecuteIfBound(Path, SafeName);
        });
    return Request->ProcessRequest();
}
