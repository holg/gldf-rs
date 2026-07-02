//! Batch conversion: many `.ldt` / `.ies` photometries + one JSON
//! template → one GLDF (and optionally IFC) per input file.
//!
//! The template is either
//!
//! - a [`WizardDraft`] JSON **object** — the same shape `gldf-cli wizard
//!   --draft` consumes. Fields the template leaves out fall back first to
//!   `#[serde(default)]`, then to values auto-filled from the input LDT
//!   (photometry bytes, manufacturer, light-source name/power, a default
//!   emitter). A minimal template of `{}` therefore already produces a
//!   valid, wired-up GLDF per input file; or
//! - a [`GldfAction`] JSON **array** — the same shape `gldf-cli apply`
//!   consumes. Nothing is auto-filled; the script spells out every step.
//!
//! Two substitution passes run over the template per input file:
//!
//! 1. **Placeholders** in string values: `{input.filename}`,
//!    `{input.stem}`, `{ldt.name}`, `{ldt.manufacturer}`,
//!    `{ldt.luminaire_number}`, `{ldt.lamp_type}`, `{ldt.lamp_count}`,
//!    `{ldt.watts}`, `{ldt.lumens}`, `{ldt.measurement_report}` — values
//!    come from the parsed EULUMDAT/IES header. Missing values
//!    substitute as the empty string; unknown `{…}` sequences pass
//!    through verbatim.
//! 2. **`@file` refs** (see [`crate::script::expand_file_refs`]),
//!    resolved against the caller's [`FileResolver`] — plus the reserved
//!    path `"@input"`, which resolves to the current input file's bytes
//!    (e.g. `"photometry_bytes": {"@file": "@input"}`).
//!
//! The byte-level core ([`process_one`]) is platform-agnostic; the
//! directory walker and output writer ([`run_batch`], [`collect_inputs`])
//! are compiled out on wasm32, mirroring [`crate::script::FsResolver`].

use crate::action::GldfAction;
use crate::emitter_draft::EmitterDraft;
use crate::script::{expand_file_refs, FileResolver};
use crate::state::GldfState;
use crate::wizard::{apply_wizard, WizardDraft, WizardLightSourceKind};
use anyhow::{anyhow, bail, Context, Result};
use gldf_rs::convert::{ldt_metadata, LdtMetadata};

/// Reserved `@file` path that resolves to the current input file's bytes.
pub const INPUT_REF: &str = "@input";

/// A parsed batch template: a [`WizardDraft`] object or a [`GldfAction`]
/// script array. Kept as raw JSON because placeholders and `@file` refs
/// are substituted per input file before deserializing.
#[derive(Clone, Debug)]
pub enum BatchTemplate {
    Wizard(serde_json::Value),
    Script(serde_json::Value),
}

impl BatchTemplate {
    /// Parse template JSON, classifying by shape: object → wizard draft,
    /// array → action script.
    pub fn parse(raw: &str) -> Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(raw).context("parse template JSON")?;
        match &value {
            serde_json::Value::Object(_) => Ok(Self::Wizard(value)),
            serde_json::Value::Array(_) => Ok(Self::Script(value)),
            _ => bail!("template must be a WizardDraft object or a GldfAction array"),
        }
    }
}

/// Which artifacts [`process_one`] produces.
#[derive(Clone, Copy, Debug)]
pub struct BatchOptions {
    pub write_gldf: bool,
    pub write_ifc: bool,
}

impl Default for BatchOptions {
    fn default() -> Self {
        Self {
            write_gldf: true,
            write_ifc: false,
        }
    }
}

/// Result of converting one input file.
#[derive(Clone, Debug)]
pub struct BatchOutput {
    /// `.gldf` zip bytes (when [`BatchOptions::write_gldf`]).
    pub gldf: Option<Vec<u8>>,
    /// IFC STEP text (when [`BatchOptions::write_ifc`]).
    pub ifc: Option<String>,
    /// Luminaire name from the input photometry — for progress display.
    pub product_name: String,
}

/// Resolver wrapper that serves the reserved [`INPUT_REF`] path from the
/// current input file and delegates everything else.
struct InputResolver<'a> {
    input: &'a [u8],
    inner: &'a dyn FileResolver,
}

impl FileResolver for InputResolver<'_> {
    fn resolve(&self, path: &str) -> Result<Vec<u8>> {
        if path == INPUT_REF {
            Ok(self.input.to_vec())
        } else {
            self.inner.resolve(path)
        }
    }
}

/// Format a float placeholder without a trailing `.0` for whole numbers
/// (a `{ldt.watts}` of 24.0 reads better as `"24"` in a product name).
fn fmt_f64(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}

/// Build the placeholder table for one input file.
fn placeholder_table(input_filename: &str, meta: &LdtMetadata) -> Vec<(&'static str, String)> {
    let stem = std::path::Path::new(input_filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(input_filename)
        .to_string();
    vec![
        ("{input.filename}", input_filename.to_string()),
        ("{input.stem}", stem),
        ("{ldt.name}", meta.name.clone()),
        ("{ldt.manufacturer}", meta.manufacturer.clone().unwrap_or_default()),
        (
            "{ldt.luminaire_number}",
            meta.luminaire_number.clone().unwrap_or_default(),
        ),
        ("{ldt.lamp_type}", meta.lamp_type.clone().unwrap_or_default()),
        (
            "{ldt.lamp_count}",
            meta.lamp_count.map(|n| n.to_string()).unwrap_or_default(),
        ),
        ("{ldt.watts}", meta.watts.map(fmt_f64).unwrap_or_default()),
        ("{ldt.lumens}", meta.lumens.map(fmt_f64).unwrap_or_default()),
        (
            "{ldt.measurement_report}",
            meta.measurement_report.clone().unwrap_or_default(),
        ),
    ]
}

/// Walk the template JSON and substitute placeholders in every string
/// value. Object keys are left untouched (action tags / field names must
/// stay literal).
fn substitute_placeholders(value: &mut serde_json::Value, table: &[(&str, String)]) {
    match value {
        serde_json::Value::String(s) => {
            if s.contains('{') {
                let mut out = s.clone();
                for (k, v) in table {
                    if out.contains(k) {
                        out = out.replace(k, v);
                    }
                }
                *s = out;
            }
        }
        serde_json::Value::Object(map) => {
            for v in map.values_mut() {
                substitute_placeholders(v, table);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                substitute_placeholders(v, table);
            }
        }
        _ => {}
    }
}

/// Fill draft fields the template left empty from the input photometry,
/// so a `{}` template still yields a complete product: photometry bytes,
/// manufacturer, light-source name/power, and a default emitter bound to
/// the freshly added photometry + light source.
fn autofill_draft(
    draft: &mut WizardDraft,
    input_bytes: &[u8],
    input_filename: &str,
    meta: &LdtMetadata,
) {
    if draft.photometry_bytes.is_none() {
        draft.photometry_bytes = Some(input_bytes.to_vec());
        draft.photometry_filename = input_filename.to_string();
    }
    if draft.header_manufacturer.trim().is_empty() {
        draft.header_manufacturer = meta.manufacturer.clone().unwrap_or_default();
    }
    if draft.ls_name.trim().is_empty() {
        draft.ls_name = meta
            .lamp_type
            .clone()
            .unwrap_or_else(|| meta.name.clone());
    }
    if draft.ls_rated_power.trim().is_empty() {
        if let Some(watts) = meta.watts {
            draft.ls_rated_power = fmt_f64(watts);
        }
    }
    if draft.emitter_draft.is_none() {
        let kind = match draft.ls_kind {
            WizardLightSourceKind::Fixed => crate::action::LightEmitterKind::Fixed,
            WizardLightSourceKind::Changeable => crate::action::LightEmitterKind::Changeable,
            WizardLightSourceKind::MultiChannel => crate::action::LightEmitterKind::MultiChannel,
        };
        // Empty ids → the reducer auto-binds the last-added photometry /
        // light source, same convention the CLI wizard relies on.
        draft.emitter_draft = Some(EmitterDraft {
            child_name: "Primary".to_string(),
            kind,
            photometry_id: String::new(),
            fixed_light_source_id: String::new(),
            multi_channel_light_source_id: String::new(),
            sensor_id: String::new(),
        });
    }
}

/// Form model for authoring a wizard batch template without hand-editing
/// JSON. Wraps the [`WizardDraft`] scalar fields plus optional `@file`
/// paths for the shared byte slots (geometry / sensor / emergency —
/// per-input photometry is always resolved from the input LDT), so the
/// saved template stays a plain JSON file whose bytes resolve at run
/// time. UI-framework-agnostic: the egui batch app renders it as a
/// form; other frontends can reuse it unchanged.
#[derive(Clone, Debug, Default)]
pub struct TemplateDraft {
    /// Scalar wizard fields. The byte fields (`photometry_bytes`,
    /// `geometry_bytes`, `sensor_bytes`, `emergency_bytes`) stay `None`
    /// here — shared assets live in the `*_file` paths below instead.
    pub draft: WizardDraft,
    /// `@file` path for a shared L3D geometry (empty → none).
    pub geometry_file: String,
    /// `@file` path for a shared sensor sheet (empty → none).
    pub sensor_file: String,
    /// `@file` path for a shared emergency photometry (empty → none).
    pub emergency_file: String,
}

/// The four `WizardDraft` byte slots and their filename twins.
const BYTE_FIELDS: [(&str, &str); 4] = [
    ("photometry_bytes", "photometry_filename"),
    ("geometry_bytes", "geometry_filename"),
    ("sensor_bytes", "sensor_filename"),
    ("emergency_bytes", "emergency_filename"),
];

impl TemplateDraft {
    /// Serialize to template JSON. Byte slots become `{"@file": path}`
    /// refs (with the filename twin defaulted to the path's basename);
    /// empty slots are dropped rather than serialized as `null`.
    pub fn to_template_value(&self) -> serde_json::Value {
        let mut value = serde_json::to_value(&self.draft)
            .expect("WizardDraft serializes to JSON");
        let obj = value.as_object_mut().expect("draft is an object");
        for (bytes_key, _) in BYTE_FIELDS {
            obj.remove(bytes_key);
        }
        if obj.get("emitter_draft").is_some_and(|v| v.is_null()) {
            obj.remove("emitter_draft");
        }
        let mut set_file_ref = |bytes_key: &str, filename_key: &str, path: &str| {
            if path.trim().is_empty() {
                return;
            }
            obj.insert(
                bytes_key.to_string(),
                serde_json::json!({ "@file": path }),
            );
            let filename_missing = obj
                .get(filename_key)
                .and_then(|v| v.as_str())
                .map(|s| s.trim().is_empty())
                .unwrap_or(true);
            if filename_missing {
                let basename = std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path);
                obj.insert(
                    filename_key.to_string(),
                    serde_json::Value::String(basename.to_string()),
                );
            }
        };
        set_file_ref("geometry_bytes", "geometry_filename", &self.geometry_file);
        set_file_ref("sensor_bytes", "sensor_filename", &self.sensor_file);
        set_file_ref(
            "emergency_bytes",
            "emergency_filename",
            &self.emergency_file,
        );
        value
    }

    /// Pretty-printed [`Self::to_template_value`].
    pub fn to_template_json(&self) -> String {
        serde_json::to_string_pretty(&self.to_template_value())
            .expect("template JSON serializes")
    }

    /// Parse a wizard template back into the form model. `{"@file": …}`
    /// byte refs land in the `*_file` paths; templates that embed raw
    /// byte arrays (or aren't a wizard object at all) are rejected —
    /// those stay JSON-only.
    pub fn from_template_value(value: &serde_json::Value) -> Result<Self> {
        let mut value = value.clone();
        let obj = value
            .as_object_mut()
            .ok_or_else(|| anyhow!("not a wizard template object"))?;
        let mut files = std::collections::HashMap::new();
        for (bytes_key, _) in BYTE_FIELDS {
            match obj.remove(bytes_key) {
                None | Some(serde_json::Value::Null) => {}
                Some(serde_json::Value::Object(map)) => {
                    let path = map
                        .get("@file")
                        .and_then(|p| p.as_str())
                        .ok_or_else(|| anyhow!("{}: expected an @file ref", bytes_key))?;
                    files.insert(bytes_key, path.to_string());
                }
                Some(_) => bail!(
                    "{}: template embeds raw bytes — edit it as JSON instead",
                    bytes_key
                ),
            }
        }
        if files.contains_key("photometry_bytes") {
            bail!("photometry_bytes @file ref is per-run fixed — batch templates take photometry from each input file; edit as JSON if intentional");
        }
        let draft: WizardDraft =
            serde_json::from_value(value).context("parse WizardDraft template")?;
        Ok(Self {
            draft,
            geometry_file: files.remove("geometry_bytes").unwrap_or_default(),
            sensor_file: files.remove("sensor_bytes").unwrap_or_default(),
            emergency_file: files.remove("emergency_bytes").unwrap_or_default(),
        })
    }
}

/// Convert one input file through the template. Platform-agnostic:
/// bytes in, bytes out; the caller owns all I/O.
///
/// Inputs are `.ldt` / `.ies` photometries; with the `uld` feature,
/// DIALux `.uld` containers are also accepted (detected by OLE magic or
/// extension). A ULD already carries a complete product, so it is
/// converted to GLDF first and the template then applies on top: wizard
/// templates contribute only their header overrides (author,
/// manufacturer, created-with, language — the construction steps would
/// duplicate what the ULD defines), while action scripts apply in full.
pub fn process_one(
    input_bytes: &[u8],
    input_filename: &str,
    template: &BatchTemplate,
    resolver: &dyn FileResolver,
    options: &BatchOptions,
) -> Result<BatchOutput> {
    #[cfg(feature = "uld")]
    if light_convert::is_uld(input_bytes)
        || input_filename.to_ascii_lowercase().ends_with(".uld")
    {
        return process_uld(input_bytes, input_filename, template, resolver, options);
    }

    let meta = ldt_metadata(input_bytes)
        .map_err(|e| anyhow!("parse photometry {}: {}", input_filename, e))?;
    let table = placeholder_table(input_filename, &meta);
    let input_resolver = InputResolver {
        input: input_bytes,
        inner: resolver,
    };

    let mut state = GldfState::default();
    match template {
        BatchTemplate::Wizard(value) => {
            let mut value = value.clone();
            substitute_placeholders(&mut value, &table);
            expand_file_refs(&mut value, &input_resolver)
                .context("expand @file refs in template")?;
            let mut draft: WizardDraft =
                serde_json::from_value(value).context("parse WizardDraft template")?;
            autofill_draft(&mut draft, input_bytes, input_filename, &meta);
            apply_wizard(&mut state, &draft);
            if let Some(err) = &state.last_error {
                bail!("apply template: {}", err);
            }
        }
        BatchTemplate::Script(value) => {
            apply_script_template(&mut state, value, &table, &input_resolver)?;
        }
    }

    finish_outputs(&state, options, meta.name)
}

/// Substitute placeholders + `@file` refs in a script template and apply
/// its actions to `state`.
fn apply_script_template(
    state: &mut GldfState,
    value: &serde_json::Value,
    table: &[(&str, String)],
    resolver: &dyn FileResolver,
) -> Result<()> {
    let mut value = value.clone();
    substitute_placeholders(&mut value, table);
    expand_file_refs(&mut value, resolver).context("expand @file refs in template")?;
    let actions: Vec<GldfAction> =
        serde_json::from_value(value).context("parse GldfAction script template")?;
    for action in actions {
        state.apply(action);
        if let Some(err) = &state.last_error {
            bail!("apply template action: {}", err);
        }
    }
    Ok(())
}

/// Serialize the finished state into the requested artifacts.
fn finish_outputs(
    state: &GldfState,
    options: &BatchOptions,
    product_name: String,
) -> Result<BatchOutput> {
    let gldf = if options.write_gldf {
        Some(state.save_to_buf().context("serialize GLDF")?)
    } else {
        None
    };
    let ifc = if options.write_ifc {
        Some(state.export_ifc().context("export IFC")?)
    } else {
        None
    };
    Ok(BatchOutput {
        gldf,
        ifc,
        product_name,
    })
}

/// ULD path: convert the container to GLDF via `light-convert`, load it
/// as the working product, then layer the template on top.
#[cfg(feature = "uld")]
fn process_uld(
    input_bytes: &[u8],
    input_filename: &str,
    template: &BatchTemplate,
    resolver: &dyn FileResolver,
    options: &BatchOptions,
) -> Result<BatchOutput> {
    let conv = light_convert::convert_to_gldf_full(input_bytes, Some(input_filename), None)
        .map_err(|e| anyhow!("convert {}: {}", input_filename, e))?;
    let loaded = gldf_rs::GldfProduct::load_gldf_from_buf_all(conv.gldf_bytes)
        .map_err(|e| anyhow!("re-load converted GLDF for {}: {}", input_filename, e))?;
    let mut embedded_files = std::collections::HashMap::new();
    for f in loaded.files {
        if let (Some(id), Some(content)) = (f.file_id, f.content) {
            embedded_files.insert(id, content);
        }
    }
    let mut state = GldfState::default();
    state.apply(GldfAction::LoadWithFiles {
        product: loaded.gldf,
        embedded_files,
    });

    // Map the ULD's product metadata onto the same placeholder keys the
    // LDT path fills, so one template serves both input kinds.
    let meta = LdtMetadata {
        name: conv.product_name.clone(),
        manufacturer: conv.manufacturer.clone(),
        luminaire_number: conv.article_number.clone(),
        ..Default::default()
    };
    let table = placeholder_table(input_filename, &meta);
    let input_resolver = InputResolver {
        input: input_bytes,
        inner: resolver,
    };

    match template {
        BatchTemplate::Wizard(value) => {
            let mut value = value.clone();
            substitute_placeholders(&mut value, &table);
            expand_file_refs(&mut value, &input_resolver)
                .context("expand @file refs in template")?;
            let draft: WizardDraft =
                serde_json::from_value(value).context("parse WizardDraft template")?;
            apply_header_overrides(&mut state, &draft)?;
        }
        BatchTemplate::Script(value) => {
            apply_script_template(&mut state, value, &table, &input_resolver)?;
        }
    }

    finish_outputs(&state, options, conv.product_name)
}

/// Apply only the header fields of a wizard template to an
/// already-complete product (the ULD path). Blank fields leave the
/// product's own values alone.
#[cfg(feature = "uld")]
fn apply_header_overrides(state: &mut GldfState, draft: &WizardDraft) -> Result<()> {
    let mut dispatch = |action: GldfAction| -> Result<()> {
        state.apply(action);
        if let Some(err) = &state.last_error {
            bail!("apply template header: {}", err);
        }
        Ok(())
    };
    let author = draft.header_author.trim();
    if !author.is_empty() {
        dispatch(GldfAction::SetAuthor(author.to_string()))?;
    }
    let manufacturer = draft.header_manufacturer.trim();
    if !manufacturer.is_empty() {
        dispatch(GldfAction::SetManufacturer(manufacturer.to_string()))?;
    }
    let created_with = draft.header_created_with.trim();
    if !created_with.is_empty() {
        dispatch(GldfAction::SetCreatedWithApplication(created_with.to_string()))?;
    }
    let lang = draft.header_default_language.trim();
    if !lang.is_empty() {
        dispatch(GldfAction::SetDefaultLanguage(Some(lang.to_string())))?;
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub use fs_impl::{collect_inputs, run_batch, BatchEvent, BatchJob, FileReport};

#[cfg(not(target_arch = "wasm32"))]
mod fs_impl {
    use super::{process_one, BatchOptions, BatchTemplate};
    use crate::script::FsResolver;
    use anyhow::{Context, Result};
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};

    /// A full batch run: template + inputs + destination.
    pub struct BatchJob {
        pub template: BatchTemplate,
        /// Directory `@file` refs in the template resolve against
        /// (normally the template file's parent directory).
        pub template_dir: PathBuf,
        pub inputs: Vec<PathBuf>,
        pub out_dir: PathBuf,
        pub options: BatchOptions,
    }

    /// Per-input outcome of [`run_batch`]. `error` is `Some` when the
    /// file failed; the paths record what was actually written.
    #[derive(Clone, Debug)]
    pub struct FileReport {
        pub input: PathBuf,
        pub gldf_path: Option<PathBuf>,
        pub ifc_path: Option<PathBuf>,
        pub product_name: Option<String>,
        pub error: Option<String>,
    }

    /// Progress callback payload. Return `false` from the callback to
    /// cancel the run after the current event.
    pub enum BatchEvent<'a> {
        Started {
            index: usize,
            total: usize,
            input: &'a Path,
        },
        Finished {
            index: usize,
            total: usize,
            report: &'a FileReport,
        },
    }

    /// Expand a mixed list of files and directories into the input
    /// files to convert: `.ldt` / `.ies` (case-insensitive; plus `.uld`
    /// with the `uld` feature), directories walked recursively. Sorted
    /// and deduplicated.
    pub fn collect_inputs(paths: &[PathBuf]) -> Vec<PathBuf> {
        fn is_photometry(path: &Path) -> bool {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| {
                    e.eq_ignore_ascii_case("ldt")
                        || e.eq_ignore_ascii_case("ies")
                        || (cfg!(feature = "uld") && e.eq_ignore_ascii_case("uld"))
                })
                .unwrap_or(false)
        }
        fn walk(path: &Path, out: &mut Vec<PathBuf>) {
            if path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        walk(&entry.path(), out);
                    }
                }
            } else if is_photometry(path) {
                out.push(path.to_path_buf());
            }
        }
        let mut out = Vec::new();
        for p in paths {
            walk(p, &mut out);
        }
        out.sort();
        out.dedup();
        out
    }

    /// Run the whole job: one GLDF (and optionally IFC) per input, named
    /// after the input's file stem (`_2`, `_3`, … suffixes on stem
    /// collisions across directories). A failing input is recorded in
    /// its [`FileReport`] and the run continues. The callback receives
    /// [`BatchEvent`]s; returning `false` cancels after the current file.
    pub fn run_batch(
        job: &BatchJob,
        on_event: &mut dyn FnMut(BatchEvent) -> bool,
    ) -> Result<Vec<FileReport>> {
        std::fs::create_dir_all(&job.out_dir)
            .with_context(|| format!("create output dir {}", job.out_dir.display()))?;

        let total = job.inputs.len();
        let mut reports = Vec::with_capacity(total);
        let mut used_stems: HashSet<String> = HashSet::new();

        for (index, input) in job.inputs.iter().enumerate() {
            if !on_event(BatchEvent::Started {
                index,
                total,
                input,
            }) {
                break;
            }

            let base_stem = input
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("luminaire")
                .to_string();
            let mut stem = base_stem.clone();
            let mut counter = 2;
            while !used_stems.insert(stem.clone()) {
                stem = format!("{}_{}", base_stem, counter);
                counter += 1;
            }

            let report = convert_file(job, input, &stem);
            let go_on = on_event(BatchEvent::Finished {
                index,
                total,
                report: &report,
            });
            reports.push(report);
            if !go_on {
                break;
            }
        }
        Ok(reports)
    }

    fn convert_file(job: &BatchJob, input: &Path, stem: &str) -> FileReport {
        let mut report = FileReport {
            input: input.to_path_buf(),
            gldf_path: None,
            ifc_path: None,
            product_name: None,
            error: None,
        };
        let result = (|| -> Result<()> {
            let bytes = std::fs::read(input)
                .with_context(|| format!("read {}", input.display()))?;
            let filename = input
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("photometry.ldt");
            let resolver = FsResolver {
                root: job.template_dir.clone(),
            };
            let output =
                process_one(&bytes, filename, &job.template, &resolver, &job.options)?;
            report.product_name = Some(output.product_name.clone());
            if let Some(gldf) = &output.gldf {
                let path = job.out_dir.join(format!("{}.gldf", stem));
                std::fs::write(&path, gldf)
                    .with_context(|| format!("write {}", path.display()))?;
                report.gldf_path = Some(path);
            }
            if let Some(ifc) = &output.ifc {
                let path = job.out_dir.join(format!("{}.ifc", stem));
                std::fs::write(&path, ifc)
                    .with_context(|| format!("write {}", path.display()))?;
                report.ifc_path = Some(path);
            }
            Ok(())
        })();
        if let Err(e) = result {
            report.error = Some(format!("{:#}", e));
        }
        report
    }
}
