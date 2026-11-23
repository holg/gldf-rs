#include "SGldfWebPanel.h"

#include "GldfSearchLibrary.h"

#include "SWebBrowser.h"
#include "IWebBrowserWindow.h" // FWebNavigationRequest
#include "IWebBrowserSingleton.h"
#include "WebBrowserModule.h"
#include "GenericPlatform/GenericPlatformHttp.h"
#include "HAL/PlatformProcess.h"

DEFINE_LOG_CATEGORY_STATIC(LogGldfWebPanel, Log, All);

// MUST be the clean URL — no filter query params. The Leptos app's SSR
// ignores the query string (url_sync.rs is wasm-only), so a deep-linked
// filter makes the client hydrate against non-matching server DOM and
// tachys panics (hydration.rs "unreachable"). The default filters are
// instead applied post-hydration by the injected script, via UI clicks.
static const TCHAR* GldfWebSearchUrl = TEXT("https://gldf-search.de/");

static bool IsGldfSearchUrl(const FString& Url)
{
    return Url.StartsWith(TEXT("https://gldf-search.de"))
        || Url.StartsWith(TEXT("https://www.gldf-search.de"))
        || Url.StartsWith(TEXT("about:"));
}

/** Decoded value of `Key` in `Url`'s query string, or empty. */
static FString QueryParam(const FString& Url, const FString& Key)
{
    int32 QueryStart = INDEX_NONE;
    if (!Url.FindChar(TEXT('?'), QueryStart))
    {
        return FString();
    }
    TArray<FString> Pairs;
    Url.Mid(QueryStart + 1).ParseIntoArray(Pairs, TEXT("&"));
    for (const FString& Pair : Pairs)
    {
        FString K, V;
        if (Pair.Split(TEXT("="), &K, &V) && K == Key)
        {
            return FGenericPlatformHttp::UrlDecode(V);
        }
    }
    return FString();
}

void SGldfWebPanel::Construct(const FArguments& /*InArgs*/)
{
    // Own browser window: gives us the document state + error code for the log.
    if (IWebBrowserSingleton* Singleton = IWebBrowserModule::Get().GetSingleton())
    {
        FCreateBrowserWindowSettings Settings;
        Settings.InitialURL = GldfWebSearchUrl;
        Settings.bUseTransparency = false;
        Settings.bShowErrorMessage = true;
        Window = Singleton->CreateBrowserWindow(Settings);
        if (Window.IsValid())
        {
            TWeakPtr<IWebBrowserWindow> Weak = Window;
            Window->OnDocumentStateChanged().AddLambda([Weak](EWebBrowserDocumentState State)
            {
                TSharedPtr<IWebBrowserWindow> W = Weak.Pin();
                UE_LOG(LogGldfWebPanel, Log, TEXT("document state %d (%s) url=%s error=%d"), (int32)State,
                    State == EWebBrowserDocumentState::Completed ? TEXT("completed") : State == EWebBrowserDocumentState::Error ? TEXT("ERROR") : State == EWebBrowserDocumentState::Loading ? TEXT("loading") : TEXT("none"),
                    W.IsValid() ? *W->GetUrl() : TEXT("?"), W.IsValid() ? W->GetLoadError() : 0);
            });
            Window->OnUrlChanged().AddLambda([](FString NewUrl) { UE_LOG(LogGldfWebPanel, Log, TEXT("url → %s"), *NewUrl); });
        }
        else
        {
            UE_LOG(LogGldfWebPanel, Warning, TEXT("CreateBrowserWindow returned null — no web browser backend on this platform/build"));
        }
    }
    else
    {
        UE_LOG(LogGldfWebPanel, Warning, TEXT("WebBrowser singleton unavailable"));
    }

    ChildSlot
    [
        SAssignNew(Browser, SWebBrowser, Window)
            .InitialURL(GldfWebSearchUrl)
            .ShowControls(false)
            .SupportsTransparency(false)
            .OnBeforeNavigation(SWebBrowserView::FOnBeforeBrowse::CreateSP(this, &SGldfWebPanel::HandleBeforeNavigation))
            .OnLoadCompleted(FSimpleDelegate::CreateSP(this, &SGldfWebPanel::HandleLoadCompleted))
            .OnLoadError(FSimpleDelegate::CreateLambda([this]()
            {
                // On the Mac WKWebView backend this also fires once per Place
                // click: the ueplace: scheme intentionally fails to load
                // (failed provisional navigation, page unaffected).
                UE_LOG(LogGldfWebPanel, Log, TEXT("Browser load error %d at %s (network outage, or the expected ueplace: side-channel failure after a Place click)"),
                    Window.IsValid() ? Window->GetLoadError() : -1, Window.IsValid() ? *Window->GetUrl() : TEXT("?"));
            }))
    ];
}

bool SGldfWebPanel::HandleBeforeNavigation(const FString& Url, const FWebNavigationRequest& Request)
{
    // The place bridge: the injected click handler navigates to
    // "ueplace://p?s=<seq>&u=<encoded viewer url>" instead of the real link.
    // This is the one channel that works on every backend — on Mac the
    // engine's WKWebView proxy IGNORES this handler's return value (policy
    // is always Allow) and never reports dynamic document.title changes, but
    // an unknown scheme merely fails as a provisional navigation and the
    // page stays. On CEF (Win/Linux) the `return true` cancels it outright.
    if (Url.StartsWith(TEXT("ueplace:")))
    {
        const FString ViewerUrl = QueryParam(Url, TEXT("u"));
        if (ViewerUrl.IsEmpty() || !TryPlaceFromViewerUrl(ViewerUrl))
        {
            UE_LOG(LogGldfWebPanel, Warning, TEXT("ueplace signal with unrecognized url: %s"), *Url);
        }
        return true;
    }

    if (!Request.bIsMainFrame || IsGldfSearchUrl(Url))
    {
        return false; // proceed in-panel
    }

    // Fallback for viewer links the injected JS missed (e.g. clicked before
    // injection ran). NOTE: on the Mac WKWebView backend the cancel is
    // ignored, so this may still navigate the panel away — the JS click
    // interception is the real guard there.
    if (TryPlaceFromViewerUrl(Url))
    {
        return true;
    }

    // Anything else leaving gldf-search.de → system browser; the panel is a
    // GLDF tool, not a general web browser.
    UE_LOG(LogGldfWebPanel, Log, TEXT("External link -> system browser: %s"), *Url);
    FPlatformProcess::LaunchURL(*Url, nullptr, nullptr);
    return true; // cancel in-panel navigation
}

bool SGldfWebPanel::TryPlaceFromViewerUrl(const FString& Url)
{
    // A viewer-link click IS the place action: the GLDF viewer link carries
    // the downloadable .gldf URL, the LDC viewer link carries doc+variant.
    // (`uename` is appended to the hrefs by our injected JS for asset naming.)

    // https://gldf.icu?url=<encoded .gldf url>  →  fetch + import directly.
    if (Url.Contains(TEXT("gldf.icu")))
    {
        const FString GldfUrl = QueryParam(Url, TEXT("url"));
        if (!GldfUrl.IsEmpty())
        {
            UE_LOG(LogGldfWebPanel, Log, TEXT("Place (file): %s"), *GldfUrl);
            UGldfSearchLibrary::PlaceFromUrl(GldfUrl, QueryParam(Url, TEXT("uename")));
            return true;
        }
    }

    // https://iesna.eu?url=<base>/api/ldc/<doc_hex>/<variant>  →  resolve the
    // variant's .gldf over the REST API and place that.
    if (Url.Contains(TEXT("iesna.eu")))
    {
        const FString LdcUrl = QueryParam(Url, TEXT("url"));
        const FString Marker = TEXT("/api/ldc/");
        const int32 MarkerIdx = LdcUrl.Find(Marker);
        if (MarkerIdx != INDEX_NONE)
        {
            FString DocHex, VariantStr;
            if (LdcUrl.Mid(MarkerIdx + Marker.Len()).Split(TEXT("/"), &DocHex, &VariantStr))
            {
                const int32 Variant = FCString::Atoi(*VariantStr);
                UE_LOG(LogGldfWebPanel, Log, TEXT("Place (variant): doc %s v%d (resolved via /api/doc)"), *DocHex, Variant);
                UGldfSearchLibrary::DownloadAndPlace(DocHex, Variant, QueryParam(Url, TEXT("uename")));
                return true;
            }
        }
    }

    return false;
}

void SGldfWebPanel::HandleLoadCompleted()
{
    if (!Browser.IsValid())
    {
        return;
    }

    // Embed-side tailoring of the Leptos UI (the site itself stays untouched).
    // CRITICAL: no DOM mutation until Leptos hydration is done — tachys walks
    // the SSR DOM expecting an exact match, and any earlier edit panics it
    // (hydration.rs "unreachable"). So:
    //  - click interception is a capture-phase listener (mutation-free, works
    //    pre-hydration): a viewer-link click navigates to a ueplace:// scheme
    //    URL → HandleBeforeNavigation places the GLDF; the unknown scheme
    //    can't load, so the panel never leaves the search page;
    //  - the boot loop retries click() on the "files" toggle until its active
    //    class flips — the flip needs Leptos' handler, so it doubles as the
    //    hydration-complete signal;
    //  - after that flip it applies the default filters the way a user would
    //    (click "present" under "Has photometry" / "Has 3D model" — only
    //    those docs are importable; deep-linking them would panic hydration,
    //    see above), verifying each via its facet-selected class;
    //  - only then do the cosmetic relabels ("Place in level") start, kept
    //    fresh across re-renders by a MutationObserver.
    Browser->ExecuteJavascript(TEXT(
        "(function(){"
        "if(window.__ueGldf)return;window.__ueGldf=true;"
        "function rowName(a){"
            "var row=a.closest('li.hit');if(!row)return '';"
            "var name='';"
            "['hit-manufacturer','hit-product','hit-variant-name'].forEach(function(c){"
                "var el=row.querySelector(':scope > .hit-header > .'+c);"
                "if(el&&el.textContent.trim())name+=(name?'_':'')+el.textContent.trim();"
            "});"
            "return name;"
        "}"
        "document.addEventListener('click',function(e){"
            "var a=e.target&&e.target.closest?e.target.closest('a.viewer-link'):null;"
            "if(!a)return;"
            "e.preventDefault();e.stopPropagation();"
            "var n=rowName(a);"
            "var u=a.href+(a.href.indexOf('uename=')<0&&n?'&uename='+encodeURIComponent(n):'');"
            "window.__ueSeq=(window.__ueSeq||0)+1;"
            "location.href='ueplace://p?s='+window.__ueSeq+'&u='+encodeURIComponent(u);"
        "},true);"
        "function relabel(){"
            "document.querySelectorAll('a.viewer-link').forEach(function(a){"
                "var label=a.href.indexOf('gldf.icu')>=0?'Place in level \\u2913':'Place variant \\u2913';"
                "if(a.textContent!==label)a.textContent=label;"
            "});"
        "}"
        "function ensureTristate(group,value){"
            "var sel=false;"
            "document.querySelectorAll('section.facet-group').forEach(function(sec){"
                "var n=sec.querySelector('.facet-group-name');"
                "if(!n||n.textContent.trim()!==group)return;"
                "sec.querySelectorAll('button.facet-value').forEach(function(b){"
                    "var l=b.querySelector('.facet-label');"
                    "if(!l||l.textContent.trim()!==value)return;"
                    "if(b.classList.contains('facet-selected'))sel=true;"
                    "else b.click();"
                "});"
            "});"
            "return sel;"
        "}"
        "function finish(){"
            "relabel();"
            "new MutationObserver(relabel).observe(document.body,{childList:true,subtree:true});"
        "}"
        "var tries=0;"
        "(function boot(){"
            "var fb=null;"
            "document.querySelectorAll('.count-toggle button').forEach(function(b){"
                "if(b.textContent.trim()==='files')fb=b;"
            "});"
            "if(!fb||!fb.classList.contains('active')){"
                "if(fb)fb.click();"
                "if(++tries<60)setTimeout(boot,500);else finish();"
                "return;"
            "}"
            "var photOk=ensureTristate('Has photometry','present');"
            "var m3dOk=ensureTristate('Has 3D model','present');"
            "if(photOk&&m3dOk){finish();return;}"
            "if(++tries<60)setTimeout(boot,500);else finish();"
        "})();"
        "})();"
    ));
    UE_LOG(LogGldfWebPanel, Log, TEXT("Load completed (UE place-mode script injected, hydration-safe)"));
}
