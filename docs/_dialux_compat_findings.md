# DiaLux GLDF import compatibility — findings & method

**Goal:** discover the rules DiaLux's GLDF importer enforces (beyond the XSD) so the
gldf.icu exporter can emit GLDF that DiaLux imports. DiaLux fails **silently** (no error),
is stricter than the schema / Relux / gldf.icu, and is the dominant consumer.

**Method:** single-variable test files + manual DiaLux import + record pass/fail.
Work from a KNOWN-IMPORTING baseline and add ONE feature back at a time (adding back from a
good baseline avoids the confound where one blocker masks another). Failing files are
renamed with a `__no_import` suffix so the corpus stays self-labeling.

Baseline reference luminaire: `chapter-01-slv` vs the known-good `SLV - Tria 2.gldf`
(same `example_007.l3d` geometry; parts `LEO0`, `LEO1` = light emitters, `Sensor` = sensor).

## Confirmed results (this luminaire)

### HARMLESS to DiaLux (present in `noSP.gldf`, which imports)
| Feature | Verdict |
|---|---|
| `targetModelType="l3d"` on `EmitterReference` | harmless (redundant for single model) |
| `ControlGears` + `ControlGearReference` | harmless |
| Same emitter id bound to 2 geometry objects in one variant | harmless |
| All-emergency-only variant (no normal emitter) | harmless |
| Blank `ProductNumber` / `Name` (whitespace) | harmless |
| `GldfCreationTimeCode` without `Z`, future date (2026) | harmless |
| `DefaultLanguage`, `Author`, multiple `FixedLightSource`, `GTIN`, `image/jpg`, rich `DescriptivePhotometry` | harmless |

### BLOCKER
| Feature | Verdict |
|---|---|
| Native `<Spectrums>` + `SpectrumReference`, authored as external `spectrum/text`, **360–830 nm, with `#` comment lines** | **breaks import** (silent). Removing it → imports. |

Schema note: GLDF documents the spectrum as *"5nm steps, in the range 380–780 nm."*
The authored file was 360–830 nm + comments → out of range/format. XSD does not validate
spectrum file content, so the gldf.net validator passes it.

### RESOLVED (all 3 imported in DiaLux)
| Question | Test file | Result |
|---|---|---|
| Is `levelOfDetail` required? | `chapter-01-slv-noSP-noLOD.gldf` | ✅ imports → **NOT required** |
| External spectrum corrected to 380–780? | `chapter-01-slv-specFile780.gldf` | ✅ imports → **external is fine in range** |
| Inline `<Intensity>` spectrum? | `chapter-01-slv-specInline.gldf` | ✅ imports → inline also works |

**Conclusion (ISOLATED):** the SOLE DiaLux blocker is **`#` comment lines in the
`spectrum/text` file**. Wavelength range is NOT a DiaLux constraint — `rangeOnly` (360–830 nm,
no comments) **imports fine**; `commentsOnly` (380–780 nm, with comments) **fails**.
`levelOfDetail` is not required.

| Test | Spectrum | DiaLux |
|---|---|---|
| `rangeOnly` | 360–830 nm, NO comments | ✅ import → range is fine |
| `commentsOnly` | 380–780 nm, WITH comments | ❌ → **comments are the blocker** |

Exporter rule: emit `spectrum/text` with **no comment lines**. (Staying in 380–780 nm is still
advisable for GLDF spec conformance, but DiaLux does not require it.)

Mounting features tested: `Pendant` (pendantLength attr) and `SurfaceMounted` both ✅ import.

## Test-file ledger
| File | Change vs original | DiaLux |
|---|---|---|
| `SLV - Tria 2.gldf` | reference good file | ✅ import |
| `chapter-01-slv.gldf` | original | ❌ |
| `chapter-01-slv-noTMT__no_import.gldf` | − targetModelType (still had spectrum) | ❌ (confounded) |
| `chapter-01-slv-v4fix__no_import.gldf` | fixed dup emitter id (still had spectrum) | ❌ (confounded) |
| `chapter-01-slv-named__no_import.gldf` | filled name (still had spectrum) | ❌ (confounded) |
| `chapter-01-slv-lod__no_import.gldf` | + LOD only (still had spectrum) | ❌ |
| `chapter-01-slv-noCG__no_import.gldf` | + LOD − ControlGear (kept spectrum) | ❌ |
| `chapter-01-slv-min.gldf` | minimal Tria-like, no spectrum | ✅ import |
| `chapter-01-slv-lod-cg-sp.gldf` | + LOD − CG − spectrum | ✅ import |
| `chapter-01-slv-noSP.gldf` | + LOD − spectrum (kept CG etc.) | ✅ import |
| `chapter-01-slv-noSP-noLOD.gldf` | − spectrum, no LOD | ✅ import |
| `chapter-01-slv-specFile780.gldf` | + LOD, spectrum→external 380–780 no comments | ✅ import |
| `chapter-01-slv-specInline.gldf` | + LOD, spectrum→inline `<Intensity>` 380–780 | ✅ import |

## Notes on TM-33 (`ldc/iesxml`)
GLDF `ldc/iesxml` content type = ANSI/IES TM-33-18 XML (carries photometry + can embed SPD +
some BIM properties). It is a *photometry* content type; its embedded SPD does NOT feed
GLDF's `<Spectrums>`/`SpectrumReference`. These files use eulumdat `.ldt` photometry + native
GLDF spectrum, so TM-33 is not in play. Switching spectra to TM-33-embedded is a separate
architecture; only worth it if DiaLux reads SPD from TM-33 (untested).

### TM-33 photometry compatibility (2026-06-12)

**DiaLux rejects** any GLDF whose `<Photometry>` references an `ldc/iesxml`
file with the error **"Corrupt Eulumdat file!"** — DiaLux's parser treats
TM-33 XML as malformed EULUMDAT instead of dispatching to a TM-33 reader.
Verified against `tmp/tm33-test/chapter-01-tm33.gldf` (produced via
`gldf-cli save --tm33`).

**Relux imports it fine.** So TM-33 is a Relux-or-other-consumer feature;
a TM-33-merged GLDF is DiaLux-incompatible regardless of the
`--dialux-compat` flag (the flag fixes the `#`-in-SPD blocker, not the
photometry content-type rejection).

The export-options dialog warns about this on the TM-33 checkbox; a
follow-up should add a `dialux_compat_check` finding that surfaces the
incompatibility on read (so the validation banner flags inbound GLDFs
that won't load in DiaLux).

## Validator rule (implemented in gldf.net)
`SpectrumDialuxCompatibilityRule` (`src/Gldf.Net/Validation/Rules/Container/`), registered in
`GldfContainerValidator`, new `ErrorType.SpectrumDialuxIncompatible`. Emits **Warning** when a
native spectrum has wavelengths outside 380–780 nm, or a `spectrum/text` file has `#` comment
lines. Inline `<Intensity>` is always checked; file-based spectra are checked only when the
container is loaded with `AssetLoadBehaviour.Load` (the `GldfValidate` tool now does this).
All 1026 existing tests still pass. Verified output:
- original chapter file → 2 warnings (comments + out-of-range)
- `rangeOnly` → out-of-range warning only; `commentsOnly` → comments warning only
- `specFile780` / `noSP` → clean

## New test files (pending DiaLux)
| File | Isolates | DiaLux |
|---|---|---|
| `chapter-01-slv-rangeOnly.gldf` | spectrum 360–830, NO comments | ⏳ |
| `chapter-01-slv-commentsOnly.gldf` | spectrum 380–780, WITH comments | ⏳ |
| `chapter-01-slv-feat-pendant.gldf` | Ceiling/Pendant pendantLength=500 | ⏳ |
| `chapter-01-slv-feat-surface.gldf` | Ceiling/SurfaceMounted | ⏳ |

Note: `pendantLength` is an **attribute** of `<Pendant>`, not a child element. Earlier
`exported-gldf-icu`/`slv_fake` failed XSD because gldf.icu emitted it as an element — separate
exporter bug to fix.

## Exporter rules
1. **CONFIRMED** — `spectrum/text` files must contain **NO comment lines (`#`)**. This is the
   only spectrum-related DiaLux import blocker. (Wavelength range is NOT a DiaLux constraint —
   360–830 nm imported fine; but keep 380–780 nm / 5 nm for GLDF spec conformance.)
   Inline `<Intensity>` also works.
2. ~~Always set `levelOfDetail`~~ — **NOT required** (DiaLux imports without it).
3. `Pendant` (pendantLength as **attribute**) and `SurfaceMounted` mountings import fine.
   (gldf.icu bug: it emitted `pendantLength` as a child element → fails XSD. Emit as attribute.)
4. (Add rows as new luminaire features are tested against DiaLux.)

---

# Consolidated analysis

## What was actually wrong (and what wasn't)
A batch of gldf.icu-authored luminaires (`chapter-*`) was schema-valid, imported in Relux and
gldf.icu, but **silently failed in DiaLux** (no error dialog). Through single-variable
bisection against a known-good reference (`SLV - Tria 2.gldf`) we isolated the cause to **one
thing**: the native spectrum's `spectrum/text` file contained `#` comment/header lines.
DiaLux's spectrum parser aborts the whole import on those, silently.

Crucially, several "obvious suspects" were proven **innocent** — do NOT waste effort on them:
- wavelength range outside 380–780 nm (DiaLux imported 360–830 nm fine)
- `targetModelType` on EmitterReference, `levelOfDetail` on GeometryFileReference
- ControlGear blocks, multiple light sources, GTIN, future timestamps, blank product name
- an all-emergency variant, the same emitter id bound twice in one variant

The early single-fix tests (`noTMT`, `v4fix`, `named`) all *looked* like failures for their
respective fixes, but were **confounded** — every one still carried the bad spectrum, so the
spectrum failed them regardless. Lesson for this kind of reverse-engineering: **bisect from a
known-GOOD baseline and add one feature back at a time**, never subtract one suspect at a time
from a broken file (one blocker masks all others).

## DiaLux import rules discovered so far
| # | Rule | Severity | Notes |
|---|---|---|---|
| 1 | `spectrum/text` files must contain **no `#` comment lines** | HARD blocker | silent fail; the actual root cause |
| 2 | Spectrum wavelengths *should* be 380–780 nm / 5 nm | advisory | GLDF spec recommendation; DiaLux tolerates 360–830 |
| 3 | `pendantLength` is an **attribute** of `<Pendant>` | XSD-hard | gldf.icu emitted it as a child element → fails XSD before DiaLux even sees it |
| — | `Pendant`, `SurfaceMounted`, `Recessed` mountings | OK | import fine |

# Notes for gldf-rs / gldf.icu (Rust) integration

Goal: replicate these checks (validation) and fixes (export sanitisation) in the Rust codebase.
Below is what we built in gldf.net (C#) and the language-agnostic algorithms to port.

## gldf.net changes made (reference implementation)
- `Validation/Model/ErrorType.cs` — added enum variant `SpectrumDialuxIncompatible`.
- `Validation/Rules/Container/SpectrumDialuxCompatibilityRule.cs` — **new** rule (the meat).
- `Validation/GldfContainerValidator.cs` — registered the rule in the default pipeline.
- `tools/GldfValidate/` — standalone CLI that loads the container **with asset bytes**
  (`AssetLoadBehaviour.Load`) so file-based spectra can actually be read, then prints every hint.

Design note worth carrying over: GLDF validation is split into independent *rules*
(`IContainerValidationRule { ValidateGldf(container) -> Vec<Hint> }`), each returning warnings/
errors. The container exposes the parsed product POCO **plus** a side-channel of asset bytes
(`Assets.Spectrums: Vec<ContainerFile{ file_name, bytes }>`). Spectra are referenced two ways:
inline `<Intensity wavelength=.. >value</Intensity>` OR `<SpectrumFileReference fileId=..>` that
points at a `<File id=.. >name</File>` whose bytes live in the container zip. A rule must resolve
`spectrumId -> fileId -> file name -> asset bytes` to inspect a file-based spectrum.

## Data model touchpoints (mirror in Rust)
```text
Product
 └ GeneralDefinitions
    ├ files:      Vec<File { id, content_type, file_type, name }>   // content_type "spectrum/text"
    └ spectrums:  Vec<Spectrum { id, def }>
                    def = SpectrumDef::File { file_id }            // external
                        | SpectrumDef::Inline(Vec<Intensity{wavelength:i32, value:f64}>)
Assets.spectrums: Vec<ContainerFile { file_name, bytes }>          // zip payloads
```

## Validation: DiaLux spectrum check (port of SpectrumDialuxCompatibilityRule)
```rust
const MIN_NM: i32 = 380;
const MAX_NM: i32 = 780;

pub enum DialuxHint {
    SpectrumHasComments { spectrum_id: String, file: String }, // HARD: DiaLux silent-fails
    SpectrumOutOfRange  { spectrum_id: String, nm: Vec<i32> },  // advisory only
}

pub fn check_spectra(p: &Product, assets: &Assets) -> Vec<DialuxHint> {
    let mut hints = Vec::new();
    for s in &p.general_definitions.spectrums {
        match &s.def {
            SpectrumDef::Inline(points) => {
                let bad = out_of_range(points.iter().map(|i| i.wavelength));
                if !bad.is_empty() { hints.push(DialuxHint::SpectrumOutOfRange { spectrum_id: s.id.clone(), nm: bad }); }
            }
            SpectrumDef::File { file_id } => {
                let Some(name) = p.file_name(file_id) else { continue };          // id -> name
                let Some(bytes) = assets.spectrum_bytes(name) else { continue };   // name -> bytes (needs full load)
                let text = String::from_utf8_lossy(bytes);
                if text.lines().any(|l| l.trim_start().starts_with('#')) {        // <-- the blocker
                    hints.push(DialuxHint::SpectrumHasComments { spectrum_id: s.id.clone(), file: name.to_owned() });
                }
                let nm = text.lines().map(str::trim)
                    .filter(|l| !l.is_empty() && !l.starts_with('#'))
                    .filter_map(|l| l.split([' ', '\t']).next())
                    .filter_map(|t| t.parse::<i32>().ok());
                let bad = out_of_range(nm);
                if !bad.is_empty() { hints.push(DialuxHint::SpectrumOutOfRange { spectrum_id: s.id.clone(), nm: bad }); }
            }
        }
    }
    hints
}

fn out_of_range(it: impl Iterator<Item = i32>) -> Vec<i32> {
    let mut v: Vec<i32> = it.filter(|w| *w < MIN_NM || *w > MAX_NM).collect();
    v.sort_unstable(); v.dedup(); v
}
```

## Export fix: make a spectrum DiaLux-safe (the important one for gldf.icu)
The exporter should simply never write comment/header lines into `.spd` payloads. To also
sanitise spectra that arrive from elsewhere:
```rust
/// Strip comment lines from a spectrum/text payload; keep `wavelength<ws>value` rows.
pub fn sanitize_spd(spd: &str) -> String {
    spd.lines()
       .filter(|l| !l.trim_start().starts_with('#'))
       .map(|l| format!("{l}\n"))
       .collect()
}
```
Optional spec-conformance pass (not required by DiaLux): resample/clamp to 380–780 nm, 5 nm.
Equivalent alternative that sidesteps the file entirely: emit the spectrum **inline** as
`<Intensity wavelength="380">..</Intensity> … <Intensity wavelength="780">..</Intensity>`
(both forms import in DiaLux).

## Other exporter fix
`pendantLength` must be serialised as an **attribute**: `<Pendant pendantLength="500"/>` —
NOT `<Pendant><pendantLength>500</pendantLength></Pendant>`. The element form fails GLDF XSD
(seen in `exported-gldf-icu`, `slv_fake`), so it never reaches DiaLux.

## Method notes (for extending the corpus)
- One variable per test file; build on a known-importing baseline; name failures `*__no_import`.
- A pure schema validator (gldf.net or otherwise) **cannot** catch these — the offending content
  is schema-legal. DiaLux-compat checks have to live as extra semantic rules + asset-content
  inspection (hence the need to load asset bytes, not just file names).
