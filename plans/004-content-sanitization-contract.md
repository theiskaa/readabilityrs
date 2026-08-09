# Plan 004: Document the sanitization contract and add an opt-in sanitizer for extracted HTML

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- src/content_extractor.rs src/options.rs src/lib.rs README.md`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: none (002 recommended first for the regression net)
- **Category**: security
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

`article.content` — the library's primary output — is built by
`element_to_html` in `src/content_extractor.rs`, which serializes **every**
attribute of every kept element verbatim (values are entity-escaped for
parsing correctness only). Event-handler attributes (`onerror`, `onclick`,
`onload`, …) and dangerous URL schemes (`javascript:` in `href`,
`data:text/html` in `src`) from the ingested page survive into the output.
Mozilla's Readability.js has the same contract — *the consumer must
sanitize* — and documents it. This port documents nothing, so a consumer who
renders `article.content` into a webview or browser DOM inherits stored XSS
from arbitrary pages. The fix is two-part: (1) state the contract loudly in
the docs, and (2) offer an opt-in `sanitize_content` option that strips the
two highest-risk vectors during serialization for consumers who want a safer
default without pulling a full sanitizer crate.

## Current state

- `src/content_extractor.rs:901-951` — `element_to_html(element: ElementRef)
  -> String`, recursive serializer. Attribute loop:

```rust
// src/content_extractor.rs:919-921
    for (name, value) in elem_data.attrs.iter() {
        html.push_str(&format!(" {}=\"{}\"", name.local, escape(value)));
    }
```

  `escape` is `v_htmlescape::escape` (entity-escaping only — it does not and
  cannot address event handlers or URL schemes). Call sites of
  `element_to_html`: `src/content_extractor.rs:651` (best candidate) and
  `:692` (siblings); it recurses at `:934`.
- `src/options.rs` — `ReadabilityOptions` struct (line 60) + builder
  (line 240). Existing bool options follow this exact pattern (field with
  doc comment + builder method), e.g. `clean_styles: bool` at line 166 with
  builder method at line 325. Match it.
- `src/lib.rs:1-114` — crate-level docs with usage examples; `README.md` has
  "Error Handling" and "Configuration" sections.
- `src/post_processor.rs:224-244` (`clean_styles`) strips `style`/`align`/
  `bgcolor`/`valign` presentational attributes via regex — it is NOT a
  sanitizer and must not be advertised as one.
- Repo conventions: options are plumbed from `ReadabilityOptions` through
  `grab_article` (`src/content_extractor.rs`) via the `options` parameter
  already passed to `try_extract_with_flags` → `extract_article_content`.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Build   | `cargo build` | exit 0 |
| Tests   | `cargo test` | all pass |
| Docs    | `cargo doc --no-deps` | exit 0, no warnings about broken links |

## Scope

**In scope**:
- `src/options.rs` — new `sanitize_content: bool` (default `false`) + builder
  method
- `src/content_extractor.rs` — thread the flag into `element_to_html`;
  attribute filtering logic + unit tests
- `src/lib.rs` — crate-docs security note
- `README.md` — security note section

**Out of scope** (do NOT touch):
- Adding a sanitizer dependency (`ammonia` etc.) — deliberate decision:
  recommend it in docs for full sanitization, don't depend on it.
- `src/cleaner.rs` / `src/post_processor.rs` — their regex passes are not
  part of the sanitization story.
- Changing default behavior: with `sanitize_content(false)` (default), output
  must be byte-identical to today.

## Git workflow

- Branch: `feature/sanitize-content-option`
- Commit style: `feat(security): add opt-in sanitize_content option and
  document the sanitization contract`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Add the option

In `src/options.rs`, add to `ReadabilityOptions`:

```rust
    /// Whether to strip high-risk markup during serialization of the
    /// extracted content: event-handler attributes (`on*`) and `href`/`src`
    /// values with `javascript:`, `vbscript:`, or `data:` schemes
    /// (`data:image/*` is allowed).
    ///
    /// Default is `false`, matching Readability.js: the extracted HTML is
    /// NOT sanitized and must be treated as untrusted input by consumers.
    /// This option is a harm reducer, not a substitute for a real HTML
    /// sanitizer.
    pub sanitize_content: bool,
```

Add the builder method following the exact pattern of `clean_styles`
(`src/options.rs:325`), update `Default`/`build()` accordingly (read how the
other bools flow through `ReadabilityOptionsBuilder::build`).

**Verify**: `cargo build` → exit 0.

### Step 2: Thread the flag to the serializer

`element_to_html` currently takes only `ElementRef`. Change it to
`element_to_html(element: ElementRef, sanitize: bool)` and update the three
call/recursion sites (`src/content_extractor.rs:651`, `:692`, `:934`). The
callers (`extract_article_content` and its sibling loop) already have
`options: &ReadabilityOptions` in scope or one level up — pass
`options.sanitize_content` down; follow the call chain from
`try_extract_with_flags` (line ~69) and add a parameter where needed rather
than a global.

**Verify**: `cargo build` → exit 0; `cargo test` → all pass (flag is false by
default, so nothing changes).

### Step 3: Implement the filter

Inside the attribute loop, when `sanitize` is true:

1. Skip any attribute whose lowercased local name starts with `"on"` AND has
   length > 2 (`onclick`, `onerror`, …). Note: this also drops rare
   legitimate attributes starting with "on" — acceptable for an opt-in.
2. For attributes named `href`, `src`, or `xlink:href`: trim the value's
   leading whitespace/control chars, isolate the scheme with `split_once(':')`,
   bail out (treat as safe) if that raw segment contains `/`, `?`, or `#`
   (relative / protocol-relative URL), then **strip ALL embedded whitespace
   and control characters from the scheme segment before comparing it
   case-insensitively**. Skip the attribute if the normalized scheme is
   `javascript`, `vbscript`, or `data` — EXCEPT allow `data:image/` prefixes
   (lazy-image placeholders in the corpus rely on them; see
   `is_placeholder_src` in `src/elements/images.rs:80-85`).

   **Prefix-trimming alone is NOT sufficient and is a known bypass.** Browsers
   remove ASCII tab/newline characters while parsing a URL, so `java\tscript:`
   reaches the page as `javascript:` and executes. A filter that compares the
   raw scheme segment lets `java\tscript:`, `java\nscript:`, `java\rscript:`
   and `jav\0ascript:` straight through. Normalize, then compare.

   Normalization affects the skip/keep DECISION only — never rewrite the value
   that gets serialized; kept attributes must be byte-identical to today.
3. Everything else serializes exactly as before.

Put the predicate in two small private functions
(`is_event_handler_attr(name: &str) -> bool`,
`is_dangerous_url(value: &str) -> bool`) directly above `element_to_html`, so
they're unit-testable.

**Verify**: `cargo build` → exit 0.

### Step 4: Unit tests

In the existing `#[cfg(test)] mod tests` of `src/content_extractor.rs`
(starts line ~967; use `test_attribute_values_are_escaped` at `:972` as the
structural model — it builds a full HTML page with a substantial paragraph so
`grab_article` picks it up, then asserts on the output):

1. With `sanitize_content(true)`: an `<img>` carrying an event-handler
   attribute and an `<a>` with a `javascript:` href inside article content →
   output contains neither the handler attribute name nor the scheme.
2. With `sanitize_content(true)`: `data:image/png;base64,...` src survives;
   `https://` href survives; scheme check is case-insensitive and tolerates
   leading whitespace in the value.
3. With default options: the same input passes the handler attribute through
   unchanged (contract preserved).
4. Direct unit tests for `is_dangerous_url` covering: `javascript:` mixed
   case, value with leading tab/newline before the scheme, `data:text/html`,
   `data:image/png` (allowed), relative URL (allowed), protocol-relative
   `//host/path` (allowed).
5. Bypass battery — each of these MUST be blocked: `java\tscript:`,
   `java\nscript:`, `java\rscript:`, `jav\0ascript:`, `vb\tscript:`, and a
   form-feed-prefixed `javascript:`. Plus a multi-byte UTF-8 value (e.g.
   containing `İ` or an emoji) asserting the function returns rather than
   panicking.

**Verify**: `cargo test --lib content_extractor` → new tests pass;
`cargo test` → full suite passes.

### Step 5: Document the contract

- `src/lib.rs` crate docs: add a `## Security` section stating that
  `article.content` is derived from untrusted input and is not sanitized by
  default; renderers must sanitize (recommend the `ammonia` crate) or enable
  `sanitize_content(true)` as a partial mitigation.
- `README.md`: add the same note as a short "Security" section after
  "Error Handling", using the same wording.

**Verify**: `cargo doc --no-deps` → exit 0;
`grep -n "sanitize" README.md src/lib.rs` → hits in both.

## Test plan

Covered in Step 4 (6+ new tests). Also run the Mozilla suite (if plan 002 has
landed: `cargo test --test mozilla_test_suite`; otherwise with `-- --ignored
--nocapture`) and confirm the pass/fail set is unchanged — the default path
must be untouched.

## Done criteria

- [ ] `cargo test` exits 0 with new sanitization tests present
- [ ] Default-path output unchanged: Mozilla suite results identical to
      baseline
- [ ] `ReadabilityOptions::builder().sanitize_content(true)` compiles in a
      doctest or unit test
- [ ] README and lib.rs both contain the security note
- [ ] `git status` shows only in-scope files modified
- [ ] `plans/README.md` status row updated

## STOP conditions

- `element_to_html`'s signature or attribute loop no longer matches the
  excerpt (plans 005/008 also touch this function — if one landed first,
  reconcile: the filter logic is unchanged, only the threading differs;
  report if the merge isn't obvious).
- Enabling the option changes any KNOWN-passing Mozilla-suite case (the
  corpus shouldn't contain event handlers in expected article bodies — if it
  does, report which case).
- You find yourself wanting to add regex-based sanitization of the final
  HTML string — that approach is explicitly rejected (bypassable); the filter
  must live in the serializer.

## Maintenance notes

- The scheme/attribute lists are intentionally minimal. If consumers ask for
  more (e.g. `formaction`, `srcdoc`), grow the predicate functions — never
  bolt on post-hoc regex.
- Plan 009 (single-parse pipeline) will move serialization; the predicates
  must move with it.
- Reviewers: scrutinize the `data:image/` allowance — it's a usability
  tradeoff, documented in the option's doc comment.
