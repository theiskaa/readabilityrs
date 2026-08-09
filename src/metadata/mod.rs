//! Metadata extraction from HTML documents (JSON-LD, meta tags, etc.).

mod byline;
mod image;
mod json_ld;
mod language;
mod title;

pub use json_ld::get_json_ld;

use crate::utils;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::sync::LazyLock;

use byline::{
    extract_byline_from_document, extract_standfirst_caps_byline, should_prefer_caps_standfirst,
    should_prefer_dom_byline,
};
use image::extract_image_from_document;
use language::extract_language_from_document;
use title::extract_title_from_document;

/// Metadata extracted from the document
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub byline: Option<String>,
    pub excerpt: Option<String>,
    pub site_name: Option<String>,
    pub published_time: Option<String>,
    pub lang: Option<String>,
    pub image: Option<String>,
}

/// Extract article metadata from meta tags
///
/// Supports OpenGraph, Twitter Cards, Dublin Core, and standard meta tags.
pub fn get_article_metadata(document: &Html, json_ld: Metadata) -> Metadata {
    let mut values: HashMap<String, String> = HashMap::new();
    static META_PROPERTY_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r"(?i)\s*(article|dc|dcterm|og|twitter)\s*:\s*(author|creator|description|published_time|title|site_name|image:url|image:secure_url|image$)\s*"
        ).unwrap()
    });
    static META_NAME_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r"(?i)^\s*(?:(?:article|dc|dcterm|og|twitter|parsely|weibo:(?:article|webpage))\s*[-\.:]\s*)?(author|author_name|creator|pub-date|description|title|site_name|image|thumbnail)\s*$"
        ).unwrap()
    });

    let property_pattern = &*META_PROPERTY_REGEX;
    let name_pattern = &*META_NAME_REGEX;

    let meta_selector = Selector::parse("meta").unwrap();
    for meta in document.select(&meta_selector) {
        let element_name = meta.value().attr("name");
        let element_property = meta.value().attr("property");
        let content = meta.value().attr("content");

        if content.is_none() || content.unwrap().is_empty() {
            continue;
        }

        let content = content.unwrap();
        let mut matched_name: Option<String> = None;

        if let Some(property) = element_property {
            // Handle space-separated properties (e.g., "dc:creator twitter:site_name")
            // Split on whitespace and process each property separately
            for prop in property.split_whitespace() {
                if let Some(mat) = property_pattern.find(prop) {
                    let key = prop[mat.start()..mat.end()]
                        .to_lowercase()
                        .replace(char::is_whitespace, "");
                    values.insert(key, content.trim().to_string());
                    matched_name = Some(property.to_string());
                }
            }
        }
        // Check name attribute if property didn't match
        if matched_name.is_none() {
            if let Some(name) = element_name {
                if name_pattern.is_match(name) {
                    let normalized = name
                        .to_lowercase()
                        .replace(char::is_whitespace, "")
                        .replace('.', ":");
                    values.insert(normalized, content.trim().to_string());
                }
            }
        }
    }

    let mut metadata = Metadata {
        title: json_ld.title.or_else(|| {
            values
                .get("dc:title")
                .or_else(|| values.get("dcterm:title"))
                .or_else(|| values.get("og:title"))
                .or_else(|| values.get("weibo:article:title"))
                .or_else(|| values.get("weibo:webpage:title"))
                .or_else(|| values.get("title"))
                .or_else(|| values.get("twitter:title"))
                .or_else(|| values.get("parsely-title"))
                .cloned()
        }),
        ..Default::default()
    };

    if metadata.title.is_none() {
        metadata.title = extract_title_from_document(document);
    }

    if metadata.title.is_none() {
        metadata.title = Some(String::new());
    }

    let article_author = values
        .get("article:author")
        .or_else(|| values.get("article:author_name"))
        .filter(|v| !utils::is_url(v))
        .cloned();

    let dom_byline = extract_byline_from_document(document);
    let mut meta_byline = json_ld.byline.or_else(|| {
        values
            .get("dc:creator")
            .or_else(|| values.get("dcterm:creator"))
            .or_else(|| values.get("author"))
            .or_else(|| values.get("parsely-author"))
            .or(article_author.as_ref())
            .cloned()
    });

    if let Some(dom_value) = dom_byline.clone() {
        let dom_text = dom_value.text.clone();
        match &meta_byline {
            Some(existing) => {
                if should_prefer_dom_byline(existing, &dom_text, dom_value.confidence) {
                    meta_byline = Some(dom_text);
                }
            }
            None => meta_byline = Some(dom_text),
        }
    }

    metadata.byline = meta_byline;

    metadata.excerpt = json_ld.excerpt.or_else(|| {
        values
            .get("dc:description")
            .or_else(|| values.get("dcterm:description"))
            .or_else(|| values.get("og:description"))
            .or_else(|| values.get("weibo:article:description"))
            .or_else(|| values.get("weibo:webpage:description"))
            .or_else(|| values.get("description"))
            .or_else(|| values.get("twitter:description"))
            .cloned()
    });

    metadata.site_name = json_ld
        .site_name
        .or_else(|| values.get("og:site_name").cloned());

    metadata.published_time = json_ld.published_time.or_else(|| {
        values
            .get("article:published_time")
            .or_else(|| values.get("parsely-pub-date"))
            .cloned()
    });

    // Extract image from meta tags with priority order
    metadata.image = json_ld.image.or_else(|| {
        values
            .get("og:image:secure_url")
            .or_else(|| values.get("og:image:url"))
            .or_else(|| values.get("og:image"))
            .or_else(|| values.get("twitter:image"))
            .or_else(|| values.get("thumbnail"))
            .or_else(|| values.get("image"))
            .cloned()
    });

    // If no image found in standard meta tags, try additional sources
    if metadata.image.is_none() {
        metadata.image = extract_image_from_document(document);
    }

    metadata.lang = extract_language_from_document(document);

    metadata.title = metadata.title.map(|t| utils::unescape_html_entities(&t));
    metadata.byline = metadata
        .byline
        .map(|b| utils::unescape_html_entities(&b))
        .and_then(|b| utils::clean_byline_text(&b));
    metadata.excerpt = metadata
        .excerpt
        .map(|e| utils::unescape_html_entities(&e))
        .and_then(|e| {
            let trimmed = e.trim();
            if trimmed.is_empty() {
                return None;
            }
            if utils::looks_like_bracket_menu(trimmed) {
                return None;
            }
            Some(e)
        });
    metadata.site_name = metadata
        .site_name
        .map(|s| utils::unescape_html_entities(&s));

    if let (Some(existing), Some(dom_value)) = (metadata.byline.clone(), dom_byline.clone()) {
        if should_prefer_dom_byline(&existing, &dom_value.text, dom_value.confidence) {
            metadata.byline =
                utils::clean_byline_text(&dom_value.text).or_else(|| Some(dom_value.text.clone()));
        }
    }

    if let Some(caps_candidate) = extract_standfirst_caps_byline(document) {
        match &metadata.byline {
            Some(existing) => {
                if should_prefer_caps_standfirst(existing, &caps_candidate) {
                    metadata.byline = Some(caps_candidate);
                }
            }
            None => metadata.byline = Some(caps_candidate),
        }
    }

    if let (Some(byline), Some(site_name)) = (metadata.byline.clone(), metadata.site_name.clone()) {
        if utils::is_byline_redundant_with_site_name(&byline, &site_name) {
            metadata.byline = None;
        }
    }

    metadata.published_time = metadata
        .published_time
        .map(|p| utils::unescape_html_entities(&p));

    // Clean up image URL
    metadata.image = metadata.image.and_then(|img| {
        let trimmed = img.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(utils::unescape_html_entities(trimmed))
    });

    metadata
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_tag_extraction() {
        let html = r#"
            <html>
                <head>
                    <meta property="og:title" content="OG Title" />
                    <meta name="author" content="Jane Smith" />
                    <meta property="og:description" content="OG Description" />
                </head>
            </html>
        "#;

        let document = Html::parse_document(html);
        let json_ld = Metadata::default();
        let metadata = get_article_metadata(&document, json_ld);

        assert_eq!(metadata.title, Some("OG Title".to_string()));
        assert_eq!(metadata.byline, Some("Jane Smith".to_string()));
        assert_eq!(metadata.excerpt, Some("OG Description".to_string()));
    }

    #[test]
    fn test_og_image_extraction() {
        let html = r#"
            <html>
                <head>
                    <meta property="og:title" content="OG Title" />
                    <meta property="og:image" content="https://example.com/og-image.jpg" />
                </head>
            </html>
        "#;

        let document = Html::parse_document(html);
        let metadata = get_article_metadata(&document, Metadata::default());

        assert_eq!(
            metadata.image,
            Some("https://example.com/og-image.jpg".to_string())
        );
    }

    #[test]
    fn test_og_image_secure_url_priority() {
        let html = r#"
            <html>
                <head>
                    <meta property="og:image" content="http://example.com/image.jpg" />
                    <meta property="og:image:secure_url" content="https://example.com/secure-image.jpg" />
                </head>
            </html>
        "#;

        let document = Html::parse_document(html);
        let metadata = get_article_metadata(&document, Metadata::default());

        assert_eq!(
            metadata.image,
            Some("https://example.com/secure-image.jpg".to_string())
        );
    }

    #[test]
    fn test_twitter_image_extraction() {
        let html = r#"
            <html>
                <head>
                    <meta name="twitter:image" content="https://example.com/twitter-image.jpg" />
                    <meta name="twitter:image:alt" content="Twitter alt text" />
                </head>
            </html>
        "#;

        let document = Html::parse_document(html);
        let metadata = get_article_metadata(&document, Metadata::default());

        assert_eq!(
            metadata.image,
            Some("https://example.com/twitter-image.jpg".to_string())
        );
    }

    #[test]
    fn test_json_ld_image_takes_priority() {
        let html = r#"
            <html>
                <head>
                    <script type="application/ld+json">
                    {
                        "@context": "https://schema.org",
                        "@type": "Article",
                        "headline": "Test Article",
                        "image": "https://example.com/json-ld-image.jpg"
                    }
                    </script>
                    <meta property="og:image" content="https://example.com/og-image.jpg" />
                </head>
            </html>
        "#;

        let document = Html::parse_document(html);
        let json_ld = get_json_ld(&document);
        let metadata = get_article_metadata(&document, json_ld);

        assert_eq!(
            metadata.image,
            Some("https://example.com/json-ld-image.jpg".to_string())
        );
    }

    #[test]
    fn test_link_image_src_extraction() {
        let html = r#"
            <html>
                <head>
                    <link rel="image_src" href="https://example.com/link-image.jpg" />
                </head>
            </html>
        "#;

        let document = Html::parse_document(html);
        let metadata = get_article_metadata(&document, Metadata::default());

        assert_eq!(
            metadata.image,
            Some("https://example.com/link-image.jpg".to_string())
        );
    }

    #[test]
    fn test_itemprop_image_extraction() {
        let html = r#"
            <html>
                <head>
                    <meta itemprop="image" content="https://example.com/itemprop-image.jpg" />
                </head>
            </html>
        "#;

        let document = Html::parse_document(html);
        let metadata = get_article_metadata(&document, Metadata::default());

        assert_eq!(
            metadata.image,
            Some("https://example.com/itemprop-image.jpg".to_string())
        );
    }

    #[test]
    fn test_thumbnail_meta_extraction() {
        let html = r#"
            <html>
                <head>
                    <meta name="thumbnail" content="https://example.com/thumbnail.jpg" />
                </head>
            </html>
        "#;

        let document = Html::parse_document(html);
        let metadata = get_article_metadata(&document, Metadata::default());

        assert_eq!(
            metadata.image,
            Some("https://example.com/thumbnail.jpg".to_string())
        );
    }

    #[test]
    fn test_article_author_name_meta_is_respected() {
        let html = r#"
            <html>
                <head>
                    <meta name="article:author_name" content="Hazel Sheffield" />
                </head>
            </html>
        "#;

        let document = Html::parse_document(html);
        let metadata = get_article_metadata(&document, Metadata::default());

        assert_eq!(metadata.byline, Some("Hazel Sheffield".to_string()));
    }
}
