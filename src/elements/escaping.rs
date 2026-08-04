//! Entity-aware HTML escaping shared by the standardization passes.
//!
//! The standardization pipeline runs on HTML that has already been serialized
//! and escaped by the extractor, so a blind `&` -> `&amp;` replace doubles every
//! character reference that is already there: `?a=1&amp;b=2` becomes
//! `?a=1&amp;amp;b=2`. Every pass in this directory escapes through here so that
//! logic exists once.

/// Maximum characters scanned after an `&` when looking for a terminating `;`.
/// Bounds the entity lookahead so a `&` followed by megabytes of alphanumerics
/// can't turn this into an unbounded scan.
const ENTITY_LOOKAHEAD_LIMIT: usize = 32;

/// Escape `<`, `>`, and bare `&` for inclusion in serialized HTML, plus `"` when
/// `escape_quotes` is set.
///
/// An `&` that already begins a well-formed character reference (`&name;`,
/// `&#123;`, `&#x1F;`) is left untouched, so pre-escaped input is not doubled. A
/// bare `&`, or one that doesn't resolve to a terminated entity within
/// [`ENTITY_LOOKAHEAD_LIMIT`] characters, is still escaped.
///
/// Attribute values need `escape_quotes`; code block bodies do not, since a
/// quote inside `<pre><code>` is ordinary text and escaping it would corrupt the
/// snippet.
pub(crate) fn escape_html_preserving_entities(s: &str, escape_quotes: bool) -> String {
    let mut result = String::with_capacity(s.len());

    for (i, c) in s.char_indices() {
        match c {
            '"' if escape_quotes => result.push_str("&quot;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '&' => {
                if is_entity_start(&s[i..]) {
                    result.push('&');
                } else {
                    result.push_str("&amp;");
                }
            }
            _ => result.push(c),
        }
    }

    result
}

/// Returns true if `s` (which starts with `&`) begins a well-formed HTML
/// character reference terminated by `;` within [`ENTITY_LOOKAHEAD_LIMIT`]
/// characters: `&name;`, `&#digits;`, or `&#x`/`&#X` + hex digits + `;`.
fn is_entity_start(s: &str) -> bool {
    let rest = &s[1..];
    let bounded_end = rest
        .char_indices()
        .nth(ENTITY_LOOKAHEAD_LIMIT)
        .map(|(idx, _)| idx)
        .unwrap_or(rest.len());
    let window = &rest[..bounded_end];

    let Some(semi_offset) = window.find(';') else {
        return false;
    };
    let body = &window[..semi_offset];

    if let Some(numeric) = body.strip_prefix('#') {
        if let Some(hex) = numeric
            .strip_prefix('x')
            .or_else(|| numeric.strip_prefix('X'))
        {
            return !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit());
        }
        return !numeric.is_empty() && numeric.chars().all(|c| c.is_ascii_digit());
    }

    let mut chars = body.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => chars.all(|c| c.is_ascii_alphanumeric()),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preserves_existing_entity() {
        assert_eq!(
            escape_html_preserving_entities("a?x=1&amp;y=2", true),
            "a?x=1&amp;y=2"
        );
    }

    #[test]
    fn test_escapes_bare_ampersand() {
        assert_eq!(
            escape_html_preserving_entities("a?x=1&y=2", true),
            "a?x=1&amp;y=2"
        );
    }

    #[test]
    fn test_numeric_entities_and_bare_ampersand() {
        assert_eq!(
            escape_html_preserving_entities("&#38; &#x26; &notanentity", true),
            "&#38; &#x26; &amp;notanentity"
        );
    }

    #[test]
    fn test_quotes_escaped_only_when_requested() {
        assert_eq!(
            escape_html_preserving_entities(r#"say "hi""#, true),
            "say &quot;hi&quot;"
        );
        assert_eq!(
            escape_html_preserving_entities(r#"say "hi""#, false),
            r#"say "hi""#
        );
    }

    #[test]
    fn test_multibyte_utf8_does_not_panic() {
        let value = "caf\u{e9} \u{1F600} \u{4f60}\u{597d} & <tag> \"quoted\"";
        assert_eq!(
            escape_html_preserving_entities(value, true),
            "caf\u{e9} \u{1F600} \u{4f60}\u{597d} &amp; &lt;tag&gt; &quot;quoted&quot;"
        );
    }

    #[test]
    fn test_unterminated_entity_beyond_lookahead_is_escaped() {
        let long_run = "a".repeat(ENTITY_LOOKAHEAD_LIMIT + 5);
        let input = format!("&{long_run};");
        assert!(escape_html_preserving_entities(&input, true).starts_with("&amp;"));
    }
}
