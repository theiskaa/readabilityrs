use once_cell::sync::Lazy;
use regex::Regex;

static BASE64_PLACEHOLDER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^data:image/(gif|png|jpeg|svg);base64,[A-Za-z0-9+/=]{0,200}$").unwrap());

static SRCSET_ENTRY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\S+)\s+(\d+\.?\d*)([wx])").unwrap());

static IMG_TAG_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?si)<img\s[^>]*?/?>").unwrap());

static WIDTH_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"width="(\d+)""#).unwrap());

static HEIGHT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"height="(\d+)""#).unwrap());

static SRC_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"\ssrc="([^"]*)""#).unwrap());

static DATA_SRC_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"data-src="([^"]*)""#).unwrap());

static DATA_SRCSET_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"data-srcset="([^"]*)""#).unwrap());

static SRCSET_ATTR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"srcset="([^"]*)""#).unwrap());

/// Standardize images:
/// 1. Resolve lazy-loaded images (`data-src` → `src`).
/// 2. Pick best source from `srcset`.
/// 3. Remove tiny images (width AND height both < 100).
pub fn standardize_images(html: &str) -> String {
    IMG_TAG_RE.replace_all(html, |caps: &regex::Captures| {
        let full = &caps[0];

        // Check for small images
        let width: Option<u32> = WIDTH_RE.captures(full).and_then(|c| c[1].parse().ok());
        let height: Option<u32> = HEIGHT_RE.captures(full).and_then(|c| c[1].parse().ok());
        if let (Some(w), Some(h)) = (width, height) {
            if w < 100 && h < 100 {
                return String::new();
            }
        }

        // Resolve lazy-loaded src
        let src = SRC_RE.captures(full).map(|c| c[1].to_string()).unwrap_or_default();
        let data_src = DATA_SRC_RE.captures(full).map(|c| c[1].to_string()).unwrap_or_default();

        let mut result = full.to_string();

        if (src.is_empty() || is_placeholder_src(&src)) && !data_src.is_empty() {
            if src.is_empty() {
                result = result.replacen("<img", &format!("<img src=\"{}\"", escape_attr(&data_src)), 1);
            } else {
                // Use space-prefixed pattern to avoid matching data-src
                result = replace_src_attr(&result, &src, &escape_attr(&data_src));
            }
        }

        // Handle srcset / data-srcset
        let srcset = SRCSET_ATTR_RE.captures(full).map(|c| c[1].to_string()).unwrap_or_default();
        let data_srcset = DATA_SRCSET_RE.captures(full).map(|c| c[1].to_string()).unwrap_or_default();
        let effective = if !data_srcset.is_empty() && srcset.is_empty() { &data_srcset } else { &srcset };
        if !effective.is_empty() {
            if let Some(best) = pick_best_srcset(effective) {
                let current_src = SRC_RE.captures(&result).map(|c| c[1].to_string()).unwrap_or_default();
                if !current_src.is_empty() {
                    result = replace_src_attr(&result, &current_src, &escape_attr(&best));
                }
            }
        }

        result
    }).to_string()
}

fn is_placeholder_src(src: &str) -> bool {
    if src.starts_with("data:") {
        return BASE64_PLACEHOLDER_RE.is_match(src);
    }
    src.contains("placeholder") || src.contains("blank.gif") || src.contains("spacer.gif")
}

/// Parse srcset and pick the largest source by width or density.
///
/// Handles both space-separated and comma-only-separated entries:
/// `"small.jpg 400w, large.jpg 1200w"` and `"small.jpg 400w,large.jpg 1200w"`.
pub fn pick_best_srcset(srcset: &str) -> Option<String> {
    let mut best_url = None;
    let mut best_value: f64 = 0.0;

    // Split on comma first, then parse each entry individually
    for entry in srcset.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        if let Some(cap) = SRCSET_ENTRY_RE.captures(entry) {
            let url = cap[1].to_string();
            let value: f64 = cap[2].parse().unwrap_or(0.0);
            if value > best_value {
                best_value = value;
                best_url = Some(url);
            }
        } else {
            // Entry without a descriptor — use as fallback if nothing better found
            let url = entry.split_whitespace().next().unwrap_or("");
            if best_url.is_none() && !url.is_empty() {
                best_url = Some(url.to_string());
            }
        }
    }

    best_url
}

/// Replace the `src` attribute value without accidentally matching `data-src`.
fn replace_src_attr(html: &str, old_val: &str, new_val: &str) -> String {
    let old_pattern = format!(" src=\"{}\"", old_val);
    let new_pattern = format!(" src=\"{}\"", new_val);
    html.replacen(&old_pattern, &new_pattern, 1)
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_load_resolution() {
        let html = r#"<img data-src="real.jpg" src="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7" alt="test"/>"#;
        let result = standardize_images(html);
        assert!(result.contains("src=\"real.jpg\""));
    }

    #[test]
    fn test_small_image_removal() {
        let html = r#"<p>Text <img src="tiny.png" width="32" height="32"/> more</p>"#;
        let result = standardize_images(html);
        assert!(!result.contains("tiny.png"));
    }

    #[test]
    fn test_srcset_best_pick() {
        let srcset = "small.jpg 400w, medium.jpg 800w, large.jpg 1200w";
        assert_eq!(pick_best_srcset(srcset), Some("large.jpg".to_string()));
    }

    #[test]
    fn test_normal_image_preserved() {
        let html = r#"<img src="photo.jpg" alt="Nice photo" width="800" height="600"/>"#;
        let result = standardize_images(html);
        assert!(result.contains("photo.jpg"));
    }
}
