# Batch processing: LDT/IES + JSON template → GLDF / IFC

The batch engine lives in the **library** (`gldf-rs-editor::batch`) and is
shared by two frontends:

- `gldf-cli batch` — headless (CI, scripting)
- `gldf-batch-egui` — desktop app (crates/gldf-rs-egui, bin `gldf-batch-egui`)

## Pipeline (per input file)

1. Parse the `.ldt` / `.ies` → `LdtMetadata` (`gldf_rs::convert::ldt_metadata`).
2. Substitute placeholders in the template's string values.
3. Expand `{"@file": "<path>"}` byte refs (paths relative to the template
   file; the reserved path `"@input"` is the current input's bytes).
4. Wizard template: deserialize `WizardDraft` → autofill blanks from the
   LDT → `apply_wizard`. Script template: deserialize `GldfAction[]` →
   apply each action.
5. Write `<stem>.gldf` (`GldfState::save_to_buf`) and/or `<stem>.ifc`
   (`GldfState::export_ifc` — embeds L3D mesh + photometry files).

## Template formats

**Wizard template** — JSON object, same shape as `gldf-cli wizard --draft`.
All fields optional (`#[serde(default)]`). Blanks are auto-filled from each
input LDT: photometry bytes/filename, header manufacturer, light-source
name (lamp type) and rated power, and a default `Primary` emitter bound to
the photometry + light source. `{}` is a valid template.

```json
{
  "header_author": "ACME batch",
  "header_manufacturer": "{ldt.manufacturer}",
  "ls_kind": "Fixed",
  "variant_name": "{ldt.name}",
  "mounting_type": "ceiling/recessed",
  "mounting_recessed_depth_mm": "120",
  "geometry_bytes": { "@file": "shared_housing.l3d" },
  "geometry_filename": "shared_housing.l3d"
}
```

**Action script** — JSON array, same shape as `gldf-cli apply --script`.
Nothing is auto-filled; reference the input explicitly:

```json
[
  { "action": "AddPhotometryFile",
    "data": { "bytes": { "@file": "@input" }, "filename": "{input.filename}" } },
  { "action": "SetManufacturer", "data": "{ldt.manufacturer}" }
]
```

## Placeholders

`{input.filename}`, `{input.stem}`, `{ldt.name}`, `{ldt.manufacturer}`,
`{ldt.luminaire_number}`, `{ldt.lamp_type}`, `{ldt.lamp_count}`,
`{ldt.watts}`, `{ldt.lumens}`, `{ldt.measurement_report}`.
Missing values → empty string; unknown `{…}` passes through.

## CLI

```bash
gldf-cli batch <files-or-dirs...> \
  --template tpl.json --out-dir out/ \
  --ifc            # also write .ifc per input
  --no-gldf        # IFC-only
  --validate       # XSD-validate every written .gldf (exit 1 on findings)
```

Directories are searched recursively for `.ldt`/`.ies`. Stem collisions
get `_2`, `_3`, … suffixes.

## Desktop app

Batch conversion is the **"Batch convert" page of the GLDF editor**
(there is no separate batch binary):

```bash
cargo run -p gldf-rs-egui --bin gldf-editor-egui
```

While that page is open, dropped LDT/IES/ULD files or folders feed the
input list and a dropped `.json` becomes the template. The **template
editor** (New… / Edit…) has a Form tab (all wizard fields,
mounting/driver, shared `@file` assets, click-to-copy placeholders) and a
two-way raw JSON tab; Save makes the file the active template. Runs happen
on a worker thread with per-file status, progress, and Cancel.

## Library API (gldf-rs-editor)

- `BatchTemplate::parse` — classify wizard object vs action script
- `process_one(bytes, filename, &template, &resolver, &options)` → GLDF bytes + IFC text
- `collect_inputs`, `run_batch(&BatchJob, callback)` — fs walker + runner
  (callback returns `false` to cancel); compiled out on wasm32
- `TemplateDraft` — form model behind the egui template editor
- `GldfState::export_ifc()` — IFC with embedded L3D mesh + photometries
