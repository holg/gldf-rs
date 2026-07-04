# Enhancing GLDF Display: Viewer Gaps, Type-Layer Cleanup, and the Real Schema Limits

## Context

This note documents what the `gldf-rs-wasm` viewer currently shows
versus what real vendor GLDF files actually carry, using the LEDVANCE
BIOLUX HCL Downlight Zigbee Gen 2 file as the running example
(`light-other-rs/tests/data/reluxnet/ledvance/family/ldv___biolux_hcl_dl/ldv___4058075724600.gldf`,
emitted by `reluxnet::convert::gldf`).

It supersedes an earlier draft that conflated three different problems
into one "schema bug" narrative. After cross-checking against
`tests/data/gldf-1.0.0-rc-3.xsd`, the picture is clearer: most of the
visible loss is in the **viewer**, a smaller part is in the **Rust
type layer**, and only a narrow slice is a genuine **schema** limit.

**Hard constraint:** every change has to keep producing GLDF that
validates against the official XSD at
`https://gldf.io/xsd/gldf/1.0.0/gldf.xsd`. We do not extend the schema
unilaterally; we live within its choice space and use
`CustomProperties` for anything outside it.

---

## Three layers, three different problems

### Layer A — The viewer drops most of what the file carries

This is the largest source of perceived data loss and the cheapest to
fix. Nothing about it requires schema or type changes.

**A1. Multi-locale display is broken everywhere.**
Every editor renders `.locale.first()` and ignores the rest. The
BIOLUX file carries seven languages (cs / de / en / es / fr / it / ru)
on `ProductMetaData.Name`, `Description`, `ProductSerie.Name`,
`Variant.Name`, `Variant.Description`. Users see one language. The
existing `crates/gldf-rs-wasm/src/components/locale_input.rs`
component is exported from `mod.rs` and never rendered anywhere.
**Fix:** wire `LocaleInput` (or a global active-language selector) into
every `LocaleFoo` field, plus a per-field "show all locales" affordance
for completeness.

**A2. `CustomProperties` has no UI at all.**
The BIOLUX file has 29 `<Property>` entries under
`Variant/DescriptiveAttributes/CustomProperties`, including the full
multilingual `reluxnet:description` JSON, GAV id arrays, family link,
dimensions, and equipment metadata. The Rust types parse them; no tab
or component displays them. This is the single biggest hidden payload.
**Fix:** add a "Properties" tab that lists `id`, multilingual `Name`,
`PropertySource`, and the `Value` (or `FileReference`) — read-only is
fine for v1.

**A3. Variant detail is incomplete.**
`variant_editor.rs` does not render `Geometry.ModelGeometryReference`
or the variant's `ProductSeries` link, even though both are modeled
in `gldf-rs-lib`. Without the series link, the family-grouping concept
GLDF actually offers (see "ProductSeries vs. Variant" below) is
invisible.

**A4. `Mechanical.IKRating` has no editor.**
The file says `<IKRating>IK03</IKRating>`. The field exists on the
Rust type. There is no UI tab for `Mechanical` at all — `electrical_editor.rs`
covers electrical, nothing covers mechanical.

**A5. Tunable-white range is hidden.**
`FixedLightSource.ColorInformation.ColorTemperatureAdjustingRange`
(2700 → 6500 K in the BIOLUX file) is the defining feature of a
tunable-white luminaire. The light-source editor shows the scalar
`CorrelatedColorTemperature` only.

**A6. `GeneralDefinitions` sub-trees are not browsable.**
`Photometries`, `Equipments`, `Emitters`, `Geometries` are reachable
only indirectly via variants. A standalone tab listing each by id
(with cross-references back to the variants that consume them) would
mirror the file's actual structure.

### Layer B — Rust types diverge from the XSD

Two `gldf-rs-lib` types use a single-locale wrapper where the XSD
specifies the multi-locale type. This is a **type-layer bug**, not a
schema gap.

**B1. `ChangeableLightSource.Description`** is `Option<Locale>` in
`crates/gldf-rs-lib/src/gldf/general_definitions/lightsources.rs:303`,
but the XSD declares the element with `type="Locale"` at
`gldf-1.0.0-rc-3.xsd:938` — and "Locale" in XSD-land is the
**multi-locale** complex type defined at line 5571 (a sequence of
`<Locale language="…">` children).

In Rust the project uses `LocaleFoo` for that same complex type and
`Locale` for the inner single-language element. So the field should be
`Option<LocaleFoo>`. `FixedLightSource` already has it right
(`pub description: Option<LocaleFoo>`, line 347).

The earlier draft of this doc framed this as an "asymmetry in the
schema". That was wrong: the schema is symmetric, the Rust types
aren't.

**B2. `ChangeableLightSource.Name`** at line 295 is `pub name: Locale`,
but the XSD again uses the multi-locale complex type. Same fix:
`LocaleFoo`.

**Migration:** because `LocaleFoo` parses the same XML the multi-Locale
shape produces, the swap is mechanical. A `#[serde(untagged)]` enum is
**not needed** here — there's no ambiguity in the wire format, only a
mistyped Rust struct.

### Layer C — Genuine schema limits (XSD-bounded, work around with CustomProperties)

These are real, but the "fix" is **not** to invent attributes: those
would fail XSD validation. The fix is to recognize the design intent
and use the schema's own escape hatch (`CustomProperties`) cleanly.

**C1. `Applications`, `ApprovalMarks`, `Labels`, `ProductForm` are
controlled vocabularies, not free text.**
- `Application` (XSD line 4902): `xs:string` restricted to a closed
  enumeration of ~150 hierarchical labels like
  `"Interior: Office: Group Offices"`. The hierarchy is encoded in
  the colon-separated path itself.
- `Label` (line 4870): enum of 14 values (`CE`, `GS`, `ENEC`, `UL`, …).
- `ApprovalMark` (line 4842): plain `xs:string` (not enumerated, but
  language-neutral by intent — `"VDE"` doesn't translate).
- `ProductForm` (line 4455): enum of 10 values (`Round`, `Cuboid`, …).

The earlier draft proposed adding `@id`, `@language`, `@tier`
attributes to these elements. **That would break XSD compliance** —
the schema permits only the enumerated string content with no
attributes. The implication is: GLDF deliberately treats these as
language-neutral controlled vocabularies, with the hierarchy baked
into the enumeration values.

**Where this actually hurts:** vendor source data (Reluxnet GAV ids,
DIALux activity codes) has its own taxonomy that doesn't map 1:1 to
the GLDF enumeration. The current `reluxnet::convert::gldf` correctly
handles this by:
1. Picking the closest GLDF enumeration value(s) for `<Application>`.
2. Stashing the original GAV id list in `CustomProperties` under
   `reluxnet:application*` so round-trip stays lossless.

That's the right pattern, not a workaround. Document it as such.

**C2. `ProductForm` cannot be localized.** Single `xs:string` enum,
no Locale wrapper. If a UI needs to display "Quader" instead of
"Cuboid", the translation table lives outside GLDF — same as for any
controlled vocabulary. Not a bug.

**C3. `CustomProperties.Value` is a single `xs:string`.**
`Property/Name` is type `Locale` (multi-locale, per XSD line 5529),
so the property *name* can carry seven languages. But `Value` is
`xs:string` once, with no locale. A vendor with multi-locale property
values (like the BIOLUX file's `reluxnet:description` carrying seven
language strings as JSON) has two options inside the schema:
- Encode the multi-locale payload as JSON in `Value` (current
  approach — opaque to generic consumers but XSD-valid).
- Emit one `<Property>` per language, distinguished by `id` suffix
  or by the localized `Name`.

Neither is elegant. The current converter chose JSON-in-Value. That's
fine; consumers who care about it know to look. Don't propose schema
changes here; just expose Properties in the viewer (Layer A2) so the
data is at least visible.

---

## ProductSeries vs. Variant — what GLDF *does* offer for product families

A product family in GLDF is **not** modeled as a single Variant with
sub-options. The schema's family concept is `ProductSeries` /
`ProductSerie`, defined under `GeneralDefinitions/ProductSeries` and
referenced from both `ProductMetaData` and individual `Variant`
elements. Each `Variant` is a full SKU (own `productNumber`, `gtin`,
mountings, geometry); the `ProductSeries` reference groups them.

The BIOLUX file demonstrates the pattern: one `<Variant>` (this
particular SKU, GTIN 4058075724600) plus a `<ProductSeries>` block
naming the series ("BIOLUX HCL DOWNLIGHT ZIGBEE GEN 2", id `10093`)
in seven languages. Sibling SKUs (DN150 M, DN200 S, …) would be
emitted as additional `<Variant>` entries in the same file, all
referencing the same `ProductSerie@id`.

**Implication for the viewer:** the variant tab needs to render the
series link (Layer A3) so users see how a SKU sits inside its family.
A future "family browser" UI would group variants by series id, but
that's an enhancement, not a fix.

---

## Migration plan

**Phase 1 — viewer-only (no schema, no breaking type changes).**
Tackle Layer A in this order, each as its own PR:

1. **Locale picker.** Wire `LocaleInput` into the existing editors that
   render `LocaleFoo` fields. Pick a global active-language signal in
   `state/gldf_state.rs`; default to the value of
   `Header.DefaultLanguage` if set, else `en`, else first available.
2. **CustomProperties tab.** Read-only list view of all `Property`
   entries: `id`, multi-locale `Name`, `PropertySource`, `Value` (with
   JSON pretty-print when it parses as JSON), or `FileReference`.
3. **Variant detail completeness.** Add geometry reference and
   `ProductSeries` link rendering to `variant_editor.rs`.
4. **Mechanical tab.** New editor file
   (`components/mechanical_editor.rs`) covering at minimum `IKRating`,
   `ProductForm`, `EmergencyBallastLifetime`, weight, dimensions.
5. **CCT range display.** Update `light_source_editor.rs` to render
   `ColorTemperatureAdjustingRange.Lower` / `Upper` next to the scalar
   CCT.
6. **GeneralDefinitions browser.** Tabs for `Photometries`,
   `Equipments`, `Emitters`, `Geometries` listing each by id, with
   back-references to consuming variants.

After Phase 1, the BIOLUX file is fully visible without changing a
single byte of emitted XML.

**Phase 2 — Rust type cleanup (Layer B).**
Promote `ChangeableLightSource.Name` and
`ChangeableLightSource.Description` from `Locale` to `LocaleFoo` in
`general_definitions/lightsources.rs`. This is XSD-aligning, not
XSD-extending — the wire format already accepts multi-Locale here;
the Rust types just modeled it wrong. Run the full test suite and the
round-trip fixtures to confirm no existing GLDF breaks.

**Phase 3 — converter cleanup (cross-repo, in `light-other-rs`).**
Once Phase 1 surfaces `CustomProperties` and Phase 2 fixes the
single-locale change-able-light-source types, revisit
`reluxnet::convert::gldf`. The `reluxnet:*` opaque-backup properties
remain the right tool for data the XSD genuinely cannot carry (GAV
ids, activity codes, multi-locale property values), but anything that
*can* land in a schema-native field should land there. Audit the
property list against the new viewer surfaces and shrink it where
possible.

---

## Summary — what we ask of which repo

* **`gldf-rs` (this repo), viewer crate:** all of Layer A. Six
  bite-sized PRs.
* **`gldf-rs` (this repo), lib crate:** Layer B. One PR. No schema
  change.
* **`light-other-rs`:** Layer C cleanup after Phase 1+2 land.
* **GLDF schema upstream:** **nothing**, deliberately. The earlier
  draft's `LocalizedTag` proposal would have added attributes to
  enumerated string elements, which the XSD does not allow. The right
  posture is to use the schema's `CustomProperties` channel for
  payload that exceeds the controlled vocabularies, and to surface
  that channel in the viewer.

The result is a viewer that shows everything a compliant GLDF file
carries, a Rust type layer that matches the XSD, and a converter that
uses `CustomProperties` only for data the schema genuinely cannot
represent — not as a workaround for things the viewer simply does not
render yet.
