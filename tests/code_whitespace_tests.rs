//! Regression tests for issue #24: post-processing collapsed the indentation
//! inside `<pre>`/`<code>`, so extracted code listings came out flattened.

use readabilityrs::{Article, Readability, ReadabilityOptions};

const PADDING: &str = "This paragraph exists only to push the article over the \
    scoring threshold so the extractor keeps the section that holds the code \
    listing under test, padding padding padding padding padding.";

fn parse(body: &str) -> Article {
    parse_with(
        body,
        ReadabilityOptions {
            output_markdown: true,
            ..Default::default()
        },
    )
}

fn parse_with(body: &str, options: ReadabilityOptions) -> Article {
    let html = format!(
        "<html><head><title>Code</title></head><body><article>\
         <h1>Code</h1><p>{PADDING}</p>{body}<p>{PADDING}</p></article></body></html>"
    );

    Readability::new(&html, Some("https://example.com/a"), Some(options))
        .expect("parser construction")
        .parse()
        .expect("article extraction")
}

#[test]
fn test_pre_block_keeps_indentation_in_content_and_markdown() {
    let article = parse(
        "<pre tabindex=\"0\" class=\"chroma\"><code class=\"language-rust\">fn main() {\n\
         \x20   let x = 1;\n\
         \x20       deeper();\n\
         }\n</code></pre>",
    );

    let content = article.content.expect("content");
    assert!(content.contains("\n    let x = 1;\n"), "got: {content}");
    assert!(content.contains("\n        deeper();\n"), "got: {content}");

    let markdown = article.markdown_content.expect("markdown");
    assert!(markdown.contains("\n    let x = 1;\n"), "got: {markdown}");
    assert!(
        markdown.contains("\n        deeper();\n"),
        "got: {markdown}"
    );
}

#[test]
fn test_pre_block_keeps_blank_lines() {
    let article = parse("<pre><code>first();\n\n\n\nlast();\n</code></pre>");

    let content = article.content.expect("content");
    assert!(
        content.contains("first();\n\n\n\nlast();"),
        "got: {content}"
    );
}

#[test]
fn test_inline_code_keeps_internal_spacing() {
    let article = parse("<p>The literal <code>a    b</code> matters.</p>");

    let content = article.content.expect("content");
    assert!(content.contains("<code>a    b</code>"), "got: {content}");
}

#[test]
fn test_comment_inside_pre_does_not_end_the_listing() {
    let article = parse("<pre><code><!-- </pre> -->fn f() {\n    body();\n}\n</code></pre>");

    let content = article.content.expect("content");
    assert!(content.contains("\n    body();\n"), "got: {content}");
}

#[test]
fn test_title_removal_keeps_code_indentation() {
    let article = parse_with(
        "<pre><code>fn f() {\n\n\n    body();\n   \n    tail();\n}\n</code></pre>",
        ReadabilityOptions {
            output_markdown: true,
            remove_title_from_content: true,
            ..Default::default()
        },
    );

    let content = article.content.expect("content");
    assert!(
        !content.contains("<h1"),
        "title removal did not run: {content}"
    );
    assert!(
        content.contains("{\n\n\n    body();\n   \n"),
        "got: {content}"
    );
}

#[test]
fn test_prose_whitespace_still_collapses() {
    let article = parse("<p>ordinary    prose    spacing</p><pre>  kept  </pre>");

    let content = article.content.expect("content");
    assert!(content.contains("ordinary prose spacing"), "got: {content}");
    assert!(content.contains("<pre>  kept  </pre>"), "got: {content}");
}
