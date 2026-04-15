# Readability.rs

[![Crates.io](https://img.shields.io/crates/v/readabilityrs)](https://crates.io/crates/readabilityrs)
[![Documentation](https://img.shields.io/docsrs/readabilityrs)](https://docs.rs/readabilityrs)
[![License](https://img.shields.io/crates/l/readabilityrs)](LICENSE)
[![Downloads](https://img.shields.io/crates/d/readabilityrs)](https://crates.io/crates/readabilityrs)

A Rust port of [Mozilla's Readability.js](https://github.com/mozilla/readability) — the algorithm behind Firefox's Reader View. Hand it a page of HTML and it pulls out the article itself: title, byline, body, excerpt, and a bit more. Navigation, ads, related-article rails, and the rest of the page furniture get left behind. It passes 122 of 130 cases in Mozilla's own test suite (93.8%).

## Install
Add to your project:

```bash
cargo add readabilityrs
```

Or add to your Cargo.toml:

```toml
[dependencies]
readabilityrs = "0.1.3"
```

## Usage
The library provides a simple API for parsing HTML documents. Create a `Readability` instance with your HTML content, an optional base URL for resolving relative links, and optional configuration settings. Call `parse()` to extract the article and access properties like title, content, author, excerpt, and publication time. The extracted content is returned as clean HTML suitable for display in reader applications.

```rust
use readabilityrs::Readability;

let html = r#"
    <html>
        <head><title>Example Article</title></head>
        <body>
            <article>
                <h1>Article Title</h1>
                <p>This is the main article content.</p>
            </article>
        </body>
    </html>
"#;

let readability = Readability::new(html, None, None)?;
if let Some(article) = readability.parse() {
    println!("Title: {}", article.title.unwrap_or_default());
    println!("Content: {}", article.content.unwrap_or_default());
    println!("Length: {} chars", article.length);
}
```

## How extraction works
Before scoring anything, the document is preprocessed: scripts and styles are dropped, `<noscript>` wrappers around lazy-loaded images are unwrapped, and deprecated elements get normalized. Skipping this step and scoring raw HTML costs roughly 2.3 percentage points of accuracy on the Mozilla suite, so it's on by default.

Scoring follows Mozilla's algorithm — elements are ranked by tag type, text density, link density, and class/id patterns, and the winning subtree becomes the article body.

Metadata is pulled from JSON-LD first, then OpenGraph, Twitter Cards, Dublin Core, and finally plain meta tags, in that priority order. Authors come from `rel="author"` links and common byline patterns; titles have the site name stripped off; excerpts are taken from the first substantial paragraph.

If you pass a base URL when constructing `Readability`, relative `href`s and `src`s in the output get resolved against it — handy when the extracted HTML will be rendered somewhere other than the original page.

## Markdown Output

The library supports optional Markdown output alongside the default cleaned HTML. When enabled via `output_markdown(true)` on the options builder, the parsed `Article` includes a `markdown_content` field containing the article as Markdown. The HTML content remains available as usual — Markdown is an addition, not a replacement.

Before conversion, a content standardization pipeline runs over the cleaned HTML to normalize vendor-specific patterns into canonical forms. This covers syntax-highlighted code blocks from various libraries (Prism, Shiki, rehype, WordPress SyntaxHighlighter, GitHub), lazy-loaded images, permalink anchors in headings, footnote formats from different CMSs, and rendered math from MathJax and KaTeX. The converter then walks the normalized DOM and produces Markdown with configurable formatting — heading style, bullet character, code fence character, emphasis delimiters, and inline vs. reference link style are all adjustable through `MarkdownOptions`.

The Markdown module can also be used standalone without the readability extraction, by calling `elements::standardize_all` and `markdown::html_to_markdown` directly on any HTML string.

## Configuration
Configure parsing behavior through `ReadabilityOptions` using the builder pattern. Options include debug logging, character thresholds, candidate selection, class preservation, and link density scoring.

```rust
use readabilityrs::{Readability, ReadabilityOptions};

let options = ReadabilityOptions::builder()
    .debug(true)
    .char_threshold(500)
    .nb_top_candidates(5)
    .keep_classes(false)
    .classes_to_preserve(vec!["page".to_string()])
    .disable_json_ld(false)
    .link_density_modifier(0.0)
    .build();

let readability = Readability::new(&html, None, Some(options))?;
```

## Error Handling
The library returns `Result` types for operations that can fail. Common errors include invalid URLs and parsing failures.

```rust
use readabilityrs::{Readability, error::ReadabilityError};

fn extract_article(html: &str, url: &str) -> Result<String, ReadabilityError> {
    let readability = Readability::new(html, Some(url), None)?;
    let article = readability.parse().ok_or(ReadabilityError::NoContentFound)?;
    Ok(article.content.unwrap_or_default())
}
```

## Benchmarks

Performance comparison against Mozilla's original Readability.js using identical test documents:

### Single Document Parsing

| Test Case | Size | Rust | JavaScript | Comparison |
|-----------|------|------|------------|------------|
| 001 | 12.2 KB | 36.34 ms | 9.89 ms | JS faster |
| ars-1 | 54.7 KB | 40.58 ms | 26.10 ms | JS faster |
| medium-1 | 116.8 KB | 68.49 ms | 37.58 ms | JS faster |
| 002 | 138.9 KB | 63.99 ms | 84.25 ms | **Rust 1.3x** |
| aclu | 200.4 KB | 66.50 ms | 93.10 ms | **Rust 1.4x** |
| nytimes-1 | 301.9 KB | 58.80 ms | 157.46 ms | **Rust 2.7x** |

### Large Document Parsing

| Test Case | Size | Rust | JavaScript | Comparison |
|-----------|------|------|------------|------------|
| guardian-1 | 1.11 MB | 74.76 ms | 268.98 ms | **Rust 3.6x** |
| yahoo-2 | 1.56 MB | 133.84 ms | 368.21 ms | **Rust 2.8x** |

### Summary

- **Small documents (< 150KB)**: JavaScript is faster due to V8/JSDOM optimizations for small DOM trees
- **Large documents (>= 150KB)**: Rust is **2-4x faster** with better memory efficiency
- **Memory**: JavaScript's batch processing can hit OOM on large documents; Rust handles them consistently
- **Batch processing**: Rust processes 10 documents (1.6MB total) in ~556ms vs JavaScript's ~2.3s (4x faster)

> Benchmarks run on Apple Silicon. Run `cargo bench` to reproduce.

## Test Compatibility

The implementation passes 122 of 130 tests from Mozilla's test suite achieving 93.8% compatibility with full document preprocessing support. The 8 failing tests represent editorial judgment differences rather than implementation errors. Four cases involve more sensible choices in our implementation such as avoiding bylines extracted from related article sidebars and preferring author names over timestamps. Four cases involve subjective paragraph selection for excerpts where both the reference and our implementation make valid choices. This means the results are 93.8% identical to Mozilla's implementation, with the remaining differences being arguable improvements to the extraction logic.

## Contributing
For information regarding contributions, please refer to [CONTRIBUTING.md](CONTRIBUTING.md) file.

## License
Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) file for details.
