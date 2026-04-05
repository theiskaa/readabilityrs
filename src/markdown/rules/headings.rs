use crate::markdown::options::{HeadingStyle, MarkdownOptions};

/// Convert heading element to markdown.
/// `level` is 1–6, `inner` is the already-converted inner content.
pub fn convert_heading(level: u8, inner: &str, opts: &MarkdownOptions) -> String {
    // Collapse internal whitespace runs (from <br> or source formatting)
    let trimmed: String = inner.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.is_empty() {
        return String::new();
    }

    match opts.heading_style {
        HeadingStyle::Atx => {
            let prefix = "#".repeat(level as usize);
            format!("\n\n{} {}\n\n", prefix, trimmed)
        }
        HeadingStyle::Setext if level <= 2 => {
            let underline_char = if level == 1 { '=' } else { '-' };
            let underline = underline_char.to_string().repeat(trimmed.len().max(3));
            format!("\n\n{}\n{}\n\n", trimmed, underline)
        }
        // Setext only supports h1/h2, fall back to ATX for h3+
        HeadingStyle::Setext => {
            let prefix = "#".repeat(level as usize);
            format!("\n\n{} {}\n\n", prefix, trimmed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atx_headings() {
        let opts = MarkdownOptions::default();
        assert_eq!(convert_heading(1, "Title", &opts), "\n\n# Title\n\n");
        assert_eq!(convert_heading(3, "Sub", &opts), "\n\n### Sub\n\n");
    }

    #[test]
    fn test_setext_headings() {
        let mut opts = MarkdownOptions::default();
        opts.heading_style = HeadingStyle::Setext;
        let result = convert_heading(1, "Title", &opts);
        assert!(result.contains("Title\n====="));
    }
}
