use regex::Regex;
use std::sync::LazyLock;

use crate::markdown::options::MarkdownOptions;
use crate::markdown::rules::text::escape_url_destination;

static YOUTUBE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)youtube\.com|youtu\.be").unwrap());
static TWITTER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)twitter\.com|x\.com/\w+/status").unwrap());

/// Convert `<iframe>` to markdown placeholder.
pub fn convert_iframe(src: &str, opts: &MarkdownOptions) -> String {
    if src.is_empty() {
        return String::new();
    }
    if opts.sanitize_urls && crate::content_extractor::is_dangerous_url(src) {
        return String::new();
    }

    let label = if YOUTUBE_RE.is_match(src) {
        "Video"
    } else if TWITTER_RE.is_match(src) {
        "Tweet"
    } else {
        "Embed"
    };

    format!("[{}]({})", label, escape_url_destination(src))
}

/// Convert `<video>` to markdown placeholder.
pub fn convert_video(src: &str, opts: &MarkdownOptions) -> String {
    if src.is_empty() {
        return String::new();
    }
    if opts.sanitize_urls && crate::content_extractor::is_dangerous_url(src) {
        return String::new();
    }
    format!("[Video]({})", escape_url_destination(src))
}

/// Convert `<audio>` to markdown placeholder.
pub fn convert_audio(src: &str, opts: &MarkdownOptions) -> String {
    if src.is_empty() {
        return String::new();
    }
    if opts.sanitize_urls && crate::content_extractor::is_dangerous_url(src) {
        return String::new();
    }
    format!("[Audio]({})", escape_url_destination(src))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> MarkdownOptions {
        MarkdownOptions::default()
    }

    #[test]
    fn test_youtube_iframe() {
        let result = convert_iframe("https://www.youtube.com/embed/abc123", &opts());
        assert_eq!(result, "[Video](https://www.youtube.com/embed/abc123)");
    }

    #[test]
    fn test_twitter_iframe() {
        let result = convert_iframe("https://twitter.com/user/status/123", &opts());
        assert_eq!(result, "[Tweet](https://twitter.com/user/status/123)");
    }

    #[test]
    fn test_generic_iframe() {
        let result = convert_iframe("https://example.com/widget", &opts());
        assert_eq!(result, "[Embed](https://example.com/widget)");
    }

    #[test]
    fn test_video_src_with_paren_wrapped() {
        let result = convert_video("http://e.com/) [x](http://evil.com)", &opts());
        assert_eq!(result, "[Video](<http://e.com/) [x](http://evil.com)>)");
    }

    #[test]
    fn test_iframe_dropped_when_sanitizing_dangerous_src() {
        let opts = MarkdownOptions {
            sanitize_urls: true,
            ..Default::default()
        };
        assert_eq!(convert_iframe("javascript:evil()", &opts), "");
    }

    #[test]
    fn test_video_dropped_when_sanitizing_dangerous_src() {
        let opts = MarkdownOptions {
            sanitize_urls: true,
            ..Default::default()
        };
        assert_eq!(convert_video("javascript:evil()", &opts), "");
    }
}
