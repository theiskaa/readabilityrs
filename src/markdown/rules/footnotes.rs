/// Convert a footnote reference `<sup id="fnref:N"><a href="#fn:N">N</a></sup>` to markdown.
pub fn convert_footnote_ref(id: &str) -> String {
    format!("[^{}]", id)
}

/// Format all collected footnote definitions for appending at end of document.
pub fn format_footnote_definitions(footnotes: &[(String, String)]) -> String {
    if footnotes.is_empty() {
        return String::new();
    }

    let mut out = String::from("\n\n---\n\n");
    for (id, content) in footnotes {
        out.push_str(&format!("[^{}]: {}\n", id, content.trim()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_footnote_ref() {
        assert_eq!(convert_footnote_ref("1"), "[^1]");
    }

    #[test]
    fn test_footnote_definitions() {
        let notes = vec![
            ("1".to_string(), "First note.".to_string()),
            ("2".to_string(), "Second note.".to_string()),
        ];
        let result = format_footnote_definitions(&notes);
        assert!(result.contains("[^1]: First note."));
        assert!(result.contains("[^2]: Second note."));
    }
}
