# Security Policy

readabilityrs exists to parse HTML that its caller did not write. Anyone using it to process pages fetched from the open web is running an untrusted-input parser, usually inside a crawler, a reader app, or a content pipeline, and often at scale. That makes responsible disclosure genuinely valuable. Reports are appreciated.

## Reporting a vulnerability

**Please do not open a public issue for a security problem.**

Report privately through GitHub's [private vulnerability reporting](https://github.com/theiskaa/readabilityrs/security/advisories/new), the "Report a vulnerability" button under the repository's **Security** tab. If you cannot use that, email **me@theiskaa.com** with the details.

A useful report includes:

- the version or commit you tested,
- whether `sanitize_content` and `output_markdown` were enabled,
- a clear description of the issue and its impact,
- the steps to reproduce it, ideally a minimal HTML document that triggers it,
- and any thoughts on a fix, if you have them.

Please give a reasonable window to investigate and address the issue before any public disclosure. You will get an acknowledgement, updates as the fix progresses, and credit in the release notes if you would like it.

## The trust model, so expectations are clear

The input is hostile by assumption. The output is not trusted either, and that is the part most callers get wrong:

- **Extracted HTML is not sanitized.** `Article::content` is assembled from the page's own elements and attributes. Event handlers such as `onerror` and `onclick`, and URL schemes such as `javascript:` and `data:text/html`, are carried through by default. This matches Mozilla's Readability.js, which also leaves sanitization to the consumer. **If you render extracted content in a webview or browser DOM, sanitize it first**, for example with [`ammonia`](https://crates.io/crates/ammonia).
- **`sanitize_content` is a harm reducer, not a sanitizer.** Enabling it drops script-bearing and content-loading elements whole (`script`, `style`, `iframe`, `object`, `embed`, `form`, `noscript`, `template`), event-handler attributes, the highest-risk URL schemes, and comments. Everything else keeps every attribute it carries, and none of it applies to `markdown_content`. Do not treat it as a security boundary.
- **`markdown_content` is untrusted too.** Link and image destinations come from the page. Markdown rendered to HTML by a downstream renderer needs the same care as the HTML output.
- **This crate does no I/O.** It takes a `&str` and returns a struct. It opens no files, resolves no DNS, and makes no network requests, so there is no SSRF or path-traversal surface. A base URL, if supplied, is parsed and validated but never fetched.

## Known limitations

These are documented rather than fixed, so a report about them is a duplicate rather than a finding. If you can show impact beyond what is described here, that is worth reporting:

- **Nothing bounds the input itself.** `max_elems_to_parse` now works, but it is counted after the document has been parsed, so it caps extraction work rather than parsing work, and it is off by default. Candidate lookup also scales worse than linearly with document size. **Bound the input yourself** before parsing, and apply your own timeout, if you accept arbitrary pages.
- **Deeply nested markup can exhaust the stack.** Several tree walkers recurse once per nesting level with no depth cap. A sufficiently nested document aborts the process, which a caller cannot catch. Cap nesting depth or document size upstream if that matters to you.
- **Element removal is partly regex-based.** Some cleanup passes match elements with regular expressions rather than on the parsed tree. Regular expressions cannot match balanced tags, so removal can be incomplete or can remove more than intended on adversarial or unusual markup. Treat the output as untrusted regardless.
- **Markdown link and image destinations are not escaped.** `alt`, `href` and `title` values are written into `[]()` and `![]()` unchanged, so a crafted value can close the destination early and take control of the resulting link. `sanitize_content` does not apply to the Markdown path at all. Treat `markdown_content` from untrusted pages as untrusted markup.

## Areas of particular interest

If you are looking for where the sharp edges are:

- **Panics reachable from `Readability::parse`.** Any input that causes a panic is in scope: string slicing at a non-character boundary, arithmetic underflow, unchecked indexing, or an `unwrap` on a value an attacker influences. This is the highest-value category.
- **The serializer.** `element_to_html` in `src/content_extractor.rs` rebuilds HTML by hand. Anything that produces output which a browser parses differently than the crate intended, including attribute-escaping gaps and duplicate or malformed attributes, is worth reporting.
- **The sanitization predicates.** `is_dangerous_url` and `is_event_handler_attr` in the same file. A scheme or attribute that reaches the output with `sanitize_content` enabled, when the documented policy says it should not, is a finding.
- **The Markdown pipeline.** `src/markdown/` and `src/elements/`. Injection into link or image destinations, and resource exhaustion during conversion.
- **Resource exhaustion generally.** CPU or memory growth disproportionate to input size, beyond what the known limitations above already describe.

## What is not a vulnerability

- Unsanitized HTML in `Article::content`. That is the documented contract, stated above and in the crate docs. The fix is to sanitize downstream.
- `sanitize_content` failing to remove something it does not claim to remove. Its scope is listed above, and an allowed element keeping a benign attribute is not a finding.
- Poor extraction quality, a wrong byline, or a missed paragraph. Those are ordinary bugs. Please do open a public issue for them.
- Anything requiring you to feed the library your own hostile document on your own machine. The threat model is untrusted input reaching a service, not self-inflicted input.
