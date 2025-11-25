//! Content scoring algorithms for determining article quality.

use crate::constants::{ParseFlags, REGEXPS};
use crate::dom_utils;
use scraper::ElementRef;

/// Get an element's class/ID weight using regular expressions.
/// Uses positive/negative patterns to determine if an element looks good or bad.
///
/// # Arguments
/// * `element` - The element to get the weight for
/// * `flags` - Current parsing flags (to check if FLAG_WEIGHT_CLASSES is active)
///
/// # Returns
/// Weight as an integer (-25, 0, or +25 based on matches)
pub fn get_class_weight(element: ElementRef, flags: ParseFlags) -> i32 {
    if !flags.contains(ParseFlags::WEIGHT_CLASSES) {
        return 0;
    }

    let mut weight = 0;

    // Check class names
    if let Some(class) = element.value().attr("class") {
        if !class.is_empty() {
            if REGEXPS.negative.is_match(class) {
                weight -= 25;
            } else if REGEXPS.positive.is_match(class) {
                weight += 25;
            }
        }
    }

    // Check ID
    if let Some(id) = element.value().attr("id") {
        if !id.is_empty() {
            if REGEXPS.negative.is_match(id) {
                weight -= 25;
            } else if REGEXPS.positive.is_match(id) {
                weight += 25;
            }
        }
    }

    weight
}

/// Initialize content score for a node.
///
/// This sets the base score based on the element tag type and adds class weight.
///
/// **DIV to P Transformation**: DIVs without block-level children are treated
/// like P tags (given the same high score). This matches Mozilla's approach
/// and works with modern websites that use DIVs instead of P tags.
///
/// # Arguments
/// * `element` - The element to initialize scoring for
/// * `flags` - Current parsing flags
///
/// # Returns
/// Initial content score as a float
pub fn initialize_node_score(element: ElementRef, flags: ParseFlags) -> f64 {
    let mut score = 0.0;

    let tag_name = element.value().name().to_uppercase();

    match tag_name.as_str() {
        // P tags get the highest base score (they're what we're looking for)
        "P" => score += 5.0,

        // SECTION and ARTICLE are good semantic containers
        "SECTION" | "ARTICLE" => score += 8.0,

        // DIV gets special handling: if it has no block children, treat like P
        "DIV" => {
            if !dom_utils::has_child_block_element(element) {
                // DIV acting as paragraph - give it P tag score
                score += 5.0;
            } else {
                // DIV as container - lower score
                score += 2.0;
            }
        }

        // These tags are good content containers
        "PRE" | "TD" | "BLOCKQUOTE" => score += 3.0,

        // These tags are typically not article content
        "ADDRESS" | "OL" | "UL" | "DL" | "DD" | "DT" | "LI" | "FORM" => score -= 3.0,

        // Headers are typically not body content
        "H1" | "H2" | "H3" | "H4" | "H5" | "H6" | "TH" => score -= 5.0,

        _ => {}
    }

    score += get_class_weight(element, flags) as f64;
    score
}

/// Calculate content score for a paragraph or other scoreable element.
///
/// The score is based on:
/// 1. Base score of 1
/// 2. Number of commas (content signal)
/// 3. Character length (up to 3 points for 300+ chars)
/// 4. Link density penalty
///
/// # Arguments
/// * `element` - The element to score
/// * `link_density_modifier` - Modifier for link density calculation
///
/// # Returns
/// Content score as a float
pub fn calculate_content_score(element: ElementRef, link_density_modifier: f64) -> f64 {
    let inner_text = dom_utils::get_inner_text(element, false);
    if inner_text.len() < 25 {
        return 0.0;
    }

    let mut score = 1.0;
    let comma_count = REGEXPS.commas.find_iter(&inner_text).count();
    score += comma_count as f64;

    let length_bonus = (inner_text.len() as f64 / 100.0).min(3.0);
    score += length_bonus;

    let link_density = dom_utils::get_link_density(element);
    score *= 1.0 - link_density + link_density_modifier;

    score
}

/// Check if an element is a valid byline.
///
/// A valid byline should:
/// - Have rel="author" or itemprop containing "author", OR match byline regex
/// - Have text content
/// - Be less than 100 characters
///
/// # Arguments
/// * `element` - The element to check
/// * `match_string` - String to match against byline regex (usually class + id)
///
/// # Returns
/// True if this is a valid byline
pub fn is_valid_byline(element: ElementRef, match_string: &str) -> bool {
    let rel = element.value().attr("rel").unwrap_or("");
    let itemprop = element.value().attr("itemprop").unwrap_or("");
    let byline_length = dom_utils::get_inner_text(element, false).len();

    (rel == "author" || (itemprop.contains("author")) || REGEXPS.byline.is_match(match_string))
        && byline_length > 0
        && byline_length < 100
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Html;
    use scraper::Selector;

    #[test]
    fn test_get_class_weight() {
        let html = Html::parse_fragment(
            r#"
            <div class="article content">Positive</div>
            <div class="sidebar ad">Negative</div>
            <div id="main-content">Positive ID</div>
        "#,
        );

        let flags = ParseFlags::WEIGHT_CLASSES;

        let positive_sel = Selector::parse(".article").unwrap();
        let negative_sel = Selector::parse(".sidebar").unwrap();
        let positive_id_sel = Selector::parse("#main-content").unwrap();

        let positive = html.select(&positive_sel).next().unwrap();
        let negative = html.select(&negative_sel).next().unwrap();
        let positive_id = html.select(&positive_id_sel).next().unwrap();

        assert!(get_class_weight(positive, flags) > 0);
        assert!(get_class_weight(negative, flags) < 0);
        assert!(get_class_weight(positive_id, flags) > 0);
    }

    #[test]
    fn test_initialize_node_score() {
        let p_html = Html::parse_fragment("<p>Content</p>");
        let p_sel = Selector::parse("p").unwrap();
        let p = p_html.select(&p_sel).next().unwrap();
        assert_eq!(initialize_node_score(p, ParseFlags::WEIGHT_CLASSES), 5.0);

        let h1_html = Html::parse_fragment("<h1>Title</h1>");
        let h1_sel = Selector::parse("h1").unwrap();
        let h1 = h1_html.select(&h1_sel).next().unwrap();
        assert_eq!(initialize_node_score(h1, ParseFlags::WEIGHT_CLASSES), -5.0);

        let div_p_html = Html::parse_fragment("<div>Text content only</div>");
        let div_sel = Selector::parse("div").unwrap();
        let div_as_p = div_p_html.select(&div_sel).next().unwrap();
        assert_eq!(
            initialize_node_score(div_as_p, ParseFlags::WEIGHT_CLASSES),
            5.0
        );

        let div_container_html = Html::parse_fragment("<div><p>Nested paragraph</p></div>");
        let div_container = div_container_html.select(&div_sel).next().unwrap();
        assert_eq!(
            initialize_node_score(div_container, ParseFlags::WEIGHT_CLASSES),
            2.0
        );

        let article_html = Html::parse_fragment("<article>Content</article>");
        let article_sel = Selector::parse("article").unwrap();
        let article = article_html.select(&article_sel).next().unwrap();
        assert_eq!(
            initialize_node_score(article, ParseFlags::WEIGHT_CLASSES),
            8.0
        );
    }

    #[test]
    fn test_calculate_content_score() {
        let html = Html::parse_fragment(
            "<p>This is a long paragraph with enough content to be scored. It has some commas, which increase the score.</p>"
        );
        let selector = Selector::parse("p").unwrap();
        let elem = html.select(&selector).next().unwrap();

        let score = calculate_content_score(elem, 0.0);
        assert!(score > 1.0);
    }

    #[test]
    fn test_short_content_score() {
        let html = Html::parse_fragment("<p>Short</p>");
        let selector = Selector::parse("p").unwrap();
        let elem = html.select(&selector).next().unwrap();

        let score = calculate_content_score(elem, 0.0);
        assert_eq!(score, 0.0);
    }
}
