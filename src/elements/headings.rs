use once_cell::sync::Lazy;
use regex::Regex;

static PERMALINK_TEXT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[#¶§🔗\s]*$").unwrap());

static H1_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?si)<h1[^>]*>(.*?)</h1>").unwrap());

static HEADING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?si)(<h[1-6][^>]*>)(.*?)(</h[1-6]>)").unwrap());

static ANCHOR_IN_HEADING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r##"(?si)<a\s+[^>]*href="#[^"]*"[^>]*>(.*?)</a>"##).unwrap());

/// Standardize headings:
/// 1. Remove first `<h1>` if its text matches the article title.
/// 2. Rename remaining `<h1>` → `<h2>`.
/// 3. Strip permalink/anchor `<a>` children from all headings.
pub fn standardize_headings(html: &str, title: Option<&str>) -> String {
    let mut output = html.to_string();
    let mut first_h1 = true;

    // Process h1 elements
    output = H1_RE.replace_all(&output, |caps: &regex::Captures| {
        let inner = caps[1].to_string();
        let text = strip_html_tags(&inner);
        let text_normalized = normalize_title_text(&text);

        if first_h1 {
            first_h1 = false;
            if let Some(t) = title {
                let title_normalized = normalize_title_text(t);
                if text_normalized == title_normalized {
                    return String::new();
                }
            }
        }

        // Rename h1 → h2
        format!("<h2>{}</h2>", inner)
    }).to_string();

    // Strip permalink anchors from all headings
    output = strip_permalink_anchors(&output);

    output
}

/// Remove anchor links inside headings that look like permalink markers.
fn strip_permalink_anchors(html: &str) -> String {
    // Only process anchors that are inside heading tags
    HEADING_RE.replace_all(html, |caps: &regex::Captures| {
        let open_tag = &caps[1];
        let inner = &caps[2];
        let close_tag = &caps[3];

        let cleaned_inner = ANCHOR_IN_HEADING_RE.replace_all(inner, |acaps: &regex::Captures| {
            let link_text = acaps.get(1).map(|m| m.as_str()).unwrap_or("");
            if link_text.trim().is_empty() || PERMALINK_TEXT_RE.is_match(link_text.trim()) {
                String::new()
            } else {
                link_text.to_string()
            }
        });

        format!("{}{}{}", open_tag, cleaned_inner, close_tag)
    }).to_string()
}

static HTML_TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]+>").unwrap());

/// Strip all HTML tags from a string to get plain text.
fn strip_html_tags(html: &str) -> String {
    HTML_TAG_RE.replace_all(html, "").to_string()
}

/// Normalize title text for comparison.
fn normalize_title_text(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .trim_matches(|c: char| c.is_ascii_punctuation())
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_matching_h1() {
        let html = r#"<h1>My Article Title</h1><p>Content</p>"#;
        let result = standardize_headings(html, Some("My Article Title"));
        assert!(!result.contains("<h1>"));
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_rename_h1_to_h2() {
        let html = r#"<h1>Other Heading</h1><p>Content</p>"#;
        let result = standardize_headings(html, Some("Different Title"));
        assert!(result.contains("<h2>Other Heading</h2>"));
    }

    #[test]
    fn test_strip_permalink() {
        let html = r##"<h2>Heading <a href="#heading" class="anchor">#</a></h2>"##;
        let result = standardize_headings(html, None);
        assert!(!result.contains("<a"));
        assert!(result.contains("Heading"));
    }

    #[test]
    fn test_keep_real_links_in_headings() {
        let html = r#"<h2>See <a href="https://example.com">this</a></h2>"#;
        let result = standardize_headings(html, None);
        // Non-anchor links should be preserved
        assert!(result.contains("https://example.com"));
    }
}
