use once_cell::sync::Lazy;
use scraper::{Html, Selector};

static ANN_SEL: Lazy<Option<Selector>> = Lazy::new(|| {
    Selector::parse("annotation[encoding=\"application/x-tex\"]").ok()
});
static SCRIPT_SEL: Lazy<Option<Selector>> = Lazy::new(|| {
    Selector::parse("script[type=\"math/tex\"]").ok()
});

/// Standardize math elements from MathJax/KaTeX to canonical `<math data-latex="...">`.
///
/// Single-pass: parse once, collect all replacements, apply them.
pub fn standardize_math(html: &str) -> String {
    let doc = Html::parse_fragment(html);
    let mut output = html.to_string();
    let mut replacements: Vec<(String, String)> = Vec::new();

    // MathJax v3: <mjx-container>
    if let Ok(sel) = Selector::parse("mjx-container") {
        for el in doc.select(&sel) {
            let display = el.value().attr("display").unwrap_or("")
                .eq_ignore_ascii_case("block")
                || el.value().attr("class").unwrap_or("").contains("display");
            if let Some(latex) = extract_latex_source(&el) {
                let display_attr = if display { "block" } else { "inline" };
                let canonical = format!(
                    "<math data-latex=\"{}\" display=\"{}\"></math>",
                    escape_attr(&latex), display_attr
                );
                replacements.push((el.html(), canonical));
            }
        }
    }

    // MathJax v2: <span class="MathJax">
    if let Ok(sel) = Selector::parse("span.MathJax") {
        for el in doc.select(&sel) {
            let id = el.value().attr("id").unwrap_or("");
            let display = id.contains("Display")
                || el.value().attr("class").unwrap_or("").contains("Display");
            if let Some(latex) = extract_latex_source(&el) {
                let display_attr = if display { "block" } else { "inline" };
                let canonical = format!(
                    "<math data-latex=\"{}\" display=\"{}\"></math>",
                    escape_attr(&latex), display_attr
                );
                replacements.push((el.html(), canonical));
            }
        }
    }

    // KaTeX: <span class="katex">
    if let Ok(sel) = Selector::parse("span.katex") {
        for el in doc.select(&sel) {
            let display = el.value().attr("class").unwrap_or("").contains("katex-display");
            if let Some(latex) = extract_latex_source(&el) {
                let display_attr = if display { "block" } else { "inline" };
                let canonical = format!(
                    "<math data-latex=\"{}\" display=\"{}\"></math>",
                    escape_attr(&latex), display_attr
                );
                replacements.push((el.html(), canonical));
            }
        }
    }

    for (original, canonical) in &replacements {
        output = output.replacen(original, canonical, 1);
    }

    output
}

/// Extract LaTeX source from a math element, checking multiple locations.
fn extract_latex_source(el: &scraper::ElementRef) -> Option<String> {
    // 1. data-latex attribute
    if let Some(latex) = el.value().attr("data-latex") {
        if !latex.is_empty() {
            return Some(latex.to_string());
        }
    }

    // 2. alt attribute
    if let Some(alt) = el.value().attr("alt") {
        if !alt.is_empty() {
            return Some(alt.to_string());
        }
    }

    // 3. <annotation encoding="application/x-tex"> inside <semantics>
    if let Some(ref sel) = *ANN_SEL {
        if let Some(ann) = el.select(sel).next() {
            let text = ann.text().collect::<String>();
            if !text.trim().is_empty() {
                return Some(text.trim().to_string());
            }
        }
    }

    // 4. <script type="math/tex"> (MathJax v2 pattern)
    if let Some(ref sel) = *SCRIPT_SEL {
        if let Some(script) = el.select(sel).next() {
            let text = script.text().collect::<String>();
            if !text.trim().is_empty() {
                return Some(text.trim().to_string());
            }
        }
    }

    None
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_katex_standardization() {
        let html = r#"<span class="katex" data-latex="x^2 + y^2 = z^2">rendered</span>"#;
        let result = standardize_math(html);
        assert!(result.contains("data-latex=\"x^2 + y^2 = z^2\""));
        assert!(result.contains("display=\"inline\""));
    }

    #[test]
    fn test_annotation_extraction() {
        let html = r#"<mjx-container><math><semantics><annotation encoding="application/x-tex">E = mc^2</annotation></semantics></math></mjx-container>"#;
        let result = standardize_math(html);
        assert!(result.contains("E = mc^2"));
    }
}
