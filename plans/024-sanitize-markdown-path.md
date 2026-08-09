# Plan 024: Neutralize injection in the Markdown output path

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan in
> `plans/README.md` and `plans/ROADMAP.md` — unless a reviewer dispatched you
> and told you they maintain the index.
>
> **Drift check (run first)**: `git diff --stat d840b82..HEAD -- src/markdown src/content_extractor.rs src/readability.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: none (independent of 004; reuses its `is_dangerous_url` helper)
- **Category**: security
- **Planned at**: commit `d840b82`, 2026-08-09

## Why this matters

The library's whole job is to strip dangerous content out of untrusted HTML.
That guarantee has a hole on the Markdown output path: attribute values flow
into Markdown link/image syntax **unescaped**, so an attacker who controls a
page's `alt`, `href`, `title`, or `src` can break out of `![]()` / `[]()` and
inject an arbitrary link destination into `markdown_content`.

Concrete break-out: an `<img>` whose `alt` is `](javascript:evil()) [x` renders
as `![](javascript:evil()) [x](realsrc)` — the attacker's text became a real
link destination. The opt-in `sanitize_content` sanitizer added in plan 004
applies **only to the HTML path** (`element_to_html`); it never touches Markdown
conversion. Any consumer that renders `markdown_content` back to HTML (the exact
use case for an "LLM-ready markdown" / reader pipeline) inherits the injected
link. This closes that hole.

## Current state

Files and roles:
- `src/markdown/rules/images.rs` — `convert_image(alt, src, title)` builds
  `![alt](src)` / `![alt](src "title")` with raw interpolation.
- `src/markdown/rules/links.rs` — `convert_link(inner, href, title, opts, state)`
  builds `[text](href)` / `[text](href "title")` with raw interpolation.
- `src/markdown/rules/text.rs` — `escape_markdown` deliberately does **not**
  escape `[`/`]` (see its doc comment); text nodes are the only place escaping
  currently happens.
- `src/markdown/converter.rs` — dispatches elements; `"img"` arm (line ~151)
  calls `convert_image`, `"a"` arm (line ~132) calls `convert_link`. Also has a
  `convert_figure` path that calls `rules::images::convert_figure`.
- `src/markdown/options.rs` — `MarkdownOptions` struct + `Default`.
- `src/content_extractor.rs` — `is_dangerous_url(value: &str) -> bool` (line
  ~945) already implements the correct scheme check (`javascript:`, `vbscript:`,
  non-image `data:`, tab/newline-bypass-resistant). Currently a **private** fn.
- `src/readability.rs` (lines ~216–228) — builds `md_opts` from
  `self.options.markdown_options` and calls `html_to_markdown`. This is where
  the pipeline can pass its `sanitize_content` flag into Markdown conversion.

Current vulnerable code:

`src/markdown/rules/images.rs:3-12`
```rust
pub fn convert_image(alt: &str, src: &str, title: &str) -> String {
    if src.is_empty() {
        return String::new();
    }
    if title.is_empty() {
        format!("![{}]({})", alt, src)
    } else {
        format!("![{}]({} \"{}\")", alt, src, title.replace('"', "\\\""))
    }
}
```

`src/markdown/rules/links.rs:20-37`
```rust
    let text = if trimmed.is_empty() { href } else { trimmed };
    let title_part = if title.is_empty() {
        String::new()
    } else {
        format!(" \"{}\"", title.replace('"', "\\\""))
    };
    match opts.link_style {
        LinkStyle::Inline => format!("[{}]({}{})", text, href, title_part),
        LinkStyle::Reference => { /* pushes href raw into state.link_references */ }
    }
```

The scheme check to reuse, `src/content_extractor.rs:945`:
```rust
fn is_dangerous_url(value: &str) -> bool { /* javascript/vbscript/non-image data */ }
```

Repo conventions to follow (inlined — the repo's `CLAUDE.md` is untracked and
invisible to your worktree):
- Commits: single-line conventional commit, `type(scope): what changed`. **No
  body. No `Co-Authored-By` trailer.** Scopes seen in history: `markdown`,
  `elements`, `extractor`, `security`.
- **No `unwrap()`/`expect()` in library code** (fine in `#[cfg(test)]`). No
  `unsafe`. No decorative/redundant comments — inline `//` only for non-obvious
  rationale. `///` docs on public items; prefer `&str` over `String` in params.
- Match the existing test style: `#[cfg(test)] mod tests` at the bottom of each
  rules file, plus integration tests in `tests/markdown_tests.rs`.

## Commands you will need

| Purpose   | Command                                              | Expected on success |
|-----------|------------------------------------------------------|---------------------|
| Build     | `cargo build`                                        | exit 0              |
| Unit tests| `cargo test --lib`                                   | all pass            |
| MD tests  | `cargo test --test markdown_tests`                   | all pass            |
| Mozilla   | `cargo test --test mozilla_test_suite`               | all pass            |
| Format    | `cargo fmt --check`                                  | exit 0, no diff     |
| Lint      | `cargo clippy --all-targets -- -D warnings`          | exit 0              |

## Scope

**In scope** (the only files you should modify):
- `src/markdown/rules/text.rs` — add escaping helpers + their tests.
- `src/markdown/rules/images.rs` — apply escaping; new signature; tests.
- `src/markdown/rules/links.rs` — apply escaping; tests.
- `src/markdown/converter.rs` — update the two/three call sites for new signatures.
- `src/markdown/options.rs` — add one field + default.
- `src/content_extractor.rs` — change `is_dangerous_url` visibility to `pub(crate)` only.
- `src/readability.rs` — pass `sanitize_content` into `md_opts` (one small block).
- `tests/markdown_tests.rs` — add integration tests for the break-out payloads.

**Out of scope** (do NOT touch):
- The HTML path / `element_to_html` sanitization logic — plan 004 owns it; do
  not change its behavior.
- `escape_markdown`'s existing rule set for text nodes **except** the one
  addition in Step 3 (bracket escaping inside link text). Do not broaden it
  otherwise; changing it risks the 117 existing markdown expectations.
- Any change to `MarkdownOptions` fields other than the one new flag.
- Reference-style link IDs / footnote logic beyond escaping the destination.

## Git workflow

- You are on branch `advisor/remaining-plans` (already created). Commit here.
- One commit for this plan; message e.g. `fix(markdown): escape link and image destinations to prevent injection`.
- Do NOT push or open a PR.

## Steps

### Step 1: Add escaping helpers to `text.rs`

In `src/markdown/rules/text.rs`, add three `pub(crate)` helpers (place them
near `escape_markdown`). Requirements:

- `escape_link_text(text: &str) -> String` — escape the characters that let text
  break out of the `[...]` label: backslash `\`, `[`, and `]`. (Escape `\`
  first, then brackets, using the same push-`\`-then-char shape as
  `escape_markdown`.) This is used for image `alt` and for link display text
  derived from `href`.
- `escape_url_destination(url: &str) -> String` — make a URL safe to sit inside
  `(...)`:
  - First strip ASCII control characters and newlines (`\n`, `\r`, `\t`) — a
    destination may not contain them.
  - If the stripped result contains a space, `(`, `)`, `<`, or `>`: return it
    wrapped in angle brackets `<...>` with any remaining `<`/`>` percent-encoded
    (`<`→`%3C`, `>`→`%3E`). CommonMark permits and unambiguously parses the
    `<...>` destination form.
  - Otherwise return the stripped string unchanged (so ordinary URLs are
    byte-identical to today's output — this is what keeps the 117 existing
    tests green).
- `escape_md_title(title: &str) -> String` — escape `\` then `"`, and strip
  newlines. (Replaces the ad-hoc `title.replace('"', "\\\"")` currently inlined
  in both rules files.)

Add unit tests in the file's `mod tests` for each: benign input unchanged,
bracket in alt escaped, URL with `)`/space wrapped in `<>`, control chars
stripped, title quote/backslash escaped.

**Verify**: `cargo test --lib markdown::rules::text` → new tests pass.

### Step 2: Add the `sanitize_urls` flag to `MarkdownOptions`

In `src/markdown/options.rs`:
- Add field `pub sanitize_urls: bool` with a `///` doc: "When true, drop
  `javascript:`/`vbscript:`/non-image `data:` destinations from links and
  images (mirrors `ReadabilityOptions::sanitize_content`)."
- Set it to `false` in `Default` (standalone Markdown use stays a pure
  converter, matching the "not a full sanitizer" posture).

**Verify**: `cargo build` → exit 0.

### Step 3: Escape inside link text nodes

In `src/markdown/converter.rs`, the text-node conversion path escapes text with
`escape_markdown`. When `state.in_link` is `true`, also escape `[`/`]` so nested
text cannot break out of the link label. Do this by escaping brackets in
addition to `escape_markdown` **only** in the in-link branch (do not change the
default text path). Find the text-node arm that calls
`rules::text::escape_markdown(...)`; wrap or post-process its result with
bracket escaping guarded by `state.in_link`.

**Verify**: `cargo test --test markdown_tests` → still all pass (no benign
regression).

### Step 4: Apply escaping + scheme filter in `convert_image`

Change `convert_image` to `convert_image(alt: &str, src: &str, title: &str, opts: &MarkdownOptions) -> String`:
- If `opts.sanitize_urls && crate::content_extractor::is_dangerous_url(src)` →
  return `String::new()` (drop the whole image; a data-less image is useless).
- Escape `alt` via `escape_link_text`, `src` via `escape_url_destination`,
  `title` via `escape_md_title`.
- Build the same `![alt](src)` / `![alt](src "title")` shape from the escaped parts.
- Update `convert_figure` the same way (escape `alt`/caption via
  `escape_link_text`, `src` via `escape_url_destination`; apply the same
  `sanitize_urls` drop). It will also need `opts` — thread it through.

Update the call sites in `converter.rs` (the `"img"` arm and the figure path) to
pass `opts`.

**Verify**: `cargo test --lib markdown::rules::images` and
`cargo test --test markdown_tests` → all pass.

### Step 5: Apply escaping + scheme filter in `convert_link`

In `convert_link`:
- If `opts.sanitize_urls && crate::content_extractor::is_dangerous_url(href)` →
  return `trimmed.to_string()` (render the text only, no link).
- Escape the display `text` with `escape_link_text` **only when it is the
  `href` fallback** (`trimmed.is_empty()` branch); when `text` is `trimmed`
  (already-converted child markdown) leave it — Step 3 already guards its
  brackets.
- Escape `href` via `escape_url_destination` in both the inline and
  reference-style branches (the value pushed into `state.link_references`).
- Escape `title` via `escape_md_title`.

**Verify**: `cargo test --lib markdown::rules::links` → all pass.

### Step 6: Wire `sanitize_content` into the pipeline

In `src/readability.rs` (the `output_markdown` block, ~217-228), after building
`md_opts`, set `md_opts.sanitize_urls = self.options.sanitize_content;` before
calling `html_to_markdown`. `md_opts` is already a mutable owned value
(`.cloned().unwrap_or_default()`); make its binding `mut`.

**Verify**: `cargo build` → exit 0.

### Step 7: Make `is_dangerous_url` reusable

In `src/content_extractor.rs`, change `fn is_dangerous_url` to
`pub(crate) fn is_dangerous_url`. Change nothing else about it. Confirm it is
importable as `crate::content_extractor::is_dangerous_url` from the markdown
rules (the module is `mod content_extractor;` in `lib.rs`, so `pub(crate)` is
sufficient — no need to re-export).

**Verify**: `cargo build` → exit 0.

## Test plan

- Unit tests (Steps 1, 4, 5) in `text.rs`, `images.rs`, `links.rs` `mod tests`,
  modeled on the existing tests already in those files.
- Integration tests in `tests/markdown_tests.rs` (model after existing tests
  there — find one that constructs HTML and asserts on `html_to_markdown`
  output). Add cases:
  1. **alt break-out**: `<img src="a.jpg" alt="](javascript:evil()) [x">` →
     assert the output does NOT contain `](javascript:` as a live destination
     (brackets in alt are escaped as `\[`/`\]`).
  2. **href with `)`**: `<a href="http://e.com/a)b">t</a>` → assert destination
     is wrapped `<http://e.com/a)b>` so the `)` cannot close it.
  3. **title quote**: `<a href="u" title='a"b'>t</a>` → assert `\"` in output.
  4. **sanitize on**: build via `Readability` with `.sanitize_content(true)`
     and `.output_markdown(true)`, feed an article containing
     `<a href="javascript:evil()">x</a>`, assert `markdown_content` contains
     neither `javascript:` nor a `](` link for it.
  5. **sanitize off (default) leaves benign links intact**: a normal
     `<a href="https://example.com">link</a>` still becomes
     `[link](https://example.com)` byte-for-byte.
- Verification: `cargo test` → all pass, including the new tests; the 117
  existing markdown tests unchanged.

## Done criteria

Machine-checkable. ALL must hold:

- [ ] `cargo build` exits 0
- [ ] `cargo test` exits 0; new tests exist and pass; the 117 existing
      `markdown_tests` still pass unchanged
- [ ] `cargo fmt --check` exits 0
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0
- [ ] `grep -n "format!(\"!\[{}\]({})\", alt, src)" src/markdown/rules/images.rs`
      returns nothing (raw interpolation replaced)
- [ ] No files outside the in-scope list are modified (`git status`)
- [ ] `plans/README.md` and `plans/ROADMAP.md` status rows updated

## STOP conditions

Stop and report back (do not improvise) if:

- The code at the locations in "Current state" doesn't match the excerpts
  (the codebase drifted since `d840b82`).
- Any existing `markdown_tests` expectation changes for a benign input — that
  means an escaping helper is too aggressive; do not "fix" the test, report it.
- Escaping the link display text in Step 5 requires touching the reference-link
  or footnote machinery beyond the destination value.
- Making `is_dangerous_url` `pub(crate)` surfaces a name/visibility conflict.
- A verification command fails twice after a reasonable fix attempt.

## Maintenance notes

- If a future change lets `markdown_content` be produced with a real base-URL
  resolution step, the destination escaping must run **after** URL resolution,
  not before.
- Reviewer should scrutinize: that benign URLs/alts are byte-identical to the
  old output (regression surface is the 117 existing tests), and that the
  `sanitize_urls` scheme drop and the always-on structural escaping are not
  conflated — structural escaping (`[`/`]`/`(`/`)`/title) is unconditional;
  only scheme-dropping is gated on the flag.
- Deferred out of this plan: percent-encoding the entire destination (we only
  wrap in `<>` when needed). Revisit if a consumer needs strict RFC-3986 output.
