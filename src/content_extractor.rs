//! Core content extraction algorithm (_grabArticle implementation).

use crate::constants::{ParseFlags, DEFAULT_TAGS_TO_SCORE, REGEXPS};
use crate::error::Result;
use crate::options::ReadabilityOptions;
use crate::{dom_utils, scoring};
use scraper::{ElementRef, Html, Selector};
use std::collections::HashMap;

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
pub fn grab_article(document: &Html, options: &ReadabilityOptions) -> Result<Option<String>> {
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
        attempts.sort_by(|a, b| b.text_length.cmp(&a.text_length));
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
            let match_string = format!("{} {}", class, id);

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
                let match_string = format!("{} {}", class, id);

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
) -> HashMap<String, f64> {
    let mut scores: HashMap<String, f64> = HashMap::new();

    for candidate in candidates {
        let content_score =
            scoring::calculate_content_score(candidate, options.link_density_modifier);

        if content_score == 0.0 {
            continue;
        }

        // Ensure the candidate itself is tracked; in Mozilla's implementation the
        // element owns the score before propagating to ancestors.
        let candidate_id = get_element_id(&candidate);
        let candidate_entry = scores
            .entry(candidate_id)
            .or_insert_with(|| scoring::initialize_node_score(candidate, flags));
        *candidate_entry += content_score;

        let ancestors = dom_utils::get_node_ancestors(candidate, Some(5));

        // Propagate score to ancestors
        // Parent gets 1x, grandparent gets 0.5x, great-grandparent gets 0.33x, etc.
        for (level, ancestor) in ancestors.iter().enumerate() {
            let ancestor_id = get_element_id(ancestor);
            if !scores.contains_key(&ancestor_id) {
                let base_score = scoring::initialize_node_score(*ancestor, flags);
                scores.insert(ancestor_id.clone(), base_score);
            }

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
fn apply_link_density_penalty(document: &Html, scores: &mut HashMap<String, f64>) {
    for (element_id, score) in scores.iter_mut() {
        if let Some(element) = find_element_by_id(document, element_id) {
            let penalty = (1.0 - dom_utils::get_link_density(element)).max(0.0);
            *score *= penalty;
        }
    }
}

/// Find the best candidate based on scores, promoting parents when needed.
fn find_best_candidate(
    document: &Html,
    scores: &HashMap<String, f64>,
    options: &ReadabilityOptions,
) -> Option<String> {
    let mut sorted_scores: Vec<_> = scores.iter().collect();
    sorted_scores.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());

    let top_candidates: Vec<(String, f64)> = sorted_scores
        .iter()
        .take(options.nb_top_candidates)
        .map(|(id, score)| ((*id).clone(), **score))
        .collect();

    if top_candidates.is_empty() {
        return None;
    }

    let mut best_id = top_candidates[0].0.clone();
    let mut best_score = top_candidates[0].1;

    for (candidate_id, candidate_score) in &top_candidates {
        if let Some(elem) = find_element_by_id(document, candidate_id) {
            if is_viable_best_candidate(elem, *candidate_score) {
                best_id = candidate_id.clone();
                best_score = *candidate_score;
                break;
            }
        }
    }

    if let Some(promoted) =
        promote_shared_top_candidate_parent(document, &best_id, best_score, &top_candidates)
    {
        best_id = promoted;
        best_score = scores.get(&best_id).copied().unwrap_or(best_score);
    }

    if let Some(promoted) = promote_high_scoring_parents(document, &best_id, best_score, scores) {
        best_id = promoted;
        best_score = scores.get(&best_id).copied().unwrap_or(best_score);
    }

    // If the best candidate lives inside a single-child parent chain, walk up so we can pull siblings later.
    if let Some(promoted) = promote_single_child_parents(document, &best_id) {
        best_id = promoted;
    }

    if let Some(promoted) = promote_dense_wrapper_child(document, &best_id, scores, &sorted_scores)
    {
        best_id = promoted;
        best_score = scores.get(&best_id).copied().unwrap_or(best_score);
    }

    if let Some(promoted) =
        promote_semantic_descendant(document, &best_id, best_score, &sorted_scores)
    {
        best_id = promoted;
    }

    Some(best_id)
}

/// Promote parent nodes when the current candidate is the only child, mirroring Mozilla's logic.
fn promote_single_child_parents(document: &Html, best_id: &str) -> Option<String> {
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
            let parent_id = get_element_id(&parent);
            promoted_id = Some(parent_id.clone());
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
    best_id: &str,
    best_score: f64,
    top_candidates: &[(String, f64)],
) -> Option<String> {
    const MINIMUM_TOP_CANDIDATES: usize = 3;
    if best_score <= 0.0 {
        return None;
    }

    let mut ancestor_lists: Vec<Vec<String>> = Vec::new();

    for (candidate_id, candidate_score) in top_candidates.iter().skip(1) {
        if *candidate_score < best_score * 0.75 {
            continue;
        }

        let Some(candidate_elem) = find_element_by_id(document, candidate_id) else {
            continue;
        };
        let ancestors = dom_utils::get_node_ancestors(candidate_elem, None);
        if ancestors.is_empty() {
            continue;
        }

        let ancestor_ids = ancestors
            .into_iter()
            .map(|ancestor| get_element_id(&ancestor))
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
        let parent_id = get_element_id(&parent_opt);
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
    best_id: &str,
    best_score: f64,
    scores: &HashMap<String, f64>,
) -> Option<String> {
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

        let parent_id = get_element_id(&parent);
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
    best_id: &str,
    scores: &HashMap<String, f64>,
    sorted_scores: &[(&String, &f64)],
) -> Option<String> {
    let Some(best_elem) = find_element_by_id(document, best_id) else {
        return None;
    };

    let tag = best_elem.value().name().to_uppercase();
    if matches!(tag.as_str(), "ARTICLE" | "SECTION" | "MAIN") {
        return None;
    }

    let parent_score = scores.get(best_id).copied().unwrap_or(0.0);
    let best_link_density = dom_utils::get_link_density(best_elem);

    let mut fallback = None;

    for (candidate_id, candidate_score) in sorted_scores.iter().take(20) {
        if *candidate_id == best_id {
            continue;
        }
        let Some(candidate_elem) = find_element_by_id(document, candidate_id) else {
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
            fallback = Some(((*candidate_id).clone(), score));
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
    best_id: &str,
    best_score: f64,
    sorted_scores: &[(&String, &f64)],
) -> Option<String> {
    if best_score <= 0.0 {
        return None;
    }

    let Some(best_elem) = find_element_by_id(document, best_id) else {
        return None;
    };

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

    let mut promoted_child: Option<(String, f64)> = None;

    for (candidate_id, candidate_score) in sorted_scores.iter().take(40) {
        if *candidate_id == best_id {
            continue;
        }

        let Some(candidate_elem) = find_element_by_id(document, candidate_id) else {
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
            promoted_child = Some(((*candidate_id).clone(), score));
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
    best_candidate_id: String,
    all_scores: &HashMap<String, f64>,
    _options: &ReadabilityOptions,
) -> Result<String> {
    let Some(best_candidate) = find_element_by_id(document, &best_candidate_id) else {
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
        let html = element_to_html(best_candidate);
        let html = crate::cleaner::replace_brs(&html);
        return Ok(html);
    };

    for child_node in parent.children() {
        let Some(sibling) = ElementRef::wrap(child_node) else {
            continue;
        };

        let sibling_id = get_element_id(&sibling);
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
            if weighted_sibling_score >= sibling_score_threshold {
                true
            } else if is_good_sibling_paragraph(sibling) {
                true
            } else {
                should_keep_block_element(sibling, best_score)
            }
        };

        if should_include {
            let mut sibling_html = element_to_html(sibling);
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
    let match_string = format!("{} {}", class, id);

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
    element
        .children()
        .filter_map(|child| ElementRef::wrap(child))
        .count()
}

fn is_descendant_of(element: ElementRef, ancestor_id: &str) -> bool {
    let mut parent_opt = element.parent();
    while let Some(parent_node) = parent_opt {
        if let Some(parent_elem) = ElementRef::wrap(parent_node) {
            if get_element_id(&parent_elem) == ancestor_id {
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

/// Serialize an element and its children to proper HTML (without ancestor tags)
///
/// The scraper crate's `.html()` method includes ancestor tags as empty elements,
/// which creates malformed HTML like `<body></body><html></html><div>content</div>`.
/// This function properly serializes just the element and its descendants.
///
/// Additionally, this function implements DIV→P transformation: DIVs without
/// block-level children are converted to P tags to match Mozilla's behavior.
fn element_to_html(element: ElementRef) -> String {
    use scraper::node::Node;
    if !dom_utils::is_probably_visible(element) {
        return String::new();
    }

    let elem_data = element.value();
    let original_tag_name = elem_data.name();

    let tag_name = if should_convert_div_to_p(element) {
        "p"
    } else {
        original_tag_name
    };

    let mut html = String::new();
    html.push_str(&format!("<{}", tag_name));

    for (name, value) in elem_data.attrs.iter() {
        html.push_str(&format!(" {}=\"{}\"", name.local, value));
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
                    let child_html = element_to_html(child_elem);
                    if !child_html.is_empty() {
                        html.push_str(&child_html);
                    }
                }
            }
            Node::Text(text) => {
                html.push_str(&text.text);
            }
            Node::Comment(comment) => {
                html.push_str(&format!("<!--{}-->", comment.comment));
            }
            _ => {}
        }
    }

    html.push_str(&format!("</{}>", tag_name));
    html
}

fn get_element_id(element: &ElementRef) -> String {
    format!("{:?}", element.id())
}

/// Find an element by our generated ID
fn find_element_by_id<'a>(document: &'a Html, id: &str) -> Option<ElementRef<'a>> {
    // This is a simplified approach - in production we'd need better element tracking
    // For now, search for elements and match by generated ID

    let all_selector = Selector::parse("*").unwrap();
    for elem in document.select(&all_selector) {
        if get_element_id(&elem) == id {
            return Some(elem);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(candidates.len() > 0);

        let scores = score_candidates(&document, candidates, &options, flags);
        assert!(scores.len() > 0);
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
}
