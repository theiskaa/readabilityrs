//! Language detection from the document.

use scraper::{Html, Selector};

/// Extract language from document's <html> element or meta tags
///
/// Checks in priority order:
/// 1. <html lang=""> attribute
/// 2. Content-Language meta tag
/// 3. http-equiv="Content-Language"
pub(super) fn extract_language_from_document(document: &Html) -> Option<String> {
    if let Some(html_elem) = document.root_element().first_child() {
        if let Some(node_ref) = scraper::ElementRef::wrap(html_elem) {
            if node_ref.value().name() == "html" {
                if let Some(lang) = node_ref.value().attr("lang") {
                    let lang = lang.trim();
                    if !lang.is_empty() {
                        return Some(lang.to_string());
                    }
                }
            }
        }
    }

    if let Ok(meta_selector) =
        Selector::parse("meta[http-equiv='Content-Language'], meta[http-equiv='content-language']")
    {
        for meta in document.select(&meta_selector) {
            if let Some(content) = meta.value().attr("content") {
                let lang = content.trim();
                if !lang.is_empty() {
                    return Some(lang.to_string());
                }
            }
        }
    }

    if let Ok(meta_selector) = Selector::parse("meta[name='lang'], meta[name='language']") {
        for meta in document.select(&meta_selector) {
            if let Some(content) = meta.value().attr("content") {
                let lang = content.trim();
                if !lang.is_empty() {
                    return Some(lang.to_string());
                }
            }
        }
    }

    None
}
