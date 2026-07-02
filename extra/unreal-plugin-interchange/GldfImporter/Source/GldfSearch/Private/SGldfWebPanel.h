// Hosts the gldf-search-leptos web UI (gldf-search.de) inside the editor via
// SWebBrowser, tailored to what UE can import: deep-links to has-photometry +
// has-3D filters, defaults to files mode (one row per complete GLDF), and
// turns the viewer links into Place actions — clicking one intercepts the
// navigation, extracts the .gldf URL (or doc+variant for LDC links), and
// imports it into the level via Interchange. The site itself stays untouched;
// all tailoring is navigation interception + injected JS.

#pragma once

#include "CoreMinimal.h"
#include "Widgets/SCompoundWidget.h"

class SWebBrowser;
struct FWebNavigationRequest;

class SGldfWebPanel : public SCompoundWidget
{
public:
    SLATE_BEGIN_ARGS(SGldfWebPanel) {}
    SLATE_END_ARGS()

    void Construct(const FArguments& InArgs);

private:
    bool HandleBeforeNavigation(const FString& Url, const FWebNavigationRequest& Request);
    void HandleLoadCompleted();

    /** If `Url` is a recognized viewer link (gldf.icu / iesna.eu), kick off
     *  the download + Interchange placement and return true. */
    bool TryPlaceFromViewerUrl(const FString& Url);

    TSharedPtr<SWebBrowser> Browser;
};
