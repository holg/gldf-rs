//! `<Tm33Panel>` — render a TM-33-23 XML file's two halves (photometry +
//! SPD) using the existing LdtPanel / SpdPanel renderers.
//!
//! Parses the bytes via [`eulumdat::atla::xml::parse`] into a
//! `LuminaireOpticalData`. The IntensityDistribution becomes an
//! `Eulumdat` via the lib's `From<&LuminaireOpticalData>` impl, then
//! `to_ldt()` serialises it back to text so [`LdtPanel`] can consume it
//! through its existing byte-based parser. The SpectralDistribution
//! gets re-emitted as `wavelength<space>value` rows so the generic
//! two-column path inside [`eulumdat::atla::spd_loader::parse`] picks
//! it up unchanged inside [`SpdPanel`].
//!
//! Why convert through bytes rather than skipping a layer: it keeps the
//! viewer stack identical to the `.ldt` / `.spd` paths — the user gets
//! the same diagrams, the same tab bar, the same colorimetry box. Any
//! future polish on those panels lands here for free.

use crate::components::{LdtPanel, SpdPanel};
use eulumdat::atla::xml as atla_xml;
use eulumdat::Eulumdat;
use leptos::prelude::*;

/// Renders a TM-33-23 XML file with both photometric AND spectral
/// halves visible. Falls back to a friendly error message if parsing
/// fails — keeps the file viewer chrome usable even on bad XML.
#[component]
pub fn Tm33Panel(bytes: Vec<u8>) -> impl IntoView {
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let doc = match atla_xml::parse(&text) {
        Ok(d) => d,
        Err(e) => {
            return view! {
                <div style="padding: 12px; background: #2a1818; border-radius: 4px; color: #ff8888; font-family: var(--font-mono); font-size: 12px;">
                    { format!("TM-33 parse error: {}", e) }
                </div>
            }
            .into_any();
        }
    };

    // Photometric half — synthesise an LDT and feed LdtPanel's
    // byte-driven parser. Empty when the TM-33 document doesn't carry
    // an emitter (well-formed TM-33 always does).
    let ldt_bytes: Option<Vec<u8>> = if doc.emitters.is_empty() {
        None
    } else {
        let ldt: Eulumdat = (&doc).into();
        Some(ldt.to_ldt().into_bytes())
    };

    // Spectral half — pull the first emitter's SpectralDistribution
    // and re-emit as `wavelength<space>value` rows. spd_loader's
    // generic two-column path accepts that shape unchanged.
    let spd_bytes: Option<Vec<u8>> = doc
        .emitters
        .first()
        .and_then(|e| e.spectral_distribution.as_ref())
        .and_then(|spd| {
            if spd.wavelengths.is_empty() || spd.wavelengths.len() != spd.values.len() {
                return None;
            }
            let mut buf = String::with_capacity(spd.wavelengths.len() * 16);
            buf.push_str("# Synthesised from TM-33 SpectralDistribution\n");
            for (w, v) in spd.wavelengths.iter().zip(spd.values.iter()) {
                buf.push_str(&format!("{:.3}\t{:.6}\n", w, v));
            }
            Some(buf.into_bytes())
        });

    let has_photo = ldt_bytes.is_some();
    let has_spd = spd_bytes.is_some();

    view! {
        <div class="tm33-panel" style="display: flex; flex-direction: column; gap: 16px;">
            <div style="display: flex; gap: 12px; flex-wrap: wrap; align-items: center; font-size: 12px; color: var(--text-secondary);">
                <span style="background: var(--accent-blue); color: white; border-radius: 4px; padding: 2px 8px; font-weight: 600;">{ "TM-33-23" }</span>
                <span>{ format!("schema {}", doc.version) }</span>
                <span>{ format!("{} emitter(s)", doc.emitters.len()) }</span>
                <span style=if has_photo { "color: #4caf50;" } else { "color: #aaa;" }>
                    { if has_photo { "✓ photometry" } else { "— no photometry" } }
                </span>
                <span style=if has_spd { "color: #4caf50;" } else { "color: #aaa;" }>
                    { if has_spd { "✓ spectrum" } else { "— no spectrum" } }
                </span>
            </div>
            {ldt_bytes.map(|b| view! {
                <section>
                    <h4 style="margin: 0 0 8px 0; font-size: 13px; color: var(--text-secondary);">{ "Photometric distribution" }</h4>
                    <LdtPanel bytes=b />
                </section>
            })}
            {spd_bytes.map(|b| view! {
                <section>
                    <h4 style="margin: 0 0 8px 0; font-size: 13px; color: var(--text-secondary);">{ "Spectral power distribution" }</h4>
                    <SpdPanel bytes=b />
                </section>
            })}
        </div>
    }
    .into_any()
}

/// Heuristic content-type / filename detection. The FileViewer calls
/// this before picking a renderer — `.tm33.xml`, generic `.xml` with
/// TM-33 markers in the content, or the GLDF File's content_type
/// (`ldc/tm33` if/when we ever start tagging exports that way).
pub fn is_tm33(filename: &str, content_type: &str, sniff: &[u8]) -> bool {
    let lower = filename.to_lowercase();
    if lower.ends_with(".tm33.xml") {
        return true;
    }
    if content_type.eq_ignore_ascii_case("ldc/tm33")
        || content_type.eq_ignore_ascii_case("application/vnd.iestm33+xml")
    {
        return true;
    }
    // ldc/iesxml is the XSD-legal slot for any XML-shaped photometric
    // file — including TM-33-23. The bytes-sniff disambiguates TM-33
    // from plain IES-XML.
    let head = std::str::from_utf8(&sniff[..sniff.len().min(2048)]).unwrap_or("");
    let has_tm33_root = head.contains("IESTM33-22") || head.contains("LuminaireOpticalData");
    if content_type.eq_ignore_ascii_case("ldc/iesxml") && has_tm33_root {
        return true;
    }
    if lower.ends_with(".xml") && has_tm33_root {
        return true;
    }
    false
}
