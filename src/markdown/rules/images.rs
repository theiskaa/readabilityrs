/// Convert `<img>` element to markdown.
/// `title` is the optional title attribute.
pub fn convert_image(alt: &str, src: &str, title: &str) -> String {
    if src.is_empty() {
        return String::new();
    }
    if title.is_empty() {
        format!("![{}]({})", alt, src)
    } else {
        format!("![{}]({} \"{}\")", alt, src, title.replace('"', "\\\""))
    }
}

/// Convert `<figure>` with `<img>` and optional `<figcaption>` to markdown.
pub fn convert_figure(img_alt: &str, img_src: &str, caption: Option<&str>) -> String {
    if img_src.is_empty() {
        return String::new();
    }
    let alt = match caption {
        Some(c) if !c.trim().is_empty() => c,
        _ => img_alt,
    };
    format!("\n\n![{}]({})\n\n", alt, img_src)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_image() {
        assert_eq!(convert_image("photo", "img.jpg", ""), "![photo](img.jpg)");
    }

    #[test]
    fn test_image_with_title() {
        assert_eq!(
            convert_image("photo", "img.jpg", "A nice photo"),
            "![photo](img.jpg \"A nice photo\")"
        );
    }

    #[test]
    fn test_figure_with_caption() {
        let result = convert_figure("alt", "img.jpg", Some("A nice photo"));
        assert!(result.contains("![A nice photo](img.jpg)"));
    }

    #[test]
    fn test_figure_empty_caption_falls_back_to_alt() {
        let result = convert_figure("photo alt", "img.jpg", Some(""));
        assert!(result.contains("![photo alt](img.jpg)"));
    }

    #[test]
    fn test_figure_whitespace_caption_falls_back_to_alt() {
        let result = convert_figure("photo alt", "img.jpg", Some("   "));
        assert!(result.contains("![photo alt](img.jpg)"));
    }

    #[test]
    fn test_figure_none_caption_uses_alt() {
        let result = convert_figure("photo alt", "img.jpg", None);
        assert!(result.contains("![photo alt](img.jpg)"));
    }
}
