//! Utility functions for text processing and manipulation.

use crate::constants::REGEXPS;
use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

/// Unescape basic and numeric HTML entities in a string.
pub fn unescape_html_entities(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if text.as_bytes()[i] == b'&' {
            if let Some(semi_offset) = text[i..].find(';') {
                let end = i + semi_offset + 1;
                let entity = &text[i..end];
                if let Some(decoded) = decode_html_entity(entity) {
                    result.push_str(&decoded);
                    i = end;
                    continue;
                }
            }
        }

        if let Some(ch) = text[i..].chars().next() {
            result.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    result
}

fn decode_html_entity(entity: &str) -> Option<String> {
    match entity {
        "&lt;" => Some("<".to_string()),
        "&gt;" => Some(">".to_string()),
        "&amp;" => Some("&".to_string()),
        "&quot;" => Some("\"".to_string()),
        "&apos;" | "&#39;" => Some("'".to_string()),
        _ => {
            if entity.starts_with("&#x") || entity.starts_with("&#X") {
                let hex = entity.get(3..entity.len() - 1)?;
                u32::from_str_radix(hex, 16)
                    .ok()
                    .and_then(std::char::from_u32)
                    .map(|c| c.to_string())
            } else if entity.starts_with("&#") && entity.ends_with(';') {
                let digits = entity.get(2..entity.len() - 1)?;
                digits
                    .parse::<u32>()
                    .ok()
                    .and_then(std::char::from_u32)
                    .map(|c| c.to_string())
            } else {
                None
            }
        }
    }
}

/// Normalize whitespace in a string
pub fn normalize_whitespace(text: &str) -> String {
    REGEXPS.normalize.replace_all(text, " ").to_string()
}

/// Check if a string is a valid URL
pub fn is_url(s: &str) -> bool {
    url::Url::parse(s).is_ok()
}

static BY_PREFIX_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(by|par)[\s:,\-–—]+").unwrap());

static SOFT_SPACE_CHARS: &[char] = &['\u{00a0}', '\u{200b}', '\u{feff}'];

/// Returns true if the provided text looks like a byline ("By <name> ...").
pub fn looks_like_byline(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    if !BY_PREFIX_REGEX.is_match(trimmed) {
        return false;
    }

    let remainder = BY_PREFIX_REGEX.replace(trimmed, "");
    let remainder = remainder.trim_start();
    match remainder.chars().next() {
        Some(ch) => ch.is_uppercase(),
        None => false,
    }
}

/// Remove invisible space characters that frequently wrap metadata text.
fn trim_soft_space(text: &str) -> &str {
    text.trim_matches(|c| SOFT_SPACE_CHARS.contains(&c))
}

fn looks_like_social_handle(text: &str) -> bool {
    let normalized = text.trim().to_lowercase();
    if normalized.is_empty() {
        return false;
    }

    if normalized.starts_with('@') || normalized.contains(" @") {
        return true;
    }

    if normalized.contains("twitter.com/") || normalized.contains("facebook.com/") {
        return true;
    }

    if normalized.starts_with("follow ") && normalized.contains('@') {
        return true;
    }

    if normalized.contains(" follow @") {
        return true;
    }

    if normalized.contains(" follow us") && normalized.contains("twitter") {
        return true;
    }

    if normalized.contains(" follow on") && normalized.contains("twitter") {
        return true;
    }

    false
}

/// Heuristic check for anchor text that looks like a personal name.
pub fn looks_like_author_name(text: &str) -> bool {
    let trimmed = trim_soft_space(text.trim());
    if trimmed.is_empty() || trimmed.len() > 80 {
        return false;
    }

    if !trimmed.chars().any(char::is_whitespace) {
        return false;
    }

    if trimmed.chars().any(|ch| ch.is_ascii_digit()) {
        return false;
    }

    let lower = trimmed.to_lowercase();
    if lower.starts_with("follow ") || lower.contains('@') {
        return false;
    }

    let letter_count = trimmed.chars().filter(|ch| ch.is_alphabetic()).count();
    if letter_count < 3 {
        return false;
    }

    const DISQUALIFIERS: [&str; 24] = [
        "reporter",
        "editor",
        "writer",
        "staff",
        "senior",
        "team",
        "desk",
        "anchor",
        "producer",
        "analyst",
        "correspondent",
        "contributor",
        "technologist",
        "developer",
        "developers",
        "news",
        "press",
        "service",
        "bureau",
        "foreign",
        "android",
        "buzzfeed",
        "telegraph",
        "view",
    ];

    !lower
        .split_whitespace()
        .any(|token| DISQUALIFIERS.contains(&token))
}

fn contains_author_like_segment(text: &str) -> bool {
    if looks_like_author_name(text) {
        return true;
    }

    for segment in split_candidate_segments(text) {
        if looks_like_author_name(segment) {
            return true;
        }
    }

    false
}

fn split_candidate_segments(text: &str) -> Vec<&str> {
    let mut segments = Vec::new();

    for line in text.split('\n') {
        segments.push(line);
        for delim in ['|', '/', '•', '·'] {
            if line.contains(delim) {
                segments.extend(line.split(delim));
            }
        }
        for separator in [" - ", " – ", " — "] {
            if line.contains(separator) {
                segments.extend(line.split(separator));
            }
        }
    }

    segments
}

fn looks_like_datetime_segment(segment: &str) -> bool {
    let lower = segment.trim().to_lowercase();
    if lower.is_empty() {
        return false;
    }

    let has_digit = lower.chars().any(|c| c.is_ascii_digit());
    let mentions_month = [
        "jan",
        "feb",
        "mar",
        "apr",
        "may",
        "jun",
        "jul",
        "aug",
        "sep",
        "sept",
        "oct",
        "nov",
        "dec",
        "january",
        "february",
        "march",
        "april",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ]
    .iter()
    .any(|month| lower.contains(month));

    if lower.contains("ago")
        || lower.contains("updated")
        || lower.contains("yesterday")
        || lower.contains("today")
        || (has_digit
            && (lower.contains("am")
                || lower.contains("pm")
                || lower.contains("utc")
                || lower.contains("gmt")
                || lower.contains("est")
                || lower.contains("pst")
                || lower.contains("cet")))
        || (has_digit && mentions_month)
    {
        return true;
    }

    if has_digit && lower.contains(':') {
        return true;
    }

    false
}

fn strip_trailing_datetime_clause<'a>(text: &'a str, allow_strip: bool) -> Cow<'a, str> {
    if !allow_strip {
        return Cow::Borrowed(text);
    }

    for separator in [" | ", " - ", " – ", " — ", " · "] {
        if let Some(idx) = text.rfind(separator) {
            let tail = text[idx + separator.len()..].trim();
            if looks_like_datetime_segment(tail) {
                return Cow::Owned(text[..idx].trim_end().to_string());
            }
        }
    }

    Cow::Borrowed(text)
}

fn remove_timestamp_lines(text: &str) -> Option<String> {
    let mut changed = false;
    let mut kept = Vec::new();

    for line in text.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() || !looks_like_live_timestamp_segment(trimmed) {
            kept.push(line);
            continue;
        }

        changed = true;
    }

    if !changed {
        None
    } else {
        Some(kept.join("\n").trim_end_matches('\n').to_string())
    }
}

fn looks_like_live_timestamp_segment(segment: &str) -> bool {
    let lower = segment.trim().to_lowercase();
    if lower.is_empty() {
        return false;
    }

    // Match relative/dynamic timestamps like "1 day ago", "updated", etc.
    if lower.contains("ago")
        || lower.contains("updated")
        || lower.contains("update")
        || lower.contains("yesterday")
        || lower.contains("today")
    {
        return true;
    }

    // Check if this is an absolute date (has a month name)
    // Absolute dates like "March 11, 2015 3:46 PM" should be kept, not removed
    let has_month = [
        "jan",
        "feb",
        "mar",
        "apr",
        "may",
        "jun",
        "jul",
        "aug",
        "sep",
        "sept",
        "oct",
        "nov",
        "dec",
        "january",
        "february",
        "march",
        "april",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ]
    .iter()
    .any(|month| lower.contains(month));

    if has_month {
        return false;
    }

    let has_digit = lower.chars().any(|c| c.is_ascii_digit());
    let has_time_sep = lower.contains(':');

    if has_time_sep && has_digit {
        return true;
    }

    // Match time-only indicators like "3 PM" or "14:30 UTC" (without dates)
    if has_digit
        && (lower.contains("am")
            || lower.contains("pm")
            || lower.contains("a.m")
            || lower.contains("p.m")
            || lower.contains("utc")
            || lower.contains("gmt")
            || lower.contains("est")
            || lower.contains("pst")
            || lower.contains("cet"))
    {
        return true;
    }

    false
}

pub(crate) fn looks_like_org_credit(text: &str) -> bool {
    if contains_author_like_segment(text) {
        return false;
    }

    let normalized = normalize_whitespace(text).to_lowercase();
    if normalized.is_empty() {
        return false;
    }

    const EXACT_AGENCIES: [&str; 10] = [
        "afp",
        "ap",
        "associated press",
        "reuters",
        "bloomberg",
        "press association",
        "kyodo",
        "ansa",
        "dpa",
        "upi",
    ];

    if EXACT_AGENCIES.contains(&normalized.as_str()) {
        return true;
    }

    let keywords = [
        "staff",
        "news",
        "newsroom",
        "desk",
        "team",
        "press",
        "service",
        "bureau",
        "foreign",
        "reporter",
        "reporters",
        "developers",
        "android",
        "buzzfeed",
        "wire",
        "agency",
        "agencies",
        "telegraph",
        "our",
        "editors",
        "view",
    ];

    let hits = normalized
        .split_whitespace()
        .filter(|token| keywords.contains(token))
        .count();

    hits >= 2
}

pub fn looks_like_bracket_menu(text: &str) -> bool {
    let mut remainder = text.trim();
    if !remainder.starts_with('[') {
        return false;
    }

    let mut matched = 0;
    while remainder.starts_with('[') {
        if let Some(end) = remainder.find(']') {
            let token = remainder[1..end].trim();
            if token.is_empty() {
                return false;
            }
            matched += 1;
            remainder = remainder[end + 1..].trim_start();
        } else {
            break;
        }
    }

    if matched < 2 {
        return false;
    }

    let remainder_trimmed = remainder.trim();
    remainder_trimmed.is_empty()
        || remainder_trimmed.starts_with("Versions")
        || remainder_trimmed
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_whitespace())
}

pub(crate) fn looks_like_dateline(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.len() > 40 {
        return false;
    }

    let stripped = trimmed
        .trim_start_matches(['-', '–', '—'])
        .trim_end_matches(['-', '–', '—']);
    if stripped.is_empty() {
        return false;
    }

    let mut has_letters = false;
    for word in stripped.split(|c: char| c.is_whitespace() || c == ',' || c == '—' || c == '-') {
        let word = word.trim_matches(|c: char| !c.is_alphanumeric());
        if word.is_empty() {
            continue;
        }
        if word.chars().any(|c| c.is_lowercase()) {
            return false;
        }
        if word.chars().any(|c| c.is_alphabetic()) {
            has_letters = true;
        }
    }

    has_letters
}

/// Check if text looks like a navigation menu (multiple pipes, location pairs, etc.)
fn looks_like_navigation_menu(text: &str) -> bool {
    let pipe_count = text.chars().filter(|&c| c == '|').count();
    if pipe_count >= 2 {
        return true;
    }

    // Check for location-pair pattern (e.g., "HOLLYWOOD\nNEW YORK")
    // Two or more all-caps short phrases that look like location names
    // Split by newline first (before normalizing whitespace)
    let lines: Vec<_> = text
        .split('\n')
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() >= 2 {
        let all_look_like_locations = lines.iter().all(|line| {
            let stripped = line.trim();
            if stripped.is_empty() || stripped.len() > 30 || stripped.len() < 3 {
                return false;
            }

            let words: Vec<&str> = stripped.split_whitespace().collect();
            if words.is_empty() || words.len() > 3 {
                return false;
            }

            for word in words {
                let letters: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
                if letters.is_empty() {
                    return false;
                }
                if !letters.iter().all(|c| c.is_uppercase()) {
                    return false;
                }
            }

            let lower = stripped.to_lowercase();
            if lower.contains("by") || lower.contains("staff") || lower.contains("editor") {
                return false;
            }

            true
        });

        if all_look_like_locations && lines.len() >= 2 {
            return true;
        }
    }

    false
}

/// Normalize byline text by trimming whitespace and removing dangling separators.
pub enum CleanBylineOutcome {
    Accepted(String),
    DroppedOrgCredit,
    Dropped,
}

pub fn clean_byline_text_with_reason(text: &str) -> CleanBylineOutcome {
    let trimmed = trim_soft_space(text.trim());
    if trimmed.is_empty() {
        return CleanBylineOutcome::Dropped;
    }

    // Remove trailing separators/dashes that often wrap author credits.
    let cleaned = trimmed
        .trim_end_matches(|c: char| c.is_whitespace())
        .trim_end_matches(|c: char| {
            matches!(c, '-' | '–' | '—' | '|' | '•' | ':' | ';' | ',' | '.')
        })
        .trim();

    if cleaned.is_empty() {
        return CleanBylineOutcome::Dropped;
    }

    let mut canonical = cleaned.replace("\r\n", "\n");
    canonical = collapse_blank_lines_preserve_indent(&canonical);

    let has_author_segment = contains_author_like_segment(&canonical);
    canonical = strip_trailing_datetime_clause(&canonical, has_author_segment).into_owned();

    if has_author_segment {
        if let Some(filtered) = remove_timestamp_lines(&canonical) {
            if filtered.trim().is_empty() {
                return CleanBylineOutcome::Dropped;
            }
            canonical = filtered;
        }
    }

    if let Some(filtered) = remove_social_handle_lines(&canonical) {
        if filtered.trim().is_empty() {
            return CleanBylineOutcome::Dropped;
        }
        canonical = filtered;
    }

    let trimmed_lower = canonical.trim_start().to_lowercase();
    if trimmed_lower.starts_with("posted by") || trimmed_lower.starts_with("promoted by") {
        return CleanBylineOutcome::DroppedOrgCredit;
    }

    if looks_like_navigation_menu(&canonical) {
        return CleanBylineOutcome::Dropped;
    }

    let normalized = normalize_whitespace(&canonical);
    if normalized.is_empty() {
        return CleanBylineOutcome::Dropped;
    }

    if looks_like_social_handle(&normalized) {
        return CleanBylineOutcome::Dropped;
    }

    if !normalized.chars().any(|c| c.is_alphabetic()) {
        return CleanBylineOutcome::Dropped;
    }

    if looks_like_org_credit(&canonical) {
        return CleanBylineOutcome::DroppedOrgCredit;
    }

    CleanBylineOutcome::Accepted(canonical)
}

pub fn clean_byline_text(text: &str) -> Option<String> {
    match clean_byline_text_with_reason(text) {
        CleanBylineOutcome::Accepted(value) => Some(value),
        _ => None,
    }
}

pub fn is_byline_redundant_with_site_name(byline: &str, site_name: &str) -> bool {
    let normalized_byline = normalize_whitespace(byline).to_lowercase();
    if normalized_byline.len() < 3 {
        return false;
    }

    let normalized_site = normalize_whitespace(site_name).to_lowercase();
    if let Some(pos) = normalized_site.find(&normalized_byline) {
        let prefix = normalized_site[..pos].trim_end_matches(|c: char| {
            c.is_whitespace() || matches!(c, ':' | '-' | '–' | '—' | '|' | '•')
        });
        if prefix.ends_with("by") {
            return true;
        }

        let suffix =
            normalized_site[pos + normalized_byline.len()..].trim_start_matches(|c: char| {
                c.is_whitespace() || matches!(c, ':' | '-' | '–' | '—' | '|' | '•')
            });
        if suffix.starts_with("by") {
            return true;
        }
    }

    false
}

fn collapse_blank_lines_preserve_indent(text: &str) -> String {
    let mut result = String::new();
    let mut pending_indent: Option<String> = None;
    let mut first_line_written = false;

    for line in text.split('\n') {
        if line.trim().is_empty() {
            if pending_indent.is_none() && !line.is_empty() {
                pending_indent = Some(line.to_string());
            }
            continue;
        }

        if first_line_written {
            result.push('\n');
        }
        if let Some(indent) = pending_indent.take() {
            result.push_str(&indent);
        }

        result.push_str(line);
        first_line_written = true;
    }

    result
}

fn remove_social_handle_lines(text: &str) -> Option<String> {
    let mut changed = false;
    let mut kept = Vec::new();

    for line in text.split('\n') {
        if looks_like_social_handle(line) {
            changed = true;
            continue;
        }
        kept.push(line);
    }

    if !changed {
        None
    } else {
        Some(kept.join("\n").trim_end_matches('\n').to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unescape_html_entities() {
        assert_eq!(unescape_html_entities("&lt;div&gt;"), "<div>");
        assert_eq!(unescape_html_entities("A &amp; B"), "A & B");
    }

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(normalize_whitespace("hello   world"), "hello world");
        assert_eq!(normalize_whitespace("a  b  c"), "a b c");
    }

    #[test]
    fn test_looks_like_byline() {
        assert!(looks_like_byline("By Alice Smith"));
        assert!(looks_like_byline("BY: Bob Jones"));
        assert!(!looks_like_byline("Alice Smith"));
        assert!(!looks_like_byline("By clicking \"Submit\""));
    }

    #[test]
    fn test_clean_byline_text_trims_delimiters() {
        let cleaned = clean_byline_text("Nicolas Perriault — ").unwrap();
        assert_eq!(cleaned, "Nicolas Perriault");
    }

    #[test]
    fn test_clean_byline_text_rejects_social_follow() {
        assert!(clean_byline_text("Follow @example").is_none());
        assert!(clean_byline_text("@example on Twitter").is_none());
    }

    #[test]
    fn test_clean_byline_text_collapses_blank_line_but_keeps_indent() {
        let input = "By Brenda  Goodman, MA\n            \nWebMD Health News";
        let expected = "By Brenda  Goodman, MA\n            WebMD Health News";
        assert_eq!(clean_byline_text(input).unwrap(), expected);
    }

    #[test]
    fn test_clean_byline_text_strips_trailing_timestamp() {
        let input = "Dan Goodin - Apr 16, 2015 8:02 pm UTC";
        assert_eq!(clean_byline_text(input).unwrap(), "Dan Goodin");
    }

    #[test]
    fn test_clean_byline_text_drops_relative_time_line() {
        let input = "Alex Perry\n                                                1 day ago";
        assert_eq!(clean_byline_text(input).unwrap(), "Alex Perry");
    }

    #[test]
    fn test_clean_byline_text_keeps_timestamp_without_author() {
        let input = "April 28, 2019 at 6:01 am Updated April 29, 2019 at 3:33 pm";
        assert_eq!(
            clean_byline_text(input).unwrap(),
            "April 28, 2019 at 6:01 am Updated April 29, 2019 at 3:33 pm"
        );
    }

    #[test]
    fn test_clean_byline_text_preserves_name_with_plain_date() {
        let input = "By Nathan Willis\nMarch 25, 2015";
        assert_eq!(clean_byline_text(input).unwrap(), input);
    }

    #[test]
    fn test_clean_byline_text_drops_org_credit() {
        assert!(clean_byline_text("Our Foreign Staff").is_none());
        assert!(clean_byline_text("BuzzFeed News Reporter").is_none());
        assert!(clean_byline_text("Android Developers").is_none());
    }

    #[test]
    fn test_looks_like_author_name() {
        assert!(looks_like_author_name("Daniel Kahn Gillmor"));
        assert!(looks_like_author_name("R.J. Eskow"));
        assert!(!looks_like_author_name("BuzzFeed News Reporter"));
        assert!(!looks_like_author_name("Follow @example"));
        assert!(!looks_like_author_name("SingleWord"));
    }

    #[test]
    fn test_is_byline_redundant_with_site_name_rejects_duplicate() {
        assert!(is_byline_redundant_with_site_name(
            "Joe Wee",
            "SIMPLYFOUND.COM | BY: Joe Wee"
        ));
    }

    #[test]
    fn test_is_byline_redundant_with_site_name_keeps_unique() {
        assert!(!is_byline_redundant_with_site_name(
            "Nicolas Perriault",
            "Code"
        ));
    }

    #[test]
    fn test_clean_byline_text_handles_inline_date_and_count() {
        let input = "by Lucas Nolan22 Dec 2016651";
        let cleaned = clean_byline_text(input).expect("byline should be kept");
        assert!(cleaned.contains("Lucas Nolan"));
    }

    #[test]
    fn test_clean_byline_text_strips_social_handle_lines() {
        let input = "By John Smith\n@johnsmith\nJanuary 1, 2020";
        let cleaned = clean_byline_text(input).expect("byline should be kept");
        assert_eq!(cleaned, "By John Smith\nJanuary 1, 2020");
    }

    #[test]
    fn test_looks_like_dateline_detection() {
        assert!(looks_like_dateline("CAIRO"));
        assert!(looks_like_dateline("PARIS —"));
        assert!(!looks_like_dateline("By Erin Cunningham"));
        assert!(!looks_like_dateline("Washington Post Staff"));
    }

    #[test]
    fn test_strip_trailing_datetime_clause_does_not_panic_on_unicode_length_shift() {
        // "İ" (U+0130) lowercases to a 3-byte sequence from a 2-byte original,
        // so a byte index computed on the lowercased string can land mid-character
        // when applied to the original string. This input previously panicked.
        let input = "İİİİ | Ş12:30, 5 May 2024";
        assert_eq!(strip_trailing_datetime_clause(input, true), "İİİİ");
    }

    #[test]
    fn test_strip_trailing_datetime_clause_no_strip_on_unicode_leaves_input_unchanged() {
        // Lowercased length differs from the original here too, but the tail does not
        // look like a datetime, so no strip should occur and the call must not panic.
        let input = "İstanbul Correspondent | Senior Editor";
        assert_eq!(strip_trailing_datetime_clause(input, true), input);
    }

    #[test]
    fn test_clean_byline_text_strips_trailing_datetime_with_turkish_name() {
        let input = "DİLARA ŞENKAYA | 12:30, 5 May 2024";
        assert_eq!(clean_byline_text(input).unwrap(), "DİLARA ŞENKAYA");
    }

    #[test]
    fn test_clean_byline_text_strips_trailing_datetime_ascii_pipe_separator() {
        let input = "Jane Doe | 08:15, 3 June 2024";
        assert_eq!(clean_byline_text(input).unwrap(), "Jane Doe");
    }
}
