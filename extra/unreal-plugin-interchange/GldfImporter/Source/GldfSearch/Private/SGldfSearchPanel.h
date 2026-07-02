// Native Slate panel for the GLDF search demo: a filter sidebar (facet
// checklists + the has_3d/has_photometry/has_emergency tristates + numeric
// range boxes) on the left, the results list (with per-row Place) on the
// right — mirroring the Leptos/WASM FacetPanel. Drives UGldfSearchLibrary
// directly (no Python, no UMG asset). Lives in a nomad tab
// (Window > Tools > GLDF Search).
//
// 4c-flat: flat facet checklists + tristates + ranges. The Application
// 3-level tree and removable filter chips are a later pass (4c-tree).

#pragma once

#include "CoreMinimal.h"
#include "Widgets/SCompoundWidget.h"
#include "GldfSearchLibrary.h"  // FGldfSearchHit, FGldfSearchFilter, FGldfFacet, FGldfRangeBounds

class SEditableTextBox;
class SVerticalBox;
template <typename ItemType> class SListView;
class ITableRow;
class STableViewBase;

class SGldfSearchPanel : public SCompoundWidget
{
public:
    SLATE_BEGIN_ARGS(SGldfSearchPanel) {}
    SLATE_END_ARGS()

    void Construct(const FArguments& InArgs);

private:
    /** The live filter state. Every UI change mutates this, then Refresh(). */
    FGldfSearchFilter Filter;

    /** Last query's results (shared ptrs, as SListView requires). */
    TArray<TSharedPtr<FGldfSearchHit>> Results;

    TSharedPtr<SEditableTextBox> QueryBox;
    TSharedPtr<SListView<TSharedPtr<FGldfSearchHit>>> ResultList;
    /** The facet/range controls live here; rebuilt from facet counts on each
     *  Refresh so only present values + valid ranges show (like the WASM UI). */
    TSharedPtr<SVerticalBox> FilterBox;

    FText StatusText;

    /** Re-run SearchFiltered + Facets + RangeBounds against the current
     *  Filter, then rebuild the results list + the filter sidebar. */
    void Refresh();

    /** (Re)build the facet/range widgets in FilterBox from the given facet
     *  counts + range bounds. */
    void RebuildFilterUI(const TArray<FGldfFacet>& Facets, const FGldfRangeBounds& Bounds);

    /** SListView row factory. */
    TSharedRef<ITableRow> OnGenerateRow(
        TSharedPtr<FGldfSearchHit> Item, const TSharedRef<STableViewBase>& Owner);
    FReply OnPlaceClicked(TSharedPtr<FGldfSearchHit> Item);

    FText GetStatusText() const { return StatusText; }

    // ── filter mutation helpers (map a facet field name → the Filter slot) ──

    /** The TArray<int32> id-list in Filter for a given cabi facet field name,
     *  or nullptr if the field isn't an id-list (manufacturer / tristates). */
    TArray<int32>* IdListForField(const FString& Field);

    /** Toggle an id in the list for `Field`; re-run on change. */
    void ToggleFacetId(FString Field, int32 Id);
    bool IsFacetIdSelected(const FString& Field, int32 Id) const;

    /** Manufacturer is a string list, not ids. */
    void ToggleManufacturer(FString Value);
    bool IsManufacturerSelected(const FString& Value) const;

    /** Cycle a tristate (-1 unset → 1 present → 0 missing → -1). The two
     *  facet fields has_photometry/has_3d come as "present"/"missing" values;
     *  we map the toggle onto the int tristate on Filter. */
    void SetTristate(int32& Slot, int32 NewValue);
};
