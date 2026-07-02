#include "GldfSearchLibrary.h"

#include "GldfLuminaireActor.h"  // from the GldfImporter module

#include "HttpModule.h"
#include "Interfaces/IHttpRequest.h"
#include "Interfaces/IHttpResponse.h"
#include "InterchangeManager.h"
#include "InterchangeProjectSettings.h"
#include "InterchangeSourceData.h"
#include "InterchangeGldfPipeline.h"  // the root-retype pipeline (GldfImporter module)
#include "InterchangeGldfTranslator.h"
#include "Editor.h"
#include "LevelEditorViewport.h"  // GCurrentLevelEditingViewportClient
#include "Engine/World.h"
#include "Engine/HitResult.h"
#include "Misc/FileHelper.h"
#include "Misc/Paths.h"
#include "HAL/PlatformFileManager.h"
#include "Logging/LogMacros.h"

extern "C" {
#include "gldf_search.h"
}

DEFINE_LOG_CATEGORY_STATIC(LogGldfSearch, Log, All);

namespace
{
    // RAII wrapper for an owned cabi engine handle.
    struct FEngineHandle
    {
        GldfEngine* Engine = nullptr;
        explicit FEngineHandle(const FString& BaseUrl)
        {
            Engine = gldf_engine_from_remote(TCHAR_TO_UTF8(*BaseUrl));
        }
        ~FEngineHandle()
        {
            if (Engine) { gldf_engine_free(Engine); }
        }
        FEngineHandle(const FEngineHandle&) = delete;
        FEngineHandle& operator=(const FEngineHandle&) = delete;
        explicit operator bool() const { return Engine != nullptr; }
    };

    // The cabi sets a thread-local error string on failure (return < 0 / NULL).
    FString LastError()
    {
        const char* Msg = gldf_last_error();
        return Msg ? FString(UTF8_TO_TCHAR(Msg)) : FString(TEXT("(no message)"));
    }

    // Owned cabi C-string → FString, freeing the cabi allocation.
    FString TakeCabiString(char* Raw)
    {
        if (!Raw) { return FString(); }
        FString Out(UTF8_TO_TCHAR(Raw));
        gldf_string_free(Raw);
        return Out;
    }

    // GldfFacetField discriminants (mirror the cabi header) — used both to
    // resolve a facet value's id and to dispatch facet field names. Order
    // matches GldfFacetField_*.
    constexpr uint16 FF_Manufacturer = 0, FF_Applications = 1, FF_Labels = 2,
        FF_IpCode = 3, FF_SafetyClass = 4, FF_IkRating = 5, FF_ProductForm = 6,
        FF_Adjustability = 7, FF_MountingPlace = 8, FF_MountingType = 9,
        FF_LightDistribution = 10, FF_HasPhotometry = 11, FF_Has3d = 12,
        FF_HasEmergencyLighting = 13, FF_LampType = 14, FF_ControlGearInterface = 15,
        FF_EmergencyLightingType = 16;

    // cabi field name string → GldfFacetField id, so we can resolve a facet
    // value's id via gldf_facet_value_from_str. Manufacturer + the three
    // has_* tristates aren't id-keyed (left at -1).
    uint16 FacetFieldId(const FString& Field, bool& bOutIdKeyed)
    {
        bOutIdKeyed = true;
        if (Field == TEXT("applications")) return FF_Applications;
        if (Field == TEXT("labels")) return FF_Labels;
        if (Field == TEXT("ip_code")) return FF_IpCode;
        if (Field == TEXT("safety_class")) return FF_SafetyClass;
        if (Field == TEXT("ik_rating")) return FF_IkRating;
        if (Field == TEXT("product_form")) return FF_ProductForm;
        if (Field == TEXT("adjustability")) return FF_Adjustability;
        if (Field == TEXT("mounting_place")) return FF_MountingPlace;
        if (Field == TEXT("mounting_type")) return FF_MountingType;
        if (Field == TEXT("light_distribution")) return FF_LightDistribution;
        if (Field == TEXT("lamp_type")) return FF_LampType;
        if (Field == TEXT("control_gear_interfaces")) return FF_ControlGearInterface;
        if (Field == TEXT("emergency_lighting_type")) return FF_EmergencyLightingType;
        bOutIdKeyed = false; // manufacturer, has_photometry, has_3d, has_emergency_lighting
        return 0;
    }

    // Owns the UTF-8 / id buffers a GldfFilter borrows, so the pointers stay
    // valid for the duration of one cabi call. Build it, call `.Get()`, use
    // the result only while this object lives.
    struct FFilterMarshal
    {
        FTCHARToUTF8 Text;
        FTCHARToUTF8 Locale;
        bool bHasText = false;
        bool bHasLocale = false;

        // Manufacturer: UTF-8 conversions + a const char* array into them.
        TArray<TUniquePtr<FTCHARToUTF8>> MfrUtf8;
        TArray<const char*> MfrPtrs;

        // One uint16 buffer per enum-id list field.
        TArray<uint16> Applications, Labels, Adjustability, IpCode, SafetyClass,
            IkRating, ProductForm, LightDistribution, LampType, ControlGearInterfaces,
            EmergencyLightingType, MountingPlace, MountingType;

        GldfFilter F{};

        explicit FFilterMarshal(const FGldfSearchFilter& In)
            : Text(*In.Text), Locale(*In.Locale)
        {
            auto Ids = [](const TArray<int32>& Src, TArray<uint16>& Dst)
            {
                Dst.Reserve(Src.Num());
                for (int32 V : Src) { Dst.Add(static_cast<uint16>(V)); }
            };
            Ids(In.Applications, Applications);
            Ids(In.Labels, Labels);
            Ids(In.Adjustability, Adjustability);
            Ids(In.IpCode, IpCode);
            Ids(In.SafetyClass, SafetyClass);
            Ids(In.IkRating, IkRating);
            Ids(In.ProductForm, ProductForm);
            Ids(In.LightDistribution, LightDistribution);
            Ids(In.LampType, LampType);
            Ids(In.ControlGearInterfaces, ControlGearInterfaces);
            Ids(In.EmergencyLightingType, EmergencyLightingType);
            Ids(In.MountingPlace, MountingPlace);
            Ids(In.MountingType, MountingType);

            for (const FString& M : In.Manufacturer)
            {
                MfrUtf8.Add(MakeUnique<FTCHARToUTF8>(*M));
                MfrPtrs.Add(MfrUtf8.Last()->Get());
            }

            bHasText = !In.Text.IsEmpty();
            bHasLocale = !In.Locale.IsEmpty();

            F = GldfFilter{}; // zero-init: all null ptrs, 0 lens, Unset tristates
            if (bHasText) { F.text = Text.Get(); }
            if (bHasLocale) { F.locale = Locale.Get(); }

            auto SetList = [](const TArray<uint16>& Buf, const uint16_t*& Ptr, uintptr_t& Len)
            {
                if (Buf.Num() > 0) { Ptr = Buf.GetData(); Len = static_cast<uintptr_t>(Buf.Num()); }
            };
            SetList(Applications, F.applications, F.applications_len);
            SetList(Labels, F.labels, F.labels_len);
            SetList(Adjustability, F.adjustability, F.adjustability_len);
            SetList(IpCode, F.ip_code, F.ip_code_len);
            SetList(SafetyClass, F.safety_class, F.safety_class_len);
            SetList(IkRating, F.ik_rating, F.ik_rating_len);
            SetList(ProductForm, F.product_form, F.product_form_len);
            SetList(LightDistribution, F.light_distribution, F.light_distribution_len);
            SetList(LampType, F.lamp_type, F.lamp_type_len);
            SetList(ControlGearInterfaces, F.control_gear_interfaces, F.control_gear_interfaces_len);
            SetList(EmergencyLightingType, F.emergency_lighting_type, F.emergency_lighting_type_len);
            SetList(MountingPlace, F.mounting_place, F.mounting_place_len);
            SetList(MountingType, F.mounting_type, F.mounting_type_len);

            if (MfrPtrs.Num() > 0)
            {
                F.manufacturer = MfrPtrs.GetData();
                F.manufacturer_len = static_cast<uintptr_t>(MfrPtrs.Num());
            }

            // Ranges.
            F.flux_lm = { In.bHasFluxMin, In.FluxMin, In.bHasFluxMax, In.FluxMax };
            F.power_w = { In.bHasPowerMin, In.PowerMin, In.bHasPowerMax, In.PowerMax };
            F.efficacy = { In.bHasEfficacyMin, In.EfficacyMin, In.bHasEfficacyMax, In.EfficacyMax };
            F.beam_deg = { In.bHasBeamMin, In.BeamMin, In.bHasBeamMax, In.BeamMax };
            F.cct_k = { In.bHasCctMin, static_cast<uint16_t>(In.CctMin),
                        In.bHasCctMax, static_cast<uint16_t>(In.CctMax) };
            F.recessed_depth_mm = { In.bHasDepthMin, static_cast<uint32_t>(In.DepthMin),
                                    In.bHasDepthMax, static_cast<uint32_t>(In.DepthMax) };

            if (In.CriMin >= 0) { F.has_cri_min = true; F.cri_min = static_cast<uint8_t>(In.CriMin); }

            auto Tri = [](int32 V) -> GldfTriState
            {
                return V == 1 ? GldfTriState_True : (V == 0 ? GldfTriState_False : GldfTriState_Unset);
            };
            F.has_photometry = Tri(In.HasPhotometry);
            F.has_3d = Tri(In.Has3d);
            F.has_emergency_lighting = Tri(In.HasEmergencyLighting);
        }

        const GldfFilter* Get() const { return &F; }

        FFilterMarshal(const FFilterMarshal&) = delete;
        FFilterMarshal& operator=(const FFilterMarshal&) = delete;
    };

    FGldfRange RangeFromF32(const GldfRangeF32& R)
    {
        FGldfRange Out;
        Out.bHasMin = R.has_min; Out.Min = R.min;
        Out.bHasMax = R.has_max; Out.Max = R.max;
        return Out;
    }

    // Turn a cabi search result into display hits, enriching each with the
    // doc's manufacturer/product + the matching variant name (one doc lookup
    // per hit — fine for a page of results). Frees the per-hit docs; the
    // caller still owns + frees `Res`.
    TArray<FGldfSearchHit> BuildHits(GldfEngine* Eng, const GldfSearchResult& Res)
    {
        TArray<FGldfSearchHit> Results;
        Results.Reserve(static_cast<int32>(Res.hits_len));
        for (uintptr_t i = 0; i < Res.hits_len; ++i)
        {
            const GldfSearchHit& H = Res.hits[i];
            FGldfSearchHit Out;
            Out.DocId = H.doc ? FString(UTF8_TO_TCHAR(H.doc)) : FString();
            Out.Variant = static_cast<int32>(H.variant);

            if (!Out.DocId.IsEmpty())
            {
                GldfLuminaireDoc Doc{};
                bool bFound = false;
                if (gldf_engine_doc(Eng, TCHAR_TO_UTF8(*Out.DocId), &Doc, &bFound) == GLDF_OK && bFound)
                {
                    Out.Manufacturer = Doc.manufacturer ? FString(UTF8_TO_TCHAR(Doc.manufacturer)) : FString();
                    Out.Product = Doc.product ? FString(UTF8_TO_TCHAR(Doc.product)) : FString();
                    for (uintptr_t v = 0; v < Doc.variants_len; ++v)
                    {
                        if (Doc.variants[v].id == H.variant && Doc.variants[v].name)
                        {
                            Out.VariantName = FString(UTF8_TO_TCHAR(Doc.variants[v].name));
                            break;
                        }
                    }
                    gldf_doc_free(&Doc);
                }
            }
            Results.Add(MoveTemp(Out));
        }
        return Results;
    }
}

TArray<FGldfSearchHit> UGldfSearchLibrary::Search(
    const FString& Query, const FString& BaseUrl, int32 Limit)
{
    TArray<FGldfSearchHit> Results;

    FEngineHandle Eng(BaseUrl);
    if (!Eng)
    {
        UE_LOG(LogGldfSearch, Error, TEXT("Search: engine_from_remote failed: %s"), *LastError());
        return Results;
    }

    // Minimal filter: free-text only, everything else empty/zero. The struct is
    // borrowed for the call; the UTF-8 query must outlive gldf_engine_search.
    const FTCHARToUTF8 QueryUtf8(*Query);
    GldfFilter Filter{};                 // value-init → all null ptrs / 0 lens
    Filter.text = QueryUtf8.Get();

    GldfSearchResult Res{};
    const int32 Rc = gldf_engine_search(
        Eng.Engine, &Filter, /*offset*/ 0, static_cast<uint32>(FMath::Max(1, Limit)), &Res);
    if (Rc != GLDF_OK)
    {
        UE_LOG(LogGldfSearch, Error, TEXT("Search: gldf_engine_search rc=%d: %s"), Rc, *LastError());
        return Results;
    }

    Results = BuildHits(Eng.Engine, Res);
    UE_LOG(LogGldfSearch, Log, TEXT("Search '%s': %d hit(s) (total=%u, files=%u)"),
        *Query, Results.Num(), Res.total, Res.files_total);

    gldf_search_result_free(&Res);
    return Results;
}

TArray<FGldfSearchHit> UGldfSearchLibrary::SearchFiltered(
    const FGldfSearchFilter& Filter, const FString& BaseUrl, int32 Offset, int32 Limit)
{
    TArray<FGldfSearchHit> Results;
    FEngineHandle Eng(BaseUrl);
    if (!Eng)
    {
        UE_LOG(LogGldfSearch, Error, TEXT("SearchFiltered: engine failed: %s"), *LastError());
        return Results;
    }

    FFilterMarshal M(Filter);
    GldfSearchResult Res{};
    const int32 Rc = gldf_engine_search(
        Eng.Engine, M.Get(), static_cast<uint32>(FMath::Max(0, Offset)),
        static_cast<uint32>(FMath::Max(1, Limit)), &Res);
    if (Rc != GLDF_OK)
    {
        UE_LOG(LogGldfSearch, Error, TEXT("SearchFiltered: rc=%d: %s"), Rc, *LastError());
        return Results;
    }

    Results = BuildHits(Eng.Engine, Res);
    UE_LOG(LogGldfSearch, Log, TEXT("SearchFiltered: %d hit(s) (total=%u, files=%u)"),
        Results.Num(), Res.total, Res.files_total);
    gldf_search_result_free(&Res);
    return Results;
}

TArray<FGldfFacet> UGldfSearchLibrary::Facets(
    const FGldfSearchFilter& Filter, const FString& BaseUrl)
{
    TArray<FGldfFacet> Out;
    FEngineHandle Eng(BaseUrl);
    if (!Eng)
    {
        UE_LOG(LogGldfSearch, Error, TEXT("Facets: engine failed: %s"), *LastError());
        return Out;
    }

    FFilterMarshal M(Filter);
    GldfFacetCounts Counts{};
    // NULL fields = all fields.
    const int32 Rc = gldf_engine_facets(Eng.Engine, M.Get(), nullptr, 0, &Counts);
    if (Rc != GLDF_OK)
    {
        UE_LOG(LogGldfSearch, Error, TEXT("Facets: rc=%d: %s"), Rc, *LastError());
        return Out;
    }

    Out.Reserve(static_cast<int32>(Counts.facets_len));
    for (uintptr_t f = 0; f < Counts.facets_len; ++f)
    {
        const GldfFacet& Fac = Counts.facets[f];
        FGldfFacet OutFac;
        OutFac.Field = Fac.field ? FString(UTF8_TO_TCHAR(Fac.field)) : FString();

        bool bIdKeyed = false;
        const uint16 FieldId = FacetFieldId(OutFac.Field, bIdKeyed);

        OutFac.Values.Reserve(static_cast<int32>(Fac.values_len));
        for (uintptr_t v = 0; v < Fac.values_len; ++v)
        {
            const GldfFacetValue& Val = Fac.values[v];
            FGldfFacetValue OutVal;
            OutVal.Value = Val.value ? FString(UTF8_TO_TCHAR(Val.value)) : FString();
            OutVal.Count = static_cast<int32>(Val.count);
            // Resolve the value string back to its enum id (for id-keyed
            // fields) so the UI can toggle it into the filter. Manufacturer +
            // the has_* tristates aren't id-keyed → leave Id = -1.
            if (bIdKeyed && !OutVal.Value.IsEmpty())
            {
                uint16 Id = 0;
                if (gldf_facet_value_from_str(FieldId, TCHAR_TO_UTF8(*OutVal.Value), &Id))
                {
                    OutVal.Id = static_cast<int32>(Id);
                }
            }
            OutFac.Values.Add(MoveTemp(OutVal));
        }
        Out.Add(MoveTemp(OutFac));
    }

    gldf_facets_free(&Counts);
    return Out;
}

FGldfRangeBounds UGldfSearchLibrary::RangeBounds(
    const FGldfSearchFilter& Filter, const FString& BaseUrl)
{
    FGldfRangeBounds Out; // bValid defaults false
    FEngineHandle Eng(BaseUrl);
    if (!Eng)
    {
        UE_LOG(LogGldfSearch, Error, TEXT("RangeBounds: engine failed: %s"), *LastError());
        return Out;
    }

    FFilterMarshal M(Filter);
    GldfRangeBounds B{};
    const int32 Rc = gldf_engine_range_bounds(Eng.Engine, M.Get(), &B);
    if (Rc != GLDF_OK)
    {
        // Most likely a remote host without /api/range_bounds (405). Not
        // fatal — the UI disables the sliders. Logged, not errored loudly.
        UE_LOG(LogGldfSearch, Warning, TEXT("RangeBounds unavailable: %s"), *LastError());
        return Out;
    }

    Out.Flux = RangeFromF32(B.flux_lm);
    Out.Power = RangeFromF32(B.power_w);
    Out.Efficacy = RangeFromF32(B.efficacy);
    Out.Beam = RangeFromF32(B.beam_deg);
    Out.RecessedDepth.bHasMin = B.recessed_depth_mm.has_min;
    Out.RecessedDepth.Min = static_cast<float>(B.recessed_depth_mm.min);
    Out.RecessedDepth.bHasMax = B.recessed_depth_mm.has_max;
    Out.RecessedDepth.Max = static_cast<float>(B.recessed_depth_mm.max);
    Out.Cct.bHasMin = B.cct_k.has_min;
    Out.Cct.Min = static_cast<float>(B.cct_k.min);
    Out.Cct.bHasMax = B.cct_k.has_max;
    Out.Cct.Max = static_cast<float>(B.cct_k.max);
    Out.bValid = true;
    return Out;
}

bool UGldfSearchLibrary::DownloadAndPlace(
    const FString& DocId, int32 Variant, const FString& DisplayName, const FString& BaseUrl)
{
    // 1. Resolve the downloadable URL via the cabi (it does the {base}/gldfs/
    //    join + percent-encoding from the doc's SourceRef::Path).
    FString Url;
    {
        FEngineHandle Eng(BaseUrl);
        if (!Eng)
        {
            UE_LOG(LogGldfSearch, Error, TEXT("DownloadAndPlace: engine failed: %s"), *LastError());
            return false;
        }
        char* RawUrl = gldf_engine_download_url(
            Eng.Engine, TCHAR_TO_UTF8(*DocId), static_cast<uint16>(Variant));
        Url = TakeCabiString(RawUrl);
        if (Url.IsEmpty())
        {
            // NULL = no recoverable download (local/ContentOnly source, or a
            // bad id). Not an error per se — nothing to fetch.
            UE_LOG(LogGldfSearch, Warning,
                TEXT("DownloadAndPlace: no download URL for doc %s variant %d"), *DocId, Variant);
            return false;
        }
    }

    const FString Name = DisplayName.IsEmpty() ? DocId.Left(16) : DisplayName;
    return PlaceFromUrl(Url, FString::Printf(TEXT("%s_v%d"), *Name, Variant));
}

bool UGldfSearchLibrary::PlaceFromUrl(const FString& Url, const FString& DisplayName)
{
    // Sanitised base name for the temp file (→ the imported asset/actor name).
    // Prefer the human display name; fall back to the URL's filename. Keep it
    // filesystem- + UE-asset-safe (alnum + underscore).
    FString SafeName = DisplayName.IsEmpty() ? FPaths::GetBaseFilename(Url) : DisplayName;
    {
        FString Clean;
        Clean.Reserve(SafeName.Len());
        for (TCHAR C : SafeName)
        {
            Clean.AppendChar((FChar::IsAlnum(C)) ? C : TEXT('_'));
        }
        // Collapse the worst-case empty result.
        SafeName = Clean.IsEmpty() ? TEXT("Gldf") : Clean.Left(64);
    }

    UE_LOG(LogGldfSearch, Log, TEXT("PlaceFromUrl: fetching %s"), *Url);

    // 2. Download via UE's HTTP module (native — no Rust download code).
    TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
    Request->SetURL(Url);
    Request->SetVerb(TEXT("GET"));
    Request->OnProcessRequestComplete().BindLambda(
        [Url, SafeName](FHttpRequestPtr Req, FHttpResponsePtr Response, bool bOk)
        {
            if (!bOk || !Response.IsValid() || Response->GetResponseCode() != 200)
            {
                UE_LOG(LogGldfSearch, Error,
                    TEXT("PlaceFromUrl: HTTP failed (ok=%d, code=%d) for %s"),
                    bOk ? 1 : 0, Response.IsValid() ? Response->GetResponseCode() : -1, *Url);
                return;
            }

            const TArray<uint8>& Bytes = Response->GetContent();
            if (Bytes.Num() == 0)
            {
                UE_LOG(LogGldfSearch, Error, TEXT("PlaceFromUrl: empty .gldf from %s"), *Url);
                return;
            }

            // 3. Write a temp .gldf. Interchange reads from a path, and the
            //    importer re-opens the same path via FFI, so the file must
            //    persist through the import (not a scoped temp).
            const FString TempPath = FPaths::Combine(
                FPaths::ProjectIntermediateDir(), TEXT("GldfSearch"),
                SafeName + TEXT(".gldf"));
            if (!FFileHelper::SaveArrayToFile(Bytes, *TempPath))
            {
                UE_LOG(LogGldfSearch, Error, TEXT("PlaceFromUrl: could not write %s"), *TempPath);
                return;
            }

            // 4. Import via Interchange — spawns the AGldfLuminaireActor (the
            //    same path a drag-import / the Python helper uses). Capture the
            //    actor via the scene-object-done delegate.
            UInterchangeManager& Mgr = UInterchangeManager::GetInterchangeManager();
            UInterchangeSourceData* Source = UInterchangeManager::CreateSourceData(TempPath);

            FImportAssetParameters Params;
            Params.bIsAutomated = true;  // no modal dialog

            // CRITICAL: a programmatic ImportScene does NOT apply our
            // PerTranslatorPipelines entry the way the interactive "Import Into
            // Level" dialog does — it would run the default Scene stack, which
            // lacks UInterchangeGldfPipeline, so the root never gets retyped to
            // AGldfLuminaireActor and no emitter lights are built (→ dark).
            // Fix: pass the SAME pipeline list explicitly via OverridePipelines
            // — the Scene stack's base pipelines + our pipeline appended last
            // (mirrors FGldfImporterModule::RegisterGldfScenePipeline).
            if (const UInterchangeProjectSettings* Settings = GetDefault<UInterchangeProjectSettings>())
            {
                const FInterchangePipelineStack* SceneStack =
                    Settings->SceneImportSettings.PipelineStacks.Find(FName(TEXT("Scene")));
                if (SceneStack)
                {
                    // Prefer the GLDF per-translator list if present (base + ours);
                    // else fall back to base + append ours.
                    const TSoftClassPtr<UInterchangeTranslatorBase> GldfTranslatorClass(
                        UInterchangeGldfTranslator::StaticClass());
                    const FInterchangeTranslatorPipelines* PerGldf =
                        SceneStack->PerTranslatorPipelines.FindByPredicate(
                            [&](const FInterchangeTranslatorPipelines& E)
                            { return E.Translator == GldfTranslatorClass; });
                    if (PerGldf)
                    {
                        Params.OverridePipelines = PerGldf->Pipelines;
                    }
                    else
                    {
                        Params.OverridePipelines = SceneStack->Pipelines;
                        Params.OverridePipelines.Add(
                            FSoftObjectPath(UInterchangeGldfPipeline::StaticClass()->GetDefaultObject()));
                    }
                }
            }
            // Last-resort: at least our pipeline, so the root gets retyped.
            if (Params.OverridePipelines.Num() == 0)
            {
                Params.OverridePipelines.Add(
                    FSoftObjectPath(UInterchangeGldfPipeline::StaticClass()->GetDefaultObject()));
            }
            Params.OnSceneObjectDoneNative.BindLambda(
                [](UObject* Obj)
                {
                    if (AGldfLuminaireActor* Lum = Cast<AGldfLuminaireActor>(Obj))
                    {
                        // ImportScene spawns at the world origin, which in a
                        // real level is usually below the floor / outside the
                        // building. Drop the luminaire where the user is
                        // looking instead: trace the viewport camera ray and
                        // sit at the first hit (ceiling/wall/floor), else
                        // 4 m ahead of the camera.
                        if (GCurrentLevelEditingViewportClient)
                        {
                            const FVector Start = GCurrentLevelEditingViewportClient->GetViewLocation();
                            const FVector Dir = GCurrentLevelEditingViewportClient->GetViewRotation().Vector();
                            FVector Target = Start + Dir * 400.0f;
                            if (UWorld* World = Lum->GetWorld())
                            {
                                FCollisionQueryParams QueryParams(TEXT("GldfPlaceTrace"), /*bTraceComplex*/ true);
                                QueryParams.AddIgnoredActor(Lum);
                                FHitResult Hit;
                                if (World->LineTraceSingleByChannel(
                                        Hit, Start, Start + Dir * 100000.0f, ECC_Visibility, QueryParams))
                                {
                                    Target = Hit.ImpactPoint;
                                }
                            }
                            Lum->SetActorLocation(Target);
                        }
                        if (GEditor)
                        {
                            GEditor->SelectNone(false, true);
                            GEditor->SelectActor(Lum, true, true);
                        }
                        UE_LOG(LogGldfSearch, Log, TEXT("PlaceFromUrl: placed %s at %s"),
                            *Lum->GetName(), *Lum->GetActorLocation().ToCompactString());
                    }
                });

            const bool bStarted = Mgr.ImportScene(TEXT("/Game/GLDF"), Source, Params);
            UE_LOG(LogGldfSearch, Log,
                TEXT("PlaceFromUrl: ImportScene started=%d for %s"), bStarted ? 1 : 0, *TempPath);
        });

    return Request->ProcessRequest();
}
