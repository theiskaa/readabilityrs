//! DOM manipulation and traversal utilities.

use crate::constants::{PHRASING_ELEMS, REGEXPS};
use scraper::{ElementRef, Html, Selector};

/// Get inner text of an element - cross browser compatibly.
/// This also strips out any excess whitespace to be found.
///
/// # Arguments
/// * `element` - The element to extract text from
/// * `normalize_spaces` - Whether to normalize whitespace (default: true)
pub fn get_inner_text(element: ElementRef, normalize_spaces: bool) -> String {
    let text = element.text().collect::<String>();
    let trimmed = text.trim();

    if normalize_spaces {
        crate::utils::normalize_whitespace(trimmed)
    } else {
        trimmed.to_string()
    }
}

/// Get the density of links as a percentage of the content.
/// This is the amount of text that is inside a link divided by the total text in the node.
///
/// # Arguments
/// * `element` - The element to calculate link density for
///
/// # Returns
/// The link density as a float between 0.0 and 1.0
pub fn get_link_density(element: ElementRef) -> f64 {
    let text_length = get_inner_text(element, false).len();
    if text_length == 0 {
        return 0.0;
    }

    let mut link_length = 0.0;

    let link_selector = Selector::parse("a").unwrap();
    for link in element.select(&link_selector) {
        if let Some(href) = link.value().attr("href") {
            let coefficient = if REGEXPS.hash_url.is_match(href) {
                0.3
            } else {
                1.0
            };
            link_length += get_inner_text(link, false).len() as f64 * coefficient;
        }
    }

    link_length / text_length as f64
}

/// Check if a node is phrasing content (inline element).
///
/// Phrasing content is the text of the document, as well as elements that mark up that text
/// at the intra-paragraph level (e.g., hyperlinks, stress emphasis, etc.).
///
/// # Arguments
/// * `element` - The element to check
///
/// # Returns
/// True if the element is phrasing content
pub fn is_phrasing_content(element: ElementRef) -> bool {
    let tag_name = element.value().name().to_uppercase();

    if PHRASING_ELEMS.contains(&tag_name.as_str()) {
        return true;
    }

    // Special handling for A, DEL, INS - they're phrasing only if all children are phrasing
    if tag_name == "A" || tag_name == "DEL" || tag_name == "INS" {
        return element.children().all(|child| {
            if let Some(child_elem) = ElementRef::wrap(child) {
                is_phrasing_content(child_elem)
            } else {
                true
            }
        });
    }

    false
}

/// Check if a node is probably visible.
///
/// Checks CSS display/visibility, hidden attribute, and aria-hidden.
///
/// # Arguments
/// * `element` - The element to check
///
/// # Returns
/// True if the element is probably visible
pub fn is_probably_visible(element: ElementRef) -> bool {
    let mut current = Some(element);

    while let Some(node) = current {
        if let Some(style) = node.value().attr("style") {
            let style_lower = style.to_lowercase();
            if style_lower.contains("display:none") || style_lower.contains("display: none") {
                return false;
            }
            if style_lower.contains("visibility:hidden")
                || style_lower.contains("visibility: hidden")
            {
                return false;
            }
        }

        if node.value().attr("hidden").is_some() {
            return false;
        }

        if let Some(aria_hidden) = node.value().attr("aria-hidden") {
            if aria_hidden == "true" {
                let is_fallback_image = node
                    .value()
                    .attr("class")
                    .map(|class| class.contains("fallback-image"))
                    .unwrap_or(false);

                if !is_fallback_image {
                    return false;
                }
            }
        }

        current = node.parent().and_then(ElementRef::wrap);
    }

    true
}

/// Get the ancestors of a node up to a maximum depth.
///
/// # Arguments
/// * `element` - The element to get ancestors for
/// * `max_depth` - Maximum depth (0 = unlimited)
///
/// # Returns
/// Vector of ancestor elements (direct parent first)
pub fn get_node_ancestors<'a>(
    element: ElementRef<'a>,
    max_depth: Option<usize>,
) -> Vec<ElementRef<'a>> {
    let max = max_depth.unwrap_or(0);
    let mut ancestors = Vec::new();
    let mut current = element;
    let mut i = 0;

    while let Some(parent) = current.parent() {
        if let Some(parent_elem) = ElementRef::wrap(parent) {
            ancestors.push(parent_elem);

            if max > 0 && {
                i += 1;
                i
            } >= max
            {
                break;
            }

            current = parent_elem;
        } else {
            break;
        }
    }

    ancestors
}

/// Check if element has any child block elements.
///
/// # Arguments
/// * `element` - The element to check
///
/// # Returns
/// True if the element has at least one block-level child
pub fn has_child_block_element(element: ElementRef) -> bool {
    element
        .children()
        .filter_map(|child| ElementRef::wrap(child))
        .any(|child| !is_phrasing_content(child))
}

/// Extract text direction from document
///
/// Checks for dir attribute on <html> element.
/// Returns "ltr", "rtl", "auto", or None.
///
/// # Arguments
/// * `document` - The HTML document
///
/// # Returns
/// The text direction if found
pub fn get_article_direction(document: &Html) -> Option<String> {
    if let Some(html_elem) = document.root_element().first_child() {
        if let Some(node_ref) = ElementRef::wrap(html_elem) {
            if node_ref.value().name() == "html" {
                if let Some(dir) = node_ref.value().attr("dir") {
                    let dir = dir.trim().to_lowercase();
                    if dir == "ltr" || dir == "rtl" || dir == "auto" {
                        return Some(dir);
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_inner_text() {
        let html = Html::parse_fragment("<p>Hello   world</p>");
        let selector = Selector::parse("p").unwrap();
        let elem = html.select(&selector).next().unwrap();

        assert_eq!(get_inner_text(elem, false), "Hello   world");
        assert_eq!(get_inner_text(elem, true), "Hello world");
    }

    #[test]
    fn test_is_phrasing_content() {
        let html = Html::parse_fragment("<span>inline</span><div>block</div>");
        let span_sel = Selector::parse("span").unwrap();
        let div_sel = Selector::parse("div").unwrap();

        let span = html.select(&span_sel).next().unwrap();
        let div = html.select(&div_sel).next().unwrap();

        assert!(is_phrasing_content(span));
        assert!(!is_phrasing_content(div));
    }

    #[test]
    fn test_is_probably_visible() {
        let html = Html::parse_fragment(
            r#"
            <div id="visible">Visible</div>
            <div style="display:none">Hidden</div>
            <div hidden>Hidden</div>
        "#,
        );

        let visible_sel = Selector::parse("#visible").unwrap();
        let visible = html.select(&visible_sel).next().unwrap();
        assert!(is_probably_visible(visible));
    }
}
