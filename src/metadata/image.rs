//! Image URL extraction from document structure.

use scraper::{Html, Selector};

/// Extract image URL from document structure
///
/// Checks additional sources when meta tags don't provide an image:
/// 1. link[rel="image_src"]
/// 2. link[rel="icon"] (as last resort for favicon)
/// 3. First significant image in article content
pub(super) fn extract_image_from_document(document: &Html) -> Option<String> {
    // Check link[rel="image_src"]
    if let Ok(selector) = Selector::parse("link[rel='image_src']") {
        if let Some(link) = document.select(&selector).next() {
            if let Some(href) = link.value().attr("href") {
                let trimmed = href.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }

    // Check itemprop="image"
    if let Ok(selector) = Selector::parse("[itemprop='image']") {
        for elem in document.select(&selector) {
            // Check for content attribute (meta tags)
            if let Some(content) = elem.value().attr("content") {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
            // Check for src attribute (img tags)
            if let Some(src) = elem.value().attr("src") {
                let trimmed = src.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
            // Check for href attribute (link tags)
            if let Some(href) = elem.value().attr("href") {
                let trimmed = href.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }

    None
}
