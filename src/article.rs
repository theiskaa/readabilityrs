//! Article data structure representing the parsed output.
//!
//! This module defines the [`Article`] struct, which contains all extracted content
//! and metadata from a successfully parsed web page.
//!
//! ## Example
//!
//! ```rust,no_run
//! use readabilityrs::{Readability, ReadabilityOptions};
//!
//! let html = r#"<html><body><article><h1>My Article</h1><p>Content...</p></article></body></html>"#;
//! let readability = Readability::new(html, Some("https://example.com"), None).unwrap();
//!
//! if let Some(article) = readability.parse() {
//!     // Access article fields
//!     println!("Title: {:?}", article.title);
//!     println!("Length: {} characters", article.length);
//!     println!("Author: {:?}", article.byline);
//!
//!     // Get cleaned HTML content
//!     if let Some(content) = article.content {
//!         println!("HTML: {}", content);
//!     }
//!
//!     // Or get plain text
//!     if let Some(text) = article.text_content {
//!         println!("Text: {}", text);
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};

/// Represents a successfully parsed article with extracted content and metadata.
///
/// The `Article` struct contains all the extracted information from a web page,
/// including the main content (both HTML and plain text), metadata (title, author,
/// publish date), and other article properties.
///
/// ## Fields
///
/// All fields are optional (`Option<String>`) because not all web pages contain
/// all metadata fields. The `length` field is always present and represents the
/// character count of the extracted text.
///
/// ## Serialization
///
/// This struct implements `Serialize` and `Deserialize` from serde, making it
/// easy to save articles to JSON or other formats:
///
/// ```rust,no_run
/// use readabilityrs::{Readability, Article};
/// # let html = "<html></html>";
/// # let readability = Readability::new(html, None, None).unwrap();
///
/// if let Some(article) = readability.parse() {
///     let json = serde_json::to_string_pretty(&article).unwrap();
///     println!("{}", json);
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Article {
    /// The article title extracted from metadata or the document.
    ///
    /// The title is extracted by checking multiple sources in priority order: JSON-LD structured
    /// data is checked first, followed by OpenGraph and Twitter Card meta tags. If neither is
    /// available, the `<title>` tag is used after cleaning it of the site name. As a final
    /// fallback, the first `<h1>` tag in the document is used.
    pub title: Option<String>,

    /// Cleaned HTML content of the article.
    ///
    /// This contains the main article content with:
    /// - Ads and navigation removed
    /// - Unwanted elements filtered out
    /// - Relative URLs converted to absolute
    /// - Empty elements cleaned up
    pub content: Option<String>,

    /// Plain text content with all HTML tags removed.
    ///
    /// This is the text-only version of the article content,
    /// useful for previews, search indexing, or analysis.
    pub text_content: Option<String>,

    /// Length of the article in characters.
    ///
    /// This is the character count of the plain text content,
    /// useful for reading time estimation or content validation.
    pub length: usize,

    /// Article description or short excerpt.
    ///
    /// The excerpt is extracted from JSON-LD description if available, otherwise from
    /// OpenGraph or Twitter description meta tags. If no metadata is found, the meta
    /// description tag is used. As a fallback, the first paragraph of the extracted
    /// article content is used as the excerpt.
    pub excerpt: Option<String>,

    /// Author name(s).
    ///
    /// The author is extracted from various sources, checking JSON-LD author data first,
    /// then meta author tags. If neither is available, elements with `rel="author"` or
    /// `itemprop="author"` attributes are examined. Finally, elements with classes like
    /// "byline" or "author" are checked. Multiple authors may be included and are
    /// separated by commas.
    pub byline: Option<String>,

    /// The image is extracted from various sources, checking JSON-LD structured data first
    /// (supporting simple URLs, ImageObject with `url` property, and arrays of images).
    /// If not found, Open Graph meta tags are checked in priority order: `og:image:secure_url`,
    /// `og:image:url`, then `og:image`. Twitter card meta tags (`twitter:image`) and generic
    /// `thumbnail` or `image` meta tags are examined next. Finally, elements with
    /// `link[rel="image_src"]` or `itemprop="image"` attributes are checked as fallbacks.
    /// The first valid URL encountered in this priority order is used.
    pub image: Option<String>,

    /// Text direction hint: "ltr" (left-to-right), "rtl" (right-to-left), or "auto".
    ///
    /// Extracted from the `dir` attribute on the `<html>` element.
    pub dir: Option<String>,

    /// Name of the website or publication.
    ///
    /// The site name is extracted from the OpenGraph `og:site_name` tag or the JSON-LD
    /// publisher name field.
    pub site_name: Option<String>,

    /// Language code of the content (e.g., "en", "es", "fr").
    ///
    /// Extracted from the `lang` attribute on the `<html>` element or
    /// `Content-Language` meta tag.
    pub lang: Option<String>,

    /// Publication or modification timestamp.
    ///
    /// The publication time is extracted from the JSON-LD `datePublished` field or the
    /// `article:published_time` meta tag. The format varies depending on the source but
    /// is typically ISO 8601.
    pub published_time: Option<String>,

    /// Raw HTML content before final post-processing.
    ///
    /// This is the extracted content before the final cleaning steps,
    /// useful for debugging or custom post-processing.
    pub raw_content: Option<String>,
}

impl Article {
    pub fn new() -> Self {
        Self::default()
    }
}
