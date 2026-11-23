// Hosts the gldf-search-leptos web UI (gldf-search.de) inside Unreal via
// SWebBrowser — in the editor (Window ▸ Tools tab) and in packaged games
// (WKWebView on Mac, CEF on Windows/Linux). Tailored to what UE can import:
// files mode (one row per complete GLDF), has-photometry + has-3D filters,
// and the viewer links become Place actions — clicking one intercepts the
// navigation, extracts the .gldf URL (or doc+variant for LDC links) and
// hands it to UGldfSearchLibrary, whose PlaceHandler() places it (Interchange
// in the editor, the runtime loader in games). The site itself stays
// untouched; all tailoring is navigation interception + injected JS.

#pragma once

#include "CoreMinimal.h"
#include "Widgets/SCompoundWidget.h"

class SWebBrowser;
struct FWebNavigationRequest;

class GLDFSEARCH_API SGldfWebPanel : public SCompoundWidget
{
public:
    SLATE_BEGIN_ARGS(SGldfWebPanel) {}
    SLATE_END_ARGS()

    void Construct(const FArguments& InArgs);

private:
    bool HandleBeforeNavigation(const FString& Url, const FWebNavigationRequest& Request);
    void HandleLoadCompleted();

    /** If `Url` is a recognized viewer link (gldf.icu / iesna.eu), kick off
     *  the download + placement and return true. */
    bool TryPlaceFromViewerUrl(const FString& Url);

    TSharedPtr<SWebBrowser> Browser;
    /** Created explicitly so load errors / document states can be logged
     *  (SWebBrowser alone only says "error"). */
    TSharedPtr<class IWebBrowserWindow> Window;
};
