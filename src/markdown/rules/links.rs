use crate::markdown::options::{LinkStyle, MarkdownOptions};
use crate::markdown::rules::text::{escape_link_text, escape_md_title, escape_url_destination};
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

    if opts.sanitize_urls && crate::content_extractor::is_dangerous_url(href) {
        return trimmed.to_string();
    }

    // `trimmed` is already-converted child markdown whose brackets are guarded by
    // the in-link text path; only the `href` fallback is raw and needs escaping.
    let text = if trimmed.is_empty() {
        escape_link_text(href)
    } else {
        trimmed.to_string()
    };

    let dest = escape_url_destination(href);

    let title_part = if title.is_empty() {
        String::new()
    } else {
        format!(" \"{}\"", escape_md_title(title))
    };

    match opts.link_style {
        LinkStyle::Inline => format!("[{}]({}{})", text, dest, title_part),
        LinkStyle::Reference => {
            let ref_id = state.link_references.len() + 1;
            state.link_references.push((ref_id.to_string(), dest));
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
    fn test_href_with_paren_wrapped() {
        let opts = MarkdownOptions::default();
        let mut state = ConversionState::default();
        assert_eq!(
            convert_link("t", "http://e.com/a)b", "", &opts, &mut state),
            "[t](<http://e.com/a)b>)"
        );
    }

    #[test]
    fn test_title_quote_escaped() {
        let opts = MarkdownOptions::default();
        let mut state = ConversionState::default();
        assert_eq!(
            convert_link("t", "u", "a\"b", &opts, &mut state),
            "[t](u \"a\\\"b\")"
        );
    }

    #[test]
    fn test_href_fallback_text_escaped() {
        let opts = MarkdownOptions::default();
        let mut state = ConversionState::default();
        assert_eq!(
            convert_link("", "http://e.com/[x]", "", &opts, &mut state),
            "[http://e.com/\\[x\\]](http://e.com/[x])"
        );
    }

    #[test]
    fn test_dangerous_href_dropped_when_sanitizing() {
        let opts = MarkdownOptions {
            sanitize_urls: true,
            ..Default::default()
        };
        let mut state = ConversionState::default();
        assert_eq!(
            convert_link("click", "javascript:evil()", "", &opts, &mut state),
            "click"
        );
    }

    #[test]
    fn test_reference_style() {
        let opts = MarkdownOptions {
            link_style: LinkStyle::Reference,
            ..Default::default()
        };
        let mut state = ConversionState::default();
        let result = convert_link("click", "https://example.com", "", &opts, &mut state);
        assert_eq!(result, "[click][1]");
        assert_eq!(state.link_references.len(), 1);
    }
}
