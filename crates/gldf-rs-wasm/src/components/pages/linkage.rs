//! Linkage map — read-only diagram of the entire GLDF reference graph.
//!
//! Walks the loaded product's catalogs (Files / Photometries / Spectrums /
//! LightSources / Geometries / Emitters / Variants) and renders the
//! `xs:keyref` edges the XSD declares (see `tests/data/gldf-1.0.0-rc-3.xsd`
//! lines 86-216 for the canonical edge list).
//!
//! Why this exists: the parallel-pages model (Light Sources, Emitters,
//! Variants…) makes the *catalogs* legible but hides the *graph*. A
//! Photometry can be an orphan if no Emitter references it; a Spectrum can
//! be an orphan if no LightSource carries a `SpectrumReference`. This page
//! shows the full reachability picture in one glance and flags every
//! dangling node with `⚠`.
//!
//! Every node label is a button that jumps to that entity's editor by
//! flipping `ShellState::nav`. The reverse side (going from editor → here)
//! is the user clicking the left-nav `Linkage` entry.

use crate::components::pages::link_hint::LinkHint;
use crate::state::use_gldf;
use leptos::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Targets the page can jump to. The bin layer maps these to its private
/// `NavItem` so the library doesn't need to depend on the bin's nav enum.
#[derive(Clone, Copy, Debug)]
pub enum LinkageTarget {
    Files,
    Photometries,
    Spectrums,
    LightSources,
    Emitters,
    Variants,
}

/// Callback the bin supplies so this page can flip the active nav entry.
pub type NavJump = Arc<dyn Fn(LinkageTarget) + Send + Sync>;

#[component]
pub fn LinkagePage(jump: NavJump) -> impl IntoView {
    let state = use_gldf();
    let jump_for_view = jump.clone();

    view! {
        <div class="linkage-page" style="padding: 8px;">
            <h2 style="margin-top: 0;">{ "Linkage map" }</h2>
            <LinkHint
                title="How does this page relate to Light Sources / Emitters / Variants?".to_string()
                body="Each page on the left edits one catalog. This page shows the graph that connects them — which Photometry is referenced by which Emitter, which Spectrum is carried by which LightSource, which Emitter is mounted on which Geometry inside which Variant. ⚠ marks orphan entries (declared in a catalog but not referenced by anyone upstream). The labels in dim gray are descriptive; the colored ids are the actual XML keys.".to_string()
            />

            {move || {
                let jump = jump_for_view.clone();
                state.with(|s| {
                    let g = build_graph(s);
                    render_graph(g, jump.clone())
                })
            }}
        </div>
    }
}

/// Snapshot of every catalog + the reverse-index of who references whom.
/// Built once per render from `GldfState`, then rendered into a vertical
/// tree of `<section>` blocks. Plain owned `String`s everywhere so the
/// view layer doesn't need to hold a borrow of state.
struct Graph {
    files: Vec<FileRow>,
    photometries: Vec<PhotometryRow>,
    spectrums: Vec<SpectrumRow>,
    fixed_light_sources: Vec<FixedLsRow>,
    changeable_light_sources: Vec<ChangeableLsRow>,
    emitters: Vec<EmitterRow>,
    variants: Vec<VariantRow>,
    /// Light-source ids that the user file actually declares (used to flag
    /// dangling `LightSourceReference` ids on emitters).
    known_ls_ids: HashSet<String>,
    /// Photometry ids that exist (for emitter-side dangling check).
    known_photometry_ids: HashSet<String>,
}

struct FileRow {
    id: String,
    content_type: String,
    file_name: String,
    bytes_len: usize,
    /// Photometry/Spectrum ids that reference this File (forward fan-out).
    referenced_by: Vec<String>,
}

struct PhotometryRow {
    id: String,
    file_id: Option<String>,
    /// Emitter+LightEmitter pairs that reference this Photometry.
    used_by_emitters: Vec<String>,
    /// ChangeableLightSource ids that reference this Photometry directly.
    used_by_change_ls: Vec<String>,
}

struct SpectrumRow {
    id: String,
    file_id: Option<String>,
    /// LightSource ids that carry this Spectrum.
    used_by_light_sources: Vec<String>,
}

struct FixedLsRow {
    id: String,
    spectrum_id: Option<String>,
    /// Emitter ids whose FixedLightEmitter children reference this LS.
    used_by_emitters: Vec<String>,
}

struct ChangeableLsRow {
    id: String,
    spectrum_id: Option<String>,
    photometry_id: Option<String>,
}

struct EmitterRow {
    id: String,
    fixed_count: usize,
    changeable_count: usize,
    sensor_count: usize,
    /// Per-FixedLightEmitter (photometry_id, fixed_light_source_id_opt).
    fixed_links: Vec<(String, Option<String>)>,
    /// Per-ChangeableLightEmitter photometry_id.
    changeable_links: Vec<String>,
    /// Variant ids that mount this emitter (any of the three Geometry shapes).
    mounted_by_variants: Vec<String>,
}

struct VariantRow {
    id: String,
    name: String,
    /// Emitter ids this variant binds via Geometry/EmitterReference (in any
    /// of the three Variant/Geometry shapes the XSD allows).
    bound_emitters: Vec<String>,
}

fn build_graph(s: &crate::state::GldfState) -> Graph {
    // --- catalogs ---
    let files_iter = s.product.general_definitions.files.file.iter();
    let photometries_iter = s
        .product
        .general_definitions
        .photometries
        .as_ref()
        .map(|p| p.photometry.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    let spectrums_iter = s
        .product
        .general_definitions
        .spectrums
        .as_ref()
        .map(|sp| sp.spectrum.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    let fixed_ls_iter = s
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .map(|l| l.fixed_light_source.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    let change_ls_iter = s
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .map(|l| l.changeable_light_source.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    let emitters_iter = s
        .product
        .general_definitions
        .emitters
        .as_ref()
        .map(|e| e.emitter.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    let variants_iter = s
        .product
        .product_definitions
        .variants
        .as_ref()
        .map(|v| v.variant.iter().collect::<Vec<_>>())
        .unwrap_or_default();

    // --- reverse indices ---
    // file_id → list of "Photometry photo_1" / "Spectrum spec_2" strings
    let mut file_consumers: HashMap<String, Vec<String>> = HashMap::new();
    for p in &photometries_iter {
        if let Some(fr) = p.photometry_file_reference.as_ref() {
            file_consumers
                .entry(fr.file_id.clone())
                .or_default()
                .push(format!("Photometry {}", p.id));
        }
    }
    for sp in &spectrums_iter {
        if let Some(fr) = sp.spectrum_file_reference.as_ref() {
            file_consumers
                .entry(fr.file_id.clone())
                .or_default()
                .push(format!("Spectrum {}", sp.id));
        }
    }
    // LightSourceImages (product photos for a lamp) — XSD keyref
    // ChangeableLightSourceImage-File / FixedLightSourceImage-File (L98-115).
    for f in &fixed_ls_iter {
        if let Some(imgs) = f.light_source_images.as_ref() {
            for img in &imgs.image {
                file_consumers
                    .entry(img.file_id.clone())
                    .or_default()
                    .push(format!("FixedLightSource {} image", f.id));
            }
        }
    }
    for c in &change_ls_iter {
        if let Some(imgs) = c.light_source_images.as_ref() {
            for img in &imgs.image {
                file_consumers
                    .entry(img.file_id.clone())
                    .or_default()
                    .push(format!("ChangeableLightSource {} image", c.id));
            }
        }
    }
    // ModelGeometry GeometryFileReference — the .l3d / .m3d / .r3d files
    // each ModelGeometry wraps.
    if let Some(geoms) = s.product.general_definitions.geometries.as_ref() {
        for m in &geoms.model_geometry {
            for gfr in &m.geometry_file_reference {
                file_consumers
                    .entry(gfr.file_id.clone())
                    .or_default()
                    .push(format!("ModelGeometry {}", m.id));
            }
        }
    }
    // Sensors — SensorFileReference (sensor characteristic data).
    if let Some(sensors) = s.product.general_definitions.sensors.as_ref() {
        for sensor in &sensors.sensor {
            if let Some(fr) = sensor.sensor_file_reference.as_ref() {
                file_consumers
                    .entry(fr.file_id.clone())
                    .or_default()
                    .push(format!("Sensor {}", sensor.id));
            }
        }
    }
    // Variant-level: Symbol icon + Pictures, plus the ProductMetaData
    // marketing pictures (Variant-Symbol-File / Variant-Image-File keyrefs).
    for v in &variants_iter {
        if let Some(sym) = v.symbol.as_ref() {
            file_consumers
                .entry(sym.file_id.clone())
                .or_default()
                .push(format!("Variant {} symbol", v.id));
        }
        if let Some(pics) = v.pictures.as_ref() {
            for img in &pics.image {
                file_consumers
                    .entry(img.file_id.clone())
                    .or_default()
                    .push(format!("Variant {} picture", v.id));
            }
        }
    }
    // Product-level ProductMetaData pictures — `PicturesEditorPage` writes
    // here via `AddProductMetaPicture`. Image types include the
    // user-visible "Application Picture" / "Product Picture" / "Symbol"
    // tags (the @imageType attribute), so a freshly-added Application
    // picture would otherwise wrongly show as an orphan.
    if let Some(meta) = s.product.product_definitions.product_meta_data.as_ref() {
        if let Some(pics) = meta.pictures.as_ref() {
            for img in &pics.image {
                let label = if img.image_type.is_empty() {
                    "ProductMetaData picture".to_string()
                } else {
                    format!("ProductMetaData picture ({})", img.image_type)
                };
                file_consumers
                    .entry(img.file_id.clone())
                    .or_default()
                    .push(label);
            }
        }
    }

    // emitter walk — for each Emitter id, record which photometries / fixed
    // light sources its LightEmitter children reference.
    let mut photometry_emitter_uses: HashMap<String, Vec<String>> = HashMap::new();
    let mut fixed_ls_emitter_uses: HashMap<String, Vec<String>> = HashMap::new();
    for e in &emitters_iter {
        for fle in &e.fixed_light_emitter {
            let pid = &fle.photometry_reference.photometry_id;
            photometry_emitter_uses
                .entry(pid.clone())
                .or_default()
                .push(format!("Emitter {}", e.id));
            if let Some(ls_id) = fle.light_source_reference.fixed_light_source_id.as_ref() {
                fixed_ls_emitter_uses
                    .entry(ls_id.clone())
                    .or_default()
                    .push(format!("Emitter {}", e.id));
            }
        }
        for cle in &e.changeable_light_emitter {
            let pid = &cle.photometry_reference.photometry_id;
            photometry_emitter_uses
                .entry(pid.clone())
                .or_default()
                .push(format!("Emitter {}", e.id));
        }
    }

    // Spectrum users — LightSources only (Spectrums never reference anything
    // upstream; light sources point at them).
    let mut spectrum_ls_uses: HashMap<String, Vec<String>> = HashMap::new();
    for f in &fixed_ls_iter {
        if let Some(sr) = f.spectrum_reference.as_ref() {
            spectrum_ls_uses
                .entry(sr.spectrum_id.clone())
                .or_default()
                .push(format!("FixedLightSource {}", f.id));
        }
    }
    for c in &change_ls_iter {
        if let Some(sr) = c.spectrum_reference.as_ref() {
            spectrum_ls_uses
                .entry(sr.spectrum_id.clone())
                .or_default()
                .push(format!("ChangeableLightSource {}", c.id));
        }
    }
    // MultiChannel: spectra and photometries are referenced PER-CHANNEL
    // (XSD keyrefs MultiChannelLightSource-Spectrum L118 and
    // MultiChannelLightSource-Photometry L122). Walk each Channel inside
    // each MultiChannelLightSource so the channel refs count as users.
    let multi_channel_ls_iter = s
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .map(|l| l.multi_channel_light_source.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    for m in &multi_channel_ls_iter {
        for ch in &m.channels.channel {
            spectrum_ls_uses
                .entry(ch.spectrum_reference.spectrum_id.clone())
                .or_default()
                .push(format!("MultiChannel {} / channel", m.id));
        }
    }

    // ChangeableLS → Photometry direct reference (rare; the more common
    // path is via Emitter, but the XSD permits LS-level photometry too).
    // MultiChannel channels are the *common* path for per-channel photometry —
    // walked here so the Photometry "used by" list shows MultiChannel users.
    let mut change_ls_photometry_uses: HashMap<String, Vec<String>> = HashMap::new();
    for c in &change_ls_iter {
        if let Some(pr) = c.photometry_reference.as_ref() {
            change_ls_photometry_uses
                .entry(pr.photometry_id.clone())
                .or_default()
                .push(format!("ChangeableLightSource {}", c.id));
        }
    }
    for m in &multi_channel_ls_iter {
        for ch in &m.channels.channel {
            change_ls_photometry_uses
                .entry(ch.photometry_reference.photometry_id.clone())
                .or_default()
                .push(format!("MultiChannel {} / channel", m.id));
        }
    }

    // Variant → Emitter mounting. The XSD permits the EmitterReference to
    // appear in five different shapes (see XSD lines 182-216). We collect
    // every emitter_id that any Variant/Geometry binds.
    let mut emitter_variant_uses: HashMap<String, Vec<String>> = HashMap::new();
    let mut variant_bound_emitters: HashMap<String, Vec<String>> = HashMap::new();
    for v in &variants_iter {
        let mut bound: Vec<String> = Vec::new();
        if let Some(geom) = v.geometry.as_ref() {
            for er in &geom.emitter_reference {
                bound.push(er.emitter_id.clone());
            }
            if let Some(sr) = geom.simple_geometry_reference.as_ref() {
                bound.push(sr.emitter_id.clone());
            }
            if let Some(mr) = geom.model_geometry_reference.as_ref() {
                for er in &mr.emitter_reference {
                    bound.push(er.emitter_id.clone());
                }
            }
        }
        for eid in &bound {
            emitter_variant_uses
                .entry(eid.clone())
                .or_default()
                .push(v.id.clone());
        }
        variant_bound_emitters.insert(v.id.clone(), bound);
    }

    // --- materialise rows ---
    let files: Vec<FileRow> = files_iter
        .map(|f| FileRow {
            id: f.id.clone(),
            content_type: f.content_type.clone(),
            file_name: f.file_name.clone(),
            bytes_len: s.embedded_files.get(&f.id).map(|b| b.len()).unwrap_or(0),
            referenced_by: file_consumers.get(&f.id).cloned().unwrap_or_default(),
        })
        .collect();

    let photometries: Vec<PhotometryRow> = photometries_iter
        .iter()
        .map(|p| PhotometryRow {
            id: p.id.clone(),
            file_id: p.photometry_file_reference.as_ref().map(|r| r.file_id.clone()),
            used_by_emitters: photometry_emitter_uses.get(&p.id).cloned().unwrap_or_default(),
            used_by_change_ls: change_ls_photometry_uses.get(&p.id).cloned().unwrap_or_default(),
        })
        .collect();

    let spectrums: Vec<SpectrumRow> = spectrums_iter
        .iter()
        .map(|sp| SpectrumRow {
            id: sp.id.clone(),
            file_id: sp.spectrum_file_reference.as_ref().map(|r| r.file_id.clone()),
            used_by_light_sources: spectrum_ls_uses.get(&sp.id).cloned().unwrap_or_default(),
        })
        .collect();

    let fixed_light_sources: Vec<FixedLsRow> = fixed_ls_iter
        .iter()
        .map(|f| FixedLsRow {
            id: f.id.clone(),
            spectrum_id: f.spectrum_reference.as_ref().map(|r| r.spectrum_id.clone()),
            used_by_emitters: fixed_ls_emitter_uses.get(&f.id).cloned().unwrap_or_default(),
        })
        .collect();

    let changeable_light_sources: Vec<ChangeableLsRow> = change_ls_iter
        .iter()
        .map(|c| ChangeableLsRow {
            id: c.id.clone(),
            spectrum_id: c.spectrum_reference.as_ref().map(|r| r.spectrum_id.clone()),
            photometry_id: c.photometry_reference.as_ref().map(|r| r.photometry_id.clone()),
        })
        .collect();

    let emitters: Vec<EmitterRow> = emitters_iter
        .iter()
        .map(|e| EmitterRow {
            id: e.id.clone(),
            fixed_count: e.fixed_light_emitter.len(),
            changeable_count: e.changeable_light_emitter.len(),
            sensor_count: e.sensor_emitter.len(),
            fixed_links: e
                .fixed_light_emitter
                .iter()
                .map(|fle| {
                    (
                        fle.photometry_reference.photometry_id.clone(),
                        fle.light_source_reference.fixed_light_source_id.clone(),
                    )
                })
                .collect(),
            changeable_links: e
                .changeable_light_emitter
                .iter()
                .map(|cle| cle.photometry_reference.photometry_id.clone())
                .collect(),
            mounted_by_variants: emitter_variant_uses.get(&e.id).cloned().unwrap_or_default(),
        })
        .collect();

    let variants: Vec<VariantRow> = variants_iter
        .iter()
        .map(|v| VariantRow {
            id: v.id.clone(),
            name: variant_label(v),
            bound_emitters: variant_bound_emitters.get(&v.id).cloned().unwrap_or_default(),
        })
        .collect();

    let known_ls_ids: HashSet<String> = fixed_light_sources
        .iter()
        .map(|f| f.id.clone())
        .chain(changeable_light_sources.iter().map(|c| c.id.clone()))
        .collect();
    let known_photometry_ids: HashSet<String> =
        photometries.iter().map(|p| p.id.clone()).collect();

    Graph {
        files,
        photometries,
        spectrums,
        fixed_light_sources,
        changeable_light_sources,
        emitters,
        variants,
        known_ls_ids,
        known_photometry_ids,
    }
}

/// Best-effort label for a variant: first non-empty locale on its
/// `product_number` if present, else falls back to its id.
fn variant_label(v: &gldf_rs::gldf::product_definitions::Variant) -> String {
    if let Some(pn) = v.product_number.as_ref() {
        for l in &pn.locale {
            if !l.value.trim().is_empty() {
                return l.value.clone();
            }
        }
    }
    v.id.clone()
}

fn render_graph(g: Graph, jump: NavJump) -> impl IntoView {
    view! {
        <div class="linkage-graph" style="display: flex; flex-direction: column; gap: 20px; margin-top: 12px;">
            <FilesSection rows=g.files jump=jump.clone() />
            <PhotometriesSection rows=g.photometries jump=jump.clone() />
            <SpectrumsSection rows=g.spectrums jump=jump.clone() />
            <LightSourcesSection
                fixed=g.fixed_light_sources
                changeable=g.changeable_light_sources
                jump=jump.clone()
            />
            <EmittersSection
                rows=g.emitters
                known_ls_ids=g.known_ls_ids
                known_photometry_ids=g.known_photometry_ids
                jump=jump.clone()
            />
            <VariantsSection rows=g.variants jump=jump />
        </div>
    }
}

#[derive(Clone, Copy)]
enum SectionKind {
    Files,
    Photometries,
    Spectrums,
    LightSources,
    Emitters,
    Variants,
}

/// Clickable id chip: renders an inline `<button>` styled like `<code>` so
/// it sits inline in row text. Clicking flips the active nav entry to
/// `target`. Used for every catalog id and every cross-reference id on the
/// Linkage page, so every node label is a navigable hop.
#[component]
fn IdChip(
    label: String,
    target: LinkageTarget,
    jump: NavJump,
    #[prop(default = "var(--accent-blue)")] color: &'static str,
) -> impl IntoView {
    let style = format!(
        "background: none; border: none; padding: 0; margin: 0; font-family: var(--font-mono); font-size: inherit; color: {}; cursor: pointer; text-decoration: underline dotted; text-underline-offset: 2px;",
        color
    );
    view! {
        <button
            type="button"
            on:click=move |_| jump(target)
            style=style
            title=format!("Jump to {:?}", target)
        >
            { label }
        </button>
    }
}

#[component]
fn SectionHeader(
    kind: SectionKind,
    count: usize,
    on_jump: Arc<dyn Fn() + Send + Sync>,
) -> impl IntoView {
    let (label, icon) = match kind {
        SectionKind::Files => ("Files", "📁"),
        SectionKind::Photometries => ("Photometries", "🔬"),
        SectionKind::Spectrums => ("Spectrums", "🌈"),
        SectionKind::LightSources => ("LightSources", "💡"),
        SectionKind::Emitters => ("Emitters", "🔆"),
        SectionKind::Variants => ("Variants", "📦"),
    };
    view! {
        <h3 style="margin: 0 0 8px 0; font-size: 14px; display: flex; align-items: center; gap: 6px;">
            <span>{ icon }</span>
            <button
                on:click=move |_| on_jump()
                style="background: none; border: none; padding: 0; color: var(--accent-blue); cursor: pointer; font-size: 14px; font-weight: 600;"
            >
                { format!("{} ({})", label, count) }
            </button>
        </h3>
    }
}

#[component]
fn FilesSection(rows: Vec<FileRow>, jump: NavJump) -> impl IntoView {
    let count = rows.len();
    let on_header_jump = {
        let jump = jump.clone();
        Arc::new(move || jump(LinkageTarget::Files)) as Arc<dyn Fn() + Send + Sync>
    };
    view! {
        <section class="linkage-section" style="padding: 10px 12px; border: 1px solid var(--border-color); border-radius: 6px;">
            <SectionHeader kind=SectionKind::Files count=count on_jump=on_header_jump />
            {if rows.is_empty() {
                view! { <p style="margin: 0; color: var(--text-secondary); font-size: 12px;">{ "No files in this GLDF." }</p> }.into_any()
            } else {
                view! {
                    <ul style="list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px;">
                        {rows.into_iter().map(|f| {
                            let orphan = f.referenced_by.is_empty();
                            let badge = if orphan { "⚠ " } else { "" };
                            let jump_for_id = jump.clone();
                            view! {
                                <li style="font-size: 12px; padding: 4px 6px; background: var(--bg-secondary); border-radius: 4px;">
                                    <span style="color: var(--text-tertiary);">{ badge }</span>
                                    <IdChip label=f.id target=LinkageTarget::Files jump=jump_for_id />
                                    <span style="color: var(--text-secondary);">{ format!(" — {} — {} ({} bytes)", f.content_type, f.file_name, f.bytes_len) }</span>
                                    {if orphan {
                                        view! { <span style="color: var(--accent-orange); font-style: italic;">{ " — orphan: no Photometry/Spectrum references this File" }</span> }.into_any()
                                    } else {
                                        view! { <span style="color: var(--text-tertiary);">{ format!("  ← {}", f.referenced_by.join(", ")) }</span> }.into_any()
                                    }}
                                </li>
                            }
                        }).collect::<Vec<_>>()}
                    </ul>
                }.into_any()
            }}
        </section>
    }
}

#[component]
fn PhotometriesSection(rows: Vec<PhotometryRow>, jump: NavJump) -> impl IntoView {
    let count = rows.len();
    let on_header_jump = {
        let jump = jump.clone();
        Arc::new(move || jump(LinkageTarget::Photometries)) as Arc<dyn Fn() + Send + Sync>
    };
    view! {
        <section class="linkage-section" style="padding: 10px 12px; border: 1px solid var(--border-color); border-radius: 6px;">
            <SectionHeader kind=SectionKind::Photometries count=count on_jump=on_header_jump />
            {if rows.is_empty() {
                view! { <p style="margin: 0; color: var(--text-secondary); font-size: 12px;">{ "No Photometry blocks declared." }</p> }.into_any()
            } else {
                view! {
                    <ul style="list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px;">
                        {rows.into_iter().map(|p| {
                            let orphan = p.used_by_emitters.is_empty() && p.used_by_change_ls.is_empty();
                            let badge = if orphan { "⚠ " } else { "" };
                            let mut users: Vec<String> = Vec::new();
                            users.extend(p.used_by_emitters.iter().cloned());
                            users.extend(p.used_by_change_ls.iter().cloned());
                            let jump_id = jump.clone();
                            let jump_file = jump.clone();
                            view! {
                                <li style="font-size: 12px; padding: 4px 6px; background: var(--bg-secondary); border-radius: 4px;">
                                    <span style="color: var(--text-tertiary);">{ badge }</span>
                                    <IdChip label=p.id target=LinkageTarget::Photometries jump=jump_id color="var(--accent-purple)" />
                                    {match p.file_id {
                                        Some(fid) => view! {
                                            <span style="color: var(--text-secondary);">{ " → File " }</span>
                                            <IdChip label=fid target=LinkageTarget::Files jump=jump_file />
                                        }.into_any(),
                                        None => view! { <span style="color: var(--accent-orange);">{ " — no PhotometryFileReference" }</span> }.into_any(),
                                    }}
                                    {if orphan {
                                        view! { <span style="color: var(--accent-orange); font-style: italic;">{ " — orphan: no Emitter or ChangeableLightSource references this" }</span> }.into_any()
                                    } else {
                                        view! { <span style="color: var(--text-tertiary);">{ format!("  ← {}", users.join(", ")) }</span> }.into_any()
                                    }}
                                </li>
                            }
                        }).collect::<Vec<_>>()}
                    </ul>
                }.into_any()
            }}
        </section>
    }
}

#[component]
fn SpectrumsSection(rows: Vec<SpectrumRow>, jump: NavJump) -> impl IntoView {
    let count = rows.len();
    let on_header_jump = {
        let jump = jump.clone();
        Arc::new(move || jump(LinkageTarget::Spectrums)) as Arc<dyn Fn() + Send + Sync>
    };
    view! {
        <section class="linkage-section" style="padding: 10px 12px; border: 1px solid var(--border-color); border-radius: 6px;">
            <SectionHeader kind=SectionKind::Spectrums count=count on_jump=on_header_jump />
            {if rows.is_empty() {
                view! { <p style="margin: 0; color: var(--text-secondary); font-size: 12px;">{ "No Spectrum blocks declared." }</p> }.into_any()
            } else {
                view! {
                    <ul style="list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px;">
                        {rows.into_iter().map(|sp| {
                            let orphan = sp.used_by_light_sources.is_empty();
                            let badge = if orphan { "⚠ " } else { "" };
                            let jump_id = jump.clone();
                            let jump_file = jump.clone();
                            view! {
                                <li style="font-size: 12px; padding: 4px 6px; background: var(--bg-secondary); border-radius: 4px;">
                                    <span style="color: var(--text-tertiary);">{ badge }</span>
                                    <IdChip label=sp.id target=LinkageTarget::Spectrums jump=jump_id color="var(--accent-orange)" />
                                    {match sp.file_id {
                                        Some(fid) => view! {
                                            <span style="color: var(--text-secondary);">{ " → File " }</span>
                                            <IdChip label=fid target=LinkageTarget::Files jump=jump_file />
                                        }.into_any(),
                                        None => view! { <span style="color: var(--text-secondary);">{ " — inline <Intensity>" }</span> }.into_any(),
                                    }}
                                    {if orphan {
                                        view! { <span style="color: var(--accent-orange); font-style: italic;">{ " — orphan: no LightSource carries a SpectrumReference to this" }</span> }.into_any()
                                    } else {
                                        view! { <span style="color: var(--text-tertiary);">{ format!("  ← {}", sp.used_by_light_sources.join(", ")) }</span> }.into_any()
                                    }}
                                </li>
                            }
                        }).collect::<Vec<_>>()}
                    </ul>
                }.into_any()
            }}
        </section>
    }
}

#[component]
fn LightSourcesSection(
    fixed: Vec<FixedLsRow>,
    changeable: Vec<ChangeableLsRow>,
    jump: NavJump,
) -> impl IntoView {
    let count = fixed.len() + changeable.len();
    let on_header_jump = {
        let jump = jump.clone();
        Arc::new(move || jump(LinkageTarget::LightSources)) as Arc<dyn Fn() + Send + Sync>
    };
    view! {
        <section class="linkage-section" style="padding: 10px 12px; border: 1px solid var(--border-color); border-radius: 6px;">
            <SectionHeader kind=SectionKind::LightSources count=count on_jump=on_header_jump />
            {if count == 0 {
                view! { <p style="margin: 0; color: var(--text-secondary); font-size: 12px;">{ "No LightSource blocks declared. To ship a usable GLDF you need at least one." }</p> }.into_any()
            } else {
                view! {
                    <div style="display: flex; flex-direction: column; gap: 8px;">
                        {if !fixed.is_empty() {
                            let jump_for_fixed = jump.clone();
                            view! {
                                <div>
                                    <div style="font-size: 11px; color: var(--text-tertiary); margin-bottom: 4px;">{ "Fixed" }</div>
                                    <ul style="list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px;">
                                        {fixed.into_iter().map(|f| {
                                            let orphan_for_emitter = f.used_by_emitters.is_empty();
                                            let jump_id = jump_for_fixed.clone();
                                            let jump_spec = jump_for_fixed.clone();
                                            view! {
                                                <li style="font-size: 12px; padding: 4px 6px; background: var(--bg-secondary); border-radius: 4px;">
                                                    <IdChip label=f.id target=LinkageTarget::LightSources jump=jump_id />
                                                    {match f.spectrum_id {
                                                        Some(sid) => view! {
                                                            <span style="color: var(--text-secondary);">{ " → Spectrum " }</span>
                                                            <IdChip label=sid target=LinkageTarget::Spectrums jump=jump_spec color="var(--accent-orange)" />
                                                        }.into_any(),
                                                        None => view! { <span style="color: var(--accent-orange);">{ " — no SpectrumReference" }</span> }.into_any(),
                                                    }}
                                                    {if orphan_for_emitter {
                                                        view! { <span style="color: var(--accent-orange); font-style: italic;">{ " — orphan: no Emitter/FixedLightEmitter references this" }</span> }.into_any()
                                                    } else {
                                                        view! { <span style="color: var(--text-tertiary);">{ format!("  ← {}", f.used_by_emitters.join(", ")) }</span> }.into_any()
                                                    }}
                                                </li>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </ul>
                                </div>
                            }.into_any()
                        } else { ().into_any() }}
                        {if !changeable.is_empty() {
                            let jump_for_change = jump.clone();
                            view! {
                                <div>
                                    <div style="font-size: 11px; color: var(--text-tertiary); margin-bottom: 4px;">{ "Changeable" }</div>
                                    <ul style="list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px;">
                                        {changeable.into_iter().map(|c| {
                                            let jump_id = jump_for_change.clone();
                                            let jump_spec = jump_for_change.clone();
                                            let jump_photo = jump_for_change.clone();
                                            view! {
                                                <li style="font-size: 12px; padding: 4px 6px; background: var(--bg-secondary); border-radius: 4px;">
                                                    <IdChip label=c.id target=LinkageTarget::LightSources jump=jump_id />
                                                    {match c.spectrum_id {
                                                        Some(sid) => view! {
                                                            <span style="color: var(--text-secondary);">{ " → Spectrum " }</span>
                                                            <IdChip label=sid target=LinkageTarget::Spectrums jump=jump_spec color="var(--accent-orange)" />
                                                        }.into_any(),
                                                        None => view! { <span style="color: var(--text-tertiary);">{ " — no SpectrumReference" }</span> }.into_any(),
                                                    }}
                                                    {match c.photometry_id {
                                                        Some(pid) => view! {
                                                            <span style="color: var(--text-secondary);">{ " → Photometry " }</span>
                                                            <IdChip label=pid target=LinkageTarget::Photometries jump=jump_photo color="var(--accent-purple)" />
                                                        }.into_any(),
                                                        None => view! { <span style="color: var(--text-tertiary);">{ " — no direct PhotometryReference (picked via Emitter)" }</span> }.into_any(),
                                                    }}
                                                </li>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </ul>
                                </div>
                            }.into_any()
                        } else { ().into_any() }}
                    </div>
                }.into_any()
            }}
        </section>
    }
}

#[component]
fn EmittersSection(
    rows: Vec<EmitterRow>,
    known_ls_ids: HashSet<String>,
    known_photometry_ids: HashSet<String>,
    jump: NavJump,
) -> impl IntoView {
    let count = rows.len();
    let on_header_jump = {
        let jump = jump.clone();
        Arc::new(move || jump(LinkageTarget::Emitters)) as Arc<dyn Fn() + Send + Sync>
    };
    view! {
        <section class="linkage-section" style="padding: 10px 12px; border: 1px solid var(--border-color); border-radius: 6px;">
            <SectionHeader kind=SectionKind::Emitters count=count on_jump=on_header_jump />
            {if rows.is_empty() {
                view! { <p style="margin: 0; color: var(--accent-orange); font-size: 12px;">{ "⚠ No Emitter blocks declared. A GLDF without an Emitter is not usable in a planning tool — Variants reference Emitters to mount them on the geometry." }</p> }.into_any()
            } else {
                view! {
                    <ul style="list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px;">
                        {rows.into_iter().map(|e| {
                            let orphan_for_variant = e.mounted_by_variants.is_empty();
                            let badge = if orphan_for_variant { "⚠ " } else { "" };
                            let jump_e_id = jump.clone();
                            let jump_fixed = jump.clone();
                            let jump_change = jump.clone();
                            let jump_variants = jump.clone();
                            let mounted_for_render = e.mounted_by_variants.clone();
                            view! {
                                <li style="font-size: 12px; padding: 6px 8px; background: var(--bg-secondary); border-radius: 4px;">
                                    <div>
                                        <span style="color: var(--text-tertiary);">{ badge }</span>
                                        <IdChip label=e.id target=LinkageTarget::Emitters jump=jump_e_id />
                                        <span style="color: var(--text-secondary);">{ format!(" — {} Fixed · {} Changeable · {} Sensor", e.fixed_count, e.changeable_count, e.sensor_count) }</span>
                                    </div>
                                    {if !e.fixed_links.is_empty() {
                                        view! {
                                            <ul style="margin: 4px 0 0 16px; padding: 0; list-style: none;">
                                                {e.fixed_links.into_iter().map(|(pid, ls_opt)| {
                                                    let p_bad = !known_photometry_ids.contains(&pid);
                                                    let jump_p = jump_fixed.clone();
                                                    let jump_l = jump_fixed.clone();
                                                    view! {
                                                        <li>
                                                            <span style="color: var(--text-tertiary);">{ "Fixed: " }</span>
                                                            <IdChip
                                                                label=pid
                                                                target=LinkageTarget::Photometries
                                                                jump=jump_p
                                                                color=if p_bad { "var(--accent-orange)" } else { "var(--accent-purple)" }
                                                            />
                                                            {if p_bad { view! { <span style="color: var(--accent-orange);">{ " ⚠ unknown Photometry" }</span> }.into_any() } else { ().into_any() }}
                                                            {match ls_opt {
                                                                Some(lid) => {
                                                                    let ls_bad = !known_ls_ids.contains(&lid);
                                                                    view! {
                                                                        <>
                                                                            <span style="color: var(--text-tertiary);">{ " + LS: " }</span>
                                                                            <IdChip
                                                                                label=lid
                                                                                target=LinkageTarget::LightSources
                                                                                jump=jump_l
                                                                                color=if ls_bad { "var(--accent-orange)" } else { "var(--accent-blue)" }
                                                                            />
                                                                            {if ls_bad { view! { <span style="color: var(--accent-orange);">{ " ⚠ unknown LightSource" }</span> }.into_any() } else { ().into_any() }}
                                                                        </>
                                                                    }.into_any()
                                                                }
                                                                None => view! { <span style="color: var(--accent-orange);">{ " ⚠ no LightSourceReference (FixedLightEmitter requires one)" }</span> }.into_any(),
                                                            }}
                                                        </li>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </ul>
                                        }.into_any()
                                    } else { ().into_any() }}
                                    {if !e.changeable_links.is_empty() {
                                        view! {
                                            <ul style="margin: 4px 0 0 16px; padding: 0; list-style: none;">
                                                {e.changeable_links.into_iter().map(|pid| {
                                                    let p_bad = !known_photometry_ids.contains(&pid);
                                                    let jump_p = jump_change.clone();
                                                    view! {
                                                        <li>
                                                            <span style="color: var(--text-tertiary);">{ "Changeable: " }</span>
                                                            <IdChip
                                                                label=pid
                                                                target=LinkageTarget::Photometries
                                                                jump=jump_p
                                                                color=if p_bad { "var(--accent-orange)" } else { "var(--accent-purple)" }
                                                            />
                                                            {if p_bad { view! { <span style="color: var(--accent-orange);">{ " ⚠ unknown Photometry" }</span> }.into_any() } else { ().into_any() }}
                                                        </li>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </ul>
                                        }.into_any()
                                    } else { ().into_any() }}
                                    <div style="margin-top: 4px;">
                                        <span style="color: var(--text-tertiary);">{ "Mounted by Variants: " }</span>
                                        {if orphan_for_variant {
                                            view! { <span style="color: var(--accent-orange); font-style: italic;">{ "none — no Variant/Geometry/EmitterReference picks this emitter, so it never lands on a 3D model" }</span> }.into_any()
                                        } else {
                                            view! {
                                                <span>
                                                    {mounted_for_render.into_iter().enumerate().map(|(i, vid)| {
                                                        let jump_v = jump_variants.clone();
                                                        view! {
                                                            <>
                                                                {if i > 0 { view! { <span style="color: var(--text-tertiary);">{ ", " }</span> }.into_any() } else { ().into_any() }}
                                                                <IdChip label=vid target=LinkageTarget::Variants jump=jump_v />
                                                            </>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </span>
                                            }.into_any()
                                        }}
                                    </div>
                                </li>
                            }
                        }).collect::<Vec<_>>()}
                    </ul>
                }.into_any()
            }}
        </section>
    }
}

#[component]
fn VariantsSection(rows: Vec<VariantRow>, jump: NavJump) -> impl IntoView {
    let count = rows.len();
    let on_header_jump = {
        let jump = jump.clone();
        Arc::new(move || jump(LinkageTarget::Variants)) as Arc<dyn Fn() + Send + Sync>
    };
    view! {
        <section class="linkage-section" style="padding: 10px 12px; border: 1px solid var(--border-color); border-radius: 6px;">
            <SectionHeader kind=SectionKind::Variants count=count on_jump=on_header_jump />
            {if rows.is_empty() {
                view! { <p style="margin: 0; color: var(--accent-orange); font-size: 12px;">{ "⚠ No Variants declared. A GLDF without a Variant doesn't ship a buildable product." }</p> }.into_any()
            } else {
                view! {
                    <ul style="list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px;">
                        {rows.into_iter().map(|v| {
                            let no_emitters = v.bound_emitters.is_empty();
                            let jump_id = jump.clone();
                            let jump_emitters = jump.clone();
                            let bound_for_render = v.bound_emitters.clone();
                            view! {
                                <li style="font-size: 12px; padding: 4px 6px; background: var(--bg-secondary); border-radius: 4px;">
                                    <IdChip label=v.id target=LinkageTarget::Variants jump=jump_id />
                                    <span style="color: var(--text-secondary);">{ format!(" — {}", v.name) }</span>
                                    {if no_emitters {
                                        view! { <span style="color: var(--accent-orange); font-style: italic;">{ "  ← ⚠ no EmitterReference: this Variant has geometry but no light emission slot" }</span> }.into_any()
                                    } else {
                                        view! {
                                            <span>
                                                <span style="color: var(--text-tertiary);">{ "  ← Emitters: " }</span>
                                                {bound_for_render.into_iter().enumerate().map(|(i, eid)| {
                                                    let jump_e = jump_emitters.clone();
                                                    view! {
                                                        <>
                                                            {if i > 0 { view! { <span style="color: var(--text-tertiary);">{ ", " }</span> }.into_any() } else { ().into_any() }}
                                                            <IdChip label=eid target=LinkageTarget::Emitters jump=jump_e />
                                                        </>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </span>
                                        }.into_any()
                                    }}
                                </li>
                            }
                        }).collect::<Vec<_>>()}
                    </ul>
                }.into_any()
            }}
        </section>
    }
}
