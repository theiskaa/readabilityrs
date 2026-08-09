use crate::markdown::options::MarkdownOptions;
use crate::markdown::state::ConversionState;

/// Convert `<strong>` / `<b>` content to markdown.
pub fn convert_strong(inner: &str, opts: &MarkdownOptions, _state: &ConversionState) -> String {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    format!(
        "{}{}{}",
        opts.strong_delimiter, trimmed, opts.strong_delimiter
    )
}

/// Convert `<em>` / `<i>` content to markdown.
pub fn convert_emphasis(inner: &str, opts: &MarkdownOptions, _state: &ConversionState) -> String {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    format!(
        "{}{}{}",
        opts.emphasis_delimiter, trimmed, opts.emphasis_delimiter
    )
}

/// Convert inline `<code>` (not inside `<pre>`) to markdown.
pub fn convert_inline_code(
    inner: &str,
    _opts: &MarkdownOptions,
    _state: &ConversionState,
) -> String {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    // If inner text contains backticks, use double backticks with padding
    if trimmed.contains('`') {
        format!("`` {} ``", trimmed)
    } else {
        format!("`{}`", trimmed)
    }
}

/// Convert `<del>` / `<s>` / `<strike>` to markdown.
pub fn convert_strikethrough(
    inner: &str,
    _opts: &MarkdownOptions,
    _state: &ConversionState,
) -> String {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    format!("~~{}~~", trimmed)
}

/// Convert `<mark>` to markdown (extended syntax).
pub fn convert_highlight(inner: &str, _opts: &MarkdownOptions, _state: &ConversionState) -> String {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    format!("=={}==", trimmed)
}

/// Convert `<br>` to markdown.
pub fn convert_br() -> String {
    "  \n".to_string()
}

/// Convert `<hr>` to markdown.
pub fn convert_hr() -> String {
    "\n\n---\n\n".to_string()
}

/// Escape markdown special characters in plain text.
///
/// Escapes characters that are ambiguous inline: `\`, `` ` ``, `*`, `_`, `~`.
/// Brackets `[` and `]` are NOT escaped: they only form links when paired as
/// `[text](url)`, which the converter produces explicitly for real links.
/// Characters like `.`, `!`, `-`, `#`, `+` are only special at line-start.
pub fn escape_markdown(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' | '`' | '*' | '_' | '~' => {
                result.push('\\');
                result.push(ch);
            }
            _ => result.push(ch),
        }
    }
    result
}

/// Escape characters that let text break out of a `[...]` link/image label.
///
/// Escapes backslash first, then `[` and `]`. Used for image `alt` text and for
/// link display text derived from an `href` fallback, where an unescaped bracket
/// would prematurely close the label and let attacker-controlled text become a
/// live link destination.
pub(crate) fn escape_link_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' | '[' | ']' => {
                result.push('\\');
                result.push(ch);
            }
            _ => result.push(ch),
        }
    }
    result
}

/// Escape only `[` and `]` in already-escaped text.
///
/// Used to post-process a text node that has already run through
/// `escape_markdown` (which escapes backslashes) when it sits inside a link
/// label, so brackets cannot close the `[...]` early. Escaping backslashes here
/// too would double-escape what `escape_markdown` produced.
pub(crate) fn escape_link_brackets(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '[' | ']' => {
                result.push('\\');
                result.push(ch);
            }
            _ => result.push(ch),
        }
    }
    result
}

/// Make a URL safe to sit inside a Markdown destination `(...)`.
///
/// Control characters (including `\n`/`\r`/`\t`) are stripped first (a
/// destination may not contain them). If the result contains a space, `(`, `)`,
/// `<`, or `>`, it is wrapped in the CommonMark `<...>` destination form with any
/// remaining angle brackets percent-encoded so they cannot close the wrapper.
/// Ordinary URLs are returned byte-identical to today's output.
pub(crate) fn escape_url_destination(url: &str) -> String {
    let stripped: String = url.chars().filter(|c| !c.is_control()).collect();

    let needs_wrap = stripped
        .chars()
        .any(|c| matches!(c, ' ' | '(' | ')' | '<' | '>'));
    if !needs_wrap {
        return stripped;
    }

    let mut inner = String::with_capacity(stripped.len());
    for ch in stripped.chars() {
        match ch {
            '<' => inner.push_str("%3C"),
            '>' => inner.push_str("%3E"),
            _ => inner.push(ch),
        }
    }
    format!("<{}>", inner)
}

/// Escape a Markdown title string for the `"..."` form.
///
/// Escapes backslash first, then `"`, and strips newlines so the title cannot
/// break out of its quotes or span lines.
pub(crate) fn escape_md_title(title: &str) -> String {
    let mut result = String::with_capacity(title.len());
    for ch in title.chars() {
        match ch {
            '\n' | '\r' => {}
            '\\' | '"' => {
                result.push('\\');
                result.push(ch);
            }
            _ => result.push(ch),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strong() {
        let opts = MarkdownOptions::default();
        let state = ConversionState::default();
        assert_eq!(convert_strong("bold", &opts, &state), "**bold**");
    }

    #[test]
    fn test_emphasis() {
        let opts = MarkdownOptions::default();
        let state = ConversionState::default();
        assert_eq!(convert_emphasis("italic", &opts, &state), "*italic*");
    }

    #[test]
    fn test_inline_code_with_backticks() {
        let opts = MarkdownOptions::default();
        let state = ConversionState::default();
        assert_eq!(convert_inline_code("a`b", &opts, &state), "`` a`b ``");
    }

    #[test]
    fn test_escape_markdown() {
        assert_eq!(escape_markdown("hello *world*"), "hello \\*world\\*");
    }

    #[test]
    fn test_escape_link_text_benign_unchanged() {
        assert_eq!(escape_link_text("A nice photo"), "A nice photo");
    }

    #[test]
    fn test_escape_link_text_brackets_escaped() {
        assert_eq!(
            escape_link_text("](javascript:evil()) [x"),
            "\\](javascript:evil()) \\[x"
        );
    }

    #[test]
    fn test_escape_link_text_backslash_first() {
        assert_eq!(escape_link_text("a\\[b"), "a\\\\\\[b");
    }

    #[test]
    fn test_escape_url_destination_benign_unchanged() {
        assert_eq!(
            escape_url_destination("https://example.com/a?b=c#d"),
            "https://example.com/a?b=c#d"
        );
    }

    #[test]
    fn test_escape_url_destination_wraps_on_paren() {
        assert_eq!(
            escape_url_destination("http://e.com/a)b"),
            "<http://e.com/a)b>"
        );
    }

    #[test]
    fn test_escape_url_destination_wraps_on_space() {
        assert_eq!(
            escape_url_destination("http://e.com/a b"),
            "<http://e.com/a b>"
        );
    }

    #[test]
    fn test_escape_url_destination_percent_encodes_angles_when_wrapped() {
        assert_eq!(
            escape_url_destination("http://e.com/(a<b>"),
            "<http://e.com/(a%3Cb%3E>"
        );
    }

    #[test]
    fn test_escape_url_destination_strips_control_chars() {
        assert_eq!(
            escape_url_destination("java\tscript:alert(1)"),
            "<javascript:alert(1)>"
        );
    }

    #[test]
    fn test_escape_md_title_quote_and_backslash() {
        assert_eq!(escape_md_title("a\"b\\c"), "a\\\"b\\\\c");
    }

    #[test]
    fn test_escape_md_title_strips_newlines() {
        assert_eq!(escape_md_title("a\nb\rc"), "abc");
    }
}
