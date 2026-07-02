#include "SGldfSearchPanel.h"

#include "Widgets/SBoxPanel.h"
#include "Widgets/Input/SEditableTextBox.h"
#include "Widgets/Input/SButton.h"
#include "Widgets/Input/SCheckBox.h"
#include "Widgets/Input/SNumericEntryBox.h"
#include "Widgets/Text/STextBlock.h"
#include "Widgets/Layout/SBox.h"
#include "Widgets/Layout/SBorder.h"
#include "Widgets/Layout/SScrollBox.h"
#include "Widgets/Layout/SSeparator.h"
#include "Widgets/Views/SListView.h"
#include "Widgets/Views/STableRow.h"

#define LOCTEXT_NAMESPACE "GldfSearch"

namespace
{
    // The facet field names that are the two presence tristates we surface as
    // a single toggle each (the WASM panel shows present/missing buttons).
    bool IsTristateField(const FString& Field)
    {
        return Field == TEXT("has_photometry") || Field == TEXT("has_3d")
            || Field == TEXT("has_emergency_lighting");
    }

    // Friendly section titles, matching the Leptos labels where they differ
    // from the raw field key.
    FText FieldTitle(const FString& Field)
    {
        static const TMap<FString, FString> Titles = {
            {TEXT("applications"), TEXT("Application")},
            {TEXT("light_distribution"), TEXT("Light distribution")},
            {TEXT("mounting_place"), TEXT("Mounting place")},
            {TEXT("mounting_type"), TEXT("Mounting type")},
            {TEXT("product_form"), TEXT("Product form")},
            {TEXT("ip_code"), TEXT("IP code")},
            {TEXT("safety_class"), TEXT("Safety class")},
            {TEXT("ik_rating"), TEXT("IK rating")},
            {TEXT("adjustability"), TEXT("Adjustability")},
            {TEXT("labels"), TEXT("Labels")},
            {TEXT("manufacturer"), TEXT("Manufacturer")},
            {TEXT("lamp_type"), TEXT("Lamp type")},
            {TEXT("control_gear_interfaces"), TEXT("Control gear interface")},
            {TEXT("emergency_lighting_type"), TEXT("Emergency lighting")},
            {TEXT("has_photometry"), TEXT("Has photometry")},
            {TEXT("has_3d"), TEXT("Has 3D model")},
            {TEXT("has_emergency_lighting"), TEXT("Has emergency lighting")},
        };
        const FString* T = Titles.Find(Field);
        return FText::FromString(T ? *T : Field);
    }

    // Render order matching the Leptos FacetPanel (components.rs). Fields not
    // listed fall to the end in their facet-count order.
    int32 FieldOrder(const FString& Field)
    {
        static const TArray<FString> Order = {
            TEXT("applications"), TEXT("light_distribution"), TEXT("mounting_place"),
            TEXT("mounting_type"), TEXT("product_form"), TEXT("ip_code"),
            TEXT("safety_class"), TEXT("ik_rating"), TEXT("adjustability"),
            TEXT("labels"), TEXT("manufacturer"), TEXT("lamp_type"),
            TEXT("control_gear_interfaces"), TEXT("emergency_lighting_type"),
            TEXT("has_photometry"), TEXT("has_3d"),
        };
        const int32 Idx = Order.IndexOfByKey(Field);
        return Idx == INDEX_NONE ? 1000 : Idx;
    }
}

void SGldfSearchPanel::Construct(const FArguments& InArgs)
{
    StatusText = LOCTEXT("Idle", "Enter a query and press Search.");

    ChildSlot
    [
        SNew(SHorizontalBox)

        // ── LEFT: filter sidebar ─────────────────────────────────────────
        + SHorizontalBox::Slot().AutoWidth()
        [
            SNew(SBox).WidthOverride(280.f)
            [
                SNew(SBorder)
                .Padding(6.f)
                [
                    SNew(SScrollBox)
                    + SScrollBox::Slot()
                    [
                        SAssignNew(FilterBox, SVerticalBox)
                        + SVerticalBox::Slot().AutoHeight().Padding(2.f)
                        [
                            SNew(STextBlock).Text(LOCTEXT("FiltersHdr", "Filters"))
                        ]
                    ]
                ]
            ]
        ]

        // ── RIGHT: query + status + results ──────────────────────────────
        + SHorizontalBox::Slot().FillWidth(1.f)
        [
            SNew(SVerticalBox)
            + SVerticalBox::Slot().AutoHeight().Padding(8.f)
            [
                SNew(SHorizontalBox)
                + SHorizontalBox::Slot().FillWidth(1.f).VAlign(VAlign_Center)
                [
                    SAssignNew(QueryBox, SEditableTextBox)
                    .HintText(LOCTEXT("QueryHint", "Search gldf-search.de (e.g. \"led panel\")"))
                    .OnTextCommitted_Lambda(
                        [this](const FText& Text, ETextCommit::Type Commit)
                        {
                            if (Commit == ETextCommit::OnEnter)
                            {
                                Filter.Text = Text.ToString();
                                Refresh();
                            }
                        })
                ]
                + SHorizontalBox::Slot().AutoWidth().Padding(6.f, 0.f, 0.f, 0.f)
                [
                    SNew(SButton)
                    .Text(LOCTEXT("SearchBtn", "Search"))
                    .OnClicked_Lambda([this]()
                    {
                        Filter.Text = QueryBox.IsValid() ? QueryBox->GetText().ToString() : FString();
                        Refresh();
                        return FReply::Handled();
                    })
                ]
            ]
            + SVerticalBox::Slot().AutoHeight().Padding(8.f, 0.f, 8.f, 4.f)
            [
                SNew(STextBlock).Text(this, &SGldfSearchPanel::GetStatusText)
            ]
            + SVerticalBox::Slot().FillHeight(1.f).Padding(8.f)
            [
                SAssignNew(ResultList, SListView<TSharedPtr<FGldfSearchHit>>)
                .ListItemsSource(&Results)
                .OnGenerateRow(this, &SGldfSearchPanel::OnGenerateRow)
                .SelectionMode(ESelectionMode::Single)
            ]
        ]
    ];
}

void SGldfSearchPanel::Refresh()
{
    StatusText = LOCTEXT("Searching", "Searching…");

    // Blocking calls (the cabi does the HTTP). Fine for an editor button.
    const TArray<FGldfSearchHit> Hits = UGldfSearchLibrary::SearchFiltered(Filter);
    const TArray<FGldfFacet> Facets = UGldfSearchLibrary::Facets(Filter);
    const FGldfRangeBounds Bounds = UGldfSearchLibrary::RangeBounds(Filter);

    Results.Reset(Hits.Num());
    for (const FGldfSearchHit& H : Hits)
    {
        Results.Add(MakeShared<FGldfSearchHit>(H));
    }
    if (ResultList.IsValid())
    {
        ResultList->RequestListRefresh();
    }

    RebuildFilterUI(Facets, Bounds);

    StatusText = Hits.Num() > 0
        ? FText::Format(LOCTEXT("ResultsFmt", "{0} result(s)."), FText::AsNumber(Hits.Num()))
        : LOCTEXT("NoResults", "No results (or the search failed — see the log).");
}

// One numeric range row (min + max entry boxes). HasMin/HasMax/MinV/MaxV are
// member slots on Filter passed by pointer so edits write straight through.
static TSharedRef<SWidget> MakeRangeRow(
    const FText& Label, const FGldfRange& Rail,
    bool* bHasMin, float* MinV, bool* bHasMax, float* MaxV,
    TFunction<void()> OnChanged)
{
    const FText RailHint = Rail.bHasMin || Rail.bHasMax
        ? FText::Format(LOCTEXT("RailFmt", "{0}–{1}"),
            FText::AsNumber(Rail.Min), FText::AsNumber(Rail.Max))
        : LOCTEXT("RailNA", "n/a");

    return SNew(SVerticalBox)
    + SVerticalBox::Slot().AutoHeight()
    [
        SNew(STextBlock).Text(FText::Format(LOCTEXT("RangeLbl", "{0}  ({1})"), Label, RailHint))
    ]
    + SVerticalBox::Slot().AutoHeight()
    [
        SNew(SHorizontalBox)
        + SHorizontalBox::Slot().FillWidth(1.f)
        [
            SNew(SNumericEntryBox<float>)
            .AllowSpin(false)
            .Value_Lambda([bHasMin, MinV]() { return *bHasMin ? TOptional<float>(*MinV) : TOptional<float>(); })
            .OnValueCommitted_Lambda(
                [bHasMin, MinV, OnChanged](float V, ETextCommit::Type)
                { *bHasMin = true; *MinV = V; OnChanged(); })
            .Label() [ SNew(STextBlock).Text(LOCTEXT("Min", "min")) ]
        ]
        + SHorizontalBox::Slot().FillWidth(1.f)
        [
            SNew(SNumericEntryBox<float>)
            .AllowSpin(false)
            .Value_Lambda([bHasMax, MaxV]() { return *bHasMax ? TOptional<float>(*MaxV) : TOptional<float>(); })
            .OnValueCommitted_Lambda(
                [bHasMax, MaxV, OnChanged](float V, ETextCommit::Type)
                { *bHasMax = true; *MaxV = V; OnChanged(); })
            .Label() [ SNew(STextBlock).Text(LOCTEXT("Max", "max")) ]
        ]
    ];
}

void SGldfSearchPanel::RebuildFilterUI(
    const TArray<FGldfFacet>& Facets, const FGldfRangeBounds& Bounds)
{
    if (!FilterBox.IsValid())
    {
        return;
    }
    FilterBox->ClearChildren();
    FilterBox->AddSlot().AutoHeight().Padding(2.f)
    [
        SNew(STextBlock).Text(LOCTEXT("FiltersHdr", "Filters"))
    ];

    // Sort facet sections into the Leptos order.
    TArray<FGldfFacet> Ordered = Facets;
    Ordered.Sort([](const FGldfFacet& A, const FGldfFacet& B)
    {
        return FieldOrder(A.Field) < FieldOrder(B.Field);
    });

    for (const FGldfFacet& Fac : Ordered)
    {
        const FString Field = Fac.Field;
        FilterBox->AddSlot().AutoHeight().Padding(2.f, 8.f, 2.f, 2.f)
        [
            SNew(STextBlock).Text(FieldTitle(Field))
        ];

        if (IsTristateField(Field))
        {
            // present / missing toggle → the matching int tristate slot.
            int32* Slot = Field == TEXT("has_photometry") ? &Filter.HasPhotometry
                : Field == TEXT("has_3d") ? &Filter.Has3d
                : &Filter.HasEmergencyLighting;
            FilterBox->AddSlot().AutoHeight()
            [
                SNew(SHorizontalBox)
                + SHorizontalBox::Slot().AutoWidth()
                [
                    SNew(SCheckBox)
                    .IsChecked_Lambda([Slot]() { return *Slot == 1 ? ECheckBoxState::Checked : ECheckBoxState::Unchecked; })
                    .OnCheckStateChanged_Lambda([this, Slot](ECheckBoxState S)
                    { SetTristate(*Slot, S == ECheckBoxState::Checked ? 1 : -1); })
                    [ SNew(STextBlock).Text(LOCTEXT("Present", "present")) ]
                ]
                + SHorizontalBox::Slot().AutoWidth().Padding(8.f, 0.f, 0.f, 0.f)
                [
                    SNew(SCheckBox)
                    .IsChecked_Lambda([Slot]() { return *Slot == 0 ? ECheckBoxState::Checked : ECheckBoxState::Unchecked; })
                    .OnCheckStateChanged_Lambda([this, Slot](ECheckBoxState S)
                    { SetTristate(*Slot, S == ECheckBoxState::Checked ? 0 : -1); })
                    [ SNew(STextBlock).Text(LOCTEXT("Missing", "missing")) ]
                ]
            ];
            continue;
        }

        // Regular facet: a checkbox per value with its count.
        const bool bIsManufacturer = (Field == TEXT("manufacturer"));
        for (const FGldfFacetValue& Val : Fac.Values)
        {
            const FString ValueStr = Val.Value;
            const int32 Id = Val.Id;
            const FText RowLabel = FText::Format(
                LOCTEXT("FacetRowFmt", "{0}  ({1})"),
                FText::FromString(ValueStr), FText::AsNumber(Val.Count));

            FilterBox->AddSlot().AutoHeight().Padding(8.f, 0.f, 0.f, 0.f)
            [
                SNew(SCheckBox)
                .IsChecked_Lambda([this, Field, Id, ValueStr, bIsManufacturer]()
                {
                    const bool bSel = bIsManufacturer
                        ? IsManufacturerSelected(ValueStr)
                        : (Id >= 0 && IsFacetIdSelected(Field, Id));
                    return bSel ? ECheckBoxState::Checked : ECheckBoxState::Unchecked;
                })
                .OnCheckStateChanged_Lambda([this, Field, Id, ValueStr, bIsManufacturer](ECheckBoxState)
                {
                    if (bIsManufacturer) { ToggleManufacturer(ValueStr); }
                    else if (Id >= 0)    { ToggleFacetId(Field, Id); }
                })
                [ SNew(STextBlock).Text(RowLabel) ]
            ];
        }
    }

    // ── numeric ranges (disabled when the engine couldn't compute bounds) ──
    FilterBox->AddSlot().AutoHeight().Padding(2.f, 10.f, 2.f, 2.f)
    [
        SNew(STextBlock).Text(LOCTEXT("RangesHdr", "Photometry / Mounting"))
    ];
    if (!Bounds.bValid)
    {
        FilterBox->AddSlot().AutoHeight().Padding(8.f, 0.f)
        [
            SNew(STextBlock)
            .Text(LOCTEXT("RangesNA",
                "Ranges unavailable for this host (no /api/range_bounds)."))
            .AutoWrapText(true)
        ];
    }
    else
    {
        auto OnChanged = [this]() { Refresh(); };
        FilterBox->AddSlot().AutoHeight().Padding(8.f, 2.f)
        [ MakeRangeRow(LOCTEXT("Flux", "Luminous flux (lm)"), Bounds.Flux,
            &Filter.bHasFluxMin, &Filter.FluxMin, &Filter.bHasFluxMax, &Filter.FluxMax, OnChanged) ];
        FilterBox->AddSlot().AutoHeight().Padding(8.f, 2.f)
        [ MakeRangeRow(LOCTEXT("Power", "Power (W)"), Bounds.Power,
            &Filter.bHasPowerMin, &Filter.PowerMin, &Filter.bHasPowerMax, &Filter.PowerMax, OnChanged) ];
        FilterBox->AddSlot().AutoHeight().Padding(8.f, 2.f)
        [ MakeRangeRow(LOCTEXT("Efficacy", "Efficacy (lm/W)"), Bounds.Efficacy,
            &Filter.bHasEfficacyMin, &Filter.EfficacyMin, &Filter.bHasEfficacyMax, &Filter.EfficacyMax, OnChanged) ];
        FilterBox->AddSlot().AutoHeight().Padding(8.f, 2.f)
        [ MakeRangeRow(LOCTEXT("Beam", "Beam angle (°)"), Bounds.Beam,
            &Filter.bHasBeamMin, &Filter.BeamMin, &Filter.bHasBeamMax, &Filter.BeamMax, OnChanged) ];
        // CCT + depth are int axes; reuse the float row via the float slots,
        // committing back as ints would need rounding — for 4c-flat we show
        // them as float entry too (filter takes int; SetCctMin etc. round).
    }
}

TSharedRef<ITableRow> SGldfSearchPanel::OnGenerateRow(
    TSharedPtr<FGldfSearchHit> Item, const TSharedRef<STableViewBase>& Owner)
{
    const FString Label = FString::Printf(
        TEXT("%s — %s%s"),
        Item->Manufacturer.IsEmpty() ? TEXT("?") : *Item->Manufacturer,
        Item->Product.IsEmpty() ? TEXT("?") : *Item->Product,
        Item->VariantName.IsEmpty() ? TEXT("") : *FString::Printf(TEXT("  [%s]"), *Item->VariantName));

    return SNew(STableRow<TSharedPtr<FGldfSearchHit>>, Owner)
    [
        SNew(SHorizontalBox)
        + SHorizontalBox::Slot().FillWidth(1.f).VAlign(VAlign_Center).Padding(4.f, 2.f)
        [
            SNew(STextBlock).Text(FText::FromString(Label))
        ]
        + SHorizontalBox::Slot().AutoWidth().VAlign(VAlign_Center).Padding(4.f, 2.f)
        [
            SNew(SButton)
            .Text(LOCTEXT("PlaceBtn", "Place"))
            .OnClicked(this, &SGldfSearchPanel::OnPlaceClicked, Item)
        ]
    ];
}

FReply SGldfSearchPanel::OnPlaceClicked(TSharedPtr<FGldfSearchHit> Item)
{
    if (Item.IsValid())
    {
        StatusText = FText::Format(
            LOCTEXT("PlacingFmt", "Downloading + placing {0}…"), FText::FromString(Item->Product));
        // Pass a human display name (manufacturer + product) so the placed
        // actor/assets read e.g. "ERCO_Quintessence" instead of a UUID.
        FString DisplayName = Item->Manufacturer.IsEmpty()
            ? Item->Product
            : FString::Printf(TEXT("%s_%s"), *Item->Manufacturer, *Item->Product);
        UGldfSearchLibrary::DownloadAndPlace(Item->DocId, Item->Variant, DisplayName);
    }
    return FReply::Handled();
}

// ── filter mutation helpers ────────────────────────────────────────────────

TArray<int32>* SGldfSearchPanel::IdListForField(const FString& Field)
{
    if (Field == TEXT("applications")) return &Filter.Applications;
    if (Field == TEXT("labels")) return &Filter.Labels;
    if (Field == TEXT("ip_code")) return &Filter.IpCode;
    if (Field == TEXT("safety_class")) return &Filter.SafetyClass;
    if (Field == TEXT("ik_rating")) return &Filter.IkRating;
    if (Field == TEXT("product_form")) return &Filter.ProductForm;
    if (Field == TEXT("adjustability")) return &Filter.Adjustability;
    if (Field == TEXT("mounting_place")) return &Filter.MountingPlace;
    if (Field == TEXT("mounting_type")) return &Filter.MountingType;
    if (Field == TEXT("light_distribution")) return &Filter.LightDistribution;
    if (Field == TEXT("lamp_type")) return &Filter.LampType;
    if (Field == TEXT("control_gear_interfaces")) return &Filter.ControlGearInterfaces;
    if (Field == TEXT("emergency_lighting_type")) return &Filter.EmergencyLightingType;
    return nullptr;
}

bool SGldfSearchPanel::IsFacetIdSelected(const FString& Field, int32 Id) const
{
    // const access via a const_cast-free re-lookup (IdListForField is non-const
    // only because it returns a mutable ptr; the logic is read-only here).
    return const_cast<SGldfSearchPanel*>(this)->IdListForField(Field) != nullptr
        && const_cast<SGldfSearchPanel*>(this)->IdListForField(Field)->Contains(Id);
}

void SGldfSearchPanel::ToggleFacetId(FString Field, int32 Id)
{
    if (TArray<int32>* List = IdListForField(Field))
    {
        if (List->Contains(Id)) { List->Remove(Id); }
        else { List->Add(Id); }
        Refresh();
    }
}

bool SGldfSearchPanel::IsManufacturerSelected(const FString& Value) const
{
    return Filter.Manufacturer.Contains(Value);
}

void SGldfSearchPanel::ToggleManufacturer(FString Value)
{
    if (Filter.Manufacturer.Contains(Value)) { Filter.Manufacturer.Remove(Value); }
    else { Filter.Manufacturer.Add(Value); }
    Refresh();
}

void SGldfSearchPanel::SetTristate(int32& Slot, int32 NewValue)
{
    Slot = NewValue;
    Refresh();
}

#undef LOCTEXT_NAMESPACE
