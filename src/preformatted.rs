//! Splitting serialized HTML around spans that must not be rewritten.
//!
//! Several cleanup passes in this crate operate on serialized HTML with
//! regexes. A regex cannot tell layout whitespace from significant whitespace
//! inside a code listing, so running one over a whole document flattens the
//! listings along with the markup around them. [`map_outside_preformatted`]
//! splits the input first, so a rewrite only ever sees the markup it may touch.

/// Tag names copied through verbatim.
///
/// `<pre>` is preformatted by CSS, so its whitespace is data. `<code>` is not —
/// browsers collapse whitespace inside an inline code span — but it is listed
/// anyway: the Markdown output reproduces code spans literally, and highlighter
/// markup nests `<code>` inside `<pre>` where the inner tag carries the listing.
const PREFORMATTED_TAGS: [&str; 2] = ["pre", "code"];

/// Rewrite every stretch of `html` that lies outside a preformatted element or
/// a comment, copying those spans through unchanged.
///
/// Comments are opaque for two reasons. Their bodies are the one channel into
/// the cleanup passes carrying unescaped `<`, so a commented-out `</pre>` would
/// close a listing on markup the browser never renders, and a commented-out
/// `<code>` would open a phantom block.
///
/// An unterminated `<pre>`/`<code>` protects the rest of the input: preserving
/// too much is recoverable, mangling a listing is not.
pub(crate) fn map_outside_preformatted(html: &str, rewrite: impl Fn(&str) -> String) -> String {
    let mut out = String::with_capacity(html.len());
    let mut plain = 0usize;
    let mut cursor = 0usize;

    while let Some(offset) = html[cursor..].find('<') {
        let lt = cursor + offset;
        let tail = &html[lt..];

        let opaque_len = comment_len(tail).or_else(|| {
            preformatted_tag_at(tail).map(|tag| block_end(tail, tag).unwrap_or(tail.len()))
        });

        let Some(len) = opaque_len else {
            cursor = lt + 1;
            continue;
        };

        out.push_str(&rewrite(&html[plain..lt]));
        out.push_str(&tail[..len]);
        plain = lt + len;
        cursor = plain;
    }

    out.push_str(&rewrite(&html[plain..]));
    out
}

/// The preformatted tag opened by the `<…` at the start of `tail`, if any.
fn preformatted_tag_at(tail: &str) -> Option<&'static str> {
    let after_lt = tail.strip_prefix('<')?;
    PREFORMATTED_TAGS
        .into_iter()
        .find(|tag| starts_with_tag_name(after_lt, tag))
}

/// Byte length of the comment starting at `tail`, or `None` when `tail` does
/// not open one.
///
/// An unterminated comment counts as no comment: treating it as opaque would
/// silently swallow the whole rest of the document, and its body is not
/// rendered anyway, so rewriting it costs nothing.
fn comment_len(tail: &str) -> Option<usize> {
    let body = tail.strip_prefix("<!--")?;
    body.find("-->")
        .map(|end| tail.len() - body.len() + end + "-->".len())
}

/// Byte length of the `tag` element that starts at the beginning of `block`, or
/// `None` when the closing tag is missing.
///
/// Nesting is tracked so an inner `<code>` inside `<code>` cannot close the
/// outer one early.
fn block_end(block: &str, tag: &str) -> Option<usize> {
    let mut depth = 1usize;
    let mut cursor = "<".len() + tag.len();

    while let Some(offset) = block[cursor..].find('<') {
        let lt = cursor + offset;
        let tail = &block[lt..];

        if let Some(len) = comment_len(tail) {
            cursor = lt + len;
        } else if tail.starts_with("</") && starts_with_tag_name(&tail[2..], tag) {
            let end = lt + tail.find('>')? + 1;
            depth -= 1;
            if depth == 0 {
                return Some(end);
            }
            cursor = end;
        } else {
            if starts_with_tag_name(&tail[1..], tag) {
                depth += 1;
            }
            cursor = lt + 1;
        }
    }

    None
}

/// Whether `after_lt` begins with `tag` followed by a tag-name terminator, so
/// that `<pre>`/`<pre class=…>` match `pre` but `<precondition>` does not.
///
/// The terminators are `>` plus the HTML5 space characters (`\x0c` is form
/// feed). `/` is deliberately absent: the serializer never self-closes a
/// `<pre>`/`<code>`, and treating `<pre/>` as an opening tag would make
/// everything after it opaque.
fn starts_with_tag_name(after_lt: &str, tag: &str) -> bool {
    let (bytes, name) = (after_lt.as_bytes(), tag.as_bytes());
    bytes.len() > name.len()
        && bytes[..name.len()].eq_ignore_ascii_case(name)
        && matches!(
            bytes[name.len()],
            b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' | b'>'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collapse runs of spaces, so a test can see which stretches were rewritten.
    fn squeeze(html: &str) -> String {
        map_outside_preformatted(html, |chunk| {
            let mut out = String::with_capacity(chunk.len());
            let mut spaces = 0usize;
            for ch in chunk.chars() {
                if ch == ' ' {
                    spaces += 1;
                    if spaces > 1 {
                        continue;
                    }
                } else {
                    spaces = 0;
                }
                out.push(ch);
            }
            out
        })
    }

    #[test]
    fn test_rewrites_plain_markup() {
        assert_eq!(squeeze("<p>a    b</p>"), "<p>a b</p>");
    }

    #[test]
    fn test_preserves_pre_and_nested_code() {
        let html = "<pre tabindex=\"0\"><code>fn f() {\n    body();\n}</code></pre>";

        assert_eq!(squeeze(html), html);
    }

    #[test]
    fn test_preserves_inline_code() {
        assert_eq!(
            squeeze("<p>use    <code>a    b</code>    now</p>"),
            "<p>use <code>a    b</code> now</p>"
        );
    }

    #[test]
    fn test_preserves_every_block_in_place() {
        assert_eq!(
            squeeze("<pre>  one</pre><p>x    y</p><pre>  one</pre><p>z    w</p>"),
            "<pre>  one</pre><p>x y</p><pre>  one</pre><p>z w</p>",
            "identical-looking blocks must each be preserved where they sit"
        );
    }

    #[test]
    fn test_inner_close_tag_does_not_end_outer_block() {
        let html = "<pre><code>outer  <code>inner  </code>  tail</code></pre>";

        assert_eq!(
            squeeze(&format!("{html}<p>a    b</p>")),
            format!("{html}<p>a b</p>")
        );
    }

    #[test]
    fn test_matches_tag_names_case_insensitively() {
        let html = "<PRE>  kept  </PRE><p>a    b</p><code>  x  </code >";

        assert_eq!(
            squeeze(html),
            "<PRE>  kept  </PRE><p>a b</p><code>  x  </code >",
            "serialized markup is not guaranteed lowercase or tightly closed"
        );
    }

    #[test]
    fn test_ignores_tag_name_prefixes() {
        assert_eq!(
            squeeze("<precondition>a    b</precondition><codex>c    d</codex>"),
            "<precondition>a b</precondition><codex>c d</codex>"
        );
    }

    #[test]
    fn test_self_closing_pre_is_not_an_opening_tag() {
        assert_eq!(
            squeeze("<pre/><p>a    b</p>"),
            "<pre/><p>a b</p>",
            "the serializer never emits this; treating it as an open tag would \
             make the rest of the document opaque"
        );
    }

    #[test]
    fn test_unterminated_block_preserves_remainder() {
        let html = "<p>a    b</p><pre>  dangling    tail";

        assert_eq!(
            squeeze(html),
            "<p>a b</p><pre>  dangling    tail",
            "a missing close tag should preserve, never mangle"
        );
    }

    #[test]
    fn test_close_tag_inside_comment_does_not_end_block() {
        let html = "<pre><!-- </pre> -->fn f() {\n    body();\n}</pre>";

        assert_eq!(squeeze(html), html);
    }

    #[test]
    fn test_open_tag_inside_comment_opens_nothing() {
        assert_eq!(
            squeeze("<p>old: <!-- <code> --></p><p>a    b</p>"),
            "<p>old: <!-- <code> --></p><p>a b</p>"
        );
    }

    #[test]
    fn test_comment_body_is_copied_verbatim() {
        assert_eq!(
            squeeze("<!--  a    b  --><p>c    d</p>"),
            "<!--  a    b  --><p>c d</p>",
            "a comment is skipped whole, so its body is untouched"
        );
    }

    #[test]
    fn test_unterminated_comment_does_not_swallow_the_document() {
        assert_eq!(
            squeeze("<p>a    b</p><!-- <p>c    d</p>"),
            "<p>a b</p><!-- <p>c d</p>",
            "an unterminated comment must not disable the rest of the pass"
        );
    }

    #[test]
    fn test_never_loses_bytes() {
        let inputs = [
            "<pre><code>é→𝄞\n    ind\n</code></pre><p>a    b</p>",
            "<!--<pre>--><pre><!--</pre>--></pre>",
            "<pre",
            "<code></code></code>",
            "",
            "<",
        ];

        for html in inputs {
            assert_eq!(
                map_outside_preformatted(html, str::to_string),
                html,
                "identity rewrite must round-trip {html:?}"
            );
        }
    }
}
