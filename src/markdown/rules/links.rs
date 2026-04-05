use crate::markdown::options::{LinkStyle, MarkdownOptions};
use crate::markdown::state::ConversionState;

/// Convert `<a>` element to markdown.
/// `inner` is the already-converted child content, `href` is the link target,
/// `title` is the optional title attribute.
pub fn convert_link(
    inner: &str,
    href: &str,
    title: &str,
    opts: &MarkdownOptions,
    state: &mut ConversionState,
) -> String {
    let trimmed = inner.trim();

    if href.is_empty() {
        return trimmed.to_string();
    }

    let text = if trimmed.is_empty() { href } else { trimmed };

    let title_part = if title.is_empty() {
        String::new()
    } else {
        format!(" \"{}\"", title.replace('"', "\\\""))
    };

    match opts.link_style {
        LinkStyle::Inline => format!("[{}]({}{})", text, href, title_part),
        LinkStyle::Reference => {
            let ref_id = state.link_references.len() + 1;
            state
                .link_references
                .push((format!("{}", ref_id), href.to_string()));
            format!("[{}][{}]", text, ref_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_link() {
        let opts = MarkdownOptions::default();
        let mut state = ConversionState::default();
        assert_eq!(
            convert_link("click", "https://example.com", "", &opts, &mut state),
            "[click](https://example.com)"
        );
    }

    #[test]
    fn test_link_with_title() {
        let opts = MarkdownOptions::default();
        let mut state = ConversionState::default();
        assert_eq!(
            convert_link("click", "https://example.com", "Visit", &opts, &mut state),
            "[click](https://example.com \"Visit\")"
        );
    }

    #[test]
    fn test_empty_text() {
        let opts = MarkdownOptions::default();
        let mut state = ConversionState::default();
        assert_eq!(
            convert_link("", "https://example.com", "", &opts, &mut state),
            "[https://example.com](https://example.com)"
        );
    }

    #[test]
    fn test_empty_href() {
        let opts = MarkdownOptions::default();
        let mut state = ConversionState::default();
        assert_eq!(convert_link("text", "", "", &opts, &mut state), "text");
    }

    #[test]
    fn test_reference_style() {
        let mut opts = MarkdownOptions::default();
        opts.link_style = LinkStyle::Reference;
        let mut state = ConversionState::default();
        let result = convert_link("click", "https://example.com", "", &opts, &mut state);
        assert_eq!(result, "[click][1]");
        assert_eq!(state.link_references.len(), 1);
    }
}
