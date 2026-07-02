//! Script application: parse a JSON array of [`GldfAction`]s, expand any
//! `{"@file": "<path>"}` byte-references via a pluggable [`FileResolver`],
//! and dispatch them through the reducer.
//!
//! Two frontends consume this:
//!
//! - `gldf-cli apply` — passes an [`FsResolver`] rooted at the script's
//!   directory.
//! - The Leptos / WASM editor — passes an in-memory resolver backed by a
//!   `HashMap<String, Vec<u8>>` of files the user dropped into the
//!   browser. The editor crate has no `wasm-bindgen` or `web_sys`
//!   dependency, so the resolver crosses that boundary cleanly.
//!
//! Keeping the resolver behind a trait means the editor crate stays
//! filesystem-agnostic; only the consumer that *has* a filesystem pays
//! for one.

use crate::action::GldfAction;
use crate::state::GldfState;
use anyhow::{anyhow, Context, Result};

/// Resolve a `"@file": "<path>"` reference to its raw bytes.
///
/// Implementations decide where bytes come from: a directory on disk, a
/// `HashMap` of dropped browser files, an in-zip lookup, etc. The
/// `path` is taken verbatim from the JSON sigil; resolvers are free to
/// reject paths they consider unsafe (e.g. absolute paths, `..`).
pub trait FileResolver {
    fn resolve(&self, path: &str) -> Result<Vec<u8>>;
}

/// Filesystem-backed resolver: reads paths relative to `root`. Available
/// on every non-WASM target; the WASM editor uses its own in-memory
/// resolver instead of this.
#[cfg(not(target_arch = "wasm32"))]
pub struct FsResolver {
    pub root: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl FileResolver for FsResolver {
    fn resolve(&self, path: &str) -> Result<Vec<u8>> {
        let full = self.root.join(path);
        std::fs::read(&full).with_context(|| format!("read {}", full.display()))
    }
}

/// In-memory resolver. Useful for tests and for the WASM editor (which
/// builds one from the user's dropped files).
pub struct MemoryResolver<'a> {
    files: &'a std::collections::HashMap<String, Vec<u8>>,
}

impl<'a> MemoryResolver<'a> {
    pub fn new(files: &'a std::collections::HashMap<String, Vec<u8>>) -> Self {
        Self { files }
    }
}

impl<'a> FileResolver for MemoryResolver<'a> {
    fn resolve(&self, path: &str) -> Result<Vec<u8>> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| anyhow!("file not provided: {}", path))
    }
}

/// Walk the parsed action JSON and replace any
/// `{"<field>": {"@file": "<path>"}}` with the file's contents (as a
/// JSON number array, which is what serde expects for `Vec<u8>` fields).
pub fn expand_file_refs(
    value: &mut serde_json::Value,
    resolver: &dyn FileResolver,
) -> Result<()> {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let file_ref_keys: Vec<String> = map
                .iter()
                .filter_map(|(k, v)| {
                    v.as_object()
                        .and_then(|o| o.get("@file"))
                        .and_then(|p| p.as_str())
                        .map(|_| k.clone())
                })
                .collect();
            for k in file_ref_keys {
                let path = map[&k]["@file"].as_str().unwrap().to_string();
                let bytes = resolver.resolve(&path)?;
                map[&k] = Value::Array(
                    bytes
                        .into_iter()
                        .map(|b| Value::Number(b.into()))
                        .collect(),
                );
            }
            for v in map.values_mut() {
                expand_file_refs(v, resolver)?;
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                expand_file_refs(v, resolver)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Parse a JSON array of [`GldfAction`]s, expand `@file` byte refs via
/// `resolver`, and dispatch each action through the reducer. Stops at
/// the first reducer error (returned via `Result::Err`).
pub fn apply_script(
    state: &mut GldfState,
    script_json: &str,
    resolver: &dyn FileResolver,
) -> Result<()> {
    let mut value: serde_json::Value =
        serde_json::from_str(script_json).context("parse script JSON")?;
    expand_file_refs(&mut value, resolver).context("expand @file refs")?;
    let actions: Vec<GldfAction> =
        serde_json::from_value(value).context("parse actions")?;
    for action in actions {
        state.apply(action);
        if let Some(err) = &state.last_error {
            return Err(anyhow!("apply failed: {}", err));
        }
    }
    Ok(())
}
