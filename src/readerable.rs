//! Quick readability check without full parsing.
//!
//! This module provides the [`is_probably_readerable`] function, which performs
//! a fast pre-flight check to determine if a document is likely to have extractable
//! article content without doing a full parse.
//!
//! ## Use Case
//!
//! Use this function to quickly filter out pages that are unlikely to contain article
//! content, saving the cost of a full parse:
//!
//! ```rust
//! use readabilityrs::{is_probably_readerable, Readability};
//!
//! let html = "<html>...</html>";
//!
//! // Quick check first
//! if is_probably_readerable(html, None) {
//!     // Do full parse
//!     let readability = Readability::new(html, None, None).unwrap();
//!     if let Some(article) = readability.parse() {
//!         println!("Article extracted!");
//!     }
//! } else {
//!     println!("Not an article page, skipping parse");
//! }
//! ```
//!
//! ## Performance
//!
//! This check is significantly faster than a full parse because it only looks
//! for basic content signals without doing deep analysis or scoring.

use crate::constants::REGEXPS;
use crate::dom_utils;
use scraper::{ElementRef, Html, Selector};

/// Options for the readability pre-flight check.
///
/// Controls the thresholds used by [`is_probably_readerable`] to determine
/// if a document is likely to be parseable.
///
/// ## Example
///
/// ```rust
/// use readabilityrs::{is_probably_readerable, ReaderableOptions};
///
/// let html = "<html>...</html>";
///
/// let options = ReaderableOptions {
///     min_content_length: 200,
///     min_score: 30.0,
/// };
///
/// let is_readerable = is_probably_readerable(html, Some(options));
/// ```
#[derive(Debug, Clone)]
pub struct ReaderableOptions {
    /// Minimum content length to consider a paragraph.
    ///
    /// Paragraphs shorter than this are ignored when calculating the
    /// readability score.
    ///
    /// Default: `140`
    pub min_content_length: usize,

    /// Minimum score threshold to consider a page readerable.
    ///
    /// The score is calculated based on the length and number of content
    /// paragraphs found in the document.
    ///
    /// Default: `20.0`
    pub min_score: f64,
}

impl Default for ReaderableOptions {
    fn default() -> Self {
        Self {
            min_content_length: 140,
            min_score: 20.0,
        }
    }
}

/// Quick check to determine if a document is likely to be readerable.
///
/// This function performs a fast analysis to predict whether full article extraction
/// is likely to succeed, without doing the expensive full parse. It looks for basic
/// content signals like paragraphs with sufficient text.
///
/// ## Arguments
///
/// * `html` - The HTML document to check
/// * `options` - Optional custom thresholds (uses defaults if `None`)
///
/// ## Returns
///
/// `true` if the document likely contains extractable article content, `false` otherwise.
///
/// ## Example
///
/// ```rust
/// use readabilityrs::is_probably_readerable;
///
/// let article_html = r#"
///     <html><body>
///         <article>
///             <p>This is a substantial paragraph with enough content to indicate that this page
///             likely contains article text that can be extracted by the readability algorithm.
///             The paragraph needs to be long enough to pass the minimum content length threshold.</p>
///         </article>
///     </body></html>
/// "#;
///
/// assert!(is_probably_readerable(article_html, None));
///
/// let non_article_html = "<html><body><p>Short</p></body></html>";
/// assert!(!is_probably_readerable(non_article_html, None));
/// ```
///
/// ## With Custom Options
///
/// ```rust
/// use readabilityrs::{is_probably_readerable, ReaderableOptions};
///
/// let html = "<html>...</html>";
/// let options = ReaderableOptions {
///     min_content_length: 200,
///     min_score: 30.0,
/// };
///
/// if is_probably_readerable(html, Some(options)) {
///     println!("Likely readerable with stricter thresholds");
/// }
/// ```
///
/// ## Algorithm
///
/// This mirrors Mozilla's `isProbablyReaderable`. The candidate set is every
/// `<p>`, `<pre>` and `<article>` element, plus any `<div>` with a direct `<br>`
/// child, which is how prose laid out with line breaks instead of paragraphs
/// gets counted.
///
/// A candidate is skipped when it is hidden, when its class or id marks it as an
/// unlikely candidate (comment thread, sidebar, footer) without also matching the
/// "maybe a candidate" pattern, or when it is a `<p>` directly inside an `<li>`.
/// Surviving candidates shorter than `min_content_length` contribute nothing;
/// the rest add `sqrt(len - min_content_length)`. The function returns `true`
/// as soon as the running score passes `min_score`.
///
/// ## Performance
///
/// This is far cheaper than a full parse, which makes it useful for batch processing
/// large numbers of URLs, pre-filtering in crawlers or scrapers, and quick content
/// classification.
pub fn is_probably_readerable(html: &str, options: Option<ReaderableOptions>) -> bool {
    let options = options.unwrap_or_default();
    let document = Html::parse_document(html);

    let Ok(candidate_selector) = Selector::parse("p, pre, article, div") else {
        return false;
    };

    let mut score = 0.0;

    for node in document.select(&candidate_selector) {
        if !is_scored_candidate(node) {
            continue;
        }

        let text_len = node.text().collect::<String>().trim().len();
        if text_len < options.min_content_length {
            continue;
        }

        score += ((text_len - options.min_content_length) as f64).sqrt();

        if score > options.min_score {
            return true;
        }
    }

    false
}

/// Whether a node from the candidate selector should contribute to the score.
///
/// A `<div>` only qualifies through a direct `<br>` child, which is how Mozilla
/// picks up prose laid out with line breaks instead of paragraphs. The selector
/// matches every div so the tree is walked once, and the rest are rejected here.
fn is_scored_candidate(node: ElementRef) -> bool {
    let tag = node.value().name();

    if tag.eq_ignore_ascii_case("div") && !has_direct_br_child(node) {
        return false;
    }

    // Mozilla excludes list-item paragraphs: they are almost always navigation
    // or link lists rather than prose.
    if tag.eq_ignore_ascii_case("p") && parent_is_list_item(node) {
        return false;
    }

    if !dom_utils::is_probably_visible(node) {
        return false;
    }

    let class = node.value().attr("class").unwrap_or("");
    let id = node.value().attr("id").unwrap_or("");
    let match_string = format!("{class} {id}");

    if REGEXPS.unlikely_candidates.is_match(&match_string)
        && !REGEXPS.ok_maybe_its_a_candidate.is_match(&match_string)
    {
        return false;
    }

    true
}

fn has_direct_br_child(node: ElementRef) -> bool {
    node.children()
        .filter_map(ElementRef::wrap)
        .any(|child| child.value().name().eq_ignore_ascii_case("br"))
}

fn parent_is_list_item(node: ElementRef) -> bool {
    node.parent()
        .and_then(ElementRef::wrap)
        .is_some_and(|parent| parent.value().name().eq_ignore_ascii_case("li"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_probably_readerable() {
        let html = r#"
            <html>
                <body>
                    <article>
                        <p>This is a long enough paragraph that should make the content readerable.
                        It has sufficient content to pass the minimum threshold check. Adding more text here to ensure
                        we definitely exceed the 140 character minimum requirement for each paragraph element.</p>
                        <p>Another paragraph with more content to increase the score. This paragraph also needs to be
                        long enough to contribute to the overall readability score calculation and help us pass the test.</p>
                    </article>
                </body>
            </html>
        "#;

        assert!(is_probably_readerable(html, None));
    }

    #[test]
    fn test_not_readerable() {
        let html = r#"
            <html>
                <body>
                    <p>Short</p>
                </body>
            </html>
        "#;

        assert!(!is_probably_readerable(html, None));
    }

    /// Three paragraphs of real prose, long enough that the page is readerable
    /// unless a filter excludes them. `class_attr` lands on the paragraphs
    /// themselves, because the unlikely-candidate check reads each scored node's
    /// own class and id and never walks ancestors.
    fn prose_paragraphs(class_attr: &str) -> String {
        (1..=3)
            .map(|i| {
                format!(
                    "<p class=\"{class_attr}\">Paragraph {i} carries well over the hundred and \
                     forty character minimum that a candidate needs before it contributes to the \
                     score at all, so three of them comfortably clear the threshold.</p>"
                )
            })
            .collect()
    }

    #[test]
    fn test_unlikely_candidate_is_excluded() {
        let html = format!("<html><body>{}</body></html>", prose_paragraphs("comment"));

        assert!(!is_probably_readerable(&html, None));
    }

    #[test]
    fn test_ok_maybe_candidate_overrides_unlikely() {
        let html = format!(
            "<html><body>{}</body></html>",
            prose_paragraphs("comment main-content")
        );

        assert!(is_probably_readerable(&html, None));
    }

    /// The check is per node, matching Mozilla: a container marked unlikely does
    /// not disqualify unmarked paragraphs inside it. Worth pinning, since the
    /// opposite is the intuitive assumption.
    #[test]
    fn test_unlikely_wrapper_does_not_exclude_its_children() {
        let html = format!(
            "<html><body><div class=\"comment\">{}</div></body></html>",
            prose_paragraphs("")
        );

        assert!(is_probably_readerable(&html, None));
    }

    #[test]
    fn test_hidden_content_is_excluded() {
        let html = format!(
            "<html><body><div style=\"display:none\">{}</div></body></html>",
            prose_paragraphs("")
        );

        assert!(!is_probably_readerable(&html, None));
    }

    /// Prose separated by `<br>` rather than wrapped in paragraphs is only
    /// reachable through the `div > br` rule, so this fails without it.
    #[test]
    fn test_div_with_br_child_is_counted() {
        let line = "A line of prose long enough on its own to matter, padded out past the \
                    hundred and forty character minimum that a candidate needs before it \
                    contributes anything to the score.";
        // The div is a single candidate, so its whole text yields one sqrt term;
        // it needs to clear min_score of 20 on its own, i.e. over 540 characters.
        let body = [line; 6].join("<br>");
        let html = format!("<html><body><div>{body}</div></body></html>");

        assert!(is_probably_readerable(&html, None));
    }

    /// A bare `<div>` of prose with neither `<br>` nor `<p>` children is not a
    /// candidate at all. Without the `div > br` rule every div would score, so
    /// this is what keeps the rule honest.
    #[test]
    fn test_bare_div_of_text_is_not_a_candidate() {
        let filler = "Plenty of prose sitting directly in a div with no line breaks and no \
                      paragraph children whatsoever. "
            .repeat(10);
        let html = format!("<html><body><div>{filler}</div></body></html>");

        assert!(!is_probably_readerable(&html, None));
    }

    #[test]
    fn test_div_without_br_is_not_counted_itself() {
        // The div is skipped, but its paragraphs are scored in their own right.
        let html = format!(
            "<html><body><div>{}</div></body></html>",
            prose_paragraphs("")
        );

        assert!(is_probably_readerable(&html, None));
    }

    #[test]
    fn test_paragraphs_inside_list_items_are_excluded() {
        // Each entry is long enough that a single one would clear min_score, so
        // the page is only unreadable because the list-item rule drops them.
        let items: String = (1..=3)
            .map(|i| {
                let filler = "This entry is deliberately verbose so that its length alone would \
                              carry the page over the score threshold. "
                    .repeat(6);
                format!("<li><p>Entry {i}. {filler}</p></li>")
            })
            .collect();
        let html = format!("<html><body><ul>{items}</ul></body></html>");

        assert!(!is_probably_readerable(&html, None));
    }
}
