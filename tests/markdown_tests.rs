use readabilityrs::{MarkdownOptions, Readability, ReadabilityOptions};
use readabilityrs::markdown::options::{HeadingStyle, LinkStyle};

/// Helper: convert HTML fragment to markdown via the public API.
fn html_to_md(html: &str) -> String {
    let md_opts = MarkdownOptions::default();
    let standardized = readabilityrs::elements::standardize_all(html, None);
    readabilityrs::markdown::html_to_markdown(&standardized, &md_opts)
}

// ── Inline formatting ───────────────────────────────────────────────

#[test]
fn test_bold() {
    let md = html_to_md("<p><strong>bold text</strong></p>");
    assert!(md.contains("**bold text**"));
}

#[test]
fn test_italic() {
    let md = html_to_md("<p><em>italic text</em></p>");
    assert!(md.contains("*italic text*"));
}

#[test]
fn test_inline_code() {
    let md = html_to_md("<p>Use <code>println!</code> to print.</p>");
    assert!(md.contains("`println!`"));
}

#[test]
fn test_strikethrough() {
    let md = html_to_md("<p><del>removed</del></p>");
    assert!(md.contains("~~removed~~"));
}

#[test]
fn test_highlight() {
    let md = html_to_md("<p><mark>highlighted</mark></p>");
    assert!(md.contains("==highlighted=="));
}

// ── Headings ────────────────────────────────────────────────────────

#[test]
fn test_headings() {
    let md = html_to_md("<h2>Section</h2><h3>Subsection</h3>");
    assert!(md.contains("## Section"));
    assert!(md.contains("### Subsection"));
}

// ── Links ───────────────────────────────────────────────────────────

#[test]
fn test_link() {
    let md = html_to_md(r#"<p><a href="https://example.com">Example</a></p>"#);
    assert!(md.contains("[Example](https://example.com)"));
}

// ── Images ──────────────────────────────────────────────────────────

#[test]
fn test_image() {
    let md = html_to_md(r#"<img src="photo.jpg" alt="A nice photo"/>"#);
    assert!(md.contains("![A nice photo](photo.jpg)"));
}

#[test]
fn test_figure_with_caption() {
    let md = html_to_md(
        r#"<figure><img src="photo.jpg" alt="alt text"/><figcaption>My caption</figcaption></figure>"#,
    );
    assert!(md.contains("![My caption](photo.jpg)"));
}

// ── Lists ───────────────────────────────────────────────────────────

#[test]
fn test_unordered_list() {
    let md = html_to_md("<ul><li>Apple</li><li>Banana</li><li>Cherry</li></ul>");
    assert!(md.contains("- Apple"));
    assert!(md.contains("- Banana"));
    assert!(md.contains("- Cherry"));
}

#[test]
fn test_ordered_list() {
    let md = html_to_md("<ol><li>First</li><li>Second</li><li>Third</li></ol>");
    assert!(md.contains("1. First"));
    assert!(md.contains("2. Second"));
    assert!(md.contains("3. Third"));
}

#[test]
fn test_task_list() {
    let md = html_to_md(
        r#"<ul><li><input type="checkbox" checked/> Done</li><li><input type="checkbox"/> Todo</li></ul>"#,
    );
    assert!(md.contains("- [x] Done"));
    assert!(md.contains("- [ ] Todo"));
}

// ── Code blocks ─────────────────────────────────────────────────────

#[test]
fn test_fenced_code_block_with_language() {
    let md = html_to_md(
        r#"<pre><code class="language-rust">fn main() {
    println!("Hello");
}</code></pre>"#,
    );
    assert!(md.contains("```rust"));
    assert!(md.contains("fn main()"));
    assert!(md.contains("```"));
}

#[test]
fn test_fenced_code_block_no_language() {
    let md = html_to_md("<pre><code>some code here</code></pre>");
    assert!(md.contains("```\nsome code here\n```"));
}

// ── Blockquotes ─────────────────────────────────────────────────────

#[test]
fn test_blockquote() {
    let md = html_to_md("<blockquote><p>A wise quote.</p></blockquote>");
    assert_eq!(md.trim(), "> A wise quote.");
}

#[test]
fn test_blockquote_callout() {
    let md = html_to_md(
        r#"<blockquote data-callout="warning"><p>Be careful!</p></blockquote>"#,
    );
    assert!(md.contains("> [!WARNING]"));
    assert!(md.contains("> Be careful!"));
}

// ── Tables ──────────────────────────────────────────────────────────

#[test]
fn test_simple_table() {
    let md = html_to_md(
        "<table><thead><tr><th>Name</th><th>Age</th></tr></thead>\
         <tbody><tr><td>Alice</td><td>30</td></tr></tbody></table>",
    );
    assert!(md.contains("| Name"));
    assert!(md.contains("| Alice"));
    assert!(md.contains("|---"));
}

// ── Math ────────────────────────────────────────────────────────────

#[test]
fn test_inline_math() {
    let md = html_to_md(r#"<p>The formula <math data-latex="x^2" display="inline"></math> is simple.</p>"#);
    assert!(md.contains("$x^2$"));
}

#[test]
fn test_block_math() {
    let md = html_to_md(
        r#"<math data-latex="E = mc^2" display="block"></math>"#,
    );
    assert!(md.contains("$$E = mc^2$$"));
}

// ── Media embeds ────────────────────────────────────────────────────

#[test]
fn test_youtube_iframe() {
    let md = html_to_md(r#"<iframe src="https://www.youtube.com/embed/abc123"></iframe>"#);
    assert!(md.contains("[Video](https://www.youtube.com/embed/abc123)"));
}

#[test]
fn test_video_element() {
    let md = html_to_md(r#"<video src="movie.mp4"></video>"#);
    assert!(md.contains("[Video](movie.mp4)"));
}

// ── HR ──────────────────────────────────────────────────────────────

#[test]
fn test_horizontal_rule() {
    let md = html_to_md("<p>Above</p><hr/><p>Below</p>");
    assert!(md.contains("---"));
    assert!(md.contains("Above"));
    assert!(md.contains("Below"));
}

// ── Code block standardization ──────────────────────────────────────

#[test]
fn test_prism_code_standardization() {
    let html = r#"<pre class="language-python"><code class="language-python">print("hello")</code></pre>"#;
    let md = html_to_md(html);
    assert!(md.contains("```python"));
    assert!(md.contains("print(\"hello\")"));
}

#[test]
fn test_brush_wordpress_standardization() {
    let html = r#"<pre class="brush: ruby"><code>puts "hi"</code></pre>"#;
    let md = html_to_md(html);
    assert!(md.contains("```ruby"));
}

// ── Heading standardization ─────────────────────────────────────────

#[test]
fn test_h1_dedup_with_title() {
    let md_opts = MarkdownOptions::default();
    let standardized =
        readabilityrs::elements::standardize_all("<h1>My Title</h1><p>Content</p>", Some("My Title"));
    let md = readabilityrs::markdown::html_to_markdown(&standardized, &md_opts);
    // h1 matching title should be removed
    assert!(!md.contains("# My Title"));
    assert!(md.contains("Content"));
}

#[test]
fn test_h1_rename_to_h2() {
    let md_opts = MarkdownOptions::default();
    let standardized =
        readabilityrs::elements::standardize_all("<h1>Other Heading</h1>", Some("Different Title"));
    let md = readabilityrs::markdown::html_to_markdown(&standardized, &md_opts);
    // Non-matching h1 should become h2
    assert!(md.contains("## Other Heading"));
}

// ── Image standardization ───────────────────────────────────────────

#[test]
fn test_lazy_load_resolution() {
    let html = r#"<img data-src="real.jpg" src="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7" alt="test"/>"#;
    let md = html_to_md(html);
    assert!(md.contains("![test](real.jpg)"));
}

// ── Math standardization ────────────────────────────────────────────

#[test]
fn test_katex_to_markdown() {
    let html = r#"<span class="katex" data-latex="a^2 + b^2">rendered</span>"#;
    let md = html_to_md(html);
    assert!(md.contains("$a^2 + b^2$"));
}

// ── Full pipeline integration ───────────────────────────────────────

#[test]
fn test_full_pipeline_via_readability() {
    let html = r#"
    <html>
    <head><title>Test Article</title></head>
    <body>
        <article>
            <h1>Test Article</h1>
            <p>This is a <strong>test article</strong> with <em>rich formatting</em>.</p>
            <p>It contains multiple paragraphs to meet the character threshold.</p>
            <p>Each paragraph has enough text content to be considered substantial.</p>
            <p>We need several paragraphs to make the readability algorithm happy.</p>
            <p>The content extraction works by scoring paragraphs and selecting.</p>
            <p>Here is yet another paragraph with some more useful content.</p>
            <p>And another paragraph because we need to hit the char threshold.</p>
            <p>Final paragraph with a <a href="https://example.com">link</a>.</p>
        </article>
    </body>
    </html>
    "#;

    let options = ReadabilityOptions::builder()
        .output_markdown(true)
        .char_threshold(100)
        .build();

    let readability = Readability::new(html, None, Some(options)).unwrap();
    let article = readability.parse();

    assert!(article.is_some());
    let article = article.unwrap();

    // HTML content should still be present
    assert!(article.content.is_some());

    // Markdown content should be populated
    assert!(article.markdown_content.is_some());
    let md = article.markdown_content.unwrap();

    assert!(md.contains("**test article**"));
    assert!(md.contains("*rich formatting*"));
    assert!(md.contains("[link](https://example.com)"));
}

#[test]
fn test_markdown_not_generated_by_default() {
    let html = r#"
    <html><body><article>
        <p>Simple content that should pass the char threshold for extraction.</p>
        <p>Adding more content paragraphs to make readability extraction work.</p>
        <p>More text content here to ensure we have enough characters overall.</p>
        <p>And yet another paragraph to be safe about meeting the threshold.</p>
    </article></body></html>
    "#;

    let readability = Readability::new(html, None, None).unwrap();
    let article = readability.parse();

    if let Some(article) = article {
        // Markdown should NOT be generated by default
        assert!(article.markdown_content.is_none());
    }
}

// ── Definition lists ────────────────────────────────────────────────

#[test]
fn test_definition_list() {
    let md = html_to_md("<dl><dt>Term</dt><dd>Definition of the term.</dd></dl>");
    assert!(md.contains("**Term**"));
    assert!(md.contains(": Definition of the term."));
}

// ── Nested formatting ───────────────────────────────────────────────

#[test]
fn test_bold_inside_link() {
    let md = html_to_md(r#"<a href="https://example.com"><strong>bold link</strong></a>"#);
    assert!(md.contains("[**bold link**](https://example.com)"));
}

#[test]
fn test_mixed_inline() {
    let md = html_to_md("<p><strong>bold</strong> and <em>italic</em> and <code>code</code></p>");
    assert!(md.contains("**bold**"));
    assert!(md.contains("*italic*"));
    assert!(md.contains("`code`"));
}

// ════════════════════════════════════════════════════════════════════
// Phase 2: Comprehensive edge-case tests
// ════════════════════════════════════════════════════════════════════

// ── 2.1 Text Escaping ──────────────────────────────────────────────

#[test]
fn test_escape_asterisks_in_text() {
    let md = html_to_md("<p>Rating: *** three stars</p>");
    assert!(md.contains("\\*\\*\\*"));
}

#[test]
fn test_escape_underscores_in_text() {
    let md = html_to_md("<p>file_name_here</p>");
    assert!(md.contains("\\_name\\_"));
}

#[test]
fn test_brackets_in_text_not_escaped() {
    // Brackets are NOT escaped — they only form links when paired as [text](url)
    let md = html_to_md("<p>array[0] = value</p>");
    assert!(md.contains("array[0] = value"), "brackets should not be escaped: {}", md);
}

#[test]
fn test_escape_backslash_in_text() {
    let md = html_to_md("<p>C:\\Users\\file</p>");
    assert!(md.contains("\\\\"));
}

#[test]
fn test_escape_backtick_in_text() {
    let md = html_to_md("<p>use the `grave` accent</p>");
    assert!(md.contains("\\`grave\\`"));
}

#[test]
fn test_escape_tilde_in_text() {
    let md = html_to_md("<p>approximately ~100</p>");
    assert!(md.contains("\\~100"));
}

#[test]
fn test_no_escape_inside_code_block() {
    let md = html_to_md("<pre><code>let x = a * b + c[0];</code></pre>");
    assert!(md.contains("let x = a * b + c[0];"));
    assert!(!md.contains("\\*"));
}

// ── 2.2 Nested Formatting ──────────────────────────────────────────

#[test]
fn test_bold_inside_italic() {
    let md = html_to_md("<p><em><strong>bold italic</strong></em></p>");
    assert!(md.contains("***bold italic***") || md.contains("*__bold italic__*"));
}

#[test]
fn test_code_inside_heading() {
    let md = html_to_md("<h2>Using <code>println!</code></h2>");
    assert!(md.contains("## Using `println\\!`") || md.contains("## Using `println!`"));
}

#[test]
fn test_link_inside_heading() {
    let md = html_to_md(r#"<h3><a href="https://example.com">Link Heading</a></h3>"#);
    assert!(md.contains("### [Link Heading](https://example.com)"));
}

#[test]
fn test_image_inside_link() {
    let md = html_to_md(r#"<a href="https://example.com"><img src="icon.png" alt="icon"/></a>"#);
    assert!(md.contains("[![icon](icon.png)](https://example.com)"));
}

#[test]
fn test_bold_inside_list_item() {
    let md = html_to_md("<ul><li><strong>bold item</strong></li></ul>");
    assert!(md.contains("- **bold item**"));
}

#[test]
fn test_deeply_nested_inline() {
    let md = html_to_md("<p><strong><em><del>deep</del></em></strong></p>");
    assert!(md.contains("**") && md.contains("*") && md.contains("~~deep~~"));
}

// ── 2.3 Empty Elements ─────────────────────────────────────────────

#[test]
fn test_empty_heading() {
    let md = html_to_md("<h2></h2>");
    assert!(!md.contains("##"));
}

#[test]
fn test_empty_paragraph() {
    let md = html_to_md("<p></p>");
    assert!(md.trim().is_empty());
}

#[test]
fn test_empty_bold() {
    let md = html_to_md("<p><strong></strong></p>");
    assert!(!md.contains("****"));
}

#[test]
fn test_empty_link_text_uses_url() {
    let md = html_to_md(r#"<a href="https://example.com"></a>"#);
    assert!(md.contains("https://example.com"));
}

#[test]
fn test_empty_list() {
    let md = html_to_md("<ul></ul>");
    assert!(md.trim().is_empty());
}

#[test]
fn test_empty_table() {
    let md = html_to_md("<table></table>");
    assert!(md.trim().is_empty() || !md.contains("|---|"));
}

#[test]
fn test_empty_blockquote() {
    let md = html_to_md("<blockquote></blockquote>");
    let trimmed = md.trim();
    assert!(trimmed.is_empty() || trimmed == ">");
}

#[test]
fn test_empty_code_block() {
    let md = html_to_md("<pre><code></code></pre>");
    assert!(md.contains("```"));
}

// ── 2.4 Whitespace Edge Cases ──────────────────────────────────────

#[test]
fn test_multiple_spaces() {
    let md = html_to_md("<p>hello    world</p>");
    assert!(md.contains("hello") && md.contains("world"));
}

#[test]
fn test_nbsp_handling() {
    let md = html_to_md("<p>hello\u{00a0}world</p>");
    assert!(md.contains("hello") && md.contains("world"));
}

#[test]
fn test_crlf_in_text() {
    let md = html_to_md("<p>line1\r\nline2</p>");
    assert!(md.contains("line1") && md.contains("line2"));
}

#[test]
fn test_trailing_whitespace_trimmed_in_output() {
    let md = html_to_md("<p>text   </p><p>more text   </p>");
    for line in md.lines() {
        assert_eq!(line, line.trim_end(), "Trailing whitespace found in: {:?}", line);
    }
}

// ── 2.5 Complex Links ──────────────────────────────────────────────

#[test]
fn test_link_no_href() {
    let md = html_to_md("<a>just text</a>");
    assert!(md.contains("just text"));
    assert!(!md.contains("]("));
}

#[test]
fn test_link_fragment_only() {
    let md = html_to_md(r##"<a href="#section">Section</a>"##);
    assert!(md.contains("[Section](#section)"));
}

#[test]
fn test_link_relative_url() {
    let md = html_to_md(r#"<a href="/page/sub">relative</a>"#);
    assert!(md.contains("[relative](/page/sub)"));
}

#[test]
fn test_link_special_chars_in_url() {
    let md = html_to_md(r#"<a href="https://example.com/path?q=a&amp;b=c">query</a>"#);
    assert!(md.contains("[query]"));
    assert!(md.contains("example.com"));
}

#[test]
fn test_nested_links_no_double_brackets() {
    let md = html_to_md(r#"<a href="outer"><a href="inner">text</a></a>"#);
    assert!(!md.contains("[["));
}

// ── 2.6 Complex Images ─────────────────────────────────────────────

#[test]
fn test_image_empty_alt() {
    let md = html_to_md(r#"<img src="photo.jpg" alt=""/>"#);
    assert!(md.contains("![](photo.jpg)"));
}

#[test]
fn test_image_no_alt() {
    let md = html_to_md(r#"<img src="photo.jpg"/>"#);
    assert!(md.contains("![](photo.jpg)"));
}

#[test]
fn test_image_no_src() {
    let md = html_to_md(r#"<img alt="test"/>"#);
    // No src — should produce no image markdown
    assert!(!md.contains("![test]()"));
}

#[test]
fn test_image_special_chars_in_alt() {
    let md = html_to_md(r#"<img src="photo.jpg" alt="a photo [nice]"/>"#);
    assert!(md.contains("photo.jpg"));
}

// ── 2.7 Complex Code Blocks ────────────────────────────────────────

#[test]
fn test_code_block_with_triple_backticks() {
    let md = html_to_md(r#"<pre><code>show ```backticks``` here</code></pre>"#);
    assert!(md.contains("~~~~"));
    assert!(md.contains("```backticks```"));
}

#[test]
fn test_code_block_empty() {
    let md = html_to_md("<pre><code></code></pre>");
    assert!(md.contains("```"));
}

#[test]
fn test_code_block_tilde_fence_option() {
    let md_opts = MarkdownOptions {
        code_fence: '~',
        ..MarkdownOptions::default()
    };
    let standardized = readabilityrs::elements::standardize_all(
        "<pre><code>code here</code></pre>",
        None,
    );
    let md = readabilityrs::markdown::html_to_markdown(&standardized, &md_opts);
    assert!(md.contains("~~~"));
    assert!(!md.contains("```"));
}

#[test]
fn test_pre_without_code_child() {
    let md = html_to_md("<pre>preformatted text</pre>");
    assert!(md.contains("preformatted text"));
    assert!(md.contains("```"));
}

#[test]
fn test_code_block_mixed_line_endings() {
    let md = html_to_md("<pre><code>line1\r\nline2\nline3</code></pre>");
    assert!(md.contains("line1"));
    assert!(md.contains("line2"));
    assert!(md.contains("line3"));
}

// ── 2.8 Complex Tables ─────────────────────────────────────────────

#[test]
fn test_table_pipes_in_cells() {
    let md = html_to_md(
        "<table><thead><tr><th>Name</th></tr></thead>\
         <tbody><tr><td>A | B</td></tr></tbody></table>",
    );
    assert!(md.contains("A \\| B"));
}

#[test]
fn test_table_uneven_columns() {
    let md = html_to_md(
        "<table><thead><tr><th>A</th><th>B</th><th>C</th></tr></thead>\
         <tbody><tr><td>1</td><td>2</td></tr></tbody></table>",
    );
    assert!(md.contains("| A"));
    assert!(md.contains("| 1"));
}

#[test]
fn test_table_complex_preserved_as_html() {
    let md = html_to_md(
        r#"<table><tr><td colspan="2">merged</td></tr></table>"#,
    );
    assert!(md.contains("colspan"));
}

#[test]
fn test_table_no_headers() {
    let md = html_to_md(
        "<table><tbody><tr><td>a</td><td>b</td></tr><tr><td>c</td><td>d</td></tr></tbody></table>",
    );
    assert!(md.contains("|"));
}

// ── 2.9 Nested Lists ───────────────────────────────────────────────

#[test]
fn test_nested_unordered_list_2_levels() {
    let md = html_to_md("<ul><li>outer<ul><li>inner</li></ul></li></ul>");
    assert!(md.contains("- outer"));
    assert!(md.contains("  - inner"));
}

#[test]
fn test_nested_list_3_levels() {
    let md = html_to_md(
        "<ul><li>L1<ul><li>L2<ul><li>L3</li></ul></li></ul></li></ul>",
    );
    assert!(md.contains("- L1"));
    assert!(md.contains("  - L2"));
    assert!(md.contains("    - L3"));
}

#[test]
fn test_mixed_nested_lists() {
    let md = html_to_md("<ul><li>bullet<ol><li>numbered</li></ol></li></ul>");
    assert!(md.contains("- bullet"));
    assert!(md.contains("  1. numbered"));
}

#[test]
fn test_list_item_with_paragraph() {
    let md = html_to_md("<ul><li><p>paragraph item</p></li></ul>");
    assert_eq!(md.trim(), "- paragraph item");
}

// ── 2.10 Footnotes in Converter ────────────────────────────────────

#[test]
fn test_footnote_ref_via_sup() {
    let md = html_to_md(
        r##"<p>Text<sup id="fnref1"><a href="#fn:1" class="footnote">1</a></sup></p>
        <div id="footnotes"><ol><li class="footnote" id="fn:1">Footnote content.</li></ol></div>"##,
    );
    assert!(md.contains("[^1]") || md.contains("[^"));
}

#[test]
fn test_multiple_footnotes() {
    let md = html_to_md(
        r##"<p>A<sup id="fnref1"><a href="#fn:1" class="footnote">1</a></sup> and
        B<sup id="fnref2"><a href="#fn:2" class="footnote">2</a></sup></p>
        <div id="footnotes"><ol>
            <li class="footnote" id="fn:1">First.</li>
            <li class="footnote" id="fn:2">Second.</li>
        </ol></div>"##,
    );
    assert!(md.contains("[^1]") || md.contains("[^"));
    assert!(md.contains("First") && md.contains("Second"));
}

// ── 2.11 MarkdownOptions Variations ────────────────────────────────

#[test]
fn test_setext_heading_style() {
    let md_opts = MarkdownOptions {
        heading_style: HeadingStyle::Setext,
        ..MarkdownOptions::default()
    };
    let standardized = readabilityrs::elements::standardize_all(
        "<h2>Subtitle</h2>",
        None,
    );
    let md = readabilityrs::markdown::html_to_markdown(&standardized, &md_opts);
    assert!(md.contains("Subtitle\n---") || md.contains("Subtitle\n-"));
}

#[test]
fn test_custom_bullet_char() {
    let md_opts = MarkdownOptions {
        bullet_char: '+',
        ..MarkdownOptions::default()
    };
    let standardized = readabilityrs::elements::standardize_all(
        "<ul><li>item</li></ul>",
        None,
    );
    let md = readabilityrs::markdown::html_to_markdown(&standardized, &md_opts);
    assert!(md.contains("+ item"));
}

#[test]
fn test_underscore_emphasis() {
    let md_opts = MarkdownOptions {
        emphasis_delimiter: '_',
        strong_delimiter: "__".to_string(),
        ..MarkdownOptions::default()
    };
    let standardized = readabilityrs::elements::standardize_all(
        "<p><em>italic</em> and <strong>bold</strong></p>",
        None,
    );
    let md = readabilityrs::markdown::html_to_markdown(&standardized, &md_opts);
    assert!(md.contains("_italic_"));
    assert!(md.contains("__bold__"));
}

#[test]
fn test_reference_link_style() {
    let md_opts = MarkdownOptions {
        link_style: LinkStyle::Reference,
        ..MarkdownOptions::default()
    };
    let standardized = readabilityrs::elements::standardize_all(
        r#"<p><a href="https://example.com">click</a></p>"#,
        None,
    );
    let md = readabilityrs::markdown::html_to_markdown(&standardized, &md_opts);
    assert!(md.contains("[click]["));
    assert!(md.contains("https://example.com"));
}

#[test]
fn test_tilde_code_fence() {
    let md_opts = MarkdownOptions {
        code_fence: '~',
        ..MarkdownOptions::default()
    };
    let standardized = readabilityrs::elements::standardize_all(
        "<pre><code>code</code></pre>",
        None,
    );
    let md = readabilityrs::markdown::html_to_markdown(&standardized, &md_opts);
    assert!(md.contains("~~~"));
    assert!(!md.contains("```"));
}

// ── 2.12 Post-Processing Edge Cases ────────────────────────────────

#[test]
fn test_utf8_content_preserved() {
    let md = html_to_md("<p>Héllo wörld café naïve</p>");
    assert!(md.contains("Héllo") && md.contains("wörld") && md.contains("café"));
}

#[test]
fn test_image_empty_link_preserved() {
    let md = html_to_md(r#"<p><img src="photo.jpg" alt=""/></p>"#);
    assert!(md.contains("![](photo.jpg)"));
}

#[test]
fn test_consecutive_newlines_collapsed() {
    let md = html_to_md("<p>A</p><p></p><p></p><p>B</p>");
    assert!(!md.contains("\n\n\n"));
}

#[test]
fn test_no_trailing_whitespace_anywhere() {
    let md = html_to_md(
        "<h2>Title</h2><p>Paragraph with <strong>bold</strong> text.</p>\
         <ul><li>item one</li><li>item two</li></ul>\
         <blockquote><p>A quote.</p></blockquote>",
    );
    for line in md.lines() {
        assert_eq!(line, line.trim_end(), "Trailing whitespace in: {:?}", line);
    }
}

// ════════════════════════════════════════════════════════════════════
// Phase 3: Deep nesting / interaction tests
// ════════════════════════════════════════════════════════════════════

// ── <p> inside <li> ─────────────────────────────────────────────────

#[test]
fn test_p_inside_li_compact() {
    let md = html_to_md("<ul><li><p>text</p></li></ul>");
    assert_eq!(md.trim(), "- text");
}

#[test]
fn test_multiple_p_inside_li() {
    let md = html_to_md("<ul><li><p>first</p><p>second</p></li></ul>");
    assert!(md.contains("- first"));
    assert!(md.contains("second"));
    assert!(!md.contains("\n\n\n"));
}

#[test]
fn test_p_inside_ordered_li() {
    let md = html_to_md("<ol><li><p>item one</p></li><li><p>item two</p></li></ol>");
    assert!(md.contains("1. item one"));
    assert!(md.contains("2. item two"));
}

// ── Block elements inside <blockquote> ──────────────────────────────

#[test]
fn test_blockquote_p_exact_output() {
    let md = html_to_md("<blockquote><p>quoted text</p></blockquote>");
    assert_eq!(md.trim(), "> quoted text");
}

#[test]
fn test_blockquote_heading_and_p() {
    let md = html_to_md("<blockquote><h2>Title</h2><p>text</p></blockquote>");
    let trimmed = md.trim();
    assert!(trimmed.contains("> ## Title"), "should have prefixed heading: {}", trimmed);
    assert!(trimmed.contains("> text"), "should have prefixed text: {}", trimmed);
}

#[test]
fn test_blockquote_code_block() {
    let md = html_to_md(
        r#"<blockquote><pre><code class="language-rust">fn main() {}</code></pre></blockquote>"#,
    );
    assert!(md.contains("> ```rust"), "code fence missing > prefix: {}", md);
    assert!(md.contains("> fn main() {}"), "code body missing > prefix: {}", md);
    assert!(md.contains("> ```"), "closing fence missing > prefix: {}", md);
}

#[test]
fn test_nested_blockquote_with_p() {
    let md = html_to_md("<blockquote><blockquote><p>deep</p></blockquote></blockquote>");
    assert!(md.contains("> > deep"));
}

#[test]
fn test_blockquote_multiple_paragraphs() {
    let md = html_to_md("<blockquote><p>first</p><p>second</p></blockquote>");
    let trimmed = md.trim();
    assert!(trimmed.contains("> first"), "missing first para: {}", trimmed);
    assert!(trimmed.contains("> second"), "missing second para: {}", trimmed);
}

// ── Empty figcaption ────────────────────────────────────────────────

#[test]
fn test_figure_empty_figcaption_preserves_alt() {
    let md = html_to_md(
        r#"<figure><img alt="A nice photo" src="img.jpg"/><figcaption></figcaption></figure>"#,
    );
    assert!(md.contains("![A nice photo](img.jpg)"), "alt text lost: {}", md);
}

// ── Table cells with block content ──────────────────────────────────

#[test]
fn test_table_cell_with_p_single_line() {
    let md = html_to_md(
        "<table><thead><tr><th>H</th></tr></thead>\
         <tbody><tr><td><p>cell text</p></td></tr></tbody></table>",
    );
    assert!(md.contains("| cell text"), "cell content missing: {}", md);
    // Each table row line should be single-line (no embedded newlines)
    for line in md.lines() {
        if line.starts_with('|') && line.ends_with('|') {
            assert!(!line[1..line.len()-1].contains('\n'), "multiline cell: {}", line);
        }
    }
}

#[test]
fn test_table_cell_with_link() {
    let md = html_to_md(
        r#"<table><thead><tr><th>Name</th></tr></thead>
        <tbody><tr><td><a href="https://example.com">Link</a></td></tr></tbody></table>"#,
    );
    assert!(md.contains("[Link](https://example.com)"));
}

// ── Additional real-world smoke tests ───────────────────────────────

#[test]
fn test_real_world_ars_1() {
    let html = std::fs::read_to_string("tests/test-pages/ars-1/expected.html")
        .expect("ars-1/expected.html should exist");
    let md_opts = MarkdownOptions::default();
    let standardized = readabilityrs::elements::standardize_all(&html, None);
    let md = readabilityrs::markdown::html_to_markdown(&standardized, &md_opts);

    assert!(!md.trim().is_empty(), "ars-1 should produce non-empty markdown");
    assert!(md.contains("]("), "ars-1 should contain links");
    assert!(!md.contains("\n\n\n"), "ars-1 should have no triple newlines");
    for line in md.lines() {
        assert_eq!(line, line.trim_end(), "ars-1 trailing whitespace: {:?}", line);
    }
}

#[test]
fn test_real_world_buzzfeed_1() {
    let html = std::fs::read_to_string("tests/test-pages/buzzfeed-1/expected.html")
        .expect("buzzfeed-1/expected.html should exist");
    let md_opts = MarkdownOptions::default();
    let standardized = readabilityrs::elements::standardize_all(&html, None);
    let md = readabilityrs::markdown::html_to_markdown(&standardized, &md_opts);

    assert!(!md.trim().is_empty(), "buzzfeed-1 should produce non-empty markdown");
    assert!(!md.contains("\n\n\n"), "buzzfeed-1 should have no triple newlines");
    for line in md.lines() {
        assert_eq!(line, line.trim_end(), "buzzfeed-1 trailing whitespace: {:?}", line);
    }
}

// ════════════════════════════════════════════════════════════════════
// Phase 4: Full 130-page validation suite
// ════════════════════════════════════════════════════════════════════

/// Runs the markdown converter on ALL 130 mozilla test pages and checks
/// 10 quality invariants on each. This is the ultimate validation test.
#[test]
fn test_all_130_pages_quality_audit() {
    let md_opts = MarkdownOptions::default();
    let test_dir = "tests/test-pages";
    let mut entries: Vec<_> = std::fs::read_dir(test_dir)
        .expect("test-pages dir")
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut total = 0;
    let mut failures: Vec<String> = Vec::new();

    for entry in &entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let expected_path = format!("{}/{}/expected.html", test_dir, name);
        let html = match std::fs::read_to_string(&expected_path) {
            Ok(h) => h,
            Err(_) => continue,
        };

        total += 1;
        let standardized = readabilityrs::elements::standardize_all(&html, None);
        let md = readabilityrs::markdown::html_to_markdown(&standardized, &md_opts);
        let lines: Vec<&str> = md.lines().collect();

        // 1. No triple newlines
        if md.contains("\n\n\n") {
            failures.push(format!("{}: TRIPLE_NEWLINES", name));
        }

        // 2. No trailing whitespace
        for (i, line) in lines.iter().enumerate() {
            if *line != line.trim_end() {
                failures.push(format!("{}: TRAILING_WS line {}", name, i + 1));
                break;
            }
        }

        // 3. Non-empty output
        if md.trim().is_empty() {
            failures.push(format!("{}: EMPTY_OUTPUT", name));
        }

        // 4. No garbled blockquotes (3+ consecutive empty > lines)
        for i in 0..lines.len().saturating_sub(2) {
            if lines[i].trim().chars().all(|c| c == '>')
                && lines[i + 1].trim().chars().all(|c| c == '>')
                && lines[i + 2].trim().chars().all(|c| c == '>')
                && !lines[i].trim().is_empty()
            {
                failures.push(format!("{}: GARBLED_BLOCKQUOTE line {}", name, i + 1));
                break;
            }
        }

        // 5. No bare bullets (bullet with no text)
        for i in 0..lines.len().saturating_sub(1) {
            let l = lines[i].trim();
            if (l == "-" || l == "+" || l == "*")
                && lines.get(i + 1).map(|l| l.trim().is_empty()).unwrap_or(false)
            {
                failures.push(format!("{}: BARE_BULLET line {}", name, i + 1));
                break;
            }
        }

        // 6. No double empty blockquote lines
        for i in 0..lines.len().saturating_sub(1) {
            let a = lines[i].trim();
            let b = lines[i + 1].trim();
            if !a.is_empty() && a.chars().all(|c| c == '>')
                && !b.is_empty() && b.chars().all(|c| c == '>')
            {
                failures.push(format!("{}: DOUBLE_EMPTY_QUOTE line {}", name, i + 1));
                break;
            }
        }

        // 7. No control characters
        for ch in md.chars() {
            if ch.is_control() && ch != '\n' && ch != '\t' && ch != '\r' {
                failures.push(format!("{}: CONTROL_CHAR U+{:04X}", name, ch as u32));
                break;
            }
        }

        // 8. No empty URLs
        if md.contains("]()") {
            failures.push(format!("{}: EMPTY_URL", name));
        }

        // 9. Table alignment (all rows same pipe count)
        if md.contains("|---") {
            let table_lines: Vec<&str> = lines.iter()
                .filter(|l| l.trim().starts_with('|') && l.trim().ends_with('|'))
                .copied()
                .collect();
            if table_lines.len() >= 2 {
                let expected_pipes = table_lines[0].matches('|').count();
                for (i, tl) in table_lines.iter().enumerate().skip(1) {
                    if tl.matches('|').count() != expected_pipes {
                        failures.push(format!("{}: TABLE_MISALIGN row {}", name, i));
                        break;
                    }
                }
            }
        }

        // 10. No escaped chars inside code blocks
        let mut in_code = false;
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.starts_with("```") || t.starts_with("~~~~") {
                in_code = !in_code;
                continue;
            }
            if in_code && (line.contains("\\*") || line.contains("\\_") || line.contains("\\[")) {
                failures.push(format!("{}: ESCAPED_IN_CODE line {}", name, i + 1));
                break;
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Quality audit failed on {}/{} pages:\n{}",
        failures.len(),
        total,
        failures.join("\n")
    );
}

// ════════════════════════════════════════════════════════════════════
// Phase 5: Individual real-world page tests
// ════════════════════════════════════════════════════════════════════

#[test]
fn test_real_world_001() {
    let html = std::fs::read_to_string("tests/test-pages/001/expected.html")
        .expect("001/expected.html should exist");
    let md_opts = MarkdownOptions::default();
    let standardized = readabilityrs::elements::standardize_all(&html, None);
    let md = readabilityrs::markdown::html_to_markdown(&standardized, &md_opts);

    assert!(!md.trim().is_empty(), "001 should produce non-empty markdown");
    assert!(!md.contains("\n\n\n"), "001 should have no triple newlines");
    for line in md.lines() {
        assert_eq!(line, line.trim_end(), "001 trailing whitespace: {:?}", line);
    }
}

#[test]
fn test_real_world_bbc_1() {
    let html = std::fs::read_to_string("tests/test-pages/bbc-1/expected.html")
        .expect("bbc-1/expected.html should exist");
    let md_opts = MarkdownOptions::default();
    let standardized = readabilityrs::elements::standardize_all(&html, None);
    let md = readabilityrs::markdown::html_to_markdown(&standardized, &md_opts);

    assert!(!md.trim().is_empty(), "bbc-1 should produce non-empty markdown");
    // BBC article has links
    assert!(md.contains("]("), "bbc-1 should contain links");
    assert!(!md.contains("\n\n\n"), "bbc-1 should have no triple newlines");
    for line in md.lines() {
        assert_eq!(line, line.trim_end(), "bbc-1 trailing whitespace: {:?}", line);
    }
}

#[test]
fn test_real_world_wikipedia_2() {
    let html = std::fs::read_to_string("tests/test-pages/wikipedia-2/expected.html")
        .expect("wikipedia-2/expected.html should exist");
    let md_opts = MarkdownOptions::default();
    let standardized = readabilityrs::elements::standardize_all(&html, None);
    let md = readabilityrs::markdown::html_to_markdown(&standardized, &md_opts);

    assert!(!md.trim().is_empty(), "wikipedia-2 should produce non-empty markdown");
    // Wikipedia has many links
    assert!(md.contains("]("), "wikipedia-2 should contain links");
    assert!(!md.contains("\n\n\n"), "wikipedia-2 should have no triple newlines");
    for line in md.lines() {
        assert_eq!(line, line.trim_end(), "wikipedia-2 trailing whitespace: {:?}", line);
    }
}

// ════════════════════════════════════════════════════════════════════
// Phase 6: Improvement tests
// ════════════════════════════════════════════════════════════════════

// ── Link & image title preservation ─────────────────────────────────

#[test]
fn test_link_with_title_attribute() {
    let md = html_to_md(r#"<a href="https://example.com" title="Visit Example">click</a>"#);
    assert!(md.contains("[click](https://example.com \"Visit Example\")"), "title missing: {}", md);
}

#[test]
fn test_link_without_title() {
    let md = html_to_md(r#"<a href="https://example.com">click</a>"#);
    assert!(md.contains("[click](https://example.com)"));
    assert!(!md.contains("\"\""));
}

#[test]
fn test_image_with_title_attribute() {
    let md = html_to_md(r#"<img src="photo.jpg" alt="A photo" title="My Photo"/>"#);
    assert!(md.contains("![A photo](photo.jpg \"My Photo\")"), "title missing: {}", md);
}

// ── Superscript / subscript ─────────────────────────────────────────

#[test]
fn test_superscript() {
    let md = html_to_md("<p>E=mc<sup>2</sup></p>");
    assert!(md.contains("^2^"), "superscript missing: {}", md);
}

#[test]
fn test_subscript() {
    let md = html_to_md("<p>H<sub>2</sub>O</p>");
    assert!(md.contains("~2~"), "subscript missing: {}", md);
}

// ── Video/audio with <source> children ──────────────────────────────

#[test]
fn test_video_with_source_child() {
    let md = html_to_md(r#"<video><source src="movie.mp4" type="video/mp4"/></video>"#);
    assert!(md.contains("[Video](movie.mp4)"), "video source not found: {}", md);
}

#[test]
fn test_audio_with_source_child() {
    let md = html_to_md(r#"<audio><source src="song.mp3" type="audio/mpeg"/></audio>"#);
    assert!(md.contains("[Audio](song.mp3)"), "audio source not found: {}", md);
}

// ── Details/summary preserved as HTML ───────────────────────────────

#[test]
fn test_details_preserved_as_html() {
    let md = html_to_md(r#"<details><summary>Click</summary><p>Hidden</p></details>"#);
    assert!(md.contains("<details>"), "details not preserved: {}", md);
    assert!(md.contains("<summary>"), "summary not preserved: {}", md);
}

// ── Srcset decimal density ──────────────────────────────────────────

#[test]
fn test_srcset_decimal_density() {
    let srcset = "small.jpg 1x, medium.jpg 1.5x, large.jpg 2x";
    let result = readabilityrs::elements::images::pick_best_srcset(srcset);
    // 2x is 2.0, 1.5x is 1.5 — should pick 2x as largest
    assert_eq!(result, Some("large.jpg".to_string()));
}
