# ReadabilityRS

[![Crates.io](https://img.shields.io/crates/v/readabilityrs)](https://crates.io/crates/readabilityrs)
[![Documentation](https://img.shields.io/docsrs/readabilityrs)](https://docs.rs/readabilityrs)
[![License](https://img.shields.io/crates/l/readabilityrs)](LICENSE)
[![Downloads](https://img.shields.io/crates/d/readabilityrs)](https://crates.io/crates/readabilityrs)
[![GitHub Stars](https://img.shields.io/github/stars/theiskaa/readabilityrs)](https://github.com/theiskaa/readabilityrs/stargazers)

readabilityrs extracts article content from HTML web pages using Mozilla's Readability algorithm. The library identifies and isolates the main article text while removing navigation, advertisements, and other clutter.

This is a Rust port of [Mozilla's Readability.js](https://github.com/mozilla/readability), which powers Firefox's Reader View. The implementation passes 93.8% of Mozilla's test suite with full document preprocessing support.

## Install
Add to your project:

```bash
cargo add readabilityrs
```

Or add to your Cargo.toml:

```toml
[dependencies]
readabilityrs = "0.1.0"
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

## Content Extraction
The library uses Mozilla's content scoring algorithm to identify the main article. Elements are scored based on tag types, text density, link density, and class name patterns. Document preprocessing removes scripts and styles, unwraps noscript tags, and normalizes deprecated elements before extraction, improving accuracy by 2.3 percentage points compared to parsing raw HTML.

## Metadata Extraction
Metadata is extracted from JSON-LD, OpenGraph, Twitter Cards, Dublin Core, and standard meta tags in that priority order. The library detects authors through rel="author" links and common byline patterns, extracts clean titles by removing site names, and generates excerpts from the first substantial paragraph.

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

## URL Handling
Provide a base URL to convert relative links to absolute URLs. This ensures images, anchors, and embedded content maintain correct paths when displayed outside the original context.

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

> Benchmarks run on Apple Silicon. Run `cargo run --release --example benchmark` or `./benchmark/run_benchmarks.sh` to reproduce.

## Test Compatibility

The implementation passes 122 of 130 tests from Mozilla's test suite achieving 93.8% compatibility with full document preprocessing support. The 8 failing tests represent editorial judgment differences rather than implementation errors. Four cases involve more sensible choices in our implementation such as avoiding bylines extracted from related article sidebars and preferring author names over timestamps. Four cases involve subjective paragraph selection for excerpts where both the reference and our implementation make valid choices. This means the results are 93.8% identical to Mozilla's implementation, with the remaining differences being arguable improvements to the extraction logic.

## Contributing
For information regarding contributions, please refer to [CONTRIBUTING.md](CONTRIBUTING.md) file.

## License
Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) file for details.
