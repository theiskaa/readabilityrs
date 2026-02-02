//! Post-processing functions for article content after extraction.
//!
//! This module implements Mozilla's _prepArticle pipeline, which cleans
//! the extracted article content by removing unwanted elements.

use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};

/// Remove nav-heavy wrappers by descending into content-like children.
/// Note: "widget" is excluded from this pattern since page builders use it for content.
fn unwrap_nav_wrappers(html: &str) -> String {
    static WRAPPER_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r#"(?is)<div[^>]+class="[^"]*(?:navbar|nav|menu|sidebar|header)[^"]*"[^>]*>.*?</div>"#,
        )
        .unwrap()
    });

    WRAPPER_REGEX.replace_all(html, "").to_string()
}

/// Remove the title element from the article content if it matches the extracted title.
///
/// Finds the first h1 or h2 element whose text content matches the given title
/// (after normalization) and removes it from the HTML. Also cleans up any leftover
/// whitespace and empty wrapper elements.
///
/// # Arguments
/// * `html` - The article HTML content
/// * `title` - The extracted article title to match against
///
/// # Returns
/// The HTML with the matching title element removed, or the original HTML if no match found
pub fn remove_title_from_content(html: &str, title: &str) -> String {
    let doc = Html::parse_fragment(html);

    // Normalize the title for comparison
    let normalized_title = normalize_text(title);
    if normalized_title.is_empty() {
        return html.to_string();
    }

    // Try to find h1 or h2 elements that match the title
    let selector = Selector::parse("h1, h2").unwrap();

    for element in doc.select(&selector) {
        let element_text: String = element.text().collect();
        let normalized_element_text = normalize_text(&element_text);

        // Check if the heading text matches the title (exact or near match)
        if titles_match(&normalized_title, &normalized_element_text) {
            let tag_name = element.value().name();

            // Try direct string match first (fast path)
            let element_html = element.html();
            if let Some(pos) = html.find(&element_html) {
                let mut result = String::with_capacity(html.len());
                result.push_str(&html[..pos]);
                result.push_str(&html[pos + element_html.len()..]);
                return cleanup_after_title_removal(&result);
            }

            // Fall back to regex-based removal if direct match fails
            // (handles whitespace/attribute differences between parsed and original HTML)
            let result = remove_heading_by_regex(html, tag_name, &element_text);
            if result.len() < html.len() {
                return cleanup_after_title_removal(&result);
            }
        }
    }

    html.to_string()
}

/// Remove a heading element using regex when direct string matching fails.
/// This handles cases where scraper's serialized HTML differs from the original.
fn remove_heading_by_regex(html: &str, tag: &str, text: &str) -> String {
    let escaped_text = regex::escape(text.trim());

    // Build a pattern that matches the heading tag with any attributes,
    // allowing for whitespace variations and inline elements in the content
    // Use [\s\S]*? between words to handle newlines, <br> tags, etc.
    let text_pattern = escaped_text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(r"[\s\S]*?");

    let pattern = format!(
        r"(?is)<{tag}[^>]*>[\s\S]*?{text_pattern}[\s\S]*?</{tag}>",
        tag = tag,
        text_pattern = text_pattern
    );

    if let Ok(re) = Regex::new(&pattern) {
        re.replace(html, "").to_string()
    } else {
        html.to_string()
    }
}

/// Clean up whitespace and empty elements after title removal
fn cleanup_after_title_removal(html: &str) -> String {
    // Patterns for empty wrapper elements that might be left behind
    static EMPTY_HEADER_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?is)<header[^>]*>\s*</header>").unwrap());
    static EMPTY_HGROUP_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?is)<hgroup[^>]*>\s*</hgroup>").unwrap());
    static EMPTY_DIV_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?is)<div[^>]*>\s*</div>").unwrap());
    static EMPTY_SECTION_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?is)<section[^>]*>\s*</section>").unwrap());

    // Collapse multiple consecutive newlines/whitespace into single newline
    static MULTI_NEWLINE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n\s*\n\s*\n").unwrap());

    // Clean up whitespace-only lines (lines with only spaces/tabs)
    static WHITESPACE_LINE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n[ \t]+\n").unwrap());

    let mut result = html.to_string();

    // Remove empty wrapper elements (iterate to handle nested empties)
    for _ in 0..3 {
        let prev = result.clone();
        result = EMPTY_HEADER_REGEX.replace_all(&result, "").to_string();
        result = EMPTY_HGROUP_REGEX.replace_all(&result, "").to_string();
        result = EMPTY_DIV_REGEX.replace_all(&result, "").to_string();
        result = EMPTY_SECTION_REGEX.replace_all(&result, "").to_string();
        if result == prev {
            break;
        }
    }

    // Collapse excessive whitespace
    for _ in 0..3 {
        let prev = result.clone();
        result = MULTI_NEWLINE_REGEX.replace_all(&result, "\n\n").to_string();
        result = WHITESPACE_LINE_REGEX.replace_all(&result, "\n").to_string();
        if result == prev {
            break;
        }
    }

    result
}

/// Normalize text for title comparison: lowercase, collapse whitespace, trim
fn normalize_text(text: &str) -> String {
    static WHITESPACE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());
    WHITESPACE_REGEX
        .replace_all(text.trim(), " ")
        .to_lowercase()
}

/// Check if two normalized titles match (exact or one contains the other)
fn titles_match(title1: &str, title2: &str) -> bool {
    if title1 == title2 {
        return true;
    }

    // Allow for slight variations - one contains the other
    // This handles cases where the h1 might have extra text or vice versa
    let len1 = title1.len();
    let len2 = title2.len();

    // If lengths are similar (within 20%), check if one contains the other
    if len1 > 0 && len2 > 0 {
        let ratio = len1.min(len2) as f64 / len1.max(len2) as f64;
        if ratio > 0.8 && (title1.contains(title2) || title2.contains(title1)) {
            return true;
        }
    }

    false
}

/// Prepare extracted article content for final output
///
/// This implements Mozilla's _prepArticle() pipeline using regex-based cleaning
pub fn prep_article(html: &str) -> String {
    let mut html = html.to_string();

    // Unwrap nav wrappers before removing elements
    html = unwrap_nav_wrappers(&html);

    // Step 1: Remove unwanted elements
    html = remove_unwanted_elements(&html);

    // Step 2: Remove share buttons and social widgets
    html = remove_share_elements(&html);

    // Step 2b: Remove navigation lists/menus
    html = remove_navigation_elements(&html);

    // Step 3: Remove empty paragraphs
    html = remove_empty_paragraphs(&html);

    html
}

/// Remove unwanted elements that are never part of article content
///
/// Removes: forms, fieldsets, footer, aside, object, embed, iframe,
/// input, textarea, select, button
fn remove_unwanted_elements(html: &str) -> String {
    let mut result = html.to_string();
    let tags = vec![
        ("form", r"(?is)<form\b[^>]*?>.*?</form>"),
        ("fieldset", r"(?is)<fieldset\b[^>]*?>.*?</fieldset>"),
        ("footer", r"(?is)<footer\b[^>]*?>.*?</footer>"),
        ("aside", r"(?is)<aside\b[^>]*?>.*?</aside>"),
        ("object", r"(?is)<object\b[^>]*?>.*?</object>"),
        (
            "embed",
            r"(?is)<embed\b[^>]*?>.*?</embed>|<embed\b[^>]*?/?>",
        ),
        ("iframe", r"(?is)<iframe\b[^>]*?>.*?</iframe>"),
        (
            "input",
            r"(?is)<input\b[^>]*?>.*?</input>|<input\b[^>]*?/?>",
        ),
        ("textarea", r"(?is)<textarea\b[^>]*?>.*?</textarea>"),
        ("select", r"(?is)<select\b[^>]*?>.*?</select>"),
        ("button", r"(?is)<button\b[^>]*?>.*?</button>"),
        ("link", r"(?is)<link\b[^>]*?>.*?</link>|<link\b[^>]*?/?>"),
    ];

    for (_name, pattern) in tags {
        let re = Regex::new(pattern).unwrap();
        result = re.replace_all(&result, "").to_string();
    }

    result
}

/// Remove share buttons and social widgets
///
/// Removes elements with "share" or "social" in their class/id
fn remove_share_elements(html: &str) -> String {
    let mut result = html.to_string();
    let tags = vec!["div", "span", "aside", "section"];
    let keywords = vec!["share", "social", "sharedaddy"];

    for tag in &tags {
        for keyword in &keywords {
            let class_pattern =
                format!(r#"(?is)<{tag}\b[^>]*?class="[^"]*?{keyword}[^"]*?"[^>]*?>.*?</{tag}>"#);
            let re = Regex::new(&class_pattern).unwrap();
            result = re.replace_all(&result, "").to_string();

            let id_pattern =
                format!(r#"(?is)<{tag}\b[^>]*?id="[^"]*?{keyword}[^"]*?"[^>]*?>.*?</{tag}>"#);
            let re = Regex::new(&id_pattern).unwrap();
            result = re.replace_all(&result, "").to_string();
        }
    }

    result
}

/// Remove navigation lists and menu sections
fn remove_navigation_elements(html: &str) -> String {
    let mut result = html.to_string();

    static NAV_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?is)<nav\b[^>]*?>.*?</nav>").unwrap());
    result = NAV_REGEX.replace_all(&result, "").to_string();

    let tags = vec!["div", "section", "ul", "ol"];
    let keywords = vec!["nav", "navbar", "menu", "breadcrumbs"];

    for tag in &tags {
        for keyword in &keywords {
            let class_pattern =
                format!(r#"(?is)<{tag}\b[^>]*?class="[^"]*?{keyword}[^"]*?"[^>]*?>.*?</{tag}>"#);
            let re = Regex::new(&class_pattern).unwrap();
            result = re.replace_all(&result, "").to_string();

            let id_pattern =
                format!(r#"(?is)<{tag}\b[^>]*?id="[^"]*?{keyword}[^"]*?"[^>]*?>.*?</{tag}>"#);
            let re = Regex::new(&id_pattern).unwrap();
            result = re.replace_all(&result, "").to_string();
        }
    }

    result
}

/// Remove empty paragraphs (paragraphs with no text and no media elements)
fn remove_empty_paragraphs(html: &str) -> String {
    static EMPTY_P_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"<p[^>]*?>\s*</p>").unwrap());

    let mut html = html.to_string();
    loop {
        let new_html = EMPTY_P_REGEX.replace_all(&html, "").to_string();
        if new_html == html {
            break;
        }
        html = new_html;
    }

    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_unwanted_elements() {
        let html = r#"
            <article>
                <h1>Title</h1>
                <p>Content</p>
                <footer>Footer content</footer>
                <form><input type="text"></form>
            </article>
        "#;

        let cleaned = remove_unwanted_elements(html);

        assert!(cleaned.contains("<h1>Title</h1>"));
        assert!(cleaned.contains("<p>Content</p>"));
        assert!(!cleaned.contains("<footer"));
        assert!(!cleaned.contains("<form"));
    }

    #[test]
    fn test_remove_empty_paragraphs() {
        let html = r#"
            <div>
                <p>Good paragraph</p>
                <p></p>
                <p>   </p>
                <p>Another good one</p>
            </div>
        "#;

        let cleaned = remove_empty_paragraphs(html);

        assert!(cleaned.contains("<p>Good paragraph</p>"));
        assert!(cleaned.contains("<p>Another good one</p>"));
        assert!(!cleaned.contains("<p></p>"));
        assert!(!cleaned.contains("<p>   </p>"));
    }

    #[test]
    fn test_remove_share_elements() {
        let html = r##"
            <div>
                <p>Article content</p>
                <div class="share-buttons">
                    <a href="#">Share</a>
                </div>
                <div class="social-media">
                    <a href="#">Follow</a>
                </div>
            </div>
        "##;

        let cleaned = remove_share_elements(html);

        assert!(cleaned.contains("<p>Article content</p>"));
        assert!(!cleaned.contains("share-buttons"));
        assert!(!cleaned.contains("social-media"));
    }

    #[test]
    fn test_remove_navigation_elements() {
        let html = r##"
            <div>
                <nav>Nav content</nav>
                <div class="navbar menu">
                    <ul>
                        <li><a href="#">Home</a></li>
                        <li><a href="#">About</a></li>
                    </ul>
                </div>
                <p>Main article paragraph</p>
            </div>
        "##;

        let cleaned = remove_navigation_elements(html);

        assert!(cleaned.contains("<p>Main article paragraph</p>"));
        assert!(!cleaned.contains("<nav>"));
        assert!(!cleaned.contains("navbar"));
    }

    #[test]
    fn test_prep_article_full() {
        let html = r#"
            <article>
                <h1>Article Title</h1>
                <p>First paragraph</p>
                <p></p>
                <footer>Page footer</footer>
                <p>Second paragraph</p>
                <div class="share">Share this!</div>
                <form><input/></form>
            </article>
        "#;

        let cleaned = prep_article(html);

        assert!(cleaned.contains("<h1>Article Title</h1>"));
        assert!(cleaned.contains("<p>First paragraph</p>"));
        assert!(cleaned.contains("<p>Second paragraph</p>"));
        assert!(!cleaned.contains("<footer"));
        assert!(!cleaned.contains("<form"));
        assert!(!cleaned.contains("<p></p>"));
    }

    #[test]
    fn test_remove_title_from_content_h1() {
        let html = r#"
            <article>
                <h1>Article Title</h1>
                <p>First paragraph</p>
                <p>Second paragraph</p>
            </article>
        "#;

        let cleaned = remove_title_from_content(html, "Article Title");

        assert!(!cleaned.contains("<h1>"));
        assert!(!cleaned.contains("Article Title"));
        assert!(cleaned.contains("<p>First paragraph</p>"));
        assert!(cleaned.contains("<p>Second paragraph</p>"));
    }

    #[test]
    fn test_remove_title_from_content_h2() {
        let html = r#"
            <article>
                <h2>Article Title</h2>
                <p>First paragraph</p>
            </article>
        "#;

        let cleaned = remove_title_from_content(html, "Article Title");

        assert!(!cleaned.contains("<h2>"));
        assert!(!cleaned.contains("Article Title"));
        assert!(cleaned.contains("<p>First paragraph</p>"));
    }

    #[test]
    fn test_remove_title_from_content_with_whitespace() {
        let html = r#"
            <article>
                <h1>  Article   Title  </h1>
                <p>Content</p>
            </article>
        "#;

        let cleaned = remove_title_from_content(html, "Article Title");

        assert!(!cleaned.contains("<h1>"));
        assert!(cleaned.contains("<p>Content</p>"));
    }

    #[test]
    fn test_remove_title_from_content_case_insensitive() {
        let html = r#"
            <article>
                <h1>ARTICLE TITLE</h1>
                <p>Content</p>
            </article>
        "#;

        let cleaned = remove_title_from_content(html, "Article Title");

        assert!(!cleaned.contains("<h1>"));
        assert!(cleaned.contains("<p>Content</p>"));
    }

    #[test]
    fn test_remove_title_from_content_no_match() {
        let html = r#"
            <article>
                <h1>Different Title</h1>
                <p>Content</p>
            </article>
        "#;

        let cleaned = remove_title_from_content(html, "Article Title");

        // Should preserve the h1 when no match
        assert!(cleaned.contains("<h1>Different Title</h1>"));
        assert!(cleaned.contains("<p>Content</p>"));
    }

    #[test]
    fn test_remove_title_from_content_empty_title() {
        let html = r#"
            <article>
                <h1>Article Title</h1>
                <p>Content</p>
            </article>
        "#;

        let cleaned = remove_title_from_content(html, "");

        // Should preserve everything when title is empty
        assert!(cleaned.contains("<h1>Article Title</h1>"));
        assert!(cleaned.contains("<p>Content</p>"));
    }

    #[test]
    fn test_remove_title_cleans_empty_header() {
        let html = r#"<article>
  <header>
    <h1>Article Title</h1>
  </header>
  <p>Content</p>
</article>"#;

        let cleaned = remove_title_from_content(html, "Article Title");

        assert!(!cleaned.contains("<h1>"));
        assert!(!cleaned.contains("<header"));
        assert!(cleaned.contains("<p>Content</p>"));
    }

    #[test]
    fn test_remove_title_cleans_whitespace() {
        let html = r#"<article>
    <h1>Article Title</h1>


    <p>Content</p>
</article>"#;

        let cleaned = remove_title_from_content(html, "Article Title");

        assert!(!cleaned.contains("<h1>"));
        // Should not have excessive blank lines
        assert!(!cleaned.contains("\n\n\n"));
        assert!(cleaned.contains("<p>Content</p>"));
    }

    #[test]
    fn test_remove_title_preserves_header_with_other_content() {
        let html = r#"<article>
  <header>
    <h1>Article Title</h1>
    <p class="meta">By Author</p>
  </header>
  <p>Content</p>
</article>"#;

        let cleaned = remove_title_from_content(html, "Article Title");

        assert!(!cleaned.contains("<h1>"));
        // Header should remain because it has other content
        assert!(cleaned.contains("<header>"));
        assert!(cleaned.contains("By Author"));
        assert!(cleaned.contains("<p>Content</p>"));
    }
}
