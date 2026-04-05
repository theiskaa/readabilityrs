use crate::markdown::options::MarkdownOptions;
use crate::markdown::state::ConversionState;

/// Convert `<strong>` / `<b>` content to markdown.
pub fn convert_strong(inner: &str, opts: &MarkdownOptions, _state: &ConversionState) -> String {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    format!("{}{}{}", opts.strong_delimiter, trimmed, opts.strong_delimiter)
}

/// Convert `<em>` / `<i>` content to markdown.
pub fn convert_emphasis(inner: &str, opts: &MarkdownOptions, _state: &ConversionState) -> String {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    format!("{}{}{}", opts.emphasis_delimiter, trimmed, opts.emphasis_delimiter)
}

/// Convert inline `<code>` (not inside `<pre>`) to markdown.
pub fn convert_inline_code(inner: &str, _opts: &MarkdownOptions, _state: &ConversionState) -> String {
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
pub fn convert_strikethrough(inner: &str, _opts: &MarkdownOptions, _state: &ConversionState) -> String {
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
/// Brackets `[` and `]` are NOT escaped — they only form links when paired as
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
}
