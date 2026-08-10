# Plan 009: Collapse the serialize→re-parse round-trips into one tree pass

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- src/readability.rs src/cleaner.rs src/post_processor.rs src/content_extractor.rs`
> This plan DEPENDS on plans 002 and 006 having landed. If they haven't,
> STOP — executing this without the asserting suite and the DOM-only cleaner
> is not sanctioned.

## Status

- **Priority**: P3
- **Effort**: L
- **Risk**: MED
- **Depends on**: plans/002-asserting-mozilla-test-suite.md,
  plans/006-dom-only-element-removal.md (both MANDATORY)
- **Category**: perf
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

Between `grab_article` and the final `Article`, the article HTML is
serialized to a `String` and re-parsed into a fresh `scraper::Html` tree
roughly 5-7 times per document: extraction serializes the winning subtree
(`element_to_html`), then `clean_article_content_light`,
`prep_article`'s string passes, `clean_article_content` /
`remove_conditionally_dom` (parse #2), `get_text_content` (parse #3),
`generate_excerpt_from_html` (parse #4), and — when markdown is enabled —
`standardize_all` + `html_to_markdown` (parses #5-6). HTML parsing dominates
cost in this pipeline; most of these parses re-read the exact same content
string. After plan 006, the intermediate string passes that remain are
whitespace/attribute-level, making it feasible to keep ONE parsed tree from
cleaning through text/excerpt extraction and serialize once.

This is the highest-effort perf plan; do it only after 002/006 are green.

## Current state

The pipeline, from `src/readability.rs:160-256` (`parse()`), post-plan-006
shape (verify against live code — the excerpt below is the `c7622fd`
original, BEFORE 006; the regex removers it calls may be gone):

```rust
// src/readability.rs:172-210 (c7622fd)
match grab_article(&preprocessed_doc, &self.options) {           // parse #0 (preprocessed doc) → String
    Ok(Some(content_html)) => {
        let cleaned_wrapper_html =
            cleaner::clean_article_content_light(&content_html, ...) // string pass
        let mut prepped_html = crate::post_processor::prep_article(...) // string passes
        // optional remove_title_from_content (string pass)
        let cleaned_html = cleaner::clean_article_content(&prepped_html, ...) // Html::parse_document inside remove_conditionally_dom
        let text_content = self.get_text_content(&cleaned_html);  // Html::parse_fragment
        let excerpt ... self.generate_excerpt_from_html(&cleaned_html) // Html::parse_fragment
        ...
        let standardized = crate::elements::standardize_all(&cleaned_html, ...) // regex passes over string
        Some(crate::markdown::html_to_markdown(&standardized, &md_opts))        // Html::parse inside
```

Supporting facts:
- `get_text_content` (`src/readability.rs:259-262`) and
  `generate_excerpt_from_html` (`:274+`) each `Html::parse_fragment` the same
  `cleaned_html` string.
- `remove_conditionally_dom` (`src/cleaner.rs:85-105`) parses, detaches, and
  re-serializes.
- `element_to_html` (`src/content_extractor.rs:901`) produces the initial
  string from the scored document; plans 004/005/008 touched it.
- `Article` (`src/article.rs`) exposes `content`, `raw_content`,
  `text_content`, `length`, `excerpt` — the public shape must not change.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Tests   | `cargo test` | all pass |
| Mozilla suite | `cargo test --test mozilla_test_suite` | divergence lists unchanged |
| Bench   | `cargo bench` before AND after | end-to-end parse time drops; paste numbers |

## Scope

**In scope**:
- `src/readability.rs` — restructure `parse()`'s post-extraction flow
- `src/cleaner.rs` — expose tree-level entry points (operate on `&mut Html` /
  subtree root instead of `&str`) alongside or replacing the string ones
- `src/post_processor.rs` — convert surviving string passes that are
  tree-expressible (empty-paragraph removal, title removal); whitespace
  normalization may stay string-level applied ONCE at final serialization

**Out of scope**:
- `grab_article`/scoring internals (plan 005 owns identity; don't refactor
  scoring here).
- The markdown converter's internal parse (`html_to_markdown`) — folding that
  in requires the standardization pipeline (`elements/*`, regex-on-string) to
  become tree-based too; explicitly deferred. One extra parse for the
  optional markdown path is acceptable.
- Public API/`Article` shape.

## Git workflow

- Branch: `refactor/single-parse-pipeline`
- Incremental commits per step; style `refactor(readability): …`.
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Baseline numbers

Record `cargo bench` results and Mozilla/markdown suite outcomes. This plan
is judged by "same results, fewer parses, measured speedup".

**Verify**: baseline saved (paste into eventual PR/commit body).

### Step 2: Make cleaning tree-native

In `src/cleaner.rs`, refactor so the conditional-cleaning entry takes and
mutates a parsed tree: `pub fn clean_tree(doc: &mut Html, root: NodeId)`
(the existing `remove_conditionally_dom` body minus its parse/serialize
shell; the shell function stays as a thin `&str` wrapper so existing tests
keep passing during transition).

**Verify**: `cargo build` + `cargo test` → pass (wrappers preserve behavior).

### Step 3: Parse once in `parse()`

Restructure `Readability::parse()` so that after `grab_article` returns the
content string, it is parsed ONCE (`Html::parse_fragment` — confirm fragment
vs document consistency with what `remove_conditionally_dom` did; it used
`parse_document` and selected body — keep that exact behavior to avoid
serialization diffs), and then:

1. Tree-level cleaning (Step 2's `clean_tree`).
2. Tree-level title removal (port `remove_title_from_content`'s logic to a
   tree walk — it currently string-matches title text in heading elements;
   read `src/post_processor.rs`'s implementation first).
3. `text_content` = walk the SAME tree (`root.text().collect()`), then
   whitespace-normalize.
4. `excerpt` = first suitable `<p>` from the SAME tree (port the loop from
   `generate_excerpt_from_html`, which filters `trimmed.len() < 25` and
   `looks_like_bracket_menu` — keep identical filters).
5. Serialize ONCE at the end for `Article::content`; apply the final
   string-level whitespace normalization to that single serialization if
   `clean_whitespace` is set.

Keep `raw_content` as the pre-clean string exactly as today
(`src/readability.rs:235` stores `content_html`).

**Verify**: after each sub-step, `cargo test` → pass; Mozilla suite →
identical divergence lists.

### Step 4: Retire dead string wrappers

Delete `&str` wrapper functions with no remaining callers; keep any that
tests or public API still use (check `src/lib.rs` exports — cleaner/
post_processor are NOT `pub mod`, so internal-only).

**Verify**: `cargo build` → no dead_code warnings introduced
(`cargo clippy --all-targets -- -D warnings` → exit 0).

### Step 5: Measure

Re-run `cargo bench`. Count parses per document by temporarily instrumenting
(e.g. a debug counter or just code inspection): the non-markdown path must
parse the article content exactly once after extraction.

**Verify**: bench delta recorded; parse count = 1 confirmed by inspection.

## Test plan

No new unit tests required beyond what Steps port; the gates are the
existing suites (which after 002 include content-floor assertions). Add ONE
new test: an end-to-end parse asserting `content`, `text_content`, `length`,
and `excerpt` are mutually consistent (text_content equals the text of
content; length equals text_content.len()) — this pins the invariant the
refactor must preserve. Put it in `src/readability.rs`'s test module,
modeled on existing tests there.

## Done criteria

- [ ] `cargo test` exits 0; Mozilla + markdown suites unchanged vs Step 1
- [ ] Post-extraction, non-markdown path parses article HTML exactly once
      (code inspection, stated in report)
- [ ] `cargo bench` shows measured improvement (any positive delta; paste
      numbers)
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0
- [ ] `plans/README.md` status row updated

## STOP conditions

- Plans 002/006 not landed (drift check).
- Serialization diffs appear that the whitespace-normalization step can't
  explain (scraper's serializer quirks — see the comment referenced at
  `src/post_processor.rs:77` about relying on them). Report the diff class
  before proceeding.
- The tree-level title-removal port changes >2 Mozilla cases — its string
  semantics were subtler than they look; report.
- Effort exceeds ~2 days of work — re-scope with the operator; partial
  landing (Steps 2-3 without 4) is acceptable if tests are green.

## Maintenance notes

- After this, "parse the string again" should be treated as a code smell in
  review; new pipeline stages should take the tree.
- Deferred: making `elements::standardize_all` + markdown conversion
  tree-native (would remove the last extra parse on the markdown path).
- Deferred: streaming/incremental parsing — out of scope entirely, no demand
  signal.
