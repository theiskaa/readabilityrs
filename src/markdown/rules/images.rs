use crate::markdown::options::MarkdownOptions;
use crate::markdown::rules::text::{escape_link_text, escape_md_title, escape_url_destination};

/// Convert `<img>` element to markdown.
/// `title` is the optional title attribute.
pub fn convert_image(alt: &str, src: &str, title: &str, opts: &MarkdownOptions) -> String {
    if src.is_empty() {
        return String::new();
    }
    if opts.sanitize_urls && crate::content_extractor::is_dangerous_url(src) {
        return String::new();
    }
    let alt_esc = escape_link_text(alt);
    let dest = escape_url_destination(src);
    if title.is_empty() {
        format!("![{}]({})", alt_esc, dest)
    } else {
        format!("![{}]({} \"{}\")", alt_esc, dest, escape_md_title(title))
    }
}

/// Convert `<figure>` with `<img>` and optional `<figcaption>` to markdown.
pub fn convert_figure(
    img_alt: &str,
    img_src: &str,
    caption: Option<&str>,
    opts: &MarkdownOptions,
) -> String {
    if img_src.is_empty() {
        return String::new();
    }
    if opts.sanitize_urls && crate::content_extractor::is_dangerous_url(img_src) {
        return String::new();
    }
    let alt = match caption {
        Some(c) if !c.trim().is_empty() => c,
        _ => img_alt,
    };
    format!(
        "\n\n![{}]({})\n\n",
        escape_link_text(alt),
        escape_url_destination(img_src)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> MarkdownOptions {
        MarkdownOptions::default()
    }

    #[test]
    fn test_basic_image() {
        assert_eq!(
            convert_image("photo", "img.jpg", "", &opts()),
            "![photo](img.jpg)"
        );
    }

    #[test]
    fn test_image_with_title() {
        assert_eq!(
            convert_image("photo", "img.jpg", "A nice photo", &opts()),
            "![photo](img.jpg \"A nice photo\")"
        );
    }

    #[test]
    fn test_image_alt_breakout_escaped() {
        let result = convert_image("](javascript:evil()) [x", "real.jpg", "", &opts());
        assert_eq!(result, "![\\](javascript:evil()) \\[x](real.jpg)");
    }

    #[test]
    fn test_image_src_with_paren_wrapped() {
        assert_eq!(
            convert_image("photo", "http://e.com/a)b", "", &opts()),
            "![photo](<http://e.com/a)b>)"
        );
    }

    #[test]
    fn test_image_dropped_when_sanitizing_dangerous_src() {
        let opts = MarkdownOptions {
            sanitize_urls: true,
            ..Default::default()
        };
        assert_eq!(convert_image("x", "javascript:evil()", "", &opts), "");
    }

    #[test]
    fn test_figure_with_caption() {
        let result = convert_figure("alt", "img.jpg", Some("A nice photo"), &opts());
        assert!(result.contains("![A nice photo](img.jpg)"));
    }

    #[test]
    fn test_figure_empty_caption_falls_back_to_alt() {
        let result = convert_figure("photo alt", "img.jpg", Some(""), &opts());
        assert!(result.contains("![photo alt](img.jpg)"));
    }

    #[test]
    fn test_figure_whitespace_caption_falls_back_to_alt() {
        let result = convert_figure("photo alt", "img.jpg", Some("   "), &opts());
        assert!(result.contains("![photo alt](img.jpg)"));
    }

    #[test]
    fn test_figure_none_caption_uses_alt() {
        let result = convert_figure("photo alt", "img.jpg", None, &opts());
        assert!(result.contains("![photo alt](img.jpg)"));
    }
}
