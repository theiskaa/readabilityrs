# Plan 016: Complete is_probably_readerable to match Mozilla's algorithm

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on. On
> any STOP condition, stop and report. When done, update this plan's row in
> `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- src/readerable.rs src/constants.rs`
> On drift, re-verify the excerpts below before proceeding.

## Status

- **Priority**: P2
- **Effort**: S-M
- **Risk**: LOW
- **Depends on**: plans/002-asserting-mozilla-test-suite.md (uses the corpus
  for validation)
- **Category**: direction (stated-but-undelivered feature)
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

`is_probably_readerable` is a publicly exported, README-advertised feature
("Pre-flight Check", batch pre-filtering for crawlers) — but the
implementation is an admitted stub: it selects `p, pre, article`, sums a
length-based score, and returns. Mozilla's actual `isProbablyReaderable`
additionally (a) skips hidden nodes, (b) skips nodes matching the
`unlikelyCandidates` regex unless they match `okMaybeItsACandidate`, and
(c) includes `div > p` parents in the node set. Without those checks, pages
whose paragraphs live in nav/footer/comment containers score as readerable,
and hidden boilerplate counts. Consumers using it to gate expensive `parse()`
calls (its stated purpose) get materially worse filtering than the algorithm
it claims to port.

## Current state

```rust
// src/readerable.rs:147-178 (abridged; TODO at :151)
pub fn is_probably_readerable(html: &str, options: Option<ReaderableOptions>) -> bool {
    let options = options.unwrap_or_default();
    let document = Html::parse_document(html);

    // TODO: Implement full isProbablyReaderable logic
    // For now, just do a basic check

    let p_selector = Selector::parse("p, pre, article").unwrap();
    let paragraphs: Vec<_> = document.select(&p_selector).collect();
    ...
    for p in paragraphs {
        let text = p.text().collect::<String>();
        let text_len = text.trim().len();
        if text_len < options.min_content_length { continue; }
        score += ((text_len - options.min_content_length) as f64).sqrt();
        if score > options.min_score { return true; }
    }
    false
}
```

- `ReaderableOptions` (same file, above) has `min_content_length` (default
  140) and `min_score` (default 20) — matching Mozilla's defaults. Mozilla's
  option `visibilityChecker` is not present; a boolean or a fixed built-in
  check is acceptable (see Step 2).
- The regexes already exist: `src/constants.rs:28` declares
  `pub unlikely_candidates: Regex` inside the `REGEXPS` struct (built at
  `:45`); grep `ok_maybe` in `src/constants.rs` to find the companion
  (Mozilla's `okMaybeItsACandidate` — verify its field name; if absent,
  port the pattern from Mozilla:
  `and|article|body|column|content|main|mathjax|shadow`).
- A visibility helper already exists and is used by the extractor:
  `dom_utils::is_probably_visible` (see its use at
  `src/content_extractor.rs:903` and `:107`). Reuse it — do NOT write a
  second visibility check.
- Existing tests: `src/readerable.rs:181+` (2 unit tests).
- Mozilla reference (for the executor's understanding, logic restated here so
  no external fetch is needed): nodeset = `document.querySelectorAll("p, pre,
  article")` plus `div`s that have a direct `p` child (Mozilla uses
  `div > p` via a `br`-adjacent legacy check — implement the simple form:
  include any `div` with a direct `<p>` child, counting the div's OWN text
  the same way); for each node: skip if `!visibilityChecker(node)`; build
  `matchString = class + " " + id`; skip if
  `unlikelyCandidates.test(matchString) && !okMaybeItsACandidate.test(matchString)`;
  skip `p` nodes that are direct children of `li` (`li p` exclusion);
  textLength = trimmed textContent length; skip if <
  minContentLength; score += sqrt(textLength - minContentLength); return
  true when score > minScore.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Unit    | `cargo test --lib readerable` | pass |
| Corpus check | `cargo test --test mozilla_test_suite` | pass (see Step 4) |
| Full    | `cargo test` | all pass |

## Scope

**In scope**: `src/readerable.rs`; `src/constants.rs` ONLY if
`ok_maybe`-style regex is missing and must be added; new corpus-driven test
in `tests/` or the readerable test module.

**Out of scope**: `ReadabilityOptions`/`parse()` integration; changing
`ReaderableOptions` defaults; `dom_utils` internals.

## Git workflow

- Branch: `feature/complete-readerable`
- Commit style: `feat(readerable): implement full isProbablyReaderable
  checks (visibility, unlikely candidates, div>p)`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Extend the node set

Add `div`-with-direct-`p`-child inclusion. Selector approach: select
`p, pre, article, div` and for `div`s check
`el.children().filter_map(ElementRef::wrap).any(|c| c.value().name() == "p")`
— include the div only then. Ensure no double counting: when a div is
included because of its `p` children and those `p`s are also in the node
set, Mozilla counts both; mirror Mozilla (count both) and note it.

**Verify**: `cargo build` → exit 0; existing 2 unit tests still pass.

### Step 2: Add the visibility and unlikely-candidate filters

Inside the loop, before length checks:
1. `if !dom_utils::is_probably_visible(node) { continue; }` (confirm the
   function's visibility — it's `pub(crate)` or `pub` in `src/dom_utils.rs`;
   adjust with `pub(crate)` if needed).
2. Build `match_string = format!("{} {}", class_attr, id_attr)`; skip when
   `REGEXPS.unlikely_candidates.is_match(&match_string)` and NOT
   `okMaybe` matches (add the `ok_maybe_its_a_candidate` regex to
   `src/constants.rs`'s REGEXPS struct if not already there, pattern
   `(?i)and|article|body|column|content|main|mathjax|shadow`).
3. Skip `p` elements whose parent is `li`.

**Verify**: `cargo build` → exit 0.

### Step 3: Unit tests for each new check

Add to `src/readerable.rs` tests (model on the existing two):
1. Long paragraphs inside `<div class="comment">` wrappers → `false`
   (unlikely-candidate exclusion).
2. Same content but `class="comment main-content"` → `true` (okMaybe
   override).
3. Long paragraphs with `style="display:none"` → `false` (visibility).
4. Content in `<div><p>…</p></div>` without `article` tags → `true`
   (div>p inclusion).
5. Paragraphs only inside `<li>` items → excluded (li>p rule) — construct so
   the result flips to `false`.

**Verify**: `cargo test --lib readerable` → all pass.

### Step 4: Corpus sanity check

Write a THROWAWAY check (or a permanent `#[test]` if cheap): for every
Mozilla test-page with `"readerable": true` in `expected-metadata.json`,
`is_probably_readerable(source_html, None)` should be true; count mismatches
in both directions. Baseline BEFORE your change (temporarily on main) is not
required, but AFTER the change the false-negative count on
`readerable: true` pages must be low single digits. If making it a permanent
test, use divergence-list style (plan 002's pattern) for the known misses.

**Verify**: report the numbers (before if measured, after always).

## Test plan

Steps 3-4. The permanent corpus test (if adopted) goes in
`tests/mozilla_test_suite.rs` following plan 002's KNOWN_*_DIVERGENCES
pattern.

## Done criteria

- [ ] The `TODO: Implement full isProbablyReaderable logic` comment is gone
- [ ] All three checks implemented, each with a unit test that fails if the
      check is removed
- [ ] `cargo test` exits 0
- [ ] Corpus numbers reported in the commit/PR body
- [ ] `plans/README.md` status row updated

## STOP conditions

- `REGEXPS.unlikely_candidates`'s pattern differs materially from Mozilla's
  (it was ported for the extractor; if it includes extractor-specific
  additions, using it here may over-filter — compare and report before
  proceeding).
- Corpus false negatives exceed ~10 pages after the change — a filter is too
  aggressive; report per-check attribution (disable checks one at a time).
- `is_probably_visible` is unsuitable (e.g. it requires ElementRef context
  the readerable loop doesn't have) — report rather than duplicating logic.

## Maintenance notes

- This function must stay CHEAP — it's the pre-filter. No full scoring, no
  serialization. Reviewers should reject anything O(n²) here.
- If `ReaderableOptions` grows a `visibility_checker` callback later to
  mirror Mozilla, the built-in check becomes its default.
