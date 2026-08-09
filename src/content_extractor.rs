//! Core content extraction algorithm (_grabArticle implementation).

use crate::constants::{ParseFlags, DEFAULT_TAGS_TO_SCORE, MAX_DOM_DEPTH, REGEXPS};
use crate::error::{ReadabilityError, Result};
use crate::options::ReadabilityOptions;
use crate::{dom_utils, scoring};
use ego_tree::NodeId;
use scraper::{ElementRef, Html, Selector};
use std::collections::HashMap;
use v_htmlescape::escape;

/// Represents an extraction attempt
#[derive(Debug, Clone)]
struct Attempt {
    content: String,
    text_length: usize,
}

/// Main content extraction algorithm with retry logic
///
/// Implements Mozilla's Readability algorithm with adaptive flag removal.
/// If extraction fails with strict settings, retries with progressively
/// looser criteria until content is found or all options are exhausted.
///
/// Returns [`ReadabilityError::MaxElementsExceeded`] when the document holds more
/// elements than [`ReadabilityOptions::max_elems_to_parse`] allows. A limit of `0`
/// means unlimited.
pub fn grab_article(document: &Html, options: &ReadabilityOptions) -> Result<Option<String>> {
    if options.max_elems_to_parse > 0 {
        // Counted before scoring so an oversized document is rejected before any
        // of the expensive passes run. O(n) over an already-parsed tree.
        let element_count = document
            .tree
            .nodes()
            .filter(|node| node.value().is_element())
            .count();

        if element_count > options.max_elems_to_parse {
            return Err(ReadabilityError::MaxElementsExceeded(element_count));
        }
    }

    let mut attempts = Vec::new();
    let mut flags =
        ParseFlags::STRIP_UNLIKELYS | ParseFlags::WEIGHT_CLASSES | ParseFlags::CLEAN_CONDITIONALLY;

    // Try extraction with different flag combinations
    // Order: All flags -> Remove STRIP_UNLIKELYS -> Remove WEIGHT_CLASSES -> Remove CLEAN_CONDITIONALLY
    for attempt_num in 0..4 {
        let attempt_result = try_extract_with_flags(document, options, flags)?;

        if let Some(content) = attempt_result {
            let text_length = extract_text_length(&content);

            // Check if we have enough content
            if text_length >= options.char_threshold {
                return Ok(Some(content));
            }

            // Save this attempt for potential fallback
            attempts.push(Attempt {
                content,
                text_length,
            });
        }

        // Modify flags for next attempt
        match attempt_num {
            0 => flags.remove(ParseFlags::STRIP_UNLIKELYS),
            1 => flags.remove(ParseFlags::WEIGHT_CLASSES),
            2 => flags.remove(ParseFlags::CLEAN_CONDITIONALLY),
            _ => break, // All flags removed, no more attempts
        }
    }

    // No successful extraction with threshold, return longest attempt
    if !attempts.is_empty() {
        attempts.sort_by_key(|a| std::cmp::Reverse(a.text_length));
        if attempts[0].text_length > 0 {
            return Ok(Some(attempts[0].content.clone()));
        }
    }

    Ok(None)
}

/// Try to extract article content with specific flags
fn try_extract_with_flags(
    document: &Html,
    options: &ReadabilityOptions,
    flags: ParseFlags,
) -> Result<Option<String>> {
    let candidates = find_candidates(document, options, flags)?;
    if candidates.is_empty() {
        return Ok(None);
    }

    let mut scored_candidates = score_candidates(document, candidates, options, flags);
    apply_link_density_penalty(document, &mut scored_candidates);

    if let Some(best) = find_best_candidate(document, &scored_candidates, options) {
        let content = extract_article_content(document, best, &scored_candidates, options)?;
        return Ok(Some(content));
    }

    Ok(None)
}

/// Extract plain text length from HTML content
fn extract_text_length(html: &str) -> usize {
    let doc = Html::parse_fragment(html);
    let text: String = doc.root_element().text().collect();
    text.trim().len()
}

/// Find all potential content candidates in the document
fn find_candidates<'a>(
    document: &'a Html,
    _options: &ReadabilityOptions,
    flags: ParseFlags,
) -> Result<Vec<ElementRef<'a>>> {
    let mut candidates = Vec::new();

    let p_selector = Selector::parse("p").unwrap();
    for p in document.select(&p_selector) {
        if !dom_utils::is_probably_visible(p) {
            continue;
        }

        if flags.contains(ParseFlags::STRIP_UNLIKELYS) {
            let class = p.value().attr("class").unwrap_or("");
            let id = p.value().attr("id").unwrap_or("");
            let match_string = format!("{class} {id}");

            if REGEXPS.unlikely_candidates.is_match(&match_string)
                && !REGEXPS.ok_maybe_its_a_candidate.is_match(&match_string)
            {
                continue;
            }
        }

        let text = dom_utils::get_inner_text(p, false);
        if text.len() < 25 {
            continue;
        }

        candidates.push(p);
    }

    for tag in DEFAULT_TAGS_TO_SCORE.iter() {
        let selector = Selector::parse(tag).unwrap();
        for elem in document.select(&selector) {
            if !dom_utils::is_probably_visible(elem) {
                continue;
            }

            if flags.contains(ParseFlags::STRIP_UNLIKELYS) {
                let class = elem.value().attr("class").unwrap_or("");
                let id = elem.value().attr("id").unwrap_or("");
                let match_string = format!("{class} {id}");

                if REGEXPS.unlikely_candidates.is_match(&match_string)
                    && !REGEXPS.ok_maybe_its_a_candidate.is_match(&match_string)
                {
                    continue;
                }
            }

            let text = dom_utils::get_inner_text(elem, false);
            if text.len() >= 25 {
                candidates.push(elem);
            }
        }
    }

    Ok(candidates)
}

/// Score all candidates and their ancestors
fn score_candidates<'a>(
    _document: &'a Html,
    candidates: Vec<ElementRef<'a>>,
    options: &ReadabilityOptions,
    flags: ParseFlags,
) -> HashMap<NodeId, f64> {
    let mut scores: HashMap<NodeId, f64> = HashMap::new();

    for candidate in candidates {
        let content_score =
            scoring::calculate_content_score(candidate, options.link_density_modifier);

        if content_score == 0.0 {
            continue;
        }

        // Ensure the candidate itself is tracked; in Mozilla's implementation the
        // element owns the score before propagating to ancestors.
        let candidate_id = candidate.id();
        let candidate_entry = scores
            .entry(candidate_id)
            .or_insert_with(|| scoring::initialize_node_score(candidate, flags));
        *candidate_entry += content_score;

        let ancestors = dom_utils::get_node_ancestors(candidate, Some(5));

        // Propagate score to ancestors
        // Parent gets 1x, grandparent gets 0.5x, great-grandparent gets 0.33x, etc.
        for (level, ancestor) in ancestors.iter().enumerate() {
            let ancestor_id = ancestor.id();
            scores
                .entry(ancestor_id)
                .or_insert_with(|| scoring::initialize_node_score(*ancestor, flags));

            let score_divider = if level == 0 {
                1.0
            } else if level == 1 {
                2.0
            } else {
                (level * 3) as f64
            };

            let propagated_score = content_score / score_divider;
            *scores.get_mut(&ancestor_id).unwrap() += propagated_score;
        }
    }

    scores
}

/// Adjust candidate scores based on their actual link density.
fn apply_link_density_penalty(document: &Html, scores: &mut HashMap<NodeId, f64>) {
    for (element_id, score) in scores.iter_mut() {
        if let Some(element) = find_element_by_id(document, *element_id) {
            let penalty = (1.0 - dom_utils::get_link_density(element)).max(0.0);
            *score *= penalty;
        }
    }
}

/// Find the best candidate based on scores, promoting parents when needed.
fn find_best_candidate(
    document: &Html,
    scores: &HashMap<NodeId, f64>,
    options: &ReadabilityOptions,
) -> Option<NodeId> {
    let mut sorted_scores: Vec<_> = scores.iter().collect();
    sorted_scores.sort_by(|a, b| b.1.total_cmp(a.1));

    let top_candidates: Vec<(NodeId, f64)> = sorted_scores
        .iter()
        .take(options.nb_top_candidates)
        .map(|(id, score)| (**id, **score))
        .collect();

    if top_candidates.is_empty() {
        return None;
    }

    let mut best_id = top_candidates[0].0;
    let mut best_score = top_candidates[0].1;

    for (candidate_id, candidate_score) in &top_candidates {
        if let Some(elem) = find_element_by_id(document, *candidate_id) {
            if is_viable_best_candidate(elem, *candidate_score) {
                best_id = *candidate_id;
                best_score = *candidate_score;
                break;
            }
        }
    }

    if let Some(promoted) =
        promote_shared_top_candidate_parent(document, best_id, best_score, &top_candidates)
    {
        best_id = promoted;
        best_score = scores.get(&best_id).copied().unwrap_or(best_score);
    }

    if let Some(promoted) = promote_high_scoring_parents(document, best_id, best_score, scores) {
        best_id = promoted;
        best_score = scores.get(&best_id).copied().unwrap_or(best_score);
    }

    // If the best candidate lives inside a single-child parent chain, walk up so we can pull siblings later.
    if let Some(promoted) = promote_single_child_parents(document, best_id) {
        best_id = promoted;
    }

    if let Some(promoted) = promote_dense_wrapper_child(document, best_id, scores, &sorted_scores) {
        best_id = promoted;
        best_score = scores.get(&best_id).copied().unwrap_or(best_score);
    }

    if let Some(promoted) =
        promote_semantic_descendant(document, best_id, best_score, &sorted_scores)
    {
        best_id = promoted;
    }

    Some(best_id)
}

/// Promote parent nodes when the current candidate is the only child, mirroring Mozilla's logic.
fn promote_single_child_parents(document: &Html, best_id: NodeId) -> Option<NodeId> {
    let mut promoted_id = None;
    let mut current = find_element_by_id(document, best_id)?;

    while let Some(parent_node) = current.parent() {
        let Some(parent) = ElementRef::wrap(parent_node) else {
            break;
        };

        if parent.value().name().eq_ignore_ascii_case("body") {
            break;
        }

        if count_element_children(parent) == 1 {
            let parent_id = parent.id();
            promoted_id = Some(parent_id);
            current = parent;
            continue;
        }

        break;
    }

    promoted_id
}

/// Promote a higher scoring parent when it looks more article-like than the current candidate.
fn promote_shared_top_candidate_parent(
    document: &Html,
    best_id: NodeId,
    best_score: f64,
    top_candidates: &[(NodeId, f64)],
) -> Option<NodeId> {
    const MINIMUM_TOP_CANDIDATES: usize = 3;
    if best_score <= 0.0 {
        return None;
    }

    let mut ancestor_lists: Vec<Vec<NodeId>> = Vec::new();

    for (candidate_id, candidate_score) in top_candidates.iter().skip(1) {
        if *candidate_score < best_score * 0.75 {
            continue;
        }

        let Some(candidate_elem) = find_element_by_id(document, *candidate_id) else {
            continue;
        };
        let ancestors = dom_utils::get_node_ancestors(candidate_elem, None);
        if ancestors.is_empty() {
            continue;
        }

        let ancestor_ids = ancestors
            .into_iter()
            .map(|ancestor| ancestor.id())
            .collect::<Vec<_>>();
        ancestor_lists.push(ancestor_ids);
    }

    if ancestor_lists.len() < MINIMUM_TOP_CANDIDATES {
        return None;
    }

    let mut parent_opt = find_element_by_id(document, best_id)
        .and_then(|node| node.parent())
        .and_then(ElementRef::wrap)?;

    while !parent_opt.value().name().eq_ignore_ascii_case("body") {
        let parent_id = parent_opt.id();
        let containing_lists = ancestor_lists
            .iter()
            .filter(|ancestors| ancestors.iter().any(|id| id == &parent_id))
            .count();

        if containing_lists >= MINIMUM_TOP_CANDIDATES {
            return Some(parent_id);
        }

        parent_opt = match parent_opt.parent().and_then(ElementRef::wrap) {
            Some(parent) => parent,
            None => break,
        };
    }

    None
}

fn promote_high_scoring_parents(
    document: &Html,
    best_id: NodeId,
    best_score: f64,
    scores: &HashMap<NodeId, f64>,
) -> Option<NodeId> {
    let mut current = find_element_by_id(document, best_id)?;
    let mut last_score = best_score;
    let score_threshold = best_score / 3.0;

    while let Some(parent_node) = current.parent() {
        let Some(parent) = ElementRef::wrap(parent_node) else {
            break;
        };

        if parent.value().name().eq_ignore_ascii_case("body") {
            break;
        }

        let role_is_main = parent
            .value()
            .attr("role")
            .map(|role| role.eq_ignore_ascii_case("main"))
            .unwrap_or(false);
        let tag_name = parent.value().name().to_uppercase();
        let is_semantic_container = matches!(tag_name.as_str(), "ARTICLE" | "SECTION" | "MAIN");
        let looks_like_main = role_is_main || is_semantic_container;

        if !looks_like_main {
            current = parent;
            continue;
        }

        let parent_id = parent.id();
        let Some(parent_score) = scores.get(&parent_id) else {
            current = parent;
            continue;
        };

        if *parent_score < score_threshold {
            break;
        }

        let parent_link_density = dom_utils::get_link_density(parent);
        if parent_link_density > 0.33 {
            current = parent;
            continue;
        }

        if *parent_score > last_score {
            return Some(parent_id);
        }

        last_score = *parent_score;
        current = parent;
    }

    None
}

/// If our best candidate is a wrapper with high link density, look for a better child candidate.
fn promote_dense_wrapper_child(
    document: &Html,
    best_id: NodeId,
    scores: &HashMap<NodeId, f64>,
    sorted_scores: &[(&NodeId, &f64)],
) -> Option<NodeId> {
    let best_elem = find_element_by_id(document, best_id)?;

    let tag = best_elem.value().name().to_uppercase();
    if matches!(tag.as_str(), "ARTICLE" | "SECTION" | "MAIN") {
        return None;
    }

    let parent_score = scores.get(&best_id).copied().unwrap_or(0.0);
    let best_link_density = dom_utils::get_link_density(best_elem);

    let mut fallback = None;

    for (candidate_id, candidate_score) in sorted_scores.iter().take(20) {
        if **candidate_id == best_id {
            continue;
        }
        let Some(candidate_elem) = find_element_by_id(document, **candidate_id) else {
            continue;
        };

        if !is_descendant_of(candidate_elem, best_id) {
            continue;
        }

        let text_len = dom_utils::get_inner_text(candidate_elem, false).len();
        if text_len < 160 {
            continue;
        }

        let link_density = dom_utils::get_link_density(candidate_elem);
        if link_density >= 0.35 {
            continue;
        }

        if link_density >= best_link_density - 0.15 {
            continue;
        }

        let candidate_weight =
            scoring::get_class_weight(candidate_elem, ParseFlags::WEIGHT_CLASSES);
        if candidate_weight < 0 {
            let match_string = format!(
                "{} {}",
                candidate_elem.value().attr("class").unwrap_or(""),
                candidate_elem.value().attr("id").unwrap_or("")
            );
            if !REGEXPS.positive.is_match(&match_string) {
                continue;
            }
        }

        let paragraph_selector = Selector::parse("p").unwrap();
        let paragraph_count = candidate_elem.select(&paragraph_selector).count();
        if paragraph_count == 0 && text_len < 300 {
            continue;
        }

        if dom_utils::get_link_density(candidate_elem) >= best_link_density {
            continue;
        }

        let score = **candidate_score;
        if fallback
            .as_ref()
            .map(|(_, existing_score)| score > *existing_score)
            .unwrap_or(true)
        {
            fallback = Some((**candidate_id, score));
        }
    }

    if let Some((candidate_id, score)) = fallback {
        if parent_score == 0.0 || score >= parent_score * 0.45 {
            return Some(candidate_id);
        }
    }

    None
}

fn promote_semantic_descendant(
    document: &Html,
    best_id: NodeId,
    best_score: f64,
    sorted_scores: &[(&NodeId, &f64)],
) -> Option<NodeId> {
    if best_score <= 0.0 {
        return None;
    }

    let best_elem = find_element_by_id(document, best_id)?;

    let class_id = format!(
        "{} {}",
        best_elem.value().attr("class").unwrap_or(""),
        best_elem.value().attr("id").unwrap_or("")
    )
    .to_lowercase();

    const LAYOUT_KEYWORDS: [&str; 7] = [
        "content",
        "container",
        "main",
        "column",
        "outer",
        "inner",
        "wrapper",
    ];

    if !LAYOUT_KEYWORDS
        .iter()
        .any(|keyword| class_id.contains(keyword))
    {
        return None;
    }

    const POSITIVE_KEYWORDS: [&str; 7] =
        ["article", "post", "entry", "body", "story", "text", "blog"];

    let mut promoted_child: Option<(NodeId, f64)> = None;

    for (candidate_id, candidate_score) in sorted_scores.iter().take(40) {
        if **candidate_id == best_id {
            continue;
        }

        let Some(candidate_elem) = find_element_by_id(document, **candidate_id) else {
            continue;
        };

        if !is_descendant_of(candidate_elem, best_id) {
            continue;
        }

        let text = dom_utils::get_inner_text(candidate_elem, false);
        let text_len = text.len();
        if text_len < 200 {
            continue;
        }

        let link_density = dom_utils::get_link_density(candidate_elem);
        if link_density > 0.45 {
            continue;
        }

        let match_string = format!(
            "{} {} {}",
            candidate_elem.value().attr("class").unwrap_or(""),
            candidate_elem.value().attr("id").unwrap_or(""),
            candidate_elem.value().attr("itemprop").unwrap_or("")
        )
        .to_lowercase();

        let looks_semantic = POSITIVE_KEYWORDS
            .iter()
            .any(|keyword| match_string.contains(keyword))
            || match_string.contains("articlebody");

        if !looks_semantic {
            continue;
        }

        let score = **candidate_score;
        if score < best_score * 0.4 {
            continue;
        }

        if promoted_child
            .as_ref()
            .map(|(_, existing_score)| score > *existing_score)
            .unwrap_or(true)
        {
            promoted_child = Some((**candidate_id, score));
        }
    }

    promoted_child.map(|(id, _)| id)
}

/// Extract article content from the best candidate
///
/// This implements Mozilla's sibling aggregation strategy:
/// 1. Extract the best candidate element
/// 2. Get siblings of the best candidate's parent
/// 3. Include siblings that either:
///    - Score >= 20% of the best candidate's score, OR
///    - Are good paragraphs (low link density, decent text length)
/// 4. Aggregate all content together
fn extract_article_content(
    document: &Html,
    best_candidate_id: NodeId,
    all_scores: &HashMap<NodeId, f64>,
    options: &ReadabilityOptions,
) -> Result<String> {
    let Some(best_candidate) = find_element_by_id(document, best_candidate_id) else {
        return Ok(String::new());
    };

    let best_score = all_scores.get(&best_candidate_id).copied().unwrap_or(0.0);
    let best_candidate_class = best_candidate
        .value()
        .attr("class")
        .unwrap_or("")
        .to_string();

    let sibling_score_threshold = (best_score * 0.2).max(10.0);
    let mut article_content = Vec::new();
    let Some(parent) = best_candidate.parent() else {
        // No parent, just return the best candidate
        let html = element_to_html(best_candidate, options.sanitize_content, 0);
        let html = crate::cleaner::replace_brs(&html);
        return Ok(html);
    };

    for child_node in parent.children() {
        let Some(sibling) = ElementRef::wrap(child_node) else {
            continue;
        };

        let sibling_id = sibling.id();
        let is_best_candidate = sibling_id == best_candidate_id;

        let should_include = if is_best_candidate {
            true
        } else {
            let sibling_score = all_scores.get(&sibling_id).copied().unwrap_or(0.0);
            let class_bonus = if !best_candidate_class.is_empty() {
                sibling
                    .value()
                    .attr("class")
                    .filter(|class_name| {
                        !class_name.is_empty() && *class_name == best_candidate_class
                    })
                    .map(|_| best_score * 0.2)
                    .unwrap_or(0.0)
            } else {
                0.0
            };

            let weighted_sibling_score = sibling_score + class_bonus;
            if weighted_sibling_score >= sibling_score_threshold
                || is_good_sibling_paragraph(sibling)
            {
                true
            } else {
                should_keep_block_element(sibling, best_score)
            }
        };

        if should_include {
            let mut sibling_html = element_to_html(sibling, options.sanitize_content, 0);
            sibling_html = crate::cleaner::replace_brs(&sibling_html);

            if !sibling_html.trim().is_empty() {
                article_content.push(sibling_html);
            }
        }
    }

    Ok(article_content.join("\n"))
}

/// Check if a sibling element is a "good paragraph" worth including
///
/// A good paragraph is one that:
/// - Is a P tag (or looks like a paragraph)
/// - Has reasonable text length (> 80 chars)
/// - Has low link density (< 33%)
/// - Looks like actual content, not navigation
fn is_good_sibling_paragraph(element: ElementRef) -> bool {
    let tag_name = element.value().name();
    if tag_name != "p" {
        return false;
    }

    let text = dom_utils::get_inner_text(element, false);
    let text_length = text.len();
    if text_length == 0 {
        return false;
    }

    let class = element.value().attr("class").unwrap_or("");
    let id = element.value().attr("id").unwrap_or("");
    let match_string = format!("{class} {id}");

    if REGEXPS.unlikely_candidates.is_match(&match_string)
        && !REGEXPS.ok_maybe_its_a_candidate.is_match(&match_string)
    {
        return false;
    }

    let link_density = dom_utils::get_link_density(element);
    if text_length > 80 && link_density < 0.25 {
        return true;
    }

    if text_length <= 80 && link_density == 0.0 && has_sentence_boundary(&text) {
        return true;
    }

    false
}

/// Determine whether a non-paragraph block should be kept during sibling aggregation.
fn should_keep_block_element(element: ElementRef, best_score: f64) -> bool {
    use scraper::Selector;
    let tag = element.value().name().to_lowercase();

    if !matches!(
        tag.as_str(),
        "div" | "section" | "article" | "ul" | "ol" | "table"
    ) {
        return false;
    }

    let weight = scoring::get_class_weight(element, ParseFlags::WEIGHT_CLASSES);
    if weight < -25 && best_score < 100.0 {
        return false;
    }

    let text = dom_utils::get_inner_text(element, false);
    let text_length = text.len();
    let link_density = dom_utils::get_link_density(element);

    if text_length == 0 || link_density > 0.6 {
        return false;
    }

    match tag.as_str() {
        "ul" | "ol" => {
            let li_selector = Selector::parse("li").unwrap();
            let li_count = element.select(&li_selector).count();
            li_count >= 3 && text_length > 80 && link_density < 0.4
        }
        "table" => {
            let paragraph_selector = Selector::parse("p").unwrap();
            let paragraph_count = element.select(&paragraph_selector).count();
            (paragraph_count >= 2 || text_length > 200) && link_density < 0.45
        }
        _ => {
            if text_length > 400 {
                true
            } else {
                text_length > 140 && link_density < 0.35
            }
        }
    }
}

/// Detects whether text contains a sentence-ending period followed by whitespace or end.
fn has_sentence_boundary(text: &str) -> bool {
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '.' {
            match chars.peek() {
                Some(next) if next.is_whitespace() => return true,
                None => return true,
                _ => {}
            }
        }
    }
    false
}

/// List of void elements (self-closing tags) in HTML5
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Check if a tag is a void element (self-closing)
fn is_void_element(tag_name: &str) -> bool {
    VOID_ELEMENTS.contains(&tag_name.to_lowercase().as_str())
}

/// Check if a DIV element should be converted to a P tag
///
/// A DIV should be converted to P if it contains no block-level children.
/// This matches Mozilla's Readability.js behavior where DIVs used as
/// paragraph containers are normalized to P tags.
fn should_convert_div_to_p(element: ElementRef) -> bool {
    use crate::constants::DIV_TO_P_ELEMS;

    if element.value().name().to_uppercase() != "DIV" {
        return false;
    }

    for child in element.children() {
        if let Some(child_elem) = ElementRef::wrap(child) {
            let child_tag = child_elem.value().name().to_uppercase();

            if DIV_TO_P_ELEMS.contains(&child_tag.as_str()) {
                return false;
            }
        }
    }

    // No block children found, safe to convert to P
    true
}

/// Count element children (ignoring text/comment nodes).
fn count_element_children(element: ElementRef) -> usize {
    element.children().filter_map(ElementRef::wrap).count()
}

fn is_descendant_of(element: ElementRef, ancestor_id: NodeId) -> bool {
    let mut parent_opt = element.parent();
    while let Some(parent_node) = parent_opt {
        if let Some(parent_elem) = ElementRef::wrap(parent_node) {
            if parent_elem.id() == ancestor_id {
                return true;
            }
            parent_opt = parent_elem.parent();
        } else {
            break;
        }
    }
    false
}

fn is_viable_best_candidate(element: ElementRef, score: f64) -> bool {
    let text = dom_utils::get_inner_text(element, false);
    let text_length = text.len();
    if text_length < 150 && score < 50.0 {
        return false;
    }

    let link_density = dom_utils::get_link_density(element);
    if link_density > 0.6 {
        return false;
    }

    let match_string = format!(
        "{} {}",
        element.value().attr("class").unwrap_or(""),
        element.value().attr("id").unwrap_or("")
    )
    .to_lowercase();

    const NAV_KEYWORDS: [&str; 6] = ["nav", "navbar", "menu", "breadcrumbs", "sidebar", "widget"];
    if NAV_KEYWORDS.iter().any(|kw| match_string.contains(kw)) && link_density > 0.3 {
        return false;
    }

    true
}

/// Check whether an attribute name is an event-handler attribute (`onclick`, `onerror`, …).
///
/// Matches any lowercased name starting with `"on"` and longer than 2 characters,
/// so it also rejects the rare legitimate attribute literally named `"on"`.
fn is_event_handler_attr(name: &str) -> bool {
    name.len() > 2
        && name
            .get(0..2)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("on"))
}

/// Check whether an element must never be emitted when sanitizing.
///
/// These elements execute script, load external content, or collect input, so no
/// attribute-level filtering makes them safe to hand to a browser. Cleaning
/// already drops most of them earlier in the pipeline; this is the backstop for
/// anything a cleaning pass misses.
///
/// Matched case-insensitively rather than relying on html5ever lowercasing, which
/// it does for HTML but not for foreign (SVG/MathML) content.
fn is_unsafe_element(tag: &str) -> bool {
    const UNSAFE_TAGS: [&str; 8] = [
        "script", "style", "iframe", "object", "embed", "form", "noscript", "template",
    ];

    UNSAFE_TAGS
        .iter()
        .any(|unsafe_tag| tag.eq_ignore_ascii_case(unsafe_tag))
}

/// Check whether a URL value uses a dangerous scheme (`javascript:`, `vbscript:`, `data:`),
/// allowing `data:image/*` since lazy-loading placeholders in the wild rely on it.
///
/// Never slices `value` at an index computed from a transformed copy: the scheme is
/// isolated with `split_once` on the original string. The scheme segment is then
/// stripped of whitespace/control chars before comparison (not just its prefix),
/// because browsers do the same while parsing a URL; otherwise a scheme like
/// `java\tscript:` would bypass a filter that only checks the raw segment.
pub(crate) fn is_dangerous_url(value: &str) -> bool {
    let trimmed = value.trim_start_matches(|c: char| c.is_whitespace() || c.is_control());

    let Some((raw_scheme, rest)) = trimmed.split_once(':') else {
        return false;
    };

    // A "scheme" containing '/', '?', or '#' isn't a scheme at all (e.g. a relative
    // or protocol-relative URL that happens to contain a colon later on).
    if raw_scheme.chars().any(|c| matches!(c, '/' | '?' | '#')) {
        return false;
    }

    // Browsers strip ASCII tab/newline and other whitespace/control chars while
    // parsing a URL, so `java\tscript:` reaches the page as `javascript:`. Compare
    // on the stripped form rather than the raw scheme segment to catch this bypass.
    let scheme: String = raw_scheme
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect();

    if scheme.eq_ignore_ascii_case("javascript") || scheme.eq_ignore_ascii_case("vbscript") {
        return true;
    }

    if scheme.eq_ignore_ascii_case("data") {
        return !rest.starts_with("image/");
    }

    false
}

/// Serialize an element and its children to proper HTML (without ancestor tags)
///
/// The scraper crate's `.html()` method includes ancestor tags as empty elements,
/// which creates malformed HTML like `<body></body><html></html><div>content</div>`.
/// This function properly serializes just the element and its descendants.
///
/// Additionally, this function implements DIV→P transformation: DIVs without
/// block-level children are converted to P tags to match Mozilla's behavior.
///
/// When `sanitize` is `true`, unsafe elements are dropped whole, and event-handler
/// attributes and dangerous URL schemes in `href`/`src`/`xlink:href` are dropped
/// from the rest; see [`is_unsafe_element`], [`is_event_handler_attr`] and
/// [`is_dangerous_url`]. This is an opt-in harm reducer, not a full sanitizer.
fn element_to_html(element: ElementRef, sanitize: bool, depth: usize) -> String {
    use scraper::node::Node;
    if depth > MAX_DOM_DEPTH {
        return String::new();
    }
    if !dom_utils::is_probably_visible(element) {
        return String::new();
    }

    let elem_data = element.value();
    let original_tag_name = elem_data.name();

    if sanitize && is_unsafe_element(original_tag_name) {
        return String::new();
    }

    let tag_name = if should_convert_div_to_p(element) {
        "p"
    } else {
        original_tag_name
    };

    let mut html = String::new();
    html.push_str(&format!("<{tag_name}"));

    for (name, value) in elem_data.attrs.iter() {
        if sanitize {
            if is_event_handler_attr(&name.local) {
                continue;
            }
            if matches!(&*name.local, "href" | "src" | "xlink:href") && is_dangerous_url(value) {
                continue;
            }
        }
        html.push_str(&format!(" {}=\"{}\"", name.local, escape(value)));
    }

    if is_void_element(tag_name) {
        html.push_str(" />");
        return html;
    }

    html.push('>');

    for child in element.children() {
        match child.value() {
            Node::Element(_) => {
                if let Some(child_elem) = ElementRef::wrap(child) {
                    let child_html = element_to_html(child_elem, sanitize, depth + 1);
                    if !child_html.is_empty() {
                        html.push_str(&child_html);
                    }
                }
            }
            Node::Text(text) => {
                html.push_str(&escape(&text.text).to_string());
            }
            // A comment body containing `-->` closes the comment early, so the rest
            // of it is parsed as markup. Nothing downstream reads comments, so when
            // sanitizing just drop them rather than trying to encode them.
            Node::Comment(comment) if !sanitize => {
                html.push_str(&format!("<!--{}-->", comment.comment));
            }
            _ => {}
        }
    }

    html.push_str(&format!("</{tag_name}>"));
    html
}

/// Resolve a node id back to an element in the document it came from.
///
/// `NodeId` is only meaningful for its own tree, so passing an id from a
/// different parse returns whatever node happens to occupy that slot.
fn find_element_by_id(document: &Html, id: NodeId) -> Option<ElementRef<'_>> {
    document.tree.get(id).and_then(ElementRef::wrap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute_values_are_escaped() {
        // Regression: attribute values containing quotes/angle-brackets must be
        // re-escaped on output. Without this, a value like `{"a":"<b>"}` breaks
        // the attribute boundary and downstream parses see a mangled element.
        let html = r##"<html><body><article>
            <div data-component="liveblog" props="{&quot;keyEvents&quot;:[{&quot;id&quot;:&quot;abc&quot;,&quot;html&quot;:&quot;&lt;p&gt;x&lt;/p&gt;&quot;}]}">
            <p>This is a substantial paragraph with enough text to satisfy readability thresholds. Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
            <p>Another paragraph so the article gets picked up by grab_article. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.</p>
            </div></article></body></html>"##;

        let document = Html::parse_document(html);
        let options = ReadabilityOptions::builder().char_threshold(100).build();
        let content = grab_article(&document, &options).unwrap().unwrap();

        // The attribute value must be round-trippable: re-parsing the output
        // must yield exactly the original (decoded) attribute value.
        let reparsed = Html::parse_fragment(&content);
        let sel = Selector::parse("[data-component=\"liveblog\"]").unwrap();
        let el = reparsed
            .select(&sel)
            .next()
            .expect("liveblog div should survive the round-trip");
        assert_eq!(
            el.value().attr("props").unwrap(),
            r#"{"keyEvents":[{"id":"abc","html":"<p>x</p>"}]}"#
        );
    }

    #[test]
    fn test_sanitize_content_strips_event_handlers_and_dangerous_schemes() {
        let html = r#"<html><body><article>
            <p>This is a substantial paragraph with enough text to satisfy readability thresholds. Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
            <p>Another paragraph with plenty of content <img src="pic.jpg" onerror="alert(1)" alt="pic"> and a link <a href="javascript:alert(1)">click here</a> to make sure the section is picked up as the article body by the scoring algorithm.</p>
            </article></body></html>"#;

        let document = Html::parse_document(html);
        let options = ReadabilityOptions::builder()
            .char_threshold(100)
            .sanitize_content(true)
            .build();
        let content = grab_article(&document, &options).unwrap().unwrap();

        assert!(!content.contains("onerror"));
        assert!(!content.contains("javascript:"));
        assert!(content.contains("pic.jpg"));
        assert!(content.contains("click here"));
    }

    #[test]
    fn test_sanitize_content_preserves_safe_urls() {
        let html = "<html><body><article>
            <p>This is a substantial paragraph with enough text to satisfy readability thresholds. Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
            <p>Another paragraph <img src=\"data:image/png;base64,iVBORw0KGgo=\" alt=\"placeholder\"> with a normal <a href=\"  \tHTTPS://example.com/page\">link</a> to ensure the section scores well enough to be selected as article content.</p>
            </article></body></html>";

        let document = Html::parse_document(html);
        let options = ReadabilityOptions::builder()
            .char_threshold(100)
            .sanitize_content(true)
            .build();
        let content = grab_article(&document, &options).unwrap().unwrap();

        // Values are entity-escaped for parsing correctness (e.g. `/` -> `&#x2f;`),
        // so verify via the decoded attribute value after re-parsing, as in
        // `test_attribute_values_are_escaped`.
        let reparsed = Html::parse_fragment(&content);
        let img_sel = Selector::parse("img").unwrap();
        let img = reparsed
            .select(&img_sel)
            .next()
            .expect("img should survive sanitization");
        assert_eq!(
            img.value().attr("src").unwrap(),
            "data:image/png;base64,iVBORw0KGgo="
        );

        let a_sel = Selector::parse("a").unwrap();
        let a = reparsed
            .select(&a_sel)
            .next()
            .expect("link should survive sanitization");
        assert_eq!(
            a.value().attr("href").unwrap(),
            "  \tHTTPS://example.com/page"
        );
    }

    #[test]
    fn test_sanitize_content_default_false_preserves_event_handlers() {
        let html = r#"<html><body><article>
            <p>This is a substantial paragraph with enough text to satisfy readability thresholds. Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
            <p>Another paragraph with plenty of content <img src="pic.jpg" onerror="alert(1)" alt="pic"> and a link <a href="javascript:alert(1)">click here</a> to make sure the section is picked up as the article body by the scoring algorithm.</p>
            </article></body></html>"#;

        let document = Html::parse_document(html);
        let options = ReadabilityOptions::builder().char_threshold(100).build();
        let content = grab_article(&document, &options).unwrap().unwrap();

        assert!(content.contains("onerror"));
        assert!(content.contains("javascript:"));
    }

    #[test]
    fn test_is_dangerous_url() {
        assert!(is_dangerous_url("JavaScript:alert(1)"));
        assert!(is_dangerous_url("\t\n javascript:alert(1)"));
        assert!(is_dangerous_url("VBScript:msgbox(1)"));
        assert!(is_dangerous_url("data:text/html,<script>alert(1)</script>"));
        assert!(!is_dangerous_url("data:image/png;base64,iVBORw0KGgo="));
        assert!(!is_dangerous_url("/relative/path"));
        assert!(!is_dangerous_url("//host/path"));
        assert!(!is_dangerous_url("https://example.com"));

        // Browsers strip ASCII tab/newline/CR and C0 controls while parsing a URL,
        // so a scheme with embedded whitespace/control chars still reaches the page
        // as `javascript:`, so the filter must normalize the scheme, not just its prefix.
        assert!(is_dangerous_url("java\tscript:alert(1)"));
        assert!(is_dangerous_url("java\nscript:alert(1)"));
        assert!(is_dangerous_url("java\rscript:alert(1)"));
        assert!(is_dangerous_url("jav\0ascript:alert(1)"));

        // Must not panic on multi-byte UTF-8 input.
        assert!(!is_dangerous_url("https://example.com/İstanbul/🎉"));
        assert!(!is_dangerous_url("İ"));
        assert!(!is_dangerous_url("🎉:notreal"));
    }

    /// A full article page carrying one script, closed with the given end-tag
    /// spelling. The tokenizer accepts whitespace before the `>`, so every
    /// variant here is a real script as far as a browser is concerned.
    fn article_with_script(end_tag: &str) -> String {
        let paragraphs: String = (1..=4)
            .map(|i| {
                format!(
                    "<p>Paragraph {i} of the article body, long enough on its own to clear the \
                     character threshold so the scoring algorithm selects this article element \
                     as the winning candidate rather than something else on the page.</p>"
                )
            })
            .collect();

        format!(
            "<html><head><title>Scripted Article</title></head><body><article>\
             {paragraphs}<script>alert(1){end_tag}\n</article></body></html>"
        )
    }

    fn parse_content(html: &str, sanitize: bool) -> String {
        let options = ReadabilityOptions::builder()
            .char_threshold(100)
            .sanitize_content(sanitize)
            .build();
        crate::Readability::new(html, None, Some(options))
            .unwrap()
            .parse()
            .and_then(|a| a.content)
            .unwrap_or_default()
    }

    #[test]
    fn test_script_removed_for_every_end_tag_spelling() {
        let end_tags = [
            "</script>",
            "</script >",
            "</script\n>",
            "</script\t>",
            "</SCRIPT >",
        ];

        for end_tag in end_tags {
            let html = article_with_script(end_tag);

            for sanitize in [false, true] {
                let content = parse_content(&html, sanitize);
                assert!(
                    !content.contains("<script"),
                    "end tag {end_tag:?}, sanitize={sanitize}: script element survived"
                );
                assert!(
                    !content.contains("alert(1)"),
                    "end tag {end_tag:?}, sanitize={sanitize}: script body survived"
                );
                assert!(
                    content.contains("Paragraph 1 of the article body"),
                    "end tag {end_tag:?}, sanitize={sanitize}: article body was lost"
                );
            }
        }
    }

    fn document_with_many_elements() -> String {
        let paragraphs: String = (1..=50)
            .map(|i| {
                format!(
                    "<p>Paragraph {i} of the article body, long enough to clear the character \
                     threshold so extraction has a genuine candidate to select.</p>"
                )
            })
            .collect();

        format!("<html><body><article>{paragraphs}</article></body></html>")
    }

    #[test]
    fn test_max_elems_to_parse_rejects_oversized_document() {
        let document = Html::parse_document(&document_with_many_elements());
        let options = ReadabilityOptions::builder()
            .char_threshold(100)
            .max_elems_to_parse(10)
            .build();

        let result = grab_article(&document, &options);

        assert!(matches!(
            result,
            Err(ReadabilityError::MaxElementsExceeded(_))
        ));
    }

    #[test]
    fn test_max_elems_to_parse_zero_means_unlimited() {
        let document = Html::parse_document(&document_with_many_elements());
        let options = ReadabilityOptions::builder()
            .char_threshold(100)
            .max_elems_to_parse(0)
            .build();

        assert!(grab_article(&document, &options).unwrap().is_some());
    }

    #[test]
    fn test_max_elems_to_parse_generous_limit_parses_normally() {
        let document = Html::parse_document(&document_with_many_elements());
        let options = ReadabilityOptions::builder()
            .char_threshold(100)
            .max_elems_to_parse(10_000)
            .build();

        assert!(grab_article(&document, &options).unwrap().is_some());
    }

    /// `Readability::parse` collapses every error into `None`, so the limit
    /// surfaces publicly as "no article" rather than as a distinguishable error.
    #[test]
    fn test_max_elems_to_parse_surfaces_as_none_from_parse() {
        let html = document_with_many_elements();

        let capped = ReadabilityOptions::builder()
            .char_threshold(100)
            .max_elems_to_parse(10)
            .build();
        assert!(crate::Readability::new(&html, None, Some(capped))
            .unwrap()
            .parse()
            .is_none());

        let uncapped = ReadabilityOptions::builder().char_threshold(100).build();
        assert!(crate::Readability::new(&html, None, Some(uncapped))
            .unwrap()
            .parse()
            .is_some());
    }

    /// Comfortably past MAX_DOM_DEPTH, and verified to overflow the stack when the
    /// depth guard is removed, so these tests fail loudly if the guard regresses.
    const NESTING_BEYOND_LIMIT: usize = 2_000;

    fn deeply_nested_article(depth: usize) -> String {
        let mut html = String::from("<html><body><article>");
        html.push_str(&"<div>".repeat(depth));
        html.push_str(
            "<p>Substantial paragraph text long enough to be scored as content. Lorem ipsum \
             dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt.</p>",
        );
        html.push_str(&"</div>".repeat(depth));
        html.push_str("</article></body></html>");
        html
    }

    /// Without the depth bound this overflows the stack, which aborts the
    /// process rather than unwinding, so a caller cannot defend against it.
    #[test]
    fn test_deeply_nested_html_does_not_overflow_serializer() {
        let document = Html::parse_document(&deeply_nested_article(NESTING_BEYOND_LIMIT));
        let selector = Selector::parse("article").unwrap();
        let article = document.select(&selector).next().unwrap();

        let html = element_to_html(article, false, 0);

        assert!(html.starts_with("<article>"));
        assert!(html.ends_with("</article>"));
    }

    /// The bound has to be generous enough that ordinary markup is untouched.
    #[test]
    fn test_moderate_nesting_keeps_content() {
        let html = deeply_nested_article(100);
        let article = crate::Readability::new(&html, None, None)
            .unwrap()
            .parse()
            .expect("a hundred levels of nesting is ordinary markup");

        assert!(article
            .content
            .as_deref()
            .unwrap_or_default()
            .contains("Substantial paragraph text"));
    }

    #[test]
    fn test_find_element_by_id_round_trips() {
        let document =
            Html::parse_document("<html><body><article><p>Text</p></article></body></html>");
        let selector = Selector::parse("article").unwrap();
        let article = document.select(&selector).next().unwrap();

        let resolved = find_element_by_id(&document, article.id()).unwrap();

        assert_eq!(resolved.id(), article.id());
        assert_eq!(resolved.value().name(), "article");
    }

    #[test]
    fn test_is_unsafe_element() {
        for tag in [
            "script", "style", "iframe", "object", "embed", "form", "noscript", "template",
        ] {
            assert!(is_unsafe_element(tag), "{tag} should be denylisted");
            assert!(
                is_unsafe_element(&tag.to_uppercase()),
                "{tag} should match case-insensitively"
            );
        }

        for tag in ["p", "div", "article", "img", "a", "table", "span"] {
            assert!(!is_unsafe_element(tag), "{tag} should be allowed");
        }
    }

    #[test]
    fn test_sanitize_content_drops_comments() {
        let html = r#"<html><body><article>
            <p>This is a substantial paragraph with enough text to satisfy readability thresholds. Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
            <p>Another paragraph with plenty of content to make sure the section is picked up as the article body by the scoring algorithm.<!-- --><img src=x onerror=alert(1)> --></p>
            </article></body></html>"#;

        let document = Html::parse_document(html);
        let options = ReadabilityOptions::builder()
            .char_threshold(100)
            .sanitize_content(true)
            .build();
        let content = grab_article(&document, &options).unwrap().unwrap();

        // The comment body closes itself early, so its payload would become live
        // markup if the comment were emitted verbatim.
        assert!(!content.contains("<!--"));
        assert!(!content.contains("onerror"));
    }

    #[test]
    fn test_is_event_handler_attr() {
        assert!(is_event_handler_attr("onclick"));
        assert!(is_event_handler_attr("ONERROR"));
        assert!(is_event_handler_attr("onload"));
        assert!(!is_event_handler_attr("on"));
        assert!(!is_event_handler_attr("href"));
        // "once" starts with "on" and is longer than 2 chars: intentionally dropped too,
        // per the plan's accepted false-positive tradeoff for this opt-in filter.
        assert!(is_event_handler_attr("once"));
    }

    #[test]
    fn test_grab_article_simple() {
        let html = r#"
            <html>
                <body>
                    <article>
                        <h1>Test Article</h1>
                        <p>This is the first paragraph with some content that should be extracted.</p>
                        <p>This is the second paragraph with more content to ensure we have enough text.</p>
                        <p>And a third paragraph to make sure we exceed the minimum threshold for article extraction.</p>
                    </article>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let options = ReadabilityOptions::builder().char_threshold(100).build();

        let result = grab_article(&document, &options);
        assert!(result.is_ok());

        let content = result.unwrap();
        assert!(content.is_some());

        let content_html = content.unwrap();
        assert!(content_html.contains("first paragraph"));
    }

    #[test]
    fn test_grab_article_short_content() {
        let html = r#"
            <html>
                <body>
                    <p>Too short.</p>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let options = ReadabilityOptions::default();

        let result = grab_article(&document, &options);
        assert!(result.is_ok());

        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_candidate_scoring() {
        let html = r#"
            <html>
                <body>
                    <article>
                        <div class="content">
                            <p>First paragraph with good content, multiple sentences, and enough length to score well.</p>
                            <p>Second paragraph also with substantial content that adds to the score.</p>
                        </div>
                    </article>
                    <div class="sidebar ad">
                        <p>Advertisement text that should score poorly.</p>
                    </div>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let options = ReadabilityOptions::default();
        let flags = ParseFlags::WEIGHT_CLASSES | ParseFlags::CLEAN_CONDITIONALLY;

        let candidates = find_candidates(&document, &options, flags).unwrap();
        assert!(!candidates.is_empty());

        let scores = score_candidates(&document, candidates, &options, flags);
        assert!(!scores.is_empty());
    }

    #[test]
    fn test_sibling_aggregation() {
        let html = r#"
            <html>
                <body>
                    <div class="article">
                        <h2>Article Title</h2>
                        <p>This is the first paragraph of the article with enough content to be considered good content.</p>
                        <p>This is the second paragraph, also with substantial content that should be included in the extraction.</p>
                        <p>And a third paragraph that continues the article content with more information for the reader.</p>
                        <div class="share">
                            <a href="javascript:void(0)">Share</a>
                        </div>
                        <p>A fourth paragraph that should also be included because it has enough text and is part of the article flow.</p>
                    </div>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let options = ReadabilityOptions::builder().char_threshold(100).build();

        let result = grab_article(&document, &options);
        assert!(result.is_ok());

        let content = result.unwrap();
        assert!(content.is_some());

        let content_html = content.unwrap();

        assert!(content_html.contains("first paragraph"));
        assert!(content_html.contains("second paragraph"));
        assert!(content_html.contains("third paragraph"));
        // The fourth paragraph might not be included depending on scoring,
        // but we should have at least the first three
    }

    #[test]
    fn test_html_escape() {
        let html = r#"
            <html>
                <body>
                    <article>
                        <h1>Test Article</h1>
                        <p>This is the first paragraph with some content that should be extracted.</p>
                        <p>This is the second paragraph with more content to ensure we have enough text.</p>
                        <p>And a third paragraph to make sure we exceed the minimum threshold for article extraction.</p>
                        &lt;script&gt;
                        console.log("…but that’s good! That means your future was polled! Bla Bla Black sheep");
                        &lt;/script&gt;
                    </article>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let options = ReadabilityOptions::builder().char_threshold(100).build();

        let result = grab_article(&document, &options);
        assert!(result.is_ok());

        let content = result.unwrap();
        assert!(content.is_some());

        let content_html = content.unwrap();
        assert!(!content_html.contains("<script>"));
        assert!(content_html.contains("&lt;script&gt;"));
    }
}
