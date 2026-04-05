use once_cell::sync::Lazy;
use regex::Regex;

static YOUTUBE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)youtube\.com|youtu\.be").unwrap());
static TWITTER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)twitter\.com|x\.com/\w+/status").unwrap());

/// Convert `<iframe>` to markdown placeholder.
pub fn convert_iframe(src: &str) -> String {
    if src.is_empty() {
        return String::new();
    }

    let label = if YOUTUBE_RE.is_match(src) {
        "Video"
    } else if TWITTER_RE.is_match(src) {
        "Tweet"
    } else {
        "Embed"
    };

    format!("[{}]({})", label, src)
}

/// Convert `<video>` to markdown placeholder.
pub fn convert_video(src: &str) -> String {
    if src.is_empty() {
        return String::new();
    }
    format!("[Video]({})", src)
}

/// Convert `<audio>` to markdown placeholder.
pub fn convert_audio(src: &str) -> String {
    if src.is_empty() {
        return String::new();
    }
    format!("[Audio]({})", src)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_youtube_iframe() {
        let result = convert_iframe("https://www.youtube.com/embed/abc123");
        assert_eq!(result, "[Video](https://www.youtube.com/embed/abc123)");
    }

    #[test]
    fn test_twitter_iframe() {
        let result = convert_iframe("https://twitter.com/user/status/123");
        assert_eq!(result, "[Tweet](https://twitter.com/user/status/123)");
    }

    #[test]
    fn test_generic_iframe() {
        let result = convert_iframe("https://example.com/widget");
        assert_eq!(result, "[Embed](https://example.com/widget)");
    }
}
