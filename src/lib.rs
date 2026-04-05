//! # ReadabilityRS
//!
//! A Rust port of Mozilla's Readability library for extracting article content from web pages.
//!
//! This library is a faithful port of the [Mozilla Readability](https://github.com/mozilla/readability)
//! JavaScript library, used in Firefox Reader View.
//!
//! ## Overview
//!
//! ReadabilityRS provides intelligent extraction of main article content from HTML documents,
//! removing clutter such as advertisements, navigation elements, and other non-essential content.
//! It also extracts metadata like article title, author (byline), publish date, and more.
//!
//! ## Key Features
//!
//! - **Content Extraction**: Intelligently identifies and extracts main article content
//! - **Markdown Output**: Optional HTML-to-Markdown conversion with content standardization
//! - **Metadata Extraction**: Extracts title, author, description, site name, language, and publish date
//! - **JSON-LD Support**: Parses structured data from JSON-LD markup
//! - **Multiple Retry Strategies**: Uses adaptive algorithms to handle various page layouts
//! - **Customizable Options**: Configure thresholds, scoring, and behavior
//! - **Pre-flight Check**: Quick check to determine if a page is likely readable
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
//!     .keep_classes(true)
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
//! ## Algorithm
//!
//! The extraction algorithm works in several phases. First, scripts and styles are removed
//! to prepare the document. Then potential content containers are identified throughout the page.
//! These candidates are scored based on various content signals like paragraph count, text length,
//! and link density. The best candidate is selected using adaptive strategies with multiple fallback
//! approaches. Nearby high-quality content is aggregated by examining sibling elements. Finally,
//! the extracted content goes through post-processing to clean and finalize the output.
//!
//! ## Compatibility
//!
//! This implementation strives to match the behavior of Mozilla's Readability.js as closely
//! as possible while leveraging Rust's type system and safety guarantees.

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
