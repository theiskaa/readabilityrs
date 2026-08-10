# Plan 006: Consolidate element removal onto the DOM path (delete the regex removers)

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- src/cleaner.rs src/post_processor.rs src/readability.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: L
- **Risk**: MED
- **Depends on**: plans/002-asserting-mozilla-test-suite.md (MANDATORY —
  this refactor changes cleaning behavior on edge cases and needs the net)
- **Category**: bug + tech-debt
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

The same "remove nav/share/unwanted blocks" concern is implemented THREE
times with different semantics:

1. **DOM-based** (correct): `clean_conditionally_tag` / `should_remove_dom_node`
   in `src/cleaner.rs:608-700` — two-phase collect-then-detach on the parsed
   tree.
2. **Regex-on-string**: `remove_unwanted_elements`, `remove_share_elements`,
   `remove_navigation_elements` in `src/post_processor.rs:268-350`, and
   `remove_nav_like_sections` in `src/cleaner.rs:49-79`.
3. **Regex fallback**: `remove_conditionally_regex` / `remove_blocks_for_tag`
   / `compute_fragment_stats` in `src/cleaner.rs:119-360`, which re-parses
   every candidate block as a full HTML document.

The regex patterns are of the form `<tag[^>]*?>.*?</tag>`, which is
structurally wrong on real HTML: `[^>]*?>` terminates at a `>` inside a
quoted attribute value, and `.*?</tag>` stops at the FIRST closing tag, so
any nested same-tag element (`<div class="share"><div>…</div></div>`) is
removed only up to the inner close — leaving an unbalanced stray `</div>` in
the output and part of the unwanted block behind. The rule sets have also
already drifted between paths (e.g. `sidebar` is in `cleaner.rs`'s keyword
list but not `post_processor.rs`'s). This plan makes the DOM path the only
removal mechanism, deleting the corrupt-output class of bugs and two thirds
of the maintenance surface. It also deletes the no-op
`fix_relative_urls_in_html` (dead code that silently advertises URL fixing).

## Current state

Pipeline order (read `src/readability.rs:160-256`, `parse()`):
`grab_article` → `cleaner::clean_article_content_light` (regex nav removal +
no-op URL fix) → `post_processor::prep_article` (regex removers) →
`cleaner::clean_article_content` (light again + `remove_conditionally`, which
tries DOM and falls back to regex).

Key excerpts (verified at `c7622fd`):

```rust
// src/cleaner.rs:16-26
pub fn clean_article_content_light(html: &str, base_url: Option<&str>) -> Result<String> {
    let mut result = html.to_string();
    if let Some(base) = base_url {
        result = fix_relative_urls_in_html(&result, base);
    }
    result = remove_nav_like_sections(&result);
    Ok(result)
}

// src/cleaner.rs:42-46 — dead code, silently does nothing
fn fix_relative_urls_in_html(html: &str, _base_url: &str) -> String {
    // For now, just return as-is
    // TODO: Implement proper URL fixing without re-parsing the entire tree
    html.to_string()
}

// src/cleaner.rs:81-83
fn remove_conditionally(html: &str) -> String {
    remove_conditionally_dom(html).unwrap_or_else(|| remove_conditionally_regex(html))
}
```

```rust
// src/post_processor.rs:185-214 (prep_article) — the regex removers run here
    html = unwrap_nav_wrappers(&html);
    if clean_styles_opt { html = clean_styles(&html); }
    html = remove_unwanted_elements(&html);      // regex, 12 tag patterns
    html = remove_share_elements(&html);         // regex, tags×keywords matrix
    html = remove_navigation_elements(&html);    // regex, tags×keywords matrix
    if clean_whitespace_opt { html = remove_empty_paragraphs(&html); html = normalize_whitespace(&html); }
```

The canonical DOM idiom to extend (`src/cleaner.rs:608-641`):

```rust
fn clean_conditionally_tag(doc: &mut Html, root_id: NodeId, tag: &str, marks: &HashSet<NodeId>) {
    // Phase A: collect NodeIds to detach under immutable tree borrow.
    let to_detach: Vec<NodeId> = { /* root_el.select(selector).filter(should_remove_dom_node).map(|el| el.id()).collect() */ };
    // Phase B: detach under mutable borrow.
    for id in to_detach { if let Some(mut n) = doc.tree.get_mut(id) { n.detach(); } }
}
```

Keyword/rule inventory to preserve (union of all three paths — collect these
before deleting anything):
- Unconditional tag removal (`post_processor.rs:268-296`): form, fieldset,
  footer, aside, object, embed, iframe, input, textarea, select, button, link.
- Share/social (`post_processor.rs:302-320`): tags {div, span, aside,
  section} × keywords {share, social, sharedaddy} on class OR id.
- Navigation (`post_processor.rs:325-350`): `<nav>` always; tags {div,
  section, ul, ol} × keywords {nav, navbar, menu, breadcrumbs} on class OR id.
- Nav-like (`cleaner.rs:49-79`): same as above PLUS keyword `sidebar`, and a
  comment explaining `widget` is deliberately excluded (page builders use it
  for real content) — PRESERVE that exclusion and its comment.
- Conditional cleaning (`cleaner.rs:81-105`): tags form, fieldset, table, ul,
  ol, div, section via `should_remove_dom_node` heuristics + data-table marks.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Tests   | `cargo test` | all pass |
| Mozilla suite | `cargo test --test mozilla_test_suite` | pass; divergence lists unchanged or IMPROVED |
| Markdown suite | `cargo test --test markdown_tests` | all 117 pass |

## Scope

**In scope**:
- `src/cleaner.rs` — extend the DOM pass; delete `remove_nav_like_sections`,
  `remove_conditionally_regex`, `remove_blocks_for_tag`,
  `compute_fragment_stats`, `should_remove_block`, `fix_relative_urls_in_html`
- `src/post_processor.rs` — delete `remove_unwanted_elements`,
  `remove_share_elements`, `remove_navigation_elements` and their calls in
  `prep_article`
- `src/readability.rs` — only if the call sequence needs a parameter change

**Out of scope** (do NOT touch):
- `clean_styles`, `normalize_whitespace`, `remove_empty_paragraphs`,
  `unwrap_nav_wrappers`, `remove_title_from_content` in `post_processor.rs` —
  they operate on text/attributes, not element removal, and stay string-based
  for now (plan 009 revisits).
- `should_remove_dom_node` heuristics — do not "improve" thresholds while
  consolidating; behavior changes must come only from WHERE removal happens,
  not from new rules.
- `tests/test-pages/**`.

## Git workflow

- Branch: `refactor/dom-only-element-removal`
- Commits: one per step below; style `refactor(cleaner): …` /
  `refactor(post_processor): …`.
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Baseline

Record Mozilla-suite and markdown-suite results (exact pass/fail names).

**Verify**: baseline saved.

### Step 2: Build the unified DOM removal pass

In `src/cleaner.rs`, add a function `remove_unwanted_dom(doc: &mut Html,
root_id: NodeId)` that, using the same Phase A/Phase B shape as
`clean_conditionally_tag`:

1. Detaches all elements matching the unconditional tag list (form, fieldset,
   footer, aside, object, embed, iframe, input, textarea, select, button,
   link, nav).
2. Detaches elements of {div, span, aside, section, ul, ol} whose class or id
   (case-insensitive substring) matches share/social/sharedaddy or
   nav/navbar/menu/breadcrumbs/sidebar — replicating the keyword matrices
   above with ONE precompiled `Lazy<Regex>` per concern (e.g.
   `(?i)share|social|sharedaddy`), checked against the element's
   `class + " " + id` string (see `get_dom_class_id_string` usage at
   `src/cleaner.rs:644`). Preserve the `widget` exclusion comment verbatim.

**Verify**: `cargo build` → exit 0 (function exists, not yet wired).

### Step 3: Wire it in and delete the regex removers

1. In `remove_conditionally_dom` (`src/cleaner.rs:85-105`), call
   `remove_unwanted_dom(&mut doc, root_id)` before the
   `clean_conditionally_tag` loop.
2. Change `remove_conditionally` to call ONLY the DOM path; on `None`
   (parse produced no usable root — in practice `Html::parse_document` always
   yields a tree, so `None` means wrap failure), return the input unchanged
   instead of falling back to regex:
   `remove_conditionally_dom(html).unwrap_or_else(|| html.to_string())`.
3. Delete from `src/post_processor.rs`: `remove_unwanted_elements`,
   `remove_share_elements`, `remove_navigation_elements` + their three calls
   in `prep_article`.
4. Delete from `src/cleaner.rs`: `remove_nav_like_sections` (and its call in
   `clean_article_content_light`), `remove_conditionally_regex`,
   `remove_blocks_for_tag`, `compute_fragment_stats`, `should_remove_block`,
   `fix_relative_urls_in_html` (and its call; also remove the now-unused
   `base_url` parameter threading IF the compiler shows nothing else uses it
   in `clean_article_content_light` — check `src/readability.rs:175` and
   `:192` pass `self.base_url.as_deref()`; keep the parameter signature but
   mark `_base_url` if removal would ripple into the public API).

**Verify**: `cargo build` → exit 0;
`grep -n "replace_all" src/post_processor.rs` → only hits in `clean_styles`,
`normalize_whitespace`, and other out-of-scope functions;
`grep -n "remove_conditionally_regex\|remove_blocks_for_tag\|compute_fragment_stats" src/cleaner.rs`
→ no matches.

### Step 4: Reconcile test expectations

Run all suites. Unit tests in `cleaner.rs`/`post_processor.rs` that directly
tested deleted functions must be MIGRATED to equivalent tests through the
public path (`clean_article_content` / `prep_article`), not deleted — each
migrated test should assert the same removal outcome (e.g. "a div with class
share is gone from output"). Nested-element cases that the regex path handled
wrongly may now legitimately change output; for each Mozilla/markdown case
whose result changes, inspect whether the new output is more correct
(balanced tags, fully-removed block). Improvements are acceptable; report
them. Losses (previously-removed junk now kept, or article content now
dropped) are STOP material.

**Verify**: `cargo test` → pass; diff vs baseline reviewed and every change
explained in the commit message.

### Step 5: Add regression tests for the regex-era bugs

In `src/cleaner.rs` tests, add:
1. Nested same-tag removal: `<div class="share"><div>inner</div></div>` +
   surrounding article content → output contains neither `share` content nor
   a stray `</div>` (assert balanced by re-parsing with
   `Html::parse_fragment` and re-serializing — no orphan text).
2. Attribute containing `>`: `<iframe title="a>b" src="x"></iframe>` inside
   content → iframe fully removed, no `b" …` fragment leaks into text.
3. `sidebar` keyword removal works via the unified path (was previously only
   in one of the three implementations).

**Verify**: `cargo test --lib cleaner` → new tests pass.

## Test plan

Steps 4-5. Net-new: 3+ regression tests for regex-era corruption; migrated
equivalents for every deleted function's direct tests. Full gates: 154+ lib
tests, 117 markdown tests, Mozilla suite with explained diffs.

## Done criteria

- [ ] `cargo test` exits 0
- [ ] Zero regex-based ELEMENT-removal remains:
      `grep -n '</{tag}>\|\.\*?</' src/cleaner.rs src/post_processor.rs` → no
      element-block patterns (attribute-level regexes in `clean_styles` are
      fine)
- [ ] Mozilla suite: no case moves from pass → fail (moves from fail → pass
      are wins; update the divergence list and say so)
- [ ] `fix_relative_urls_in_html` no longer exists
- [ ] `git status` shows only in-scope files modified
- [ ] `plans/README.md` status row updated

## STOP conditions

- Any Mozilla-suite case regresses (pass → fail) and inspection shows real
  article content being removed by the unified pass.
- The `base_url` parameter removal ripples beyond
  `src/cleaner.rs`/`src/readability.rs`.
- You find genuine behavior the regex path provided that the DOM path cannot
  express (e.g. removal in malformed HTML that html5ever normalizes
  differently) — document the case and stop.
- More than ~5 markdown tests need expectation changes — that suggests the
  unified keyword matrix diverged from the union; re-check Step 2 against the
  inventory instead of editing expectations.

## Maintenance notes

- All removal rules now live in one file (`cleaner.rs`); future rule changes
  touch exactly one keyword set. Reviewers should reject new string-level
  element removal on sight.
- This unblocks plan 009 (single-parse pipeline): with `prep_article` no
  longer removing elements, the remaining string passes are
  whitespace/attribute-level and much easier to fold into the tree walk.
- Deferred deliberately: implementing real relative-URL resolution (the
  deleted no-op's promise). If wanted later, it belongs in the DOM pass using
  the `url` crate (already a dependency) — file it as its own plan.
