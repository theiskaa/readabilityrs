use scraper::{ElementRef, Html, Node, Selector};

use super::options::MarkdownOptions;
use super::rules;
use super::state::ConversionState;

/// Convert parsed HTML fragment to markdown string.
pub fn convert(doc: &Html, opts: &MarkdownOptions) -> String {
    let mut state = ConversionState::default();
    let root = doc.root_element();
    let mut output = convert_children(root, opts, &mut state);

    // Append collected footnotes
    if !state.footnotes.is_empty() {
        output.push_str(&rules::footnotes::format_footnote_definitions(&state.footnotes));
    }

    // Append collected link references (for reference-style links)
    if !state.link_references.is_empty() {
        output.push_str("\n\n");
        for (id, url) in &state.link_references {
            output.push_str(&format!("[{}]: {}\n", id, url));
        }
    }

    // Post-processing
    output = post_process(&output);

    output
}

/// Recursively convert all children of an element.
fn convert_children(
    element: ElementRef,
    opts: &MarkdownOptions,
    state: &mut ConversionState,
) -> String {
    let mut result = String::new();

    for child in element.children() {
        match child.value() {
            Node::Text(text) => {
                if state.in_code_block {
                    result.push_str(text);
                } else {
                    // Collapse consecutive whitespace to a single space,
                    // mirroring browser behavior for normal flow content.
                    let collapsed = collapse_whitespace(text);
                    result.push_str(&rules::text::escape_markdown(&collapsed));
                }
            }
            Node::Element(_) => {
                if let Some(el) = ElementRef::wrap(child) {
                    result.push_str(&convert_element(el, opts, state));
                }
            }
            _ => {}
        }
    }

    result
}

/// Convert a single element node to markdown.
fn convert_element(
    el: ElementRef,
    opts: &MarkdownOptions,
    state: &mut ConversionState,
) -> String {
    let tag = el.value().name().to_lowercase();

    match tag.as_str() {
        // Inline formatting
        "strong" | "b" => {
            let inner = convert_children(el, opts, state);
            rules::text::convert_strong(&inner, opts, state)
        }
        "em" | "i" => {
            let inner = convert_children(el, opts, state);
            rules::text::convert_emphasis(&inner, opts, state)
        }
        "code" if !state.in_code_block => {
            // Check if inside a <pre> — handled by the pre/code path
            let inner = el.text().collect::<String>();
            rules::text::convert_inline_code(&inner, opts, state)
        }
        "del" | "s" | "strike" => {
            let inner = convert_children(el, opts, state);
            rules::text::convert_strikethrough(&inner, opts, state)
        }
        "mark" => {
            let inner = convert_children(el, opts, state);
            rules::text::convert_highlight(&inner, opts, state)
        }
        "br" => {
            if state.in_heading {
                " ".to_string() // In headings, <br> becomes a space (headings must be single-line)
            } else {
                rules::text::convert_br()
            }
        }
        "hr" => rules::text::convert_hr(),

        // Headings
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let level: u8 = tag[1..].parse().unwrap_or(2);
            state.in_heading = true;
            let inner = convert_children(el, opts, state);
            state.in_heading = false;
            rules::headings::convert_heading(level, &inner, opts)
        }

        // Links
        "a" => {
            if state.in_link {
                return convert_children(el, opts, state);
            }
            let href = el.value().attr("href").unwrap_or("");

            if is_footnote_ref(&el) {
                let text = el.text().collect::<String>();
                return rules::footnotes::convert_footnote_ref(text.trim());
            }

            let title = el.value().attr("title").unwrap_or("");
            state.in_link = true;
            let inner = convert_children(el, opts, state);
            state.in_link = false;
            rules::links::convert_link(&inner, href, title, opts, state)
        }

        // Images
        "img" => {
            let alt = el.value().attr("alt").unwrap_or("");
            let src = el.value().attr("src").unwrap_or("");
            let title = el.value().attr("title").unwrap_or("");
            rules::images::convert_image(alt, src, title)
        }

        // Figures
        "figure" => convert_figure(el, opts, state),

        // Lists
        "ul" => {
            state.list_depth += 1;
            let inner = convert_children(el, opts, state);
            state.list_depth -= 1;
            if state.list_depth == 0 {
                format!("\n\n{}\n", inner.trim_end())
            } else {
                format!("\n{}", inner)
            }
        }
        "ol" => {
            state.list_depth += 1;
            state.ordered_list_counters.push(0);
            let inner = convert_children(el, opts, state);
            state.ordered_list_counters.pop();
            state.list_depth -= 1;
            if state.list_depth == 0 {
                format!("\n\n{}\n", inner.trim_end())
            } else {
                format!("\n{}", inner)
            }
        }
        "li" => {
            let prev = state.in_list_item;
            state.in_list_item = true;
            let result = convert_list_item(el, opts, state);
            state.in_list_item = prev;
            result
        }

        // Tables
        "table" => convert_table(el, opts, state),

        // Code blocks
        "pre" => convert_pre_block(el, opts, state),

        // Blockquotes
        "blockquote" => {
            state.in_blockquote_depth += 1;
            let inner = convert_children(el, opts, state);
            let callout = el.value().attr("data-callout");
            let depth = state.in_blockquote_depth;
            state.in_blockquote_depth -= 1;
            rules::blockquotes::convert_blockquote(&inner, depth, callout)
        }

        // Math
        "math" => {
            let latex = el.value().attr("data-latex").unwrap_or("");
            let display = el.value().attr("display").unwrap_or("inline");
            rules::math::convert_math(latex, display)
        }

        // Footnote definitions container
        "div" if el.value().attr("id") == Some("footnotes") => {
            collect_footnote_definitions(el, state);
            String::new()
        }

        // Media
        "iframe" => {
            let src = el.value().attr("src").unwrap_or("");
            rules::media::convert_iframe(src)
        }
        "video" => {
            let src = el.value().attr("src")
                .unwrap_or_else(|| find_source_src(&el).unwrap_or(""));
            rules::media::convert_video(src)
        }
        "audio" => {
            let src = el.value().attr("src")
                .unwrap_or_else(|| find_source_src(&el).unwrap_or(""));
            rules::media::convert_audio(src)
        }

        // Footnote reference wrapper
        "sup" if is_footnote_ref_sup(&el) => {
            let text = el.text().collect::<String>();
            rules::footnotes::convert_footnote_ref(text.trim())
        }

        // Block elements — just convert children with paragraph spacing
        "p" => {
            let inner = convert_children(el, opts, state);
            let trimmed = inner.trim();
            if trimmed.is_empty() {
                String::new()
            } else if state.in_list_item || state.in_table {
                // Compact mode inside list items and table cells —
                // no blank-line wrapping so lists stay together and cells stay single-line.
                format!("{}\n", trimmed)
            } else {
                format!("\n\n{}\n\n", trimmed)
            }
        }
        "div" | "section" | "article" | "main" | "header" | "footer" | "nav" | "aside" => {
            convert_children(el, opts, state)
        }

        // Superscript (non-footnote) and subscript — extended markdown syntax
        "sup" => {
            let inner = convert_children(el, opts, state);
            let trimmed = inner.trim();
            if trimmed.is_empty() { String::new() } else { format!("^{}^", trimmed) }
        }
        "sub" => {
            let inner = convert_children(el, opts, state);
            let trimmed = inner.trim();
            if trimmed.is_empty() { String::new() } else { format!("~{}~", trimmed) }
        }

        // Details/summary — preserve as raw HTML (most renderers support it)
        "details" => format!("\n\n{}\n\n", el.html()),

        // Spans and other inline — transparent pass-through
        "span" | "abbr" | "cite" | "dfn" | "kbd" | "samp" | "var" | "time" | "data"
        | "small" | "ins" | "u" | "q" | "bdo" | "bdi" | "wbr"
        | "ruby" | "rt" | "rp" | "summary" | "label" => {
            convert_children(el, opts, state)
        }

        // Definition lists
        "dl" => convert_children(el, opts, state),
        "dt" => {
            let inner = convert_children(el, opts, state);
            format!("\n\n**{}**\n", inner.trim())
        }
        "dd" => {
            let inner = convert_children(el, opts, state);
            format!(": {}\n", inner.trim())
        }

        // Unknown — pass through children
        _ => convert_children(el, opts, state),
    }
}

/// Convert a `<pre>` element (code block).
fn convert_pre_block(
    el: ElementRef,
    opts: &MarkdownOptions,
    state: &mut ConversionState,
) -> String {
    state.in_code_block = true;

    let mut language = String::new();
    let mut code_text = String::new();

    // Look for <code> child
    if let Ok(code_sel) = Selector::parse("code") {
        if let Some(code_el) = el.select(&code_sel).next() {
            // Extract language from class
            let class = code_el.value().attr("class").unwrap_or("");
            for cls in class.split_whitespace() {
                if let Some(lang) = cls.strip_prefix("language-") {
                    language = lang.to_string();
                    break;
                }
            }
            // Also check data-lang
            if language.is_empty() {
                if let Some(lang) = code_el.value().attr("data-lang") {
                    language = lang.to_string();
                }
            }
            code_text = code_el.text().collect::<String>();
        }
    }

    // Fallback: use pre text directly
    if code_text.is_empty() {
        code_text = el.text().collect::<String>();
    }

    state.in_code_block = false;
    rules::code::convert_code_block(&code_text, &language, opts)
}

/// Convert a `<figure>` element.
fn convert_figure(
    el: ElementRef,
    opts: &MarkdownOptions,
    state: &mut ConversionState,
) -> String {
    let img_sel = Selector::parse("img").ok();
    let caption_sel = Selector::parse("figcaption").ok();

    let (alt, src) = if let Some(ref sel) = img_sel {
        if let Some(img) = el.select(sel).next() {
            (
                img.value().attr("alt").unwrap_or("").to_string(),
                img.value().attr("src").unwrap_or("").to_string(),
            )
        } else {
            // No img — could be a code figure, pass through
            return convert_children(el, opts, state);
        }
    } else {
        return convert_children(el, opts, state);
    };

    let caption = caption_sel.and_then(|sel| {
        el.select(&sel)
            .next()
            .map(|cap| {
                let raw: String = cap.text().collect();
                collapse_whitespace(raw.trim())
            })
    });

    rules::images::convert_figure(&alt, &src, caption.as_deref())
}

/// Convert a `<li>` element.
fn convert_list_item(
    el: ElementRef,
    opts: &MarkdownOptions,
    state: &mut ConversionState,
) -> String {
    // Check for task list
    if let Ok(checkbox_sel) = Selector::parse("input[type=\"checkbox\"]") {
        if let Some(checkbox) = el.select(&checkbox_sel).next() {
            let checked = checkbox.value().attr("checked").is_some();
            let inner = convert_children_skip_checkbox(el, opts, state);
            return rules::lists::convert_task_item(&inner, checked, opts, state);
        }
    }

    let inner = convert_children(el, opts, state);

    // Skip empty list items (no visible content)
    if inner.trim().is_empty() {
        // Still consume the ordered list counter to keep numbering correct
        if let Some(counter) = state.ordered_list_counters.last_mut() {
            *counter += 1;
        }
        return String::new();
    }

    // Check if we're in an ordered list
    if let Some(counter) = state.ordered_list_counters.last_mut() {
        *counter += 1;
        let c = *counter;
        rules::lists::convert_ordered_item(&inner, c, state)
    } else {
        rules::lists::convert_unordered_item(&inner, opts, state)
    }
}

/// Convert children of a list item, skipping the leading checkbox input.
fn convert_children_skip_checkbox(
    el: ElementRef,
    opts: &MarkdownOptions,
    state: &mut ConversionState,
) -> String {
    let mut result = String::new();
    let mut skipped_checkbox = false;

    for child in el.children() {
        match child.value() {
            Node::Text(text) => result.push_str(text),
            Node::Element(_) => {
                if let Some(child_el) = ElementRef::wrap(child) {
                    if !skipped_checkbox
                        && child_el.value().name() == "input"
                        && child_el.value().attr("type") == Some("checkbox")
                    {
                        skipped_checkbox = true;
                        continue;
                    }
                    result.push_str(&convert_element(child_el, opts, state));
                }
            }
            _ => {}
        }
    }

    result
}

/// Convert a `<table>` element.
fn convert_table(
    el: ElementRef,
    opts: &MarkdownOptions,
    state: &mut ConversionState,
) -> String {
    // Check if complex
    if rules::tables::is_complex_table(&el) && opts.preserve_complex_tables {
        return format!("\n\n{}\n\n", el.html());
    }

    // Check if layout table
    if rules::tables::is_layout_table(&el) {
        return convert_children(el, opts, state);
    }

    // Extract headers and rows
    let mut headers: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();

    // Headers from <thead>
    if let Ok(thead_sel) = Selector::parse("thead") {
        if let Some(thead) = el.select(&thead_sel).next() {
            if let Ok(th_sel) = Selector::parse("th") {
                headers = thead
                    .select(&th_sel)
                    .map(|th| {
                        state.in_table = true;
                        let text = convert_children(th, opts, state);
                        state.in_table = false;
                        text.trim().replace('\n', " ")
                    })
                    .collect();
            }
        }
    }

    // If no thead, try first row with <th>
    if headers.is_empty() {
        if let Ok(tr_sel) = Selector::parse("tr") {
            if let Some(first_tr) = el.select(&tr_sel).next() {
                if let Ok(th_sel) = Selector::parse("th") {
                    let ths: Vec<String> = first_tr
                        .select(&th_sel)
                        .map(|th| {
                            state.in_table = true;
                            let text = convert_children(th, opts, state);
                            state.in_table = false;
                            text.trim().replace('\n', " ")
                        })
                        .collect();
                    if !ths.is_empty() {
                        headers = ths;
                    }
                }
            }
        }
    }

    // Data rows from <tbody> or direct <tr>
    let row_selector = Selector::parse("tbody tr, tr").ok();
    if let Some(ref sel) = row_selector {
        let td_sel = Selector::parse("td").ok();
        for tr in el.select(sel) {
            if let Some(ref td_s) = td_sel {
                let cells: Vec<String> = tr
                    .select(td_s)
                    .map(|td| {
                        state.in_table = true;
                        let text = convert_children(td, opts, state);
                        state.in_table = false;
                        text.trim().replace('\n', " ")
                    })
                    .collect();
                if !cells.is_empty() {
                    rows.push(cells);
                }
            }
        }
    }

    rules::tables::convert_simple_table(&headers, &rows)
}

/// Collapse consecutive whitespace characters to a single space, mirroring
/// how browsers render whitespace in normal flow content.
fn collapse_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_ws = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_ws {
                result.push(' ');
                prev_ws = true;
            }
        } else {
            result.push(c);
            prev_ws = false;
        }
    }
    result
}

/// Find the `src` attribute from a `<source>` child element.
fn find_source_src<'a>(el: &'a ElementRef) -> Option<&'a str> {
    Selector::parse("source[src]")
        .ok()
        .and_then(|sel| el.select(&sel).next())
        .and_then(|source| source.value().attr("src"))
}

fn is_footnote_ref(a: &ElementRef) -> bool {
    let href = a.value().attr("href").unwrap_or("");
    let class = a.value().attr("class").unwrap_or("");
    let id = a.value().attr("id").unwrap_or("");
    (href.contains("#fn:") || href.contains("#fn-") || href.contains("#footnote"))
        && (class.contains("footnote") || id.contains("fnref"))
        && href.starts_with('#')
}

fn is_footnote_ref_sup(sup: &ElementRef) -> bool {
    if let Some(id) = sup.value().attr("id") {
        if id.starts_with("fnref") {
            return true;
        }
    }
    // Check if contains an <a> pointing to a footnote
    if let Ok(a_sel) = Selector::parse("a") {
        if let Some(a) = sup.select(&a_sel).next() {
            return is_footnote_ref(&a);
        }
    }
    false
}

fn collect_footnote_definitions(el: ElementRef, state: &mut ConversionState) {
    if let Ok(li_sel) = Selector::parse("li.footnote") {
        for li in el.select(&li_sel) {
            let id = li
                .value()
                .attr("id")
                .unwrap_or("")
                .trim_start_matches("fn:")
                .trim_start_matches("fn-")
                .to_string();

            // Get content text (skip backref link)
            let mut content = String::new();
            for child in li.children() {
                if let Some(child_el) = ElementRef::wrap(child) {
                    let class = child_el.value().attr("class").unwrap_or("");
                    if class.contains("backref") || class.contains("footnote-backref") {
                        continue;
                    }
                    content.push_str(&child_el.text().collect::<String>());
                } else if let Some(text) = child.value().as_text() {
                    content.push_str(text);
                }
            }

            if !id.is_empty() && !content.trim().is_empty() {
                state.footnotes.push((id, content.trim().to_string()));
            }
        }
    }
}

/// Post-process the markdown output.
fn post_process(md: &str) -> String {
    use once_cell::sync::Lazy;
    static MULTI_NL: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"\n{3,}").unwrap());
    // Detect escaped asterisk scene breaks like \*\*\*\*\*\* (3+ escaped stars on a line)
    static SCENE_BREAK: Lazy<regex::Regex> =
        Lazy::new(|| regex::Regex::new(r"^(\\\*){3,}$").unwrap());

    let mut result = md.to_string();

    // Remove empty links [](url) but preserve images ![](url)
    result = remove_empty_links(&result);

    // Fix !![img] → ! ![img]
    result = result.replace("!![", "! ![");

    // Trim trailing whitespace from each line + convert escaped scene breaks to ---
    result = result
        .lines()
        .map(|l| {
            let trimmed = l.trim_end();
            if SCENE_BREAK.is_match(trimmed.trim()) {
                "---"
            } else {
                trimmed
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Collapse 3+ consecutive blank lines to 2
    result = MULTI_NL.replace_all(&result, "\n\n").to_string();

    // Trim leading/trailing whitespace from the whole document
    result.trim().to_string()
}

/// Remove empty links `[](url)` but preserve image links `![](url)`.
fn remove_empty_links(s: &str) -> String {
    use once_cell::sync::Lazy;
    static EMPTY_LINK_RE: Lazy<regex::Regex> =
        Lazy::new(|| regex::Regex::new(r"\[]\([^)]*\)").unwrap());

    let mut result = String::new();
    let mut last = 0;
    for m in EMPTY_LINK_RE.find_iter(s) {
        let start = m.start();
        if start > 0 && s.as_bytes()[start - 1] == b'!' {
            // Preceded by '!' — image ![](url), keep it
            result.push_str(&s[last..m.end()]);
        } else {
            // Empty link [](url), remove it
            result.push_str(&s[last..start]);
        }
        last = m.end();
    }
    result.push_str(&s[last..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert_html(html: &str) -> String {
        let doc = Html::parse_fragment(html);
        let opts = MarkdownOptions::default();
        convert(&doc, &opts)
    }

    #[test]
    fn test_paragraph() {
        let result = convert_html("<p>Hello world</p>");
        assert_eq!(result.trim(), "Hello world");
    }

    #[test]
    fn test_bold_and_italic() {
        let result = convert_html("<p><strong>bold</strong> and <em>italic</em></p>");
        assert!(result.contains("**bold**"));
        assert!(result.contains("*italic*"));
    }

    #[test]
    fn test_heading() {
        let result = convert_html("<h2>Section Title</h2>");
        assert!(result.contains("## Section Title"));
    }

    #[test]
    fn test_link() {
        let result = convert_html(r#"<a href="https://example.com">click</a>"#);
        assert!(result.contains("[click](https://example.com)"));
    }

    #[test]
    fn test_image() {
        let result = convert_html(r#"<img src="photo.jpg" alt="A photo"/>"#);
        assert!(result.contains("![A photo](photo.jpg)"));
    }

    #[test]
    fn test_unordered_list() {
        let result = convert_html("<ul><li>one</li><li>two</li></ul>");
        assert!(result.contains("- one"));
        assert!(result.contains("- two"));
    }

    #[test]
    fn test_ordered_list() {
        let result = convert_html("<ol><li>first</li><li>second</li></ol>");
        assert!(result.contains("1. first"));
        assert!(result.contains("2. second"));
    }

    #[test]
    fn test_code_block() {
        let result = convert_html(
            r#"<pre><code class="language-rust">fn main() {}</code></pre>"#,
        );
        assert!(result.contains("```rust"));
        assert!(result.contains("fn main() {}"));
    }

    #[test]
    fn test_blockquote() {
        let result = convert_html("<blockquote><p>quoted</p></blockquote>");
        assert!(result.contains("> quoted"));
    }

    #[test]
    fn test_hr() {
        let result = convert_html("<p>above</p><hr/><p>below</p>");
        assert!(result.contains("---"));
    }

    #[test]
    fn test_post_process_collapses_newlines() {
        let result = post_process("a\n\n\n\n\nb");
        assert_eq!(result, "a\n\nb");
    }
}
