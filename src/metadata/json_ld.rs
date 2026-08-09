//! JSON-LD structured-data extraction.

use super::Metadata;
use crate::constants::REGEXPS;
use scraper::{Html, Selector};
use serde_json::Value;
use std::sync::LazyLock;

/// Extract JSON-LD structured data from document
///
/// Looks for <script type="application/ld+json"> tags and parses them for article metadata.
/// Supports Schema.org Article types.
pub fn get_json_ld(document: &Html) -> Metadata {
    let mut metadata = Metadata::default();

    static SCHEMA_ORG_CONTEXT_REGEX: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^https?://schema\.org/?$").unwrap());

    let script_selector = Selector::parse("script[type='application/ld+json']").unwrap();
    let schema_regex = &*SCHEMA_ORG_CONTEXT_REGEX;

    for script in document.select(&script_selector) {
        let content = script.text().collect::<String>();

        // Strip CDATA markers if present
        let content = content
            .trim()
            .trim_start_matches("<![CDATA[")
            .trim_end_matches("]]>")
            .trim();

        if let Ok(mut parsed) = serde_json::from_str::<Value>(content) {
            if let Some(arr) = parsed.as_array() {
                if let Some(article) = arr.iter().find(|item| {
                    if let Some(type_val) = item.get("@type") {
                        if let Some(type_str) = type_val.as_str() {
                            return REGEXPS.json_ld_article_types.is_match(type_str);
                        }
                    }
                    false
                }) {
                    parsed = article.clone();
                } else {
                    continue;
                }
            }

            // Check for schema.org context
            let has_schema_context = if let Some(context) = parsed.get("@context") {
                if let Some(ctx_str) = context.as_str() {
                    schema_regex.is_match(ctx_str)
                } else if let Some(ctx_obj) = context.as_object() {
                    if let Some(vocab) = ctx_obj.get("@vocab").and_then(|v| v.as_str()) {
                        schema_regex.is_match(vocab)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if !has_schema_context {
                continue;
            }

            // Check for @graph array
            if parsed.get("@type").is_none() {
                if let Some(graph) = parsed.get("@graph").and_then(|g| g.as_array()) {
                    if let Some(article) = graph.iter().find(|item| {
                        if let Some(type_val) = item.get("@type") {
                            if let Some(type_str) = type_val.as_str() {
                                return REGEXPS.json_ld_article_types.is_match(type_str);
                            }
                        }
                        false
                    }) {
                        parsed = article.clone();
                    }
                }
            }

            // Verify it's an article type
            if let Some(type_val) = parsed.get("@type") {
                if let Some(type_str) = type_val.as_str() {
                    if !REGEXPS.json_ld_article_types.is_match(type_str) {
                        continue;
                    }
                } else {
                    continue;
                }
            } else {
                continue;
            }

            // Extract title (name or headline)
            // Schema.org is flexible: "name" can be the article title OR publisher name
            // Heuristic: if "name" matches publisher name, use "headline" instead
            let name = parsed.get("name").and_then(|v| v.as_str());
            let headline = parsed.get("headline").and_then(|v| v.as_str());
            let publisher_name = parsed
                .get("publisher")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str());

            if metadata.title.is_none() {
                if let (Some(name_str), Some(pub_name)) = (name, publisher_name) {
                    if name_str.trim() == pub_name.trim() {
                        if let Some(headline_str) = headline {
                            metadata.title = Some(headline_str.trim().to_string());
                        }
                    } else {
                        metadata.title = Some(name_str.trim().to_string());
                    }
                } else if let Some(name_str) = name {
                    metadata.title = Some(name_str.trim().to_string());
                } else if let Some(headline_str) = headline {
                    metadata.title = Some(headline_str.trim().to_string());
                }
            }

            if metadata.byline.is_none() {
                if let Some(author) = parsed.get("author") {
                    if let Some(author_name) = author.get("name").and_then(|v| v.as_str()) {
                        metadata.byline = Some(author_name.trim().to_string());
                    } else if let Some(authors) = author.as_array() {
                        let names: Vec<String> = authors
                            .iter()
                            .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                            .map(|n| n.trim().to_string())
                            .collect();
                        if !names.is_empty() {
                            metadata.byline = Some(names.join(", "));
                        }
                    }
                }
            }

            if metadata.excerpt.is_none() {
                if let Some(description) = parsed.get("description").and_then(|v| v.as_str()) {
                    metadata.excerpt = Some(description.trim().to_string());
                }
            }

            if metadata.site_name.is_none() {
                if let Some(publisher) = parsed.get("publisher") {
                    if let Some(pub_name) = publisher.get("name").and_then(|v| v.as_str()) {
                        metadata.site_name = Some(pub_name.trim().to_string());
                    }
                }
            }

            if metadata.published_time.is_none() {
                if let Some(date_published) = parsed.get("datePublished").and_then(|v| v.as_str()) {
                    metadata.published_time = Some(date_published.trim().to_string());
                }
            }

            // Extract image from JSON-LD
            if metadata.image.is_none() {
                metadata.image = extract_json_ld_image(&parsed);
            }
        }
    }

    metadata
}

/// Extract image URL from JSON-LD data
///
/// Handles various Schema.org image formats:
/// - Simple string URL
/// - ImageObject with url property
/// - Array of images (takes first)
fn extract_json_ld_image(parsed: &Value) -> Option<String> {
    if let Some(image) = parsed.get("image") {
        // Simple string URL
        if let Some(url) = image.as_str() {
            let trimmed = url.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }

        // ImageObject with url property
        if let Some(url) = image.get("url").and_then(|v| v.as_str()) {
            let trimmed = url.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }

        // ImageObject with @id property
        if let Some(id) = image.get("@id").and_then(|v| v.as_str()) {
            let trimmed = id.trim();
            if !trimmed.is_empty()
                && (trimmed.starts_with("http://") || trimmed.starts_with("https://"))
            {
                return Some(trimmed.to_string());
            }
        }

        // Array of images - take the first valid one
        if let Some(arr) = image.as_array() {
            for img in arr {
                if let Some(url) = img.as_str() {
                    let trimmed = url.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
                if let Some(url) = img.get("url").and_then(|v| v.as_str()) {
                    let trimmed = url.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
            }
        }
    }

    // Also check thumbnailUrl as fallback
    if let Some(thumbnail) = parsed.get("thumbnailUrl").and_then(|v| v.as_str()) {
        let trimmed = thumbnail.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_ld_extraction() {
        let html = r#"
            <html>
                <head>
                    <script type="application/ld+json">
                    {
                        "@context": "https://schema.org",
                        "@type": "Article",
                        "headline": "Test Article",
                        "author": {"name": "John Doe"},
                        "description": "Test description"
                    }
                    </script>
                </head>
            </html>
        "#;

        let document = Html::parse_document(html);
        let metadata = get_json_ld(&document);

        assert_eq!(metadata.title, Some("Test Article".to_string()));
        assert_eq!(metadata.byline, Some("John Doe".to_string()));
        assert_eq!(metadata.excerpt, Some("Test description".to_string()));
    }

    #[test]
    fn test_json_ld_image_extraction() {
        let html = r#"
            <html>
                <head>
                    <script type="application/ld+json">
                    {
                        "@context": "https://schema.org",
                        "@type": "Article",
                        "headline": "Test Article",
                        "image": "https://example.com/image.jpg"
                    }
                    </script>
                </head>
            </html>
        "#;

        let document = Html::parse_document(html);
        let metadata = get_json_ld(&document);

        assert_eq!(
            metadata.image,
            Some("https://example.com/image.jpg".to_string())
        );
    }

    #[test]
    fn test_json_ld_image_object_extraction() {
        let html = r#"
            <html>
                <head>
                    <script type="application/ld+json">
                    {
                        "@context": "https://schema.org",
                        "@type": "Article",
                        "headline": "Test Article",
                        "image": {
                            "@type": "ImageObject",
                            "url": "https://example.com/image.jpg",
                            "width": 1200,
                            "height": 630
                        }
                    }
                    </script>
                </head>
            </html>
        "#;

        let document = Html::parse_document(html);
        let metadata = get_json_ld(&document);

        assert_eq!(
            metadata.image,
            Some("https://example.com/image.jpg".to_string())
        );
    }

    #[test]
    fn test_json_ld_image_array_extraction() {
        let html = r#"
            <html>
                <head>
                    <script type="application/ld+json">
                    {
                        "@context": "https://schema.org",
                        "@type": "Article",
                        "headline": "Test Article",
                        "image": [
                            "https://example.com/image1.jpg",
                            "https://example.com/image2.jpg"
                        ]
                    }
                    </script>
                </head>
            </html>
        "#;

        let document = Html::parse_document(html);
        let metadata = get_json_ld(&document);

        assert_eq!(
            metadata.image,
            Some("https://example.com/image1.jpg".to_string())
        );
    }
}
