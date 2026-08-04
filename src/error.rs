//! Error types for the readability library.
//!
//! This module defines error types that can occur during article extraction and parsing.
//! All errors implement the [`std::error::Error`] trait via [`thiserror`].
//!
//! ## Example
//!
//! ```rust
//! use readabilityrs::{Readability, ReadabilityError};
//!
//! let html = "<html><body><p>Content</p></body></html>";
//! let invalid_url = "not-a-valid-url";
//!
//! match Readability::new(html, Some(invalid_url), None) {
//!     Ok(_) => println!("Success!"),
//!     Err(ReadabilityError::InvalidUrl(url)) => {
//!         eprintln!("The provided URL '{}' is not valid", url);
//!     }
//!     Err(e) => eprintln!("Other error: {}", e),
//! }
//! ```

use thiserror::Error;

/// Result type alias for readability operations.
///
/// This is a convenience type alias that uses [`ReadabilityError`] as the error type.
///
/// ## Example
///
/// ```rust
/// use readabilityrs::{Result, ReadabilityError};
///
/// fn parse_article(html: &str) -> Result<String> {
///     if html.is_empty() {
///         return Err(ReadabilityError::InvalidDocument("Empty HTML".to_string()));
///     }
///     Ok(html.to_string())
/// }
/// ```
pub type Result<T> = std::result::Result<T, ReadabilityError>;

/// Errors that can occur during readability parsing.
///
/// This enum represents all possible error conditions that may occur when
/// creating a [`Readability`](crate::Readability) instance or parsing article content.
///
/// ## Examples
///
/// ### Handling Invalid URLs
///
/// ```rust
/// use readabilityrs::{Readability, ReadabilityError};
///
/// let html = "<html><body><p>Content</p></body></html>";
///
/// match Readability::new(html, Some("invalid url"), None) {
///     Err(ReadabilityError::InvalidUrl(url)) => {
///         println!("Invalid URL: {}", url);
///         // Handle the error
///     }
///     Ok(_) => println!("URL is valid"),
///     Err(e) => println!("Other error: {}", e),
/// }
/// ```
///
/// ### Converting Errors to Strings
///
/// ```rust
/// use readabilityrs::{Readability, ReadabilityError};
///
/// let result = Readability::new("<html></html>", Some("bad-url"), None);
/// if let Err(e) = result {
///     let error_message = e.to_string();
///     println!("Error occurred: {}", error_message);
/// }
/// ```
#[derive(Error, Debug)]
pub enum ReadabilityError {
    /// Failed to parse HTML document.
    ///
    /// This error occurs when the HTML parser encounters malformed or unparseable HTML.
    /// However, the underlying parser is generally very lenient, so this error is rare.
    #[error("Failed to parse HTML: {0}")]
    ParseError(String),

    /// Invalid URL provided.
    ///
    /// This error occurs when a URL string cannot be parsed as a valid URL.
    /// URLs are validated when passed to [`Readability::new`](crate::Readability::new).
    ///
    /// ## Example
    ///
    /// ```rust
    /// use readabilityrs::{Readability, ReadabilityError};
    ///
    /// let result = Readability::new("<html></html>", Some("not a url"), None);
    /// assert!(matches!(result, Err(ReadabilityError::InvalidUrl(_))));
    /// ```
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Document structure is invalid or missing required elements.
    ///
    /// This error occurs when the document structure doesn't meet minimum requirements
    /// for extraction, such as having no parseable elements.
    #[error("Invalid document: {0}")]
    InvalidDocument(String),

    /// JSON-LD parsing error.
    ///
    /// This error occurs when JSON-LD structured data is present but cannot be parsed.
    /// This typically happens when the JSON is malformed or doesn't follow expected schemas.
    #[error("JSON-LD parsing error: {0}")]
    JsonLdError(String),

    /// Maximum element limit exceeded.
    ///
    /// Produced when a document holds more elements than
    /// [`ReadabilityOptions::max_elems_to_parse`](crate::ReadabilityOptions::max_elems_to_parse)
    /// allows. Carries the element count that was actually found.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use readabilityrs::{Readability, ReadabilityOptions};
    ///
    /// let html = "<html><body>".to_string() + &"<p>text</p>".repeat(10000) + "</body></html>";
    ///
    /// let options = ReadabilityOptions::builder()
    ///     .max_elems_to_parse(100)
    ///     .build();
    ///
    /// let readability = Readability::new(&html, None, Some(options)).unwrap();
    ///
    /// // `parse` collapses every error into `None`, so the limit surfaces here as
    /// // "no article" rather than as this variant.
    /// assert!(readability.parse().is_none());
    /// ```
    #[error("Maximum element limit exceeded: {0}")]
    MaxElementsExceeded(usize),

    /// No article content could be extracted.
    ///
    /// This error occurs when the parser cannot identify any suitable article content
    /// in the document, even after trying multiple extraction strategies. This is different
    /// from returning `None` from `parse()` - this error indicates a fatal condition.
    #[error("No article content found in document")]
    NoContentFound,

    /// General error.
    ///
    /// A catch-all error type for conditions that don't fit other categories.
    #[error("Readability error: {0}")]
    Other(String),
}
