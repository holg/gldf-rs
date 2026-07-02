//! Pure transforms that turn spec-valid-but-DIALux-hostile assets
//! into DIALux-friendly ones. Each helper is byte/string in, bytes
//! out — no `GldfProduct` mutation, no I/O. Composed into the export
//! pipeline only when the user opts into DIALux compatibility.

/// Strip `#` comment lines from a `spectrum/text` (`.spd`) payload.
///
/// DIALux's spectrum parser silently aborts the **entire** GLDF
/// import if any line starts with `#`. The XSD does not validate
/// asset content, so comment-bearing SPDs pass spec validation while
/// breaking DIALux. The rest of the file format — `wavelength<ws>value`
/// rows, blank lines — DIALux handles fine, and out-of-range
/// wavelengths (360–830 nm) are also tolerated. So the minimal fix
/// is just dropping the `#` lines.
///
/// Returns a `String` with the same line ordering, just with
/// comment lines removed. Trailing newline behaviour matches the
/// input (we don't normalize line endings — that's a separate
/// concern).
pub fn sanitize_spd(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let ends_with_newline = input.ends_with('\n');
    let mut lines = input.split('\n').peekable();
    while let Some(line) = lines.next() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        out.push_str(line);
        // Re-attach the separator unless this is the trailing chunk
        // after a final `\n` (split gives us an empty tail in that
        // case which we don't want to emit a separator for) AND
        // there's no following content.
        if lines.peek().is_some() {
            out.push('\n');
        }
    }
    if ends_with_newline && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::sanitize_spd;

    #[test]
    fn strips_leading_comment_block() {
        let input = "# header\n# meta\n380\t0.5\n385\t0.6\n";
        assert_eq!(sanitize_spd(input), "380\t0.5\n385\t0.6\n");
    }

    #[test]
    fn strips_comment_with_leading_whitespace() {
        let input = "380\t0.5\n  # inline comment\n385\t0.6\n";
        assert_eq!(sanitize_spd(input), "380\t0.5\n385\t0.6\n");
    }

    #[test]
    fn preserves_data_only_input() {
        let input = "380\t0.5\n385\t0.6\n";
        assert_eq!(sanitize_spd(input), input);
    }

    #[test]
    fn empty_stays_empty() {
        assert_eq!(sanitize_spd(""), "");
    }

    #[test]
    fn preserves_trailing_newline_absence() {
        let input = "# c\n380\t0.5";
        assert_eq!(sanitize_spd(input), "380\t0.5");
    }

    #[test]
    fn keeps_blank_lines_as_is() {
        // DIALux tolerates blank lines; the sanitiser shouldn't
        // collapse them.
        let input = "# c\n380\t0.5\n\n385\t0.6\n";
        assert_eq!(sanitize_spd(input), "380\t0.5\n\n385\t0.6\n");
    }
}
