//! # readabilityrs
//!
//! Pulls the article out of a web page. This is a Rust port of
//! [Mozilla's Readability.js](https://github.com/mozilla/readability), the algorithm
//! behind Firefox Reader View: give it a page of HTML and it returns the title,
//! byline, body, excerpt, site name, language, and publication time, leaving
//! navigation, ads, and related-article rails behind.
//!
//! It passes 119 of the 130 cases in Mozilla's test suite. The 11 differences are
//! editorial rather than failures, and each is named in `tests/mozilla_test_suite.rs`.
//!
//! Output is cleaned HTML by default, in [`Article::content`]. Enabling
//! [`ReadabilityOptions::output_markdown`] additionally produces Markdown, after a
//! standardization pass that rewrites vendor-specific markup (highlighted code,
//! lazy-loaded images, footnotes, MathJax and KaTeX output) into canonical form.
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use readabilityrs::{Readability, ReadabilityOptions};
//!
//! let html = r#"<html><body><article><h1>Title</h1><p>Content...</p></article></body></html>"#;
//! let url = "https://example.com/article";
//!
//! let options = ReadabilityOptions::default();
//! let readability = Readability::new(html, Some(url), Some(options)).unwrap();
//!
//! if let Some(article) = readability.parse() {
//!     println!("Title: {:?}", article.title);
//!     println!("Content: {:?}", article.content);
//!     println!("Author: {:?}", article.byline);
//! }
//! ```
//!
//! ## Advanced Usage
//!
//! ### Custom Options
//!
//! ```rust,no_run
//! use readabilityrs::{Readability, ReadabilityOptions};
//!
//! let html = "<html>...</html>";
//!
//! let options = ReadabilityOptions::builder()
//!     .char_threshold(300)
//!     .nb_top_candidates(10)
//!     .build();
//!
//! let readability = Readability::new(html, None, Some(options)).unwrap();
//! let article = readability.parse();
//! ```
//!
//! ### Pre-flight Check
//!
//! Use [`is_probably_readerable`] to quickly check if a document is likely to be parseable
//! before doing the full parse:
//!
//! ```rust,no_run
//! use readabilityrs::is_probably_readerable;
//!
//! let html = "<html>...</html>";
//!
//! if is_probably_readerable(html, None) {
//!     // Proceed with full parsing
//! } else {
//!     // Skip parsing or use alternative strategy
//! }
//! ```
//!
//! ## Error Handling
//!
//! ```rust,no_run
//! use readabilityrs::{Readability, ReadabilityError};
//!
//! let html = "<html>...</html>";
//! let url = "not a valid url";
//!
//! match Readability::new(html, Some(url), None) {
//!     Ok(readability) => {
//!         if let Some(article) = readability.parse() {
//!             println!("Success!");
//!         }
//!     }
//!     Err(ReadabilityError::InvalidUrl(url)) => {
//!         eprintln!("Invalid URL: {}", url);
//!     }
//!     Err(e) => {
//!         eprintln!("Error: {}", e);
//!     }
//! }
//! ```
//!
//! ## Security
//!
//! [`Article::content`] comes from untrusted input and is **not sanitized by
//! default**. This matches the Readability.js contract: every attribute of every
//! element that survives extraction is written back out, including event handlers
//! such as `onerror` and `onclick`, and URL schemes such as `javascript:` and
//! `data:text/html`. Anything that renders the output in a webview or browser DOM
//! has to sanitize it first, for example with
//! [`ammonia`](https://crates.io/crates/ammonia).
//!
//! Setting [`ReadabilityOptions::sanitize_content`] drops script-bearing and
//! content-loading elements whole, along with event-handler attributes, the
//! highest-risk URL schemes, and comments. It reduces harm and is not a substitute
//! for a real sanitizer: the allowed elements keep every other attribute they
//! carry, and none of it applies to [`Article::markdown_content`].
//!
//! ## Algorithm
//!
//! Extraction runs in phases. The document is preprocessed first: scripts and styles
//! are stripped, `<noscript>` wrappers around lazy-loaded images are unwrapped, and
//! deprecated elements are normalized. Candidate containers are then scored by tag
//! type, text density, link density, and class and id patterns. The highest-scoring
//! subtree becomes the article body, and sibling elements that look like part of the
//! same article are pulled in with it. Post-processing cleans the result.
//!
//! When a pass produces too little text, it is retried with looser flags: first
//! without the unlikely-candidate filter, then without class weighting, then without
//! conditional cleaning. If every attempt stays under the character threshold, the
//! longest one is returned.

mod article;
mod cleaner;
mod constants;
mod content_extractor;
mod dom_utils;
pub mod elements;
mod error;
pub mod markdown;
mod metadata;
mod options;
mod post_processor;
mod readability;
mod readerable;
mod scoring;
mod utils;

// Public exports
pub use article::Article;
pub use error::{ReadabilityError, Result};
pub use markdown::MarkdownOptions;
pub use options::ReadabilityOptions;
pub use readability::Readability;
pub use readerable::{is_probably_readerable, ReaderableOptions};
