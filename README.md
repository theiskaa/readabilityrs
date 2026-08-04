# Readability.rs

[![CI](https://github.com/theiskaa/readabilityrs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/theiskaa/readabilityrs/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/readabilityrs)](https://crates.io/crates/readabilityrs)
[![Documentation](https://img.shields.io/docsrs/readabilityrs)](https://docs.rs/readabilityrs)
[![License](https://img.shields.io/crates/l/readabilityrs)](LICENSE)
[![Downloads](https://img.shields.io/crates/d/readabilityrs)](https://crates.io/crates/readabilityrs)

readabilityrs pulls the article out of a web page. It is a Rust port of [Mozilla's Readability.js](https://github.com/mozilla/readability), the algorithm behind Firefox Reader View: hand it a page of HTML and it returns the title, byline, body, excerpt, site name, language, and publication time. Navigation, ads, related-article rails, and the rest of the page furniture stay behind.

It passes 119 of the 130 cases in Mozilla's own test suite. The 11 differences are editorial rather than failures: they are cases where this implementation declines a byline lifted from a related-article sidebar, prefers an author name over a timestamp, or picks a different paragraph for the excerpt. Each one is named in the test file, so a new regression cannot hide among them.

Alongside the cleaned HTML it can emit Markdown. A standardization pass runs first and rewrites vendor-specific markup into canonical form: syntax-highlighted code from Prism, Shiki, rehype, WordPress SyntaxHighlighter and GitHub; lazy-loaded images; permalink anchors in headings; footnote conventions from several CMSs; and rendered math from MathJax and KaTeX. Both halves of that pipeline are public, so the Markdown converter can be used on its own against any HTML.

## Install

```bash
cargo add readabilityrs
```

Or in `Cargo.toml`:

```toml
[dependencies]
readabilityrs = "0.1.3"
```

## Usage

Construct a `Readability` with the HTML, an optional base URL, and optional settings, then call `parse()`. It returns `Option<Article>`: `None` when the page has no article worth extracting.

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

`Article` carries the cleaned HTML in `content`, the same text without markup in `text_content`, and the raw winning subtree before cleanup in `raw_content`. Metadata comes from JSON-LD first, then OpenGraph, Twitter Cards, Dublin Core, and finally plain meta tags.

The base URL is validated and rejected if malformed, but relative links inside the extracted HTML are not currently rewritten against it. Resolve them yourself if the output will be rendered somewhere other than the original page.

## Markdown output

Enable `output_markdown(true)` and the returned `Article` carries a `markdown_content` field. The HTML output is unaffected; Markdown is an addition rather than a replacement.

```rust
use readabilityrs::{Readability, ReadabilityOptions};

let options = ReadabilityOptions::builder()
    .output_markdown(true)
    .build();

let readability = Readability::new(&html, None, Some(options))?;
let markdown = readability.parse().and_then(|a| a.markdown_content);
```

Heading style, bullet character, code fence character, emphasis delimiters, and inline versus reference links are all configurable through `MarkdownOptions`. To convert HTML that has not been through extraction, call `elements::standardize_all` and then `markdown::html_to_markdown` directly.

## Configuration

`ReadabilityOptions` is built with a builder. The settings that affect extraction are the character threshold, the number of top candidates considered, JSON-LD parsing, the link density modifier, inline style and whitespace cleanup, title removal, and Markdown output.

```rust
use readabilityrs::{Readability, ReadabilityOptions};

let options = ReadabilityOptions::builder()
    .debug(true)
    .char_threshold(500)
    .nb_top_candidates(5)
    .disable_json_ld(false)
    .link_density_modifier(0.0)
    .build();

let readability = Readability::new(&html, None, Some(options))?;
```

Four further options exist on the builder but are not yet wired into the parser and currently have no effect: `max_elems_to_parse`, `keep_classes`, `classes_to_preserve`, and `allowed_video_regex`. Do not rely on `max_elems_to_parse` as an input-size guard.

## Error handling

`Readability::new` returns a `Result`, since an invalid base URL is rejected up front. `parse()` returns an `Option` rather than a `Result`, because a page with no article is an ordinary outcome and not an error.

```rust
use readabilityrs::{Readability, ReadabilityError};

fn extract_article(html: &str, url: &str) -> Result<String, ReadabilityError> {
    let readability = Readability::new(html, Some(url), None)?;
    let article = readability.parse().ok_or(ReadabilityError::NoContentFound)?;
    Ok(article.content.unwrap_or_default())
}
```

## Benchmarks

Measured against Mozilla's Readability.js on identical documents, Apple Silicon.

| Test case  | Size     | Rust      | JavaScript |               |
| ---------- | -------- | --------- | ---------- | ------------- |
| 001        | 12.2 KB  | 36.34 ms  | 9.89 ms    | JS faster     |
| ars-1      | 54.7 KB  | 40.58 ms  | 26.10 ms   | JS faster     |
| medium-1   | 116.8 KB | 68.49 ms  | 37.58 ms   | JS faster     |
| 002        | 138.9 KB | 63.99 ms  | 84.25 ms   | **Rust 1.3x** |
| aclu       | 200.4 KB | 66.50 ms  | 93.10 ms   | **Rust 1.4x** |
| nytimes-1  | 301.9 KB | 58.80 ms  | 157.46 ms  | **Rust 2.7x** |
| guardian-1 | 1.11 MB  | 74.76 ms  | 268.98 ms  | **Rust 3.6x** |
| yahoo-2    | 1.56 MB  | 133.84 ms | 368.21 ms  | **Rust 2.8x** |

The pattern is consistent: V8 and JSDOM win on small trees, and Rust pulls ahead from roughly 150 KB upward, reaching 2 to 4 times faster on documents over a megabyte. Across a batch of 10 documents totalling 1.6 MB, Rust finishes in about 556 ms against JavaScript's 2.3 s.

`cargo bench` runs the Rust side. The JavaScript column comes from the Node harness in `benches/`, which needs its own `npm install` first.

## Test compatibility

```bash
cargo test --test mozilla_test_suite
```

The suite runs all 130 pages from Mozilla's corpus by default and asserts on both metadata and extracted body length, so a regression fails the build. The known divergences are listed by name in `KNOWN_METADATA_DIVERGENCES` and `KNOWN_CONTENT_DIVERGENCES` in `tests/mozilla_test_suite.rs`; a case has to appear there to be allowed to differ.

## Contributing

For information regarding contributions, please refer to [CONTRIBUTING.md](CONTRIBUTING.md) file. Participation is governed by our [Code of Conduct](CODE_OF_CONDUCT.md), and security issues have a private reporting channel described in [SECURITY.md](SECURITY.md). Extracted content is untrusted HTML and is not sanitized by default; that contract is set out in full in [SECURITY.md](SECURITY.md). Release notes are in [CHANGELOG.md](CHANGELOG.md).
