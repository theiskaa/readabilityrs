//! Main Readability struct and parse implementation.
//!
//! This module contains the primary [`Readability`] struct which orchestrates
//! the entire article extraction pipeline.
//!
//! ## Example
//!
//! ```rust,no_run
//! use readabilityrs::{Readability, ReadabilityOptions};
//!
//! let html = std::fs::read_to_string("article.html").unwrap();
//! let url = "https://example.com/article";
//!
//! let readability = Readability::new(&html, Some(url), None)?;
//!
//! if let Some(article) = readability.parse() {
//!     println!("Title: {:?}", article.title);
//!     println!("Author: {:?}", article.byline);
//!     println!("Content length: {} chars", article.length);
//!
//!     // Save to file
//!     if let Some(content) = article.content {
//!         std::fs::write("output.html", content)?;
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::{
    article::Article,
    cleaner,
    content_extractor::grab_article,
    dom_utils,
    error::{ReadabilityError, Result},
    metadata::{get_article_metadata, get_json_ld, Metadata},
    options::ReadabilityOptions,
    utils,
};
use scraper::{ElementRef, Html, Selector};

/// The main Readability parser.
///
/// This struct is the primary interface for extracting article content from HTML documents.
/// It implements Mozilla's Readability algorithm, which powers Firefox's Reader View.
///
/// ## Lifecycle
///
/// The typical usage pattern starts by constructing a `Readability` instance with
/// [`Readability::new()`], then calling [`parse()`](Readability::parse) to extract the content.
/// The result is an [`Article`] containing the extracted content and metadata.
///
/// ## Features
///
/// - Intelligent content identification
/// - Metadata extraction (title, author, date, etc.)
/// - JSON-LD structured data parsing
/// - Multiple retry strategies for difficult pages
/// - Configurable thresholds and behavior
///
/// ## Example
///
/// ```rust,no_run
/// use readabilityrs::{Readability, ReadabilityOptions};
///
/// let html = r#"
///     <html>
///     <head><title>Article Title</title></head>
///     <body>
///         <article>
///             <h1>Article Title</h1>
///             <p>First paragraph of content...</p>
///             <p>Second paragraph of content...</p>
///         </article>
///     </body>
///     </html>
/// "#;
///
/// let readability = Readability::new(html, None, None)?;
/// let article = readability.parse();
///
/// if let Some(article) = article {
///     println!("Success! Extracted {} characters", article.length);
/// } else {
///     println!("Could not extract article content");
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// ## With Custom Options
///
/// ```rust,no_run
/// use readabilityrs::{Readability, ReadabilityOptions};
///
/// let html = "<html>...</html>";
///
/// let options = ReadabilityOptions::builder()
///     .char_threshold(300)
///     .debug(true)
///     .build();
///
/// let readability = Readability::new(html, None, Some(options))?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct Readability {
    /// The HTML document being parsed (raw, for metadata extraction)
    document: Html,

    /// Original HTML string (stored for preprocessing before content extraction)
    html: String,

    /// Base URL for resolving relative links
    base_url: Option<String>,

    /// Configuration options
    options: ReadabilityOptions,

    /// Extracted metadata
    metadata: Metadata,
}

impl Readability {
    /// Create a new Readability instance
    ///
    /// # Arguments
    /// * `html` - The HTML content to parse
    /// * `url` - Optional base URL for resolving relative links
    /// * `options` - Optional configuration options
    ///
    /// # Returns
    /// Result containing the Readability instance or an error
    pub fn new(html: &str, url: Option<&str>, options: Option<ReadabilityOptions>) -> Result<Self> {
        // Parse raw HTML for metadata extraction
        // Preprocessing happens later in parse() before content extraction
        let document = Html::parse_document(html);

        // Validate base URL if provided
        let base_url = url
            .map(|u| {
                url::Url::parse(u)
                    .map(|_| u.to_string())
                    .map_err(|_| ReadabilityError::InvalidUrl(u.to_string()))
            })
            .transpose()?;

        let options = options.unwrap_or_default();

        Ok(Self {
            document,
            html: html.to_string(),
            base_url,
            options,
            metadata: Metadata::default(),
        })
    }

    /// Parse the document and extract article content
    ///
    /// # Returns
    /// `Option<Article>` - Some(article) if successful, None if no article found
    pub fn parse(mut self) -> Option<Article> {
        let json_ld = if !self.options.disable_json_ld {
            get_json_ld(&self.document)
        } else {
            Metadata::default()
        };

        self.metadata = get_article_metadata(&self.document, json_ld);

        let preprocessed_html = cleaner::prep_document(&self.html);
        let preprocessed_doc = Html::parse_document(&preprocessed_html);

        match grab_article(&preprocessed_doc, &self.options) {
            Ok(Some(content_html)) => {
                let cleaned_wrapper_html =
                    cleaner::clean_article_content_light(&content_html, self.base_url.as_deref())
                        .unwrap_or_else(|_| content_html.clone());

                let mut prepped_html = crate::post_processor::prep_article(&cleaned_wrapper_html);

                // Remove title from content if the option is enabled
                if self.options.remove_title_from_content {
                    if let Some(ref title) = self.metadata.title {
                        prepped_html =
                            crate::post_processor::remove_title_from_content(&prepped_html, title);
                    }
                }
                let cleaned_html =
                    match cleaner::clean_article_content(&prepped_html, self.base_url.as_deref()) {
                        Ok(html) => html,
                        Err(e) => {
                            if self.options.debug {
                                eprintln!("Error cleaning content: {e}");
                            }
                            prepped_html
                        }
                    };

                let text_content = self.get_text_content(&cleaned_html);
                let length = text_content.len();

                // Generate excerpt from content if not in metadata
                // Try first paragraph of extracted content, then fall back to text
                let excerpt = self.metadata.excerpt.clone().or_else(|| {
                    self.generate_excerpt_from_html(&cleaned_html)
                        .or_else(|| self.generate_excerpt_from_text(&text_content))
                });

                // Extract text direction from document
                let dir = crate::dom_utils::get_article_direction(&self.document);

                Some(Article {
                    title: self.metadata.title,
                    content: Some(cleaned_html),
                    raw_content: Some(content_html),
                    text_content: Some(text_content),
                    length,
                    excerpt,
                    image: self.metadata.image,
                    byline: self.metadata.byline,
                    dir,
                    site_name: self.metadata.site_name,
                    lang: self.metadata.lang,
                    published_time: self.metadata.published_time,
                })
            }
            Ok(None) => None,
            Err(e) => {
                if self.options.debug {
                    eprintln!("Error grabbing article: {e}");
                }
                None
            }
        }
    }

    /// Extract plain text from HTML content
    fn get_text_content(&self, html: &str) -> String {
        let doc = Html::parse_fragment(html);
        doc.root_element().text().collect::<String>()
    }

    /// Generate an excerpt from the first paragraph of article HTML
    ///
    /// Extracts text from the first <p> tag found in the article content.
    /// This matches Mozilla's Readability.js behavior.
    ///
    /// # Arguments
    /// * `html` - The article HTML content
    ///
    /// # Returns
    /// Option<String> - Text from first paragraph, or None if no suitable paragraph found
    fn generate_excerpt_from_html(&self, html: &str) -> Option<String> {
        let doc = Html::parse_fragment(html);
        let p_selector = Selector::parse("p").ok()?;

        for p in doc.select(&p_selector) {
            let text = p.text().collect::<String>();
            let trimmed = text.trim();

            if trimmed.len() < 25 {
                continue;
            }

            if utils::looks_like_bracket_menu(trimmed) {
                continue;
            }

            let class_attr = p.value().attr("class").unwrap_or("");
            let id_attr = p.value().attr("id").unwrap_or("");
            let class_lower = class_attr.to_lowercase();
            let id_lower = id_attr.to_lowercase();

            if Self::paragraph_is_excerpt_noise(&p, trimmed, &class_lower, &id_lower) {
                continue;
            }

            let looks_like_byline = utils::looks_like_byline(trimmed)
                || class_lower.contains("byline")
                || class_lower.contains("author")
                || id_lower.contains("byline")
                || id_lower.contains("author");
            if looks_like_byline {
                continue;
            }

            return Some(trimmed.to_string());
        }

        None
    }

    fn paragraph_is_excerpt_noise(
        element: &ElementRef,
        text: &str,
        class_lower: &str,
        id_lower: &str,
    ) -> bool {
        const CLASS_KEYWORDS: [&str; 8] = [
            "hatnote",
            "shortdescription",
            "metadata",
            "navbox",
            "dablink",
            "noprint",
            "mwe-math-element",
            "mw-empty-elt",
        ];

        if CLASS_KEYWORDS
            .iter()
            .any(|kw| class_lower.contains(kw) || id_lower.contains(kw))
        {
            return true;
        }

        if element
            .value()
            .attr("role")
            .map(|role| role.eq_ignore_ascii_case("note"))
            .unwrap_or(false)
        {
            return true;
        }

        let trimmed_lower = text.to_lowercase();
        const TEXT_PREFIXES: [&str; 5] = [
            "see also",
            "coordinates",
            "navigation menu",
            "external links",
            "further reading",
        ];
        if TEXT_PREFIXES
            .iter()
            .any(|prefix| trimmed_lower.starts_with(prefix))
        {
            return true;
        }

        let link_density = dom_utils::get_link_density(*element);
        link_density > 0.8
    }

    /// Generate an excerpt from the article text content
    ///
    /// Takes the first paragraph or first ~200 characters of the article text
    /// and uses it as an excerpt. This matches Mozilla's Readability.js behavior.
    ///
    /// # Arguments
    /// * `text` - The article text content
    ///
    /// # Returns
    /// Option<String> - Generated excerpt, or None if text is too short
    fn generate_excerpt_from_text(&self, text: &str) -> Option<String> {
        let cleaned = text.trim();

        if cleaned.is_empty() {
            return None;
        }

        // Try to find the first substantial paragraph (at least 80 chars)
        // Split by double newlines to find paragraphs
        for paragraph in cleaned.split("\n\n") {
            let para_trimmed = paragraph.trim();
            if para_trimmed.len() < 80 {
                continue;
            }

            if utils::looks_like_bracket_menu(para_trimmed) {
                continue;
            }

            return Some(self.truncate_text(para_trimmed, 300));
        }

        if utils::looks_like_bracket_menu(cleaned) {
            return None;
        }

        // Take first ~300 chars if substantial enough
        if cleaned.len() > 40 {
            Some(self.truncate_text(cleaned, 300))
        } else {
            None
        }
    }

    /// Truncate text to a maximum length, trying to break at word boundary
    ///
    /// # Arguments
    /// * `text` - Text to truncate
    /// * `max_len` - Maximum length in characters
    ///
    /// # Returns
    /// String - Truncated text
    fn truncate_text(&self, text: &str, max_len: usize) -> String {
        let char_count = text.chars().count();
        if char_count <= max_len {
            return text.to_string();
        }

        let truncated: String = text.chars().take(max_len).collect();
        if let Some(last_space_pos) = truncated.rfind(char::is_whitespace) {
            truncated[..last_space_pos].trim().to_string()
        } else {
            truncated.trim().to_string()
        }
    }

    /// Log a debug message (if debug mode is enabled)
    #[allow(dead_code)]
    fn log(&self, message: &str) {
        if self.options.debug {
            eprintln!("Reader: (Readability) {message}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_readability() {
        let html = r#"<html><body><p>Test</p></body></html>"#;
        let result = Readability::new(html, None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_url() {
        let html = r#"<html><body><p>Test</p></body></html>"#;
        let result = Readability::new(html, Some("not a url"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_simple() {
        let html = r#"
            <html>
                <body>
                    <article>
                        <h1>Test Article</h1>
                        <p>This is a test article with some content.</p>
                    </article>
                </body>
            </html>
        "#;

        let readability = Readability::new(html, None, None).unwrap();
        let _article = readability.parse();
        // For now, just test that it doesn't panic
        // Full functionality will be tested once implementation is complete
    }

    #[test]
    fn excerpt_skips_hatnote_paragraphs() {
        let html = r#"
        <p class="hatnote" role="note">See also: Something else entirely.</p>
        <p>This is the first real paragraph with sufficient length to act as an excerpt. It should be returned.</p>
        "#;
        let reader = Readability::new(html, None, None).unwrap();
        let excerpt = reader.generate_excerpt_from_html(html);
        assert_eq!(
            excerpt,
            Some(
                "This is the first real paragraph with sufficient length to act as an excerpt. It should be returned."
                    .to_string()
            )
        );
    }
}
