use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};

use super::languages::{is_known_language, normalize_language};

static LANGUAGE_CLASS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^language-(.+)$").unwrap());
static LANG_CLASS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^lang-(.+)$").unwrap());
static HIGHLIGHT_SOURCE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^highlight-source-(.+)$").unwrap());
static BRUSH_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)brush:\s*(\w+)").unwrap());
static LINE_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^\s*\d+[\s|]").unwrap());
static MULTI_NEWLINE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\n{3,}").unwrap());

/// Standardize all code blocks in the HTML to canonical `<pre><code class="language-x">` form.
///
/// Single-pass: parse HTML once, collect all replacements, apply them.
pub fn standardize_code_blocks(html: &str) -> String {
    let doc = Html::parse_fragment(html);
    let mut output = html.to_string();

    // Collect (original_html, replacement) pairs from a single parse
    let mut replacements: Vec<(String, String)> = Vec::new();

    // 1. rehype-pretty-code figures
    if let Ok(sel) = Selector::parse("figure[data-rehype-pretty-code-figure]") {
        for el in doc.select(&sel) {
            if let Some(canonical) = standardize_rehype_figure(&el) {
                replacements.push((el.html(), canonical));
            }
        }
    }

    // 2. GitHub-style highlight divs
    if let Ok(sel) = Selector::parse("div.highlight") {
        for el in doc.select(&sel) {
            let class_attr = el.value().attr("class").unwrap_or("");
            if let Some(lang) = extract_github_language(class_attr) {
                if let Some(code) = extract_pre_text(&el) {
                    let cleaned = clean_code_content(&code);
                    replacements.push((el.html(), format_canonical_code_block(&lang, &cleaned)));
                }
            }
        }
    }

    // 3. Line-number tables
    if let Ok(sel) = Selector::parse("table.highlight-table, table.rouge-table, table.code-listing") {
        for el in doc.select(&sel) {
            if let Some((lang, code)) = extract_table_code(&el) {
                let cleaned = clean_code_content(&code);
                replacements.push((el.html(), format_canonical_code_block(&lang, &cleaned)));
            }
        }
    }

    // 4. Shiki blocks
    if let Ok(sel) = Selector::parse("pre.shiki") {
        for el in doc.select(&sel) {
            let lang = detect_language_from_element(&el);
            if let Some(code) = extract_shiki_text(&el) {
                let cleaned = clean_code_content(&code);
                replacements.push((el.html(), format_canonical_code_block(&lang, &cleaned)));
            }
        }
    }

    // 5. Standard pre>code blocks that need normalization
    if let Ok(sel) = Selector::parse("pre") {
        for pre in doc.select(&sel) {
            let pre_html = pre.html();
            if pre_html.contains("data-lang=") {
                continue;
            }
            // Skip if already captured by a parent selector above
            if replacements.iter().any(|(orig, _)| orig.contains(&pre_html)) {
                continue;
            }
            let lang = detect_language_from_pre(&pre);
            if let Some(text) = extract_code_text_from_pre(&pre) {
                let cleaned = clean_code_content(&text);
                replacements.push((pre_html, format_canonical_code_block(&lang, &cleaned)));
            }
        }
    }

    // Apply all replacements
    for (original, canonical) in &replacements {
        output = output.replacen(original, canonical, 1);
    }

    output
}

fn detect_language_from_pre(pre: &scraper::ElementRef) -> String {
    // Check data-lang / data-language on pre
    if let Some(lang) = pre.value().attr("data-lang").or(pre.value().attr("data-language")) {
        return normalize_language(lang);
    }

    // Check classes on pre
    if let Some(lang) = detect_language_from_classes(pre.value().attr("class").unwrap_or("")) {
        return lang;
    }

    // Check child <code> element
    if let Ok(code_sel) = Selector::parse("code") {
        if let Some(code_el) = pre.select(&code_sel).next() {
            if let Some(lang) = code_el.value().attr("data-lang").or(code_el.value().attr("data-language")) {
                return normalize_language(lang);
            }
            if let Some(lang) = detect_language_from_classes(code_el.value().attr("class").unwrap_or("")) {
                return lang;
            }
        }
    }

    String::new()
}

fn detect_language_from_element(el: &scraper::ElementRef) -> String {
    if let Some(lang) = el.value().attr("data-lang").or(el.value().attr("data-language")) {
        return normalize_language(lang);
    }
    if let Some(lang) = detect_language_from_classes(el.value().attr("class").unwrap_or("")) {
        return lang;
    }

    // Check child code element
    if let Ok(code_sel) = Selector::parse("code") {
        if let Some(code_el) = el.select(&code_sel).next() {
            if let Some(lang) = code_el.value().attr("data-lang").or(code_el.value().attr("data-language")) {
                return normalize_language(lang);
            }
            if let Some(lang) = detect_language_from_classes(code_el.value().attr("class").unwrap_or("")) {
                return lang;
            }
        }
    }

    String::new()
}

/// Extract language from CSS classes using priority rules.
fn detect_language_from_classes(classes: &str) -> Option<String> {
    for class in classes.split_whitespace() {
        // language-*
        if let Some(caps) = LANGUAGE_CLASS_RE.captures(class) {
            return Some(normalize_language(&caps[1]));
        }
        // lang-*
        if let Some(caps) = LANG_CLASS_RE.captures(class) {
            return Some(normalize_language(&caps[1]));
        }
        // highlight-source-* (GitHub)
        if let Some(caps) = HIGHLIGHT_SOURCE_RE.captures(class) {
            return Some(normalize_language(&caps[1]));
        }
    }

    // brush: * (WordPress)
    if let Some(caps) = BRUSH_RE.captures(classes) {
        return Some(normalize_language(&caps[1]));
    }

    // Bare known language name
    for class in classes.split_whitespace() {
        if is_known_language(class) {
            return Some(normalize_language(class));
        }
    }

    None
}

fn extract_github_language(class_attr: &str) -> Option<String> {
    for class in class_attr.split_whitespace() {
        if let Some(caps) = HIGHLIGHT_SOURCE_RE.captures(class) {
            return Some(normalize_language(&caps[1]));
        }
    }
    // Fallback: any highlight class with a known language
    detect_language_from_classes(class_attr)
}

fn extract_pre_text(el: &scraper::ElementRef) -> Option<String> {
    let sel = Selector::parse("pre").ok()?;
    let pre = el.select(&sel).next()?;
    Some(pre.text().collect::<String>())
}

fn extract_shiki_text(el: &scraper::ElementRef) -> Option<String> {
    // Shiki uses <span class="line"> inside <code>
    let code_sel = Selector::parse("code").ok()?;
    if let Some(code) = el.select(&code_sel).next() {
        let line_sel = Selector::parse("span.line").ok()?;
        let lines: Vec<String> = code.select(&line_sel)
            .map(|span| span.text().collect::<String>())
            .collect();
        if !lines.is_empty() {
            return Some(lines.join("\n"));
        }
        // Fallback to full text
        return Some(code.text().collect::<String>());
    }
    Some(el.text().collect::<String>())
}

fn extract_table_code(el: &scraper::ElementRef) -> Option<(String, String)> {
    // Code is typically in td.code or the second td
    let td_sel = Selector::parse("td").ok()?;
    let tds: Vec<_> = el.select(&td_sel).collect();

    // Try to find the code cell (usually second, or one with class "code")
    for td in &tds {
        let class = td.value().attr("class").unwrap_or("");
        if class.contains("code") || class.contains("rouge-code") {
            let code_text = td.text().collect::<String>();
            let lang = detect_language_from_element(el);
            return Some((lang, code_text));
        }
    }

    // Fallback: use last td
    if tds.len() >= 2 {
        let code_text = tds.last()?.text().collect::<String>();
        let lang = detect_language_from_element(el);
        return Some((lang, code_text));
    }

    None
}

fn extract_code_text_from_pre(pre: &scraper::ElementRef) -> Option<String> {
    // Prefer <code> child text
    if let Ok(code_sel) = Selector::parse("code") {
        if let Some(code) = pre.select(&code_sel).next() {
            // Handle Verso/Lean <code class="hl block">
            let line_sel = Selector::parse("span.line").ok();
            if let Some(ref ls) = line_sel {
                let lines: Vec<String> = code.select(ls)
                    .map(|s| s.text().collect::<String>())
                    .collect();
                if !lines.is_empty() {
                    return Some(lines.join("\n"));
                }
            }
            return Some(code.text().collect::<String>());
        }
    }
    Some(pre.text().collect::<String>())
}

fn standardize_rehype_figure(el: &scraper::ElementRef) -> Option<String> {
    let pre_sel = Selector::parse("pre").ok()?;
    let pre = el.select(&pre_sel).next()?;
    let lang = detect_language_from_pre(&pre);
    let code_text = extract_code_text_from_pre(&pre)?;
    let cleaned = clean_code_content(&code_text);
    Some(format_canonical_code_block(&lang, &cleaned))
}

/// Clean code content: tabs→spaces, strip line numbers, collapse newlines, normalize nbsp.
fn clean_code_content(code: &str) -> String {
    let mut s = code.replace('\t', "    ");
    s = s.replace('\u{00a0}', " "); // non-breaking space

    // Strip leading line numbers (e.g., "  1 |", " 12\t")
    let lines: Vec<&str> = s.lines().collect();
    let has_line_numbers = lines.len() > 2
        && lines.iter().filter(|l| !l.trim().is_empty()).take(5)
            .all(|l| LINE_NUMBER_RE.is_match(l));
    if has_line_numbers {
        s = lines.iter()
            .map(|l| LINE_NUMBER_RE.replace(l, "").to_string())
            .collect::<Vec<_>>()
            .join("\n");
    }

    // Collapse 3+ newlines to 2
    s = MULTI_NEWLINE_RE.replace_all(&s, "\n\n").to_string();

    s.trim().to_string()
}

/// Escape HTML special characters in code text so it can be safely
/// embedded inside `<code>` elements without breaking the HTML structure.
fn html_escape_code(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Format a canonical code block.
fn format_canonical_code_block(lang: &str, code: &str) -> String {
    let escaped = html_escape_code(code);
    if lang.is_empty() {
        format!("<pre><code>{}</code></pre>", escaped)
    } else {
        format!(
            "<pre><code class=\"language-{}\" data-lang=\"{}\">{}</code></pre>",
            lang, lang, escaped
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prism_code_block() {
        let html = r#"<pre class="language-python"><code class="language-python">print("hello")</code></pre>"#;
        let result = standardize_code_blocks(html);
        assert!(result.contains("data-lang=\"python\""));
        assert!(result.contains("print(\"hello\")"));
    }

    #[test]
    fn test_brush_wordpress() {
        let html = r#"<pre class="brush: ruby"><code>puts "hi"</code></pre>"#;
        let result = standardize_code_blocks(html);
        assert!(result.contains("data-lang=\"ruby\""));
    }

    #[test]
    fn test_language_detection_bare() {
        assert_eq!(detect_language_from_classes("python"), Some("python".into()));
        assert_eq!(detect_language_from_classes("language-js"), Some("javascript".into()));
        assert_eq!(detect_language_from_classes("lang-ts"), Some("typescript".into()));
        assert_eq!(detect_language_from_classes("highlight-source-go"), Some("go".into()));
    }

    #[test]
    fn test_clean_code_content_tabs() {
        let code = "fn main() {\n\tprintln!(\"hi\");\n}";
        let cleaned = clean_code_content(code);
        assert!(cleaned.contains("    println!"));
    }

    #[test]
    fn test_clean_code_content_nbsp() {
        let code = "let\u{00a0}x = 1;";
        let cleaned = clean_code_content(code);
        assert_eq!(cleaned, "let x = 1;");
    }
}
