use once_cell::sync::Lazy;
use regex::Regex;

static MULTI_NEWLINE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n{3,}").unwrap());
static BLANK_LINE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n[ \t]*\n").unwrap());

/// Convert `<blockquote>` content to markdown.
/// `inner` is the already-converted child content.
/// `callout_type` is extracted from `data-callout` attribute (e.g., "warning", "note").
pub fn convert_blockquote(inner: &str, depth: usize, callout_type: Option<&str>) -> String {
    let prefix = "> ".repeat(depth.max(1));
    let mut lines: Vec<String> = Vec::new();

    // Add callout header if present
    if let Some(ct) = callout_type {
        let upper = ct.to_uppercase();
        lines.push(format!("{}[!{}]", prefix, upper));
    }

    let trimmed_inner = inner.trim();

    if trimmed_inner.is_empty() {
        return format!("\n\n{}\n\n", prefix.trim_end());
    }

    // Normalize whitespace-only lines to pure newlines, then collapse 3+ to 2.
    let normalized = BLANK_LINE_RE.replace_all(trimmed_inner, "\n\n");
    let collapsed = MULTI_NEWLINE_RE.replace_all(&normalized, "\n\n");

    for line in collapsed.lines() {
        if line.trim().is_empty() {
            lines.push(prefix.trim_end().to_string());
        } else {
            lines.push(format!("{}{}", prefix, line));
        }
    }

    // Collapse consecutive empty quote lines (e.g. ">\n>" → ">")
    let mut result_lines: Vec<String> = Vec::new();
    for line in &lines {
        let is_empty_quote = line.trim().chars().all(|c| c == '>');
        let prev_is_empty_quote = result_lines
            .last()
            .map(|l| l.trim().chars().all(|c| c == '>'))
            .unwrap_or(false);

        if is_empty_quote && prev_is_empty_quote {
            continue; // skip consecutive empty quote lines
        }
        result_lines.push(line.clone());
    }

    format!("\n\n{}\n\n", result_lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_blockquote() {
        let result = convert_blockquote("quoted text", 1, None);
        assert!(result.contains("> quoted text"));
    }

    #[test]
    fn test_nested_blockquote() {
        let result = convert_blockquote("deep", 2, None);
        assert!(result.contains("> > deep"));
    }

    #[test]
    fn test_callout() {
        let result = convert_blockquote("be careful", 1, Some("warning"));
        assert!(result.contains("> [!WARNING]"));
        assert!(result.contains("> be careful"));
    }

    #[test]
    fn test_trims_inner_blank_lines() {
        let result = convert_blockquote("\n\nquoted text\n\n", 1, None);
        assert_eq!(result.trim(), "> quoted text");
    }

    #[test]
    fn test_heading_and_paragraph() {
        let result = convert_blockquote("\n\n## Title\n\n\n\ntext\n\n", 1, None);
        let trimmed = result.trim();
        assert!(trimmed.starts_with("> ## Title"));
        assert!(trimmed.contains("> text"));
        assert!(!trimmed.contains(">\n>\n>"));
    }

    #[test]
    fn test_empty_blockquote() {
        let result = convert_blockquote("\n\n\n", 1, None);
        let trimmed = result.trim();
        assert_eq!(trimmed, ">");
    }

    #[test]
    fn test_multiple_paragraphs_no_double_empty_quote() {
        // Simulates <p>para1</p>\n<p>para2</p> inside blockquote
        let result = convert_blockquote("\n\npara1\n\n\n\npara2\n\n", 1, None);
        let trimmed = result.trim();
        assert!(trimmed.contains("> para1"));
        assert!(trimmed.contains("> para2"));
        // One blank > line between paragraphs is OK, but NOT two consecutive
        assert!(!trimmed.contains(">\n>\n>"), "triple empty quote lines: {}", trimmed);
    }

    #[test]
    fn test_whitespace_between_paragraphs() {
        // Whitespace-only lines between paragraphs
        let result = convert_blockquote("\n\npara1\n  \n  \npara2\n\n", 1, None);
        let trimmed = result.trim();
        assert!(!trimmed.contains(">\n>\n>"), "triple empty quote: {}", trimmed);
    }
}
