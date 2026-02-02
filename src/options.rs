//! Configuration options for Readability parsing.
//!
//! This module provides [`ReadabilityOptions`] and [`ReadabilityOptionsBuilder`]
//! for configuring the behavior of the content extraction algorithm.
//!
//! ## Example
//!
//! ```rust
//! use readabilityrs::{Readability, ReadabilityOptions};
//!
//! let html = "<html><body><article><p>Content...</p></article></body></html>";
//!
//! // Using default options
//! let readability = Readability::new(html, None, None).unwrap();
//!
//! // Using builder for custom options
//! let options = ReadabilityOptions::builder()
//!     .char_threshold(300)
//!     .nb_top_candidates(10)
//!     .keep_classes(true)
//!     .build();
//!
//! let readability = Readability::new(html, None, Some(options)).unwrap();
//! ```

use regex::Regex;

/// Configuration options for the Readability parser.
///
/// Controls various aspects of the content extraction algorithm, including scoring
/// thresholds, element limits, and metadata extraction behavior.
///
/// ## Creating Options
///
/// ### Using Default
///
/// ```rust
/// use readabilityrs::ReadabilityOptions;
///
/// let options = ReadabilityOptions::default();
/// ```
///
/// ### Using Builder
///
/// ```rust
/// use readabilityrs::ReadabilityOptions;
///
/// let options = ReadabilityOptions::builder()
///     .char_threshold(300)
///     .nb_top_candidates(10)
///     .debug(true)
///     .build();
/// ```
///
/// ## Field Descriptions
///
/// See individual field documentation for details on what each option controls.
#[derive(Debug, Clone)]
pub struct ReadabilityOptions {
    /// Enable debug logging to stderr.
    ///
    /// When enabled, the parser will output diagnostic messages to stderr
    /// during extraction. Useful for understanding why extraction failed
    /// or for debugging extraction behavior.
    ///
    /// Default: `false`
    pub debug: bool,

    /// Maximum number of elements to parse.
    ///
    /// This is a safety limit to prevent processing extremely large documents
    /// that could consume excessive memory or CPU time. Set to 0 to disable
    /// the limit.
    ///
    /// Default: `0` (no limit)
    pub max_elems_to_parse: usize,

    /// Number of top candidates to consider when analyzing content.
    ///
    /// The parser scores all potential article containers and considers this many
    /// of the highest-scoring candidates. Higher values increase accuracy but may
    /// also increase processing time.
    ///
    /// Default: `5`
    pub nb_top_candidates: usize,

    /// Minimum number of characters required for article content.
    ///
    /// If extracted content has fewer characters than this threshold, the parser
    /// will try alternative extraction strategies. Lower values make extraction
    /// more permissive but may capture non-article content.
    ///
    /// Default: `500`
    pub char_threshold: usize,

    /// CSS classes to preserve during cleaning.
    ///
    /// By default, the parser removes most CSS classes during cleaning. Classes
    /// in this list will be preserved in the output HTML.
    ///
    /// Default: `vec!["page"]`
    pub classes_to_preserve: Vec<String>,

    /// Keep all CSS classes in the output HTML.
    ///
    /// When `true`, preserves all CSS classes instead of removing them.
    /// This can be useful if you need to apply custom styling to the output.
    ///
    /// Default: `false`
    pub keep_classes: bool,

    /// Disable JSON-LD metadata extraction.
    ///
    /// When `true`, skips parsing of JSON-LD structured data, which can
    /// improve performance if you don't need metadata like author, publish date, etc.
    ///
    /// Default: `false`
    pub disable_json_ld: bool,

    /// Custom regex for allowed video URLs.
    ///
    /// Override the default video platform detection with a custom regex.
    /// By default, the parser recognizes common platforms like YouTube, Vimeo, etc.
    ///
    /// Default: `None` (uses built-in regex)
    ///
    /// ## Example
    ///
    /// ```rust
    /// use readabilityrs::ReadabilityOptions;
    /// use regex::Regex;
    ///
    /// let video_regex = Regex::new(r"(?i)myvideoplatform\.com").unwrap();
    /// let options = ReadabilityOptions::builder()
    ///     .allowed_video_regex(video_regex)
    ///     .build();
    /// ```
    pub allowed_video_regex: Option<Regex>,

    /// Modifier for link density scoring.
    ///
    /// Adjusts how heavily link density affects content scoring. Positive values
    /// make the algorithm more tolerant of links, negative values less tolerant.
    ///
    /// Default: `0.0`
    pub link_density_modifier: f64,

    /// Remove the title element from the extracted content.
    ///
    /// When `true`, removes the title heading (h1/h2) from the article content HTML
    /// if it matches the extracted title. This is useful when you want to render
    /// the title separately from the content for layout consistency.
    ///
    /// Default: `false`
    pub remove_title_from_content: bool,

    /// Remove inline styles from the extracted content.
    ///
    /// When `true`, removes the `style` attribute and other presentational attributes
    /// (align, bgcolor, etc.) from HTML elements. This implements Mozilla Readability's
    /// `_cleanStyles` function and prevents issues like invisible text (e.g., `color: white`
    /// on white backgrounds) or unwanted formatting.
    ///
    /// Default: `true`
    pub clean_styles: bool,

    /// Normalize whitespace in the extracted content.
    ///
    /// When `true`, removes excessive blank lines, empty paragraphs, and normalizes
    /// whitespace. This helps produce cleaner output, especially for articles from
    /// blogs and CMSs that generate verbose HTML.
    ///
    /// Default: `true`
    pub clean_whitespace: bool,
}

impl Default for ReadabilityOptions {
    fn default() -> Self {
        Self {
            debug: false,
            max_elems_to_parse: 0,
            nb_top_candidates: 5,
            char_threshold: 500,
            classes_to_preserve: vec!["page".to_string()],
            keep_classes: false,
            disable_json_ld: false,
            allowed_video_regex: None,
            link_density_modifier: 0.0,
            remove_title_from_content: false,
            clean_styles: true,
            clean_whitespace: true,
        }
    }
}

impl ReadabilityOptions {
    /// Creates a new builder for ReadabilityOptions
    pub fn builder() -> ReadabilityOptionsBuilder {
        ReadabilityOptionsBuilder::default()
    }
}

/// Builder for [`ReadabilityOptions`].
///
/// Provides a fluent interface for constructing [`ReadabilityOptions`] with custom values.
///
/// ## Example
///
/// ```rust
/// use readabilityrs::ReadabilityOptions;
///
/// let options = ReadabilityOptions::builder()
///     .char_threshold(300)
///     .nb_top_candidates(10)
///     .debug(true)
///     .keep_classes(true)
///     .build();
/// ```
#[derive(Default)]
pub struct ReadabilityOptionsBuilder {
    debug: Option<bool>,
    max_elems_to_parse: Option<usize>,
    nb_top_candidates: Option<usize>,
    char_threshold: Option<usize>,
    classes_to_preserve: Option<Vec<String>>,
    keep_classes: Option<bool>,
    disable_json_ld: Option<bool>,
    allowed_video_regex: Option<Regex>,
    link_density_modifier: Option<f64>,
    remove_title_from_content: Option<bool>,
    clean_styles: Option<bool>,
    clean_whitespace: Option<bool>,
}

impl ReadabilityOptionsBuilder {
    /// Enable or disable debug logging
    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = Some(debug);
        self
    }

    /// Set maximum number of elements to parse
    pub fn max_elems_to_parse(mut self, max: usize) -> Self {
        self.max_elems_to_parse = Some(max);
        self
    }

    /// Set number of top candidates to consider
    pub fn nb_top_candidates(mut self, nb: usize) -> Self {
        self.nb_top_candidates = Some(nb);
        self
    }

    /// Set character threshold
    pub fn char_threshold(mut self, threshold: usize) -> Self {
        self.char_threshold = Some(threshold);
        self
    }

    /// Set classes to preserve
    pub fn classes_to_preserve(mut self, classes: Vec<String>) -> Self {
        self.classes_to_preserve = Some(classes);
        self
    }

    /// Keep all CSS classes
    pub fn keep_classes(mut self, keep: bool) -> Self {
        self.keep_classes = Some(keep);
        self
    }

    /// Disable JSON-LD extraction
    pub fn disable_json_ld(mut self, disable: bool) -> Self {
        self.disable_json_ld = Some(disable);
        self
    }

    /// Set allowed video regex
    pub fn allowed_video_regex(mut self, regex: Regex) -> Self {
        self.allowed_video_regex = Some(regex);
        self
    }

    /// Set link density modifier
    pub fn link_density_modifier(mut self, modifier: f64) -> Self {
        self.link_density_modifier = Some(modifier);
        self
    }

    /// Remove the title element from the extracted content
    ///
    /// When enabled, removes the title heading (h1/h2) from the article content
    /// if it matches the extracted title. Useful for rendering the title separately.
    pub fn remove_title_from_content(mut self, remove: bool) -> Self {
        self.remove_title_from_content = Some(remove);
        self
    }

    /// Enable or disable inline style cleaning
    ///
    /// When enabled, removes the `style` attribute and other presentational attributes
    /// from HTML elements. This implements Mozilla Readability's `_cleanStyles` function.
    pub fn clean_styles(mut self, clean: bool) -> Self {
        self.clean_styles = Some(clean);
        self
    }

    /// Enable or disable whitespace normalization
    ///
    /// When enabled, removes excessive blank lines, empty paragraphs, and normalizes
    /// whitespace in the output.
    pub fn clean_whitespace(mut self, clean: bool) -> Self {
        self.clean_whitespace = Some(clean);
        self
    }

    /// Build the ReadabilityOptions
    pub fn build(self) -> ReadabilityOptions {
        let defaults = ReadabilityOptions::default();
        ReadabilityOptions {
            debug: self.debug.unwrap_or(defaults.debug),
            max_elems_to_parse: self
                .max_elems_to_parse
                .unwrap_or(defaults.max_elems_to_parse),
            nb_top_candidates: self.nb_top_candidates.unwrap_or(defaults.nb_top_candidates),
            char_threshold: self.char_threshold.unwrap_or(defaults.char_threshold),
            classes_to_preserve: self
                .classes_to_preserve
                .unwrap_or(defaults.classes_to_preserve),
            keep_classes: self.keep_classes.unwrap_or(defaults.keep_classes),
            disable_json_ld: self.disable_json_ld.unwrap_or(defaults.disable_json_ld),
            allowed_video_regex: self.allowed_video_regex.or(defaults.allowed_video_regex),
            link_density_modifier: self
                .link_density_modifier
                .unwrap_or(defaults.link_density_modifier),
            remove_title_from_content: self
                .remove_title_from_content
                .unwrap_or(defaults.remove_title_from_content),
            clean_styles: self.clean_styles.unwrap_or(defaults.clean_styles),
            clean_whitespace: self.clean_whitespace.unwrap_or(defaults.clean_whitespace),
        }
    }
}
