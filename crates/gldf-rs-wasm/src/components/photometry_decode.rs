//! Encoding-tolerant decoding + IES/EULUMDAT detection for photometric
//! files embedded in a GLDF bundle.
//!
//! Why this exists: a strict `std::str::from_utf8` fails on every IES file
//! produced by Windows tooling that drops non-ASCII (e.g. `°`, `µ`,
//! manufacturer-name accents) into `[MORE]` / `[LAMP]` comment lines as
//! Latin-1 (ISO-8859-1) bytes. The `eulumdat` library's `parse_file` does
//! a UTF-8-then-Latin-1 fallback (`read_with_encoding_fallback` in
//! ies.rs); the str-based `IesParser::parse(&str)` entry point we use
//! over here can't see the raw bytes, so we mirror the same fallback at
//! this layer.
//!
//! Magic-string detection: the schema permits LM-63-1986 files with no
//! version line at all (just numbers from line one) and LM-63-2019 files
//! start with `IES:LM-63-2019` (no `IESNA`). The previous
//! `starts_with("IESNA")` check missed both. We now try `IesParser::parse`
//! first and fall back to `Eulumdat::parse` if it errors — IES parse
//! errors are reliable enough that this disambiguates correctly.

use eulumdat::{Eulumdat, IesParser};

/// Decode raw photometric bytes to a `String`, falling back to ISO-8859-1
/// when the content isn't valid UTF-8. ISO-8859-1 is a single-byte
/// encoding where every byte maps to a valid `char`, so the second branch
/// is infallible.
///
/// Returns the decoded string plus a tag identifying which path took.
pub fn decode_photometry_bytes(bytes: &[u8]) -> (String, DecodeKind) {
    match std::str::from_utf8(bytes) {
        Ok(s) => (s.to_string(), DecodeKind::Utf8),
        Err(_) => {
            // Each byte → code point in the U+0000..U+00FF range (the
            // Latin-1 supplement). Every byte sequence is valid here.
            let s: String = bytes.iter().map(|&b| b as char).collect();
            (s, DecodeKind::Latin1Fallback)
        }
    }
}

/// Which decode path produced the string returned by
/// [`decode_photometry_bytes`]. Surfaced in the page probe so users can
/// tell at a glance whether their file's encoding was an issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeKind {
    /// File is valid UTF-8 (the common case).
    Utf8,
    /// File contained non-UTF-8 bytes; we decoded byte-by-byte as
    /// ISO-8859-1 / Latin-1.
    Latin1Fallback,
}

impl DecodeKind {
    /// Short human-readable label for the probe / log output.
    pub fn label(self) -> &'static str {
        match self {
            DecodeKind::Utf8 => "UTF-8",
            DecodeKind::Latin1Fallback => "Latin-1 (fallback)",
        }
    }
}

/// Which parser variant accepted the photometric content. Returned by
/// [`parse_photometry_with_fallback`] so callers can log / display which
/// path won.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseKind {
    /// `IesParser::parse` succeeded (LM-63 in any flavor).
    Ies,
    /// `Eulumdat::parse` succeeded (`.ldt` / EULUMDAT).
    Eulumdat,
    /// `eulumdat::atla::xml::parse` succeeded (TM-33-23 / ATLA-S001 XML).
    Tm33,
}

impl ParseKind {
    /// Short human-readable label for the probe / log output.
    pub fn label(self) -> &'static str {
        match self {
            ParseKind::Ies => "IES",
            ParseKind::Eulumdat => "EULUMDAT",
            ParseKind::Tm33 => "TM-33 XML",
        }
    }
}

/// Try IES first, then EULUMDAT, then TM-33 XML.
///
/// The previous magic-string approach (`starts_with("IESNA")`) missed
/// LM-63-2019 (`IES:LM-63-2019`) and LM-63-1986 (no header at all); IES
/// parse errors are reliable enough to drive a try-then-fall-back path
/// without false positives.
///
/// TM-33-23 / ATLA-S001 is IESNA's XML photometric format — content
/// that starts with `<` (after BOM/whitespace stripping) and parses
/// via `atla::xml::parse` converts cleanly to `Eulumdat` via
/// `From<&LuminaireOpticalData>`. We gate on the XML opener so we
/// don't waste a parse attempt on every malformed LDT.
///
/// Returns the parsed `Eulumdat` + which parser succeeded.
pub fn parse_photometry_with_fallback(content: &str) -> Option<(Eulumdat, ParseKind)> {
    if let Ok(ldt) = IesParser::parse(content) {
        return Some((ldt, ParseKind::Ies));
    }
    if let Ok(ldt) = Eulumdat::parse(content) {
        return Some((ldt, ParseKind::Eulumdat));
    }
    // TM-33 XML fallback. content.trim_start_matches('\u{feff}') drops
    // an optional UTF-8 BOM the way atla::xml::parse expects.
    let trimmed = content
        .trim_start_matches('\u{feff}')
        .trim_start();
    if trimmed.starts_with('<') {
        if let Ok(doc) = eulumdat::atla::xml::parse(content) {
            let ldt: Eulumdat = (&doc).into();
            return Some((ldt, ParseKind::Tm33));
        }
    }
    None
}
