use crate::markdown::options::MarkdownOptions;
use crate::markdown::state::ConversionState;

/// Convert `<li>` content for an unordered list item.
pub fn convert_unordered_item(
    inner: &str,
    opts: &MarkdownOptions,
    state: &ConversionState,
) -> String {
    let indent = "  ".repeat(state.list_depth.saturating_sub(1));
    let trimmed = inner.trim();
    format!("{}{} {}\n", indent, opts.bullet_char, trimmed)
}

/// Convert `<li>` content for an ordered list item.
pub fn convert_ordered_item(
    inner: &str,
    counter: usize,
    state: &ConversionState,
) -> String {
    let indent = "  ".repeat(state.list_depth.saturating_sub(1));
    let trimmed = inner.trim();
    format!("{}{}. {}\n", indent, counter, trimmed)
}

/// Convert a task list item `<li><input type="checkbox">`.
pub fn convert_task_item(
    inner: &str,
    checked: bool,
    opts: &MarkdownOptions,
    state: &ConversionState,
) -> String {
    let indent = "  ".repeat(state.list_depth.saturating_sub(1));
    let checkbox = if checked { "[x]" } else { "[ ]" };
    let trimmed = inner.trim();
    format!("{}{} {} {}\n", indent, opts.bullet_char, checkbox, trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unordered_item() {
        let opts = MarkdownOptions::default();
        let mut state = ConversionState::default();
        state.list_depth = 1;
        assert_eq!(convert_unordered_item("item", &opts, &state), "- item\n");
    }

    #[test]
    fn test_nested_ordered_item() {
        let mut state = ConversionState::default();
        state.list_depth = 2;
        assert_eq!(convert_ordered_item("item", 3, &state), "  3. item\n");
    }

    #[test]
    fn test_task_item() {
        let opts = MarkdownOptions::default();
        let mut state = ConversionState::default();
        state.list_depth = 1;
        assert_eq!(
            convert_task_item("done", true, &opts, &state),
            "- [x] done\n"
        );
    }
}
