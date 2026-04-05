use scraper::{Html, Selector};

/// Standardize footnotes from various formats into canonical form.
///
/// **Canonical reference:**
/// ```html
/// <sup id="fnref:1"><a href="#fn:1">1</a></sup>
/// ```
///
/// **Canonical definition block (at end of article):**
/// ```html
/// <div id="footnotes">
///   <ol>
///     <li class="footnote" id="fn:1">
///       <p>Footnote content.</p>
///       <a href="#fnref:1" class="footnote-backref">↩</a>
///     </li>
///   </ol>
/// </div>
/// ```
pub fn standardize_footnotes(html: &str) -> String {
    let doc = Html::parse_fragment(html);
    let mut refs: Vec<FootnoteRef> = Vec::new();
    let mut defs: Vec<FootnoteDef> = Vec::new();
    let mut output = html.to_string();

    // Detect footnote references
    collect_references(&doc, &mut refs);

    // Detect footnote definitions
    collect_definitions(&doc, &mut defs);

    if refs.is_empty() && defs.is_empty() {
        return output;
    }

    // Standardize references: replace with canonical format
    let mut counter = 1u32;
    for r in &refs {
        let canonical = format!(
            "<sup id=\"fnref:{}\"><a href=\"#fn:{}\">{}</a></sup>",
            counter, counter, counter
        );
        output = output.replace(&r.original_html, &canonical);
        counter += 1;
    }

    // Remove original definition containers and rebuild at end
    for d in &defs {
        output = output.replace(&d.container_html, "");
    }

    // Build canonical definitions block
    if !defs.is_empty() {
        let mut def_items = String::new();
        for (i, d) in defs.iter().enumerate() {
            let num = i + 1;
            def_items.push_str(&format!(
                "<li class=\"footnote\" id=\"fn:{}\"><p>{}</p><a href=\"#fnref:{}\" class=\"footnote-backref\">↩</a></li>",
                num,
                d.content.trim(),
                num
            ));
        }
        let footnote_block = format!(
            "<div id=\"footnotes\"><ol>{}</ol></div>",
            def_items
        );
        output.push_str(&footnote_block);
    }

    output
}

#[derive(Debug)]
struct FootnoteRef {
    original_html: String,
}

#[derive(Debug)]
struct FootnoteDef {
    container_html: String,
    content: String,
}

fn collect_references(doc: &Html, refs: &mut Vec<FootnoteRef>) {
    // Pattern: <sup><a href="#fn-*"> or <sup><a href="#fn:*">
    if let Ok(sel) = Selector::parse("sup") {
        for sup in doc.select(&sel) {
            if let Ok(a_sel) = Selector::parse("a") {
                if let Some(a) = sup.select(&a_sel).next() {
                    let href = a.value().attr("href").unwrap_or("");
                    if href.contains("#fn") || href.contains("#footnote") {
                        refs.push(FootnoteRef {
                            original_html: sup.html(),
                        });
                    }
                }
            }
        }
    }

    // Pattern: <a class="footnote-ref"> or <a class="footnote-anchor">
    if let Ok(sel) = Selector::parse("a.footnote-ref, a.footnote-anchor") {
        for a in doc.select(&sel) {
            // Skip if already captured inside a <sup>
            let html = a.html();
            if refs.iter().any(|r| r.original_html.contains(&html)) {
                continue;
            }
            refs.push(FootnoteRef {
                original_html: html,
            });
        }
    }
}

fn collect_definitions(doc: &Html, defs: &mut Vec<FootnoteDef>) {
    // Try various definition container selectors
    let selectors = [
        "div.footnotes",
        "section.footnotes",
        "div.footnotes-footer",
        "section[role=\"doc-endnotes\"]",
        "ol.footnote-list",
    ];

    for sel_str in &selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            for container in doc.select(&sel) {
                let container_html = container.html();
                // Extract individual footnotes from <li> elements
                if let Ok(li_sel) = Selector::parse("li") {
                    for li in container.select(&li_sel) {
                        let content = extract_footnote_content(&li);
                        if !content.is_empty() {
                            defs.push(FootnoteDef {
                                container_html: container_html.clone(),
                                content,
                            });
                        }
                    }
                }
                if !defs.is_empty() {
                    return;
                }
            }
        }
    }

    // Substack pattern: <div class="footnote" data-component-name="FootnoteToDOM">
    if let Ok(sel) = Selector::parse("div.footnote[data-component-name]") {
        for div in doc.select(&sel) {
            let content = div.text().collect::<String>();
            if !content.trim().is_empty() {
                defs.push(FootnoteDef {
                    container_html: div.html(),
                    content: content.trim().to_string(),
                });
            }
        }
    }
}

fn extract_footnote_content(li: &scraper::ElementRef) -> String {
    // Get text content, excluding back-reference links
    let mut content = String::new();
    for child in li.children() {
        if let Some(el) = scraper::ElementRef::wrap(child) {
            let tag = el.value().name();
            let class = el.value().attr("class").unwrap_or("");
            // Skip backref links
            if tag == "a" && (class.contains("backref") || class.contains("footnote-back")) {
                continue;
            }
            content.push_str(&el.text().collect::<String>());
        } else if let Some(text) = child.value().as_text() {
            content.push_str(text);
        }
    }
    content.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_footnote_standardization() {
        let html = r##"<p>Text<sup><a href="#fn-1">1</a></sup></p>
<div class="footnotes"><ol><li id="fn-1">First footnote.</li></ol></div>"##;
        let result = standardize_footnotes(html);
        assert!(result.contains("fnref:1"));
        assert!(result.contains("fn:1"));
        assert!(result.contains("footnote-backref"));
    }
}
