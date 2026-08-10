# Changelog

All notable changes to **readabilityrs** are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Each release section below is what ships as the GitHub Release notes.

## [Unreleased]

## [0.1.4] - 2026-08-10

A correctness, security, and infrastructure release. Two remotely triggerable panics and a stack overflow are fixed, three paths that silently deleted or corrupted extracted content are closed, Markdown output is no longer injectable, the Mozilla test suite becomes a real regression gate, opt-in output sanitization is added, and the project gains CI.

The minimum supported Rust version is now **1.83**, and extraction output changes for some pages — see the two notes at the end of this section before upgrading.

- **Fixed a panic on non-ASCII bylines**: `strip_trailing_datetime_clause` computed a byte offset on a lowercased copy of the byline and then used it to slice the original string. Because `to_lowercase` can change a string's byte length (the Turkish dotted capital `İ` is one example), the offset could land inside a character and panic, aborting extraction. Reachable from any page whose byline mixes such a character with a trailing timestamp, so any caller parsing untrusted pages was exposed. Separators are now located in the original string.
- **The Mozilla test suite actually asserts**: both integration tests were marked `#[ignore]` and contained no assertions at all, so the 130-page corpus could not fail a build and was skipped by `cargo test` entirely. The suite now runs by default and asserts on metadata and on extracted body length, with the known divergences listed by name in `KNOWN_METADATA_DIVERGENCES` and `KNOWN_CONTENT_DIVERGENCES`. A case has to be listed there to be allowed to differ.
- **Corrected the compatibility claim**: the README stated 122 of 130 Mozilla cases passing. The real figure, measured once the suite began asserting, is 119. The number had drifted with no test to hold it in place, which is precisely the gap the change above closes.
- **Added `sanitize_content`**: an opt-in option that drops event-handler attributes (`onerror`, `onclick`, and similar) and dangerous URL schemes (`javascript:`, `vbscript:`, and non-image `data:`) while serializing extracted HTML. Off by default, so existing output is unchanged. Scheme comparison strips embedded whitespace and control characters first, since browsers do the same while parsing a URL. It filters attributes rather than whole elements and is a harm reducer, not a substitute for a real sanitizer.
- **Documented the sanitization contract**: extracted HTML is untrusted and unsanitized by default, matching Readability.js. This is now stated in the crate documentation and in a new [SECURITY.md](SECURITY.md), together with the trust model and known limitations.
- **`<script>` no longer survives extraction**: script and style removal was a regex requiring the literal `</script>`, but the tokenizer also accepts `</script >`, `</script\n>` and `</SCRIPT\t>`, so a single space defeated it and the element reached `article.content` intact, `sanitize_content` or not. Removal now runs on the parsed tree, covering `script`, `style`, `noscript` and `template`, and adds no extra parse of the document. As defence in depth, `sanitize_content` now also drops those elements plus `iframe`, `object`, `embed` and `form` whole at serialization time, and drops comments, whose bodies could close early on a `-->` and turn the remainder into live markup.
- **Article bodies are no longer deleted by nav-wrapper removal**: a regex deleted any `<div>` whose class merely contained `header`, up to the first `</div>`. For the common theme markup of an article body inside `<div class="entry-header">`, `parse()` returned `Some` with empty content and `length` 0, which a caller could not distinguish from success. The pass is gone; the nav, menu, sidebar and breadcrumb keywords it shared with other passes are still covered, and those passes weigh content quality rather than matching class substrings blindly.
- **Code listings are no longer flattened**: the whitespace normalization pass collapsed every run of two or more spaces and three or more newlines across the whole document, including inside `<pre>` and `<code>`, where whitespace is the content rather than layout noise. Every indentation level in an extracted code block became a single space, so the nesting of any indentation-sensitive language was unrecoverable, in `content`, `text_content`, and `markdown_content` alike. The collapsing regexes now run only outside preformatted elements. Turning the pass off with `clean_whitespace(false)` was the only prior workaround, and it gave up prose normalization everywhere. Reported as [#24](https://github.com/theiskaa/readabilityrs/issues/24).
- **Markdown output is no longer injectable through link and image destinations**: a `]` or `)` in an `href`, `src`, or `alt` value closed the Markdown label or destination early, so attacker-controlled page content could become a live link pointing somewhere else. Destinations are now stripped of control characters and wrapped in the CommonMark `<...>` form when they contain a space or bracket, with any remaining angle bracket percent-encoded so it cannot close the wrapper; label text has its brackets escaped. The same treatment covers image, media (`iframe`, `video`, `audio`), and footnote-definition destinations. A new `MarkdownOptions::sanitize_urls` additionally drops `javascript:`, `vbscript:`, and non-image `data:` destinations, mirroring `sanitize_content` and off by default. Structural escaping is unconditional; only scheme-dropping is gated on the flag.
- **Fixed double-escaped ampersands in image URLs**: the Markdown standardization pass runs over already-escaped HTML, and re-escaped every `&`, so a lazy-loaded image URL containing `&amp;` became `&amp;amp;` and the link broke. Escaping now leaves existing character references alone, with a bounded lookahead so the scan cannot be turned into a performance problem.
- **Fixed a panic and a 293x output blowup on wide Markdown table cells**: column padding was computed from the widest cell with no bound. A cell over `u16::MAX` bytes exceeded the largest runtime format width Rust accepts and panicked mid-conversion, and short of that, one wide cell padded every other cell in its column, turning 62 KB of input into 18 MB of output. Widths are now capped at 200 characters, which leaves the same table shape for ordinary content and brings that case down to 2x. Oversized cells are emitted at their natural length; no content is truncated.
- **`max_elems_to_parse` now does something**: the option was documented as a safety limit against oversized or hostile documents, and `MaxElementsExceeded` existed as an error variant, but nothing counted elements and the variant was never constructed. Anyone who set the option believed they had bounded the work on a hostile page and had not. Elements are now counted before any scoring runs, and `0` still means unlimited so default behavior is unchanged. Note the count happens after parsing, so it bounds extraction rather than parsing.
- **`link_density_modifier` rejects non-finite values**: `NaN` and `±inf` were accepted by the builder and propagated into every candidate score, making the whole ranking meaningless rather than merely skewed. Such values are now ignored and the default kept.
- **Deprecated the three options that still do nothing**: `keep_classes`, `classes_to_preserve` and `allowed_video_regex` are declared, settable, and never read. They are marked `#[deprecated]` with honest documentation rather than removed, since removing them would be a breaking change. Implementing class preservation is a separate piece of work.
- **Extended the double-escape fix to math and code blocks**: `math.rs` and `code_blocks.rs` carried the same blind `&` to `&amp;` replace that was fixed in image URLs, so `&amp;` in a MathJax annotation or a fenced code block became `&amp;amp;`. Code was the worse case, being full of `&&` and embedded markup. All three now share one entity-aware escaper in `src/elements/escaping.rs`, so there is a single implementation to keep correct. Code blocks still leave quotes unescaped, since a quote inside `<pre><code>` is ordinary source text.
- **Bounded DOM recursion depth**: the HTML serializer, the Markdown converter, and the byline text collector each recursed once per nesting level with no cap, so a deeply nested page overflowed the stack. That aborts the process rather than unwinding, so an embedding server could not defend against it. All three now stop descending past 256 levels and drop the subtree. The limit was measured rather than guessed: the Markdown converter, which has the largest stack frames, survives 384 levels and aborts at 448 in a debug build, and real pages nest well under 100.
- **`is_probably_readerable` now implements the algorithm it claimed to**: the previous version was an admitted stub that summed paragraph lengths, so hidden text counted and paragraphs in comment threads, sidebars and footers counted. It now skips hidden nodes, skips nodes whose own class or id marks them unlikely candidates without also matching the "maybe a candidate" pattern, skips paragraphs directly inside list items, and adds divs with a direct `<br>` child. Measured against the flags Mozilla ships with its 130 test pages, it went from 2 disagreements to **0**.
- **Added CI**: GitHub Actions now runs `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo build`, and `cargo test` on every push and pull request. Nothing enforced any of these before.
- **Cleaned the lint baseline**: eleven clippy warnings resolved and the tree formatted with `cargo fmt`. Two of the fixes touched library code (`sort_by_key` in the extraction fallback, and the footnote counter loop) and are behavior-preserving; the rest were in tests.
- **Declared a minimum supported Rust version of 1.83**: the crate had no `rust-version` field, so an unsupported toolchain failed at compile time with an unhelpful error instead of a clear one from Cargo. `once_cell` was also dropped in favour of `std::sync::LazyLock`, removing a dependency. The 1.83 floor is driven by the dependency graph — ICU4X 2.x, reached through `url` and `idna` — not by `LazyLock`, which stabilized in 1.80.
- **Dependency hygiene**: `thiserror` moved to 2.0, and the exact pin on `v_htmlescape` (`=0.15.8`) was relaxed to `0.15` so patch fixes can flow through. `crossbeam-epoch` was updated past RUSTSEC-2026-0204, a development advisory reachable only through `criterion` that never affected the published library.
- **Replaced `kuchikikiki` with `ego-tree` for DOM manipulation**, aligning the tree representation with the one `scraper` already produces and removing a redundant parse.
- **Precompiled the removal and metadata regexes as statics**, so they are built once per process rather than on each call.
- **Split the 2171-line `metadata.rs`** into `json_ld`, `byline`, `title`, `language`, and `image` submodules. A pure move, verified byte-identical.

**Upgrade notes**

- **Rust 1.83 or later is required.** Earlier toolchains will no longer build the crate.
- **Extracted output changes for some pages, by design.** Every difference is a fix for content that was previously wrong: bodies that were being deleted by nav-wrapper removal now survive, `<script>` no longer leaks through, code listings keep their indentation, and Markdown tables are no longer padded to the width of their widest cell. If you snapshot-test extraction output, expect those snapshots to move. Separately, `max_elems_to_parse` is now enforced rather than ignored, so a caller that set it and relied on it doing nothing will start seeing `MaxElementsExceeded`; `0` still means unlimited.

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

[Unreleased]: https://github.com/theiskaa/readabilityrs/compare/v0.1.4...HEAD
[0.1.4]: https://github.com/theiskaa/readabilityrs/releases/tag/v0.1.4
[0.1.3]: https://github.com/theiskaa/readabilityrs/releases/tag/v0.1.3
[0.1.2]: https://github.com/theiskaa/readabilityrs/releases/tag/v0.1.2
[0.1.1]: https://github.com/theiskaa/readabilityrs/releases/tag/v0.1.1
[0.1.0]: https://github.com/theiskaa/readabilityrs/releases/tag/v0.1.0
