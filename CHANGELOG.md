# Changelog

All notable changes to **readabilityrs** are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Each release section below is what ships as the GitHub Release notes.

## [Unreleased]

A correctness and infrastructure release. A remotely triggerable panic is fixed, the Mozilla test suite becomes a real regression gate, opt-in output sanitization is added, and the project gains CI.

- **Fixed a panic on non-ASCII bylines**: `strip_trailing_datetime_clause` computed a byte offset on a lowercased copy of the byline and then used it to slice the original string. Because `to_lowercase` can change a string's byte length (the Turkish dotted capital `İ` is one example), the offset could land inside a character and panic, aborting extraction. Reachable from any page whose byline mixes such a character with a trailing timestamp, so any caller parsing untrusted pages was exposed. Separators are now located in the original string.
- **The Mozilla test suite actually asserts**: both integration tests were marked `#[ignore]` and contained no assertions at all, so the 130-page corpus could not fail a build and was skipped by `cargo test` entirely. The suite now runs by default and asserts on metadata and on extracted body length, with the known divergences listed by name in `KNOWN_METADATA_DIVERGENCES` and `KNOWN_CONTENT_DIVERGENCES`. A case has to be listed there to be allowed to differ.
- **Corrected the compatibility claim**: the README stated 122 of 130 Mozilla cases passing. The real figure, measured once the suite began asserting, is 119. The number had drifted with no test to hold it in place, which is precisely the gap the change above closes.
- **Added `sanitize_content`**: an opt-in option that drops event-handler attributes (`onerror`, `onclick`, and similar) and dangerous URL schemes (`javascript:`, `vbscript:`, and non-image `data:`) while serializing extracted HTML. Off by default, so existing output is unchanged. Scheme comparison strips embedded whitespace and control characters first, since browsers do the same while parsing a URL. It filters attributes rather than whole elements and is a harm reducer, not a substitute for a real sanitizer.
- **Documented the sanitization contract**: extracted HTML is untrusted and unsanitized by default, matching Readability.js. This is now stated in the crate documentation and in a new [SECURITY.md](SECURITY.md), together with the trust model and known limitations.
- **Fixed double-escaped ampersands in image URLs**: the Markdown standardization pass runs over already-escaped HTML, and re-escaped every `&`, so a lazy-loaded image URL containing `&amp;` became `&amp;amp;` and the link broke. Escaping now leaves existing character references alone, with a bounded lookahead so the scan cannot be turned into a performance problem.
- **Added CI**: GitHub Actions now runs `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo build`, and `cargo test` on every push and pull request. Nothing enforced any of these before.
- **Cleaned the lint baseline**: eleven clippy warnings resolved and the tree formatted with `cargo fmt`. Two of the fixes touched library code (`sort_by_key` in the extraction fallback, and the footnote counter loop) and are behavior-preserving; the rest were in tests.
- **Closed a development advisory**: `crossbeam-epoch` updated past RUSTSEC-2026-0204. It was reachable only through `criterion`, a development dependency, and never affected the published library.

## [0.1.3] - 2026-04-05

Optional HTML-to-Markdown conversion, closing [#18](https://github.com/theiskaa/readabilityrs/issues/18).

Enable it with `output_markdown(true)` on the options builder and the parsed `Article` gains a `markdown_content` field alongside the existing HTML `content`. Disabled by default, with no change to existing behavior.

A content standardization pipeline normalizes vendor-specific HTML before conversion. It covers syntax-highlighted code blocks (Prism, Shiki, rehype, WordPress SyntaxHighlighter, GitHub), lazy-loaded images, permalink anchors in headings, footnotes from various CMSs, and rendered math from MathJax and KaTeX. Output formatting is configurable through `MarkdownOptions`: heading style, bullet character, code fence, emphasis delimiters, and link style.

**New public API:**

- `ReadabilityOptions::output_markdown`, to enable Markdown generation
- `ReadabilityOptions::markdown_options`, to configure formatting
- `Article::markdown_content`, the Markdown output (`None` when disabled)
- `MarkdownOptions`, the formatting configuration struct
- `elements::standardize_all()`, standalone HTML standardization
- `markdown::html_to_markdown()`, standalone HTML-to-Markdown conversion

Tested against all 130 Mozilla test pages with ten quality invariants each, for 294 tests in total (153 unit, 117 integration, 24 doc).

## [0.1.2] - 2026-02-02

**Fixed**

- **Comment sections are no longer included in extracted content**: comment-related patterns (`comment`, `disqus`, `remark`, `replies`, `respond`) are now detected before the 600-character length check, so user-generated content is filtered out rather than surviving on length alone.
- **Invisible text caused by inline styles**: text carrying styles such as `color: white` is now visible after extraction.
- **Excessive whitespace and empty lines**: consecutive newlines and runs of spaces are normalized, removing the large gaps that appeared between content sections.
- **Malformed HTML tags**: style attributes that could break the surrounding structure are removed.

**Added**

- `clean_styles`, which removes `style`, `align`, `bgcolor`, `valign`, and other presentational attributes from extracted content. Enabled by default.
- `clean_whitespace`, which normalizes excessive whitespace and removes empty paragraphs. Enabled by default.

Both can be turned off through the builder:

```rust
let options = ReadabilityOptions::builder()
    .clean_styles(false)
    .clean_whitespace(false)
    .build();
```

## [0.1.1] - 2026-01-27

- **Added `remove_title_from_content`**: strips a heading matching the article title from the extracted body, along with the whitespace and empty elements left behind.
- **Attribute values are escaped on output**: values containing quotes or angle brackets previously broke the attribute boundary, so anything re-parsing the extracted HTML saw a mangled element.
- **`ReaderableOptions` is public**, so `is_probably_readerable` can be called with custom thresholds.
- **Article images are extracted into metadata.**
- **URL validation now uses `url::Url::parse`**, giving accurate rejection of malformed base URLs instead of a hand-rolled check.
- **Benchmarks added**, including Node scripts that run Mozilla's Readability.js over the same documents for a side-by-side comparison.
- **Documentation comments added across the crate**, and `DIV_TO_P_ELEMS` and `PHRASING_ELEMS` moved from lazily allocated sets to static slices.

## [0.1.0] - 2025-11-25

Initial release. A Rust port of Mozilla's Readability.js covering article extraction, content cleaning, and metadata handling, with document preprocessing before scoring and integration tests built on Mozilla's own test suite.

[Unreleased]: https://github.com/theiskaa/readabilityrs/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/theiskaa/readabilityrs/releases/tag/v0.1.3
[0.1.2]: https://github.com/theiskaa/readabilityrs/releases/tag/v0.1.2
[0.1.1]: https://github.com/theiskaa/readabilityrs/releases/tag/v0.1.1
[0.1.0]: https://github.com/theiskaa/readabilityrs/releases/tag/v0.1.0
