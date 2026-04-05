use crate::markdown::options::MarkdownOptions;

/// Convert a fenced code block `<pre><code>` to markdown.
/// `language` may be empty. `code` is the raw code text.
pub fn convert_code_block(code: &str, language: &str, opts: &MarkdownOptions) -> String {
    let fence_char = opts.code_fence;
    let fence = if code.contains("```") && fence_char == '`' {
        "~~~~".to_string()
    } else {
        fence_char.to_string().repeat(3)
    };

    format!("\n\n{}{}\n{}\n{}\n\n", fence, language, code, fence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_block_with_language() {
        let opts = MarkdownOptions::default();
        let result = convert_code_block("print('hi')", "python", &opts);
        assert!(result.contains("```python"));
        assert!(result.contains("print('hi')"));
    }

    #[test]
    fn test_code_block_with_triple_backticks() {
        let opts = MarkdownOptions::default();
        let result = convert_code_block("use ``` here", "md", &opts);
        assert!(result.contains("~~~~md"));
    }
}
