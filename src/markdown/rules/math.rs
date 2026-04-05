/// Convert `<math>` element to markdown.
/// Uses the standardized `data-latex` attribute and `display` attribute.
pub fn convert_math(latex: &str, display: &str) -> String {
    if latex.is_empty() {
        return String::new();
    }

    if display == "block" {
        format!("\n\n$${}$$\n\n", latex)
    } else {
        format!("${}$", latex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_math() {
        assert_eq!(convert_math("x^2", "inline"), "$x^2$");
    }

    #[test]
    fn test_block_math() {
        let result = convert_math("E = mc^2", "block");
        assert!(result.contains("$$E = mc^2$$"));
    }
}
