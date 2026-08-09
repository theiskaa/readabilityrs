//! Byline/author extraction heuristics from document structure.

use crate::utils;
use scraper::node::Node;
use scraper::{ElementRef, Html, Selector};
use std::borrow::Cow;
use std::sync::LazyLock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DomBylineCandidate {
    pub(super) text: String,
    pub(super) confidence: DomBylineConfidence,
}

impl DomBylineCandidate {
    fn new(text: String, confidence: DomBylineConfidence) -> Self {
        Self { text, confidence }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DomBylineConfidence {
    High,
    Medium,
    Low,
}

/// Extract byline/author from document structure
///
/// This function checks multiple sources in priority order:
/// 1. rel="author" links
/// 2. itemprop="author" elements
/// 3. Common byline CSS classes (.byline, .author, .by, etc.)
/// 4. <address> tags with author context
pub(super) fn extract_byline_from_document(document: &Html) -> Option<DomBylineCandidate> {
    use crate::scoring;

    let mut fallback_candidate: Option<DomBylineCandidate> = None;
    if let Some(candidate) = extract_standfirst_caps_byline(document) {
        return Some(DomBylineCandidate::new(
            candidate,
            DomBylineConfidence::High,
        ));
    }

    if let Ok(author_link_selector) = Selector::parse("a[rel~='author']") {
        for link in document.select(&author_link_selector) {
            if is_ignorable_byline_context(&link) {
                continue;
            }
            if is_noise_byline_context(&link) {
                continue;
            }
            if let Some(parent_text) = parent_byline_text(&link) {
                return Some(DomBylineCandidate::new(
                    parent_text,
                    DomBylineConfidence::High,
                ));
            }

            let text = collect_byline_candidate_text(link).trim().to_string();
            if !text.is_empty() {
                let class = link.value().attr("class").unwrap_or("");
                let id = link.value().attr("id").unwrap_or("");
                let rel_attr = link.value().attr("rel").unwrap_or("");
                let match_string = format!("{class} {id}");
                let has_author_rel = rel_attr
                    .split_whitespace()
                    .any(|rel| rel.eq_ignore_ascii_case("author"));

                if has_author_rel || scoring::is_valid_byline(link, &match_string) {
                    match utils::clean_byline_text_with_reason(&text) {
                        utils::CleanBylineOutcome::Accepted(cleaned) => {
                            return Some(DomBylineCandidate::new(
                                cleaned,
                                DomBylineConfidence::High,
                            ))
                        }
                        utils::CleanBylineOutcome::DroppedOrgCredit => return None,
                        utils::CleanBylineOutcome::Dropped => {}
                    }
                }
            }
        }
    }

    if let Ok(itemprop_selector) = Selector::parse("[itemprop~='author']") {
        for elem in document.select(&itemprop_selector) {
            if is_ignorable_byline_context(&elem) {
                continue;
            }
            if is_noise_byline_context(&elem) {
                continue;
            }
            if let Some(parent_text) = parent_byline_text(&elem) {
                return Some(DomBylineCandidate::new(
                    parent_text,
                    DomBylineConfidence::High,
                ));
            }

            let text = collect_byline_candidate_text(elem).trim().to_string();
            if !text.is_empty() {
                let class = elem.value().attr("class").unwrap_or("");
                let id = elem.value().attr("id").unwrap_or("");
                let itemprop = elem.value().attr("itemprop").unwrap_or("");
                let match_string = format!("{class} {id}");
                let has_author_itemprop = itemprop
                    .split_whitespace()
                    .any(|prop| prop.eq_ignore_ascii_case("author"));

                if has_author_itemprop || scoring::is_valid_byline(elem, &match_string) {
                    match utils::clean_byline_text_with_reason(&text) {
                        utils::CleanBylineOutcome::Accepted(cleaned) => {
                            return Some(DomBylineCandidate::new(
                                cleaned,
                                DomBylineConfidence::High,
                            ))
                        }
                        utils::CleanBylineOutcome::DroppedOrgCredit => return None,
                        utils::CleanBylineOutcome::Dropped => {}
                    }
                }
            }
        }
    }

    let byline_patterns = [
        ".byline",
        ".pb-byline",
        ".author",
        ".by",
        ".writer",
        ".article-author",
        ".post-author",
        ".entry-author",
        "#byline",
        "#author",
        "[class*='author']",
        "[class*='byline']",
    ];

    for pattern in &byline_patterns {
        if let Ok(selector) = Selector::parse(pattern) {
            for elem in document.select(&selector) {
                if !element_has_byline_keyword(&elem) && is_ignorable_byline_context(&elem) {
                    continue;
                }
                if !element_has_byline_keyword(&elem) && is_noise_byline_context(&elem) {
                    continue;
                }
                let text = collect_byline_candidate_text(elem).trim().to_string();
                let text_is_caps = looks_like_caps_author(&text);

                if text.is_empty() || text.len() > 100 {
                    continue;
                }

                let class = elem.value().attr("class").unwrap_or("");
                let id = elem.value().attr("id").unwrap_or("");
                let match_string = format!("{class} {id}");

                if scoring::is_valid_byline(elem, &match_string)
                    || utils::looks_like_byline(&text)
                    || text_is_caps
                {
                    let confidence = if element_has_explicit_byline_marker(&elem) {
                        DomBylineConfidence::High
                    } else {
                        DomBylineConfidence::Medium
                    };
                    match utils::clean_byline_text_with_reason(&text) {
                        utils::CleanBylineOutcome::Accepted(cleaned) => {
                            let candidate = DomBylineCandidate::new(cleaned, confidence);
                            if is_priority_dom_candidate(&candidate, text_is_caps) {
                                return Some(candidate);
                            } else if fallback_candidate.is_none() {
                                fallback_candidate = Some(candidate);
                            }
                        }
                        utils::CleanBylineOutcome::DroppedOrgCredit => return None,
                        utils::CleanBylineOutcome::Dropped => {}
                    }
                }
            }
        }
    }

    if let Ok(selector) = Selector::parse("[class], [id]") {
        for elem in document.select(&selector) {
            if is_ignorable_byline_context(&elem) {
                continue;
            }
            if is_noise_byline_context(&elem) {
                continue;
            }
            let class = elem.value().attr("class").unwrap_or("");
            let id = elem.value().attr("id").unwrap_or("");
            let class_lower = class.to_lowercase();
            let id_lower = id.to_lowercase();

            if !(class_lower.contains("byline")
                || class_lower.contains("author")
                || class_lower.contains("credit")
                || id_lower.contains("byline")
                || id_lower.contains("author"))
            {
                continue;
            }

            let text = collect_byline_candidate_text(elem).trim().to_string();
            if text.is_empty() || text.len() > 120 {
                continue;
            }

            let text_is_caps = looks_like_caps_author(&text);
            let match_string = format!("{class} {id}");
            if scoring::is_valid_byline(elem, &match_string)
                || utils::looks_like_byline(&text)
                || text_is_caps
            {
                match utils::clean_byline_text_with_reason(&text) {
                    utils::CleanBylineOutcome::Accepted(cleaned) => {
                        let candidate =
                            DomBylineCandidate::new(cleaned, DomBylineConfidence::Medium);
                        if is_priority_dom_candidate(&candidate, text_is_caps) {
                            return Some(candidate);
                        } else if fallback_candidate.is_none() {
                            fallback_candidate = Some(candidate);
                        }
                    }
                    utils::CleanBylineOutcome::DroppedOrgCredit => continue,
                    utils::CleanBylineOutcome::Dropped => {}
                }
            }
        }
    }

    if let Ok(address_selector) = Selector::parse("address") {
        for elem in document.select(&address_selector) {
            if is_ignorable_byline_context(&elem) {
                continue;
            }
            if is_noise_byline_context(&elem) {
                continue;
            }
            let text = collect_byline_candidate_text(elem).trim().to_string();

            if text.is_empty() || text.len() > 100 {
                continue;
            }

            let text_is_caps = looks_like_caps_author(&text);
            if utils::looks_like_byline(&text)
                || scoring::is_valid_byline(elem, &text)
                || text_is_caps
            {
                match utils::clean_byline_text_with_reason(&text) {
                    utils::CleanBylineOutcome::Accepted(cleaned) => {
                        let candidate = DomBylineCandidate::new(cleaned, DomBylineConfidence::Low);
                        if is_priority_dom_candidate(&candidate, text_is_caps) {
                            return Some(candidate);
                        } else if fallback_candidate.is_none() {
                            fallback_candidate = Some(candidate);
                        }
                    }
                    utils::CleanBylineOutcome::DroppedOrgCredit => continue,
                    utils::CleanBylineOutcome::Dropped => {}
                }
            }
        }
    }

    if let Ok(selector) = Selector::parse("p, div, span") {
        for elem in document.select(&selector) {
            if is_ignorable_byline_context(&elem) {
                continue;
            }
            if is_noise_byline_context(&elem) {
                continue;
            }
            let text = collect_byline_candidate_text(elem).trim().to_string();
            if text.is_empty() || text.len() > 120 {
                continue;
            }

            if utils::looks_like_dateline(&text) {
                continue;
            }

            let text_is_caps = looks_like_caps_author(&text);
            if utils::looks_like_byline(&text) || text_is_caps {
                match utils::clean_byline_text_with_reason(&text) {
                    utils::CleanBylineOutcome::Accepted(cleaned) => {
                        let candidate = DomBylineCandidate::new(cleaned, DomBylineConfidence::Low);
                        if is_priority_dom_candidate(&candidate, text_is_caps) {
                            return Some(candidate);
                        } else if fallback_candidate.is_none() {
                            fallback_candidate = Some(candidate);
                        }
                    }
                    utils::CleanBylineOutcome::DroppedOrgCredit => return None,
                    utils::CleanBylineOutcome::Dropped => {}
                }
            }
        }
    }

    if let Some(candidate) = fallback_candidate {
        return Some(candidate);
    }

    None
}

pub(super) fn extract_standfirst_caps_byline(document: &Html) -> Option<String> {
    const SELECTORS: [&str; 2] = ["em.byline", "[class*='byline']"];
    const STANDFIRST_KEYWORDS: [&str; 1] = ["standfirst"];

    for pattern in &SELECTORS {
        if let Ok(selector) = Selector::parse(pattern) {
            for elem in document.select(&selector) {
                if !ancestor_has_keyword(&elem, &STANDFIRST_KEYWORDS, 5) {
                    continue;
                }
                if is_ignorable_byline_context(&elem) || is_noise_byline_context(&elem) {
                    continue;
                }
                let text = collect_byline_candidate_text(elem).trim().to_string();
                if text.is_empty() || text.len() > 80 {
                    continue;
                }
                if !looks_like_caps_author(&text) {
                    continue;
                }
                match utils::clean_byline_text_with_reason(&text) {
                    utils::CleanBylineOutcome::Accepted(cleaned) => return Some(cleaned),
                    utils::CleanBylineOutcome::DroppedOrgCredit
                    | utils::CleanBylineOutcome::Dropped => continue,
                }
            }
        }
    }

    None
}

fn build_byline_text(element: &ElementRef) -> String {
    fn append_children_text(element: &ElementRef, out: &mut String, depth: usize) {
        if depth > crate::constants::MAX_DOM_DEPTH {
            return;
        }

        for child in element.children() {
            match child.value() {
                Node::Text(text) => {
                    let mut text_slice: &str = text.as_ref();
                    if out.ends_with('\n') && text_slice.starts_with('\n') {
                        text_slice = &text_slice[1..];
                    }
                    if out.ends_with('\n') {
                        let adjusted = strip_intermediate_newline(text_slice);
                        out.push_str(&adjusted);
                    } else {
                        out.push_str(text_slice);
                    }
                }
                Node::Element(data) => {
                    if data.name().eq_ignore_ascii_case("br") {
                        out.push('\n');
                    }
                    if let Some(child_el) = ElementRef::wrap(child) {
                        append_children_text(&child_el, out, depth + 1);
                    }
                }
                _ => {}
            }
        }
    }

    let mut buffer = String::new();
    append_children_text(element, &mut buffer, 0);
    buffer
}

fn strip_intermediate_newline(text: &str) -> Cow<'_, str> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() && bytes[i] != b'\n' {
        i += 1;
    }

    if i < bytes.len() && bytes[i] == b'\n' {
        let mut owned = String::with_capacity(text.len() - 1);
        owned.push_str(&text[..i]);
        owned.push_str(&text[i + 1..]);
        Cow::Owned(owned)
    } else {
        Cow::Borrowed(text)
    }
}

fn collect_byline_candidate_text(element: ElementRef) -> String {
    let raw_text = build_byline_text(&element);
    if let Some(names) = collect_child_author_names(&element) {
        if should_prefer_child_names(&element, &raw_text, &names) {
            return names.join(", ");
        }
    }
    raw_text
}

static ITEMPROP_NAME_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("[itemprop='name'], [itemprop~='name']").unwrap());

fn collect_child_author_names(element: &ElementRef) -> Option<Vec<String>> {
    static ANCHOR_SELECTOR: LazyLock<Selector> =
        LazyLock::new(|| Selector::parse("a").expect("valid anchor selector"));

    fn push_unique(names: &mut Vec<String>, candidate: String) {
        if !names
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&candidate))
        {
            names.push(candidate);
        }
    }

    let mut names = Vec::new();

    for child in element.select(&ITEMPROP_NAME_SELECTOR) {
        let text = child.text().collect::<String>().trim().to_string();
        if !text.is_empty() {
            push_unique(&mut names, text);
        }
    }

    for anchor in element.select(&ANCHOR_SELECTOR) {
        let text = anchor.text().collect::<String>().trim().to_string();
        if text.is_empty() || text.contains('@') || !utils::looks_like_author_name(&text) {
            continue;
        }

        if let Some(href) = anchor.value().attr("href") {
            let href_lower = href.to_lowercase();
            if href_lower.starts_with("mailto:")
                || href_lower.contains("twitter.com")
                || href_lower.contains("facebook.com")
                || href_lower.contains("linkedin.com")
            {
                continue;
            }
        }

        push_unique(&mut names, text);
    }

    (!names.is_empty()).then_some(names)
}

fn element_has_semantic_name(element: &ElementRef) -> bool {
    if let Some(itemprop) = element.value().attr("itemprop") {
        if itemprop
            .split_whitespace()
            .any(|prop| prop.eq_ignore_ascii_case("name"))
        {
            return true;
        }
    }

    element.select(&ITEMPROP_NAME_SELECTOR).next().is_some()
}

fn should_prefer_child_names(element: &ElementRef, raw_text: &str, names: &[String]) -> bool {
    if names.is_empty() {
        return false;
    }

    const AUTHORISH_CONTEX: [&str; 2] = ["authorinfo", "author-info"];
    if ancestor_has_keyword(element, &AUTHORISH_CONTEX, 4) {
        return true;
    }

    let mut class_id = String::new();
    if let Some(class) = element.value().attr("class") {
        class_id.push_str(class);
    }
    if let Some(id) = element.value().attr("id") {
        if !class_id.is_empty() {
            class_id.push(' ');
        }
        class_id.push_str(id);
    }
    let class_id_lower = class_id.to_lowercase();
    if class_id_lower.contains("authorinfo") || class_id_lower.contains("author-info") {
        return true;
    }
    if let Some(section) = element.value().attr("section") {
        if section.to_lowercase().contains("author") {
            return true;
        }
    }

    let mut normalized = raw_text.to_lowercase();
    for name in names {
        normalized = normalized.replace(&name.to_lowercase(), " ");
    }

    normalized = normalized.replace(['\u{00a0}', '\u{200b}', '\r', '\n'], " ");
    normalized = normalized.replace(['.', ',', '–', '—', '-', '|', ':', ';', '/', '(', ')'], " ");

    let tokens: Vec<_> = normalized
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect();

    let semantic_name = element_has_semantic_name(element);

    if tokens.is_empty() {
        return true;
    }

    if tokens.iter().any(|token| looks_like_job_descriptor(token)) {
        return true;
    }

    if semantic_name && tokens.iter().all(|token| *token == "by") {
        return true;
    }

    false
}

fn looks_like_job_descriptor(token: &str) -> bool {
    const JOB_KEYWORDS: [&str; 19] = [
        "reporter",
        "editor",
        "writer",
        "staff",
        "senior",
        "technologist",
        "correspondent",
        "columnist",
        "analyst",
        "producer",
        "anchor",
        "bureau",
        "desk",
        "spokesman",
        "spokeswoman",
        "spokesperson",
        "contributor",
        "team",
        "author",
    ];
    JOB_KEYWORDS.contains(&token)
}

const MONTH_KEYWORDS: [&str; 24] = [
    "jan",
    "january",
    "feb",
    "february",
    "mar",
    "march",
    "apr",
    "april",
    "may",
    "jun",
    "june",
    "jul",
    "july",
    "aug",
    "august",
    "sep",
    "sept",
    "september",
    "oct",
    "october",
    "nov",
    "november",
    "dec",
    "december",
];

pub(super) fn should_prefer_dom_byline(
    existing: &str,
    dom: &str,
    confidence: DomBylineConfidence,
) -> bool {
    let existing_clean = existing.trim();
    let dom_clean = dom.trim();

    if dom_clean.eq_ignore_ascii_case(existing_clean) {
        return false;
    }

    if utils::looks_like_org_credit(existing_clean) && !utils::looks_like_org_credit(dom_clean) {
        return true;
    }

    if utils::looks_like_dateline(existing_clean) && !utils::looks_like_dateline(dom_clean) {
        return true;
    }

    if confidence == DomBylineConfidence::High
        && looks_like_caps_author(dom_clean)
        && !looks_like_caps_author(existing_clean)
    {
        return true;
    }

    let existing_lower = existing_clean.to_lowercase();
    let dom_lower = dom_clean.to_lowercase();
    let collapse = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let dom_collapsed = collapse(&dom_lower);
    let existing_collapsed = collapse(&existing_lower);

    if !dom_collapsed.contains(&existing_collapsed) {
        return false;
    }

    let mut remainder = if let Some(idx) = dom_lower.find(&existing_lower) {
        let mut rem = String::new();
        rem.push_str(&dom_lower[..idx]);
        rem.push_str(&dom_lower[idx + existing_lower.len()..]);
        rem
    } else {
        dom_lower.clone()
    };

    remainder = remainder.replace(
        [
            '|', '-', '_', ',', '.', '–', '—', '(', ')', '[', ']', '{', '}', '"', '\'',
        ],
        " ",
    );

    let mut tokens: Vec<&str> = remainder
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect();

    if tokens.is_empty() {
        return false;
    }

    tokens.retain(|token| {
        let lower = token.trim();
        if lower.is_empty() {
            return false;
        }
        if lower.chars().all(|ch| ch.is_ascii_digit()) {
            return false;
        }
        if lower == "by" || lower == "updated" || lower == "at" || lower == "am" || lower == "pm" {
            return false;
        }
        !MONTH_KEYWORDS.contains(&lower)
    });

    if tokens.is_empty() {
        return false;
    }

    true
}

pub(super) fn should_prefer_caps_standfirst(existing: &str, candidate: &str) -> bool {
    let existing_clean = existing.trim();
    let candidate_clean = candidate.trim();

    if candidate_clean.eq_ignore_ascii_case(existing_clean) {
        return false;
    }

    if looks_like_caps_author(existing_clean) {
        return false;
    }

    looks_like_caps_author(candidate_clean)
}

fn looks_like_caps_author(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || !trimmed.chars().any(|c| c.is_whitespace()) {
        return false;
    }

    let letters: Vec<char> = trimmed.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() < 3 {
        return false;
    }

    if contains_caps_noise_token(trimmed) {
        return false;
    }

    let uppercase = letters.iter().filter(|c| c.is_uppercase()).count();
    uppercase * 10 >= letters.len() * 8
}

fn contains_caps_noise_token(text: &str) -> bool {
    const NOISE_TOKENS: [&str; 13] = [
        "views", "view", "votes", "vote", "post", "posts", "yes", "no", "hot", "stats", "trending",
        "share", "sections",
    ];

    text.split_whitespace().any(|token| {
        let cleaned = token
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        !cleaned.is_empty() && NOISE_TOKENS.contains(&cleaned.as_str())
    })
}

fn parent_byline_text(element: &ElementRef) -> Option<String> {
    let parent_node = element.parent()?;
    let parent = ElementRef::wrap(parent_node)?;
    if is_ignorable_byline_context(&parent) {
        return None;
    }
    if is_noise_byline_context(&parent) {
        return None;
    }
    if !element_has_byline_keyword(&parent) {
        return None;
    }
    let text = collect_byline_candidate_text(parent).trim().to_string();
    match utils::clean_byline_text_with_reason(&text) {
        utils::CleanBylineOutcome::Accepted(cleaned) => Some(cleaned),
        utils::CleanBylineOutcome::DroppedOrgCredit | utils::CleanBylineOutcome::Dropped => None,
    }
}

fn element_has_byline_keyword(element: &ElementRef) -> bool {
    let class = element.value().attr("class").unwrap_or("").to_lowercase();
    let id = element.value().attr("id").unwrap_or("").to_lowercase();

    class.contains("byline")
        || class.contains("author")
        || class.contains("writer")
        || class.contains("credit")
        || id.contains("byline")
        || id.contains("author")
        || id.contains("writer")
        || id.contains("credit")
}

fn element_has_explicit_byline_marker(element: &ElementRef) -> bool {
    let class = element.value().attr("class").unwrap_or("").to_lowercase();
    let id = element.value().attr("id").unwrap_or("").to_lowercase();
    class.contains("byline") || id.contains("byline")
}

fn is_priority_dom_candidate(candidate: &DomBylineCandidate, raw_caps: bool) -> bool {
    raw_caps || utils::looks_like_byline(&candidate.text)
}

fn ancestor_has_keyword(element: &ElementRef, keywords: &[&str], max_depth: usize) -> bool {
    let mut depth = 0;
    let mut current = Some(*element);

    while let Some(el) = current {
        let class = el.value().attr("class").unwrap_or("").to_lowercase();
        let id = el.value().attr("id").unwrap_or("").to_lowercase();
        if keywords
            .iter()
            .any(|keyword| class.contains(keyword) || id.contains(keyword))
        {
            return true;
        }

        if depth >= max_depth {
            break;
        }
        depth += 1;
        current = el.parent().and_then(ElementRef::wrap);
    }

    false
}

fn is_ignorable_byline_context(element: &ElementRef) -> bool {
    const KEYWORDS: [&str; 34] = [
        "post-footer",
        "entry-footer",
        "article-footer",
        "section-footer",
        "postmeta",
        "meta-footer",
        "footer",
        "profile",
        "sidebar",
        "widget",
        "comment",
        "bio",
        "related-post",
        "user-bylines",
        "byline__body",
        "byline__title",
        "post-info",
        "entry-byline",
        "entry-author",
        "assetauthor",
        "contentpromo",
        "promo",
        "asset-author",
        "videopromo",
        "poponscroll",
        "most-popular",
        "popular-stories",
        "videoslide",
        "video-container",
        "card-box",
        "article-view-box",
        "cardbox",
        "article-content",
        "story-info",
    ];
    ancestor_has_keyword(element, &KEYWORDS, 16)
}

fn is_noise_byline_context(element: &ElementRef) -> bool {
    const KEYWORDS: [&str; 27] = [
        "videopromo",
        "videoslide",
        "video-slide",
        "video-module",
        "poponscroll",
        "contentpromo",
        "promo",
        "popular",
        "most-popular",
        "popular-stories",
        "more-stories",
        "related",
        "recirc",
        "recommend",
        "newsletter",
        "signup",
        "asset",
        "social",
        "share",
        "gallery",
        "slideshow",
        "indepth",
        "indepth-module",
        "hot_stats",
        "hot-stats",
        "trending-badge",
        "views",
    ];
    ancestor_has_keyword(element, &KEYWORDS, 16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::{get_article_metadata, Metadata};
    use std::fs;

    #[test]
    fn test_byline_extraction_from_document() {
        let html = r#"
            <html>
                <body>
                    <article>
                        <a rel="author" href="/author/john">John Doe</a>
                        <p>Article content here</p>
                    </article>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let json_ld = Metadata::default();
        let metadata = get_article_metadata(&document, json_ld);

        assert_eq!(metadata.byline, Some("John Doe".to_string()));
    }

    #[test]
    fn test_byline_extraction_from_class() {
        let html = r#"
            <html>
                <body>
                    <article>
                        <p class="byline">By Jane Smith</p>
                        <p>Article content here</p>
                    </article>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let json_ld = Metadata::default();
        let metadata = get_article_metadata(&document, json_ld);

        assert!(metadata.byline.is_some());
        assert!(metadata.byline.as_ref().unwrap().contains("Jane Smith"));
    }

    #[test]
    fn test_byline_extraction_priority() {
        let html = r#"
            <html>
                <head>
                    <meta name="author" content="Meta Author" />
                </head>
                <body>
                    <article>
                        <p class="byline">Document Author</p>
                    </article>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let json_ld = Metadata::default();
        let metadata = get_article_metadata(&document, json_ld);

        assert_eq!(metadata.byline, Some("Meta Author".to_string()));
    }

    #[test]
    fn test_ignorable_byline_context_detects_footer() {
        let html = r#"
            <div class="post-footer">
                <div class="post-footer-line">
                    <span class="post-author">Posted by <span itemprop="name">Jane Doe</span></span>
                </div>
            </div>
        "#;
        let fragment = Html::parse_fragment(html);
        let selector = Selector::parse(".post-author").unwrap();
        let elem = fragment.select(&selector).next().unwrap();
        assert!(is_ignorable_byline_context(&elem));
    }

    #[test]
    fn test_ignorable_byline_context_detects_profile_widget() {
        let html = r#"
            <div class="profile widget">
                <a rel="author" href="/user/jane">Jane Doe</a>
            </div>
        "#;
        let fragment = Html::parse_fragment(html);
        let selector = Selector::parse("a[rel='author']").unwrap();
        let elem = fragment.select(&selector).next().unwrap();
        assert!(is_ignorable_byline_context(&elem));
    }

    #[test]
    fn test_ignorable_byline_context_detects_byline_body_block() {
        let html = r#"
            <div class="user-bylines">
                <div class="byline__body">
                    <a class="byline__author">Jane Doe</a>
                    <div class="byline__title">BuzzFeed News Reporter</div>
                </div>
            </div>
        "#;
        let fragment = Html::parse_fragment(html);
        let selector = Selector::parse(".byline__author").unwrap();
        let elem = fragment.select(&selector).next().unwrap();
        assert!(is_ignorable_byline_context(&elem));
    }

    #[test]
    fn test_user_bylines_block_is_ignored_during_extraction() {
        let html = r#"
            <html>
                <body>
                    <header class="page-head">
                        <div class="user-bylines">
                            <div class="byline__body">
                                <a class="byline__author">Jane Doe</a>
                                <div class="byline__title">BuzzFeed News Reporter</div>
                            </div>
                        </div>
                    </header>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let json_ld = Metadata::default();
        let metadata = get_article_metadata(&document, json_ld);

        assert!(metadata.byline.is_none());
    }

    #[test]
    fn test_article_author_class_outside_footer_is_respected() {
        let html = r#"
            <html>
                <body>
                    <article>
                        <aside>
                            <p>
                                <span class="article-author" itemprop="author" itemscope itemtype="http://schema.org/Person">
                                    <span itemprop="name">Nicolas Perriault</span>
                                </span>
                            </p>
                        </aside>
                    </article>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let metadata = get_article_metadata(&document, Metadata::default());

        assert_eq!(metadata.byline, Some("Nicolas Perriault".to_string()));
    }

    #[test]
    fn test_site_name_redundant_byline_is_removed() {
        let html = r#"
            <html>
                <head>
                    <meta property="og:site_name" content="SIMPLYFOUND.COM | BY: Joe Wee"/>
                </head>
                <body>
                    <article>
                        <p class="byline">
                            <span itemprop="author" itemscope itemtype="http://schema.org/Person">
                                <span itemprop="name">Joe Wee</span>
                            </span>
                        </p>
                    </article>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let metadata = get_article_metadata(&document, Metadata::default());

        assert!(metadata.byline.is_none());
    }

    #[test]
    fn test_breitbart_byline_is_extracted() {
        let html = fs::read_to_string("tests/test-pages/breitbart/source.html").unwrap();
        let document = Html::parse_document(&html);
        let selector = Selector::parse(".byline").unwrap();
        let mut saw_lucas = false;
        for elem in document.select(&selector) {
            if is_ignorable_byline_context(&elem) || is_noise_byline_context(&elem) {
                continue;
            }
            let text = collect_byline_candidate_text(elem).trim().to_string();
            if text.contains("Lucas Nolan") {
                saw_lucas = true;
                break;
            }
        }
        assert!(saw_lucas, "expected to find Lucas Nolan byline candidate");

        let dom_byline = extract_byline_from_document(&document);
        assert!(
            dom_byline.is_some(),
            "expected Breitbart byline to be detected"
        );
    }

    #[test]
    fn test_cnet_authorinfo_is_extracted() {
        let html = fs::read_to_string("tests/test-pages/cnet/source.html").unwrap();
        let document = Html::parse_document(&html);
        let dom_byline = extract_byline_from_document(&document).map(|c| c.text);
        assert_eq!(dom_byline, Some("Steven Musil".to_string()));
    }

    #[test]
    fn test_herald_sun_caps_byline_overrides_meta() {
        let html = fs::read_to_string("tests/test-pages/herald-sun-1/source.html").unwrap();
        let document = Html::parse_document(&html);
        let dom_byline = extract_byline_from_document(&document).expect("dom byline");
        assert_eq!(dom_byline.text, "JOE HILDEBRAND");
        assert_eq!(dom_byline.confidence, DomBylineConfidence::High);
        assert!(
            should_prefer_dom_byline("by: Laurie Oakes", &dom_byline.text, dom_byline.confidence),
            "dom byline should override Laurie Oakes"
        );
        let metadata = get_article_metadata(&document, Metadata::default());
        assert_eq!(metadata.byline, Some("JOE HILDEBRAND".to_string()));
    }

    #[test]
    fn test_caps_author_detection() {
        assert!(looks_like_caps_author("JOE HILDEBRAND"));
        assert!(!looks_like_caps_author("Laurie Oakes"));
        assert!(!looks_like_caps_author("TOP POST 653,817 VIEWS"));
    }

    #[test]
    fn test_dom_byline_overrides_agency_credit() {
        let html = r#"
            <html>
                <head>
                    <meta property="og:title" content="Titre" />
                    <meta name="author" content="AFP" />
                </head>
                <body>
                    <article>
                        <p class="byline">Par <span>Sébastien Farcis</span></p>
                        <p>Contenu principal</p>
                    </article>
                </body>
            </html>
        "#;
        let document = Html::parse_document(html);
        let metadata = get_article_metadata(&document, Metadata::default());
        assert_eq!(metadata.byline, Some("Par Sébastien Farcis".to_string()));
    }

    #[test]
    fn test_dom_byline_overrides_dateline_meta() {
        let html = r#"
            <html>
                <head>
                    <meta property="og:title" content="Titre" />
                    <meta name="author" content="CAIRO" />
                </head>
                <body>
                    <article>
                        <p class="byline">By <span>Erin Cunningham</span></p>
                        <p>Contenu principal</p>
                    </article>
                </body>
            </html>
        "#;
        let document = Html::parse_document(html);
        let metadata = get_article_metadata(&document, Metadata::default());
        assert_eq!(metadata.byline, Some("By Erin Cunningham".to_string()));
    }

    #[test]
    fn test_wapo_byline_is_detected() {
        let html = fs::read_to_string("tests/test-pages/wapo-1/source.html").unwrap();
        let document = Html::parse_document(&html);
        let selector = Selector::parse(".pb-byline").unwrap();
        assert!(
            document.select(&selector).next().is_some(),
            "pb-byline element not found"
        );
        let elem = document.select(&selector).next().unwrap();
        let text = collect_byline_candidate_text(elem);
        assert!(
            text.contains("Erin Cunningham"),
            "pb-byline text was {:?}",
            text
        );
        let dom_byline = extract_byline_from_document(&document).expect("should detect DOM byline");
        assert_eq!(dom_byline.text, "By Erin Cunningham");
    }

    /// `build_byline_text` recurses per nesting level; without the depth bound
    /// a hostile byline container overflows the stack.
    #[test]
    fn test_deeply_nested_byline_does_not_overflow() {
        let mut html = "<span>".repeat(2_000);
        html.push_str("Jane Doe");
        html.push_str(&"</span>".repeat(2_000));

        let document = Html::parse_fragment(&html);
        let selector = Selector::parse("span").unwrap();
        let outermost = document.select(&selector).next().unwrap();

        let byline = build_byline_text(&outermost);

        assert!(byline.len() < html.len());
    }

    #[test]
    fn test_moderate_nesting_keeps_byline_text() {
        let html = format!("{}Jane Doe{}", "<span>".repeat(20), "</span>".repeat(20));

        let document = Html::parse_fragment(&html);
        let selector = Selector::parse("span").unwrap();
        let outermost = document.select(&selector).next().unwrap();

        assert_eq!(build_byline_text(&outermost).trim(), "Jane Doe");
    }
}
