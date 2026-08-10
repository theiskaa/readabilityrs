# Plan 021: Fix `unwrap_nav_wrappers` silently deleting entire article bodies

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on. On
> any STOP condition, stop and report. When done, update this plan's row in
> `plans/README.md` — unless a reviewer dispatched you and said they maintain
> the index.
>
> **Drift check (run first)**: `git diff --stat 4430e24..HEAD -- src/post_processor.rs`

## Status

- **Priority**: P0 — highest priority open item
- **Effort**: S
- **Risk**: MED (changes what gets removed on the default path)
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `4430e24`, 2026-08-04 (found by post-merge bug hunt, reproduced)

## Why this matters

`unwrap_nav_wrappers` runs unconditionally on the default `parse()` path and
**silently deletes the entire article body** for extremely common markup. It
returns `Some(article)` with `content == ""` and `length == 0` — not `None` —
so a caller cannot even detect the failure.

Reproduced at `4430e24` (6 real paragraphs wrapped in one div):

| Wrapper class | `article.length` |
|---|---|
| `entry-header` | **0** |
| `post-header` | **0** |
| `article-header` | **0** |
| `entry-top` (control) | 847 |

`entry-header`, `post-header`, and `article-header` are standard WordPress /
theme class names. Any article whose content sits inside a div whose class
merely *contains* the substring `header` is destroyed.

Three compounding defects in one regex:
1. Keywords match as **substrings** of any class value — and `header` is in
   the keyword list, so `entry-header` matches.
2. `.*?</div>` binds to the **first** `</div>`, not the matching one, so for
   nested divs it consumes the wrong span.
3. It **deletes** the matched span entirely, despite the function being named
   "unwrap" (unwrapping would keep the children).

## Current state

```rust
// src/post_processor.rs:12-21 (verify against live code — the regex literal is the target)
// pattern shape:
//   (?is)<div[^>]+class="[^"]*(?:navbar|nav|menu|sidebar|header)[^"]*"[^>]*>.*?</div>
// replace_all(..., "")
```

- Called unconditionally from `prep_article` (`src/post_processor.rs:189`),
  which is on the default path from `Readability::parse()`
  (`src/readability.rs:178`).
- A DOM-based remover already exists and is the correct home for this logic:
  `should_remove_dom_node` (`src/cleaner.rs:639`) and `clean_conditionally_tag`
  (`src/cleaner.rs` — two-phase collect-`NodeId`s-then-detach). It already
  weighs class/id patterns AND content quality (link density, text length),
  which is exactly what prevents this class of false positive.
- `src/cleaner.rs:56-60` contains a comment explaining that `widget` was
  deliberately excluded from regex-based removal for this same reason
  (page builders use it on real content containers). The same reasoning
  applies to `header` and was not applied.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Build | `cargo build` | exit 0 |
| Tests | `cargo test` | all pass |
| Mozilla suite | `cargo test --test mozilla_test_suite` | passes; divergence lists unchanged |
| Lint/format | `cargo clippy --all-targets -- -D warnings` && `cargo fmt --check` | exit 0 |

## Scope

**In scope**:
- `src/post_processor.rs` — `unwrap_nav_wrappers` and its call site
- `src/cleaner.rs` — ONLY if you add the keyword coverage to the DOM path
- New regression tests

**Out of scope**:
- The other regex removers (`remove_share_elements`,
  `remove_navigation_elements`, `remove_unwanted_elements`) — plan 006 owns
  consolidating all of them. Fix ONLY the content-destroying one here.
- `tests/test-pages/**` — never edit fixtures.

## Git workflow

- Branch: `bugfix/nav-wrapper-deletes-article`
- Commit: `fix(post_processor): stop nav-wrapper removal from deleting article bodies`
- Do NOT push or open a PR.

## Steps

### Step 1: Write the failing test first

In `src/post_processor.rs`'s `#[cfg(test)] mod tests`, add a test that builds
the reproduction above (a `<div id="main">` containing a
`<div class="entry-header">` that wraps an `<h1>` and 6 substantial
paragraphs), runs it through the full `Readability::parse()` path, and
asserts the paragraph text survives in `article.content` and
`article.length > 0`.

Run it and confirm it FAILS before you change anything. Report the observed
failure.

**Verify**: `cargo test --lib post_processor` → the new test fails with
`length == 0`.

### Step 2: Remove the destructive behaviour

Pick ONE of these, in order of preference:

**Option A (preferred): delete `unwrap_nav_wrappers` entirely** and its call
at `src/post_processor.rs:189`. Rationale: the DOM path
(`should_remove_dom_node`) already removes nav/menu/sidebar containers using
class/id weight *plus* content-quality checks, and runs later in the same
pipeline (`cleaner::clean_article_content`). Verify this claim before
relying on it: check that `should_remove_dom_node`'s keyword set covers
nav/navbar/menu/sidebar. If a keyword is genuinely missing there, add it to
the DOM path rather than keeping the regex.

**Option B (fallback, only if A regresses the corpus)**: keep the function but
(i) drop `header` from the keyword list entirely, and (ii) require
whole-token class matching (match against space-delimited class tokens, not
raw substrings), and (iii) rename it to `remove_nav_wrappers` since it
deletes rather than unwraps.

Whichever you choose, say which and why in your report.

**Verify**: `cargo test --lib post_processor` → Step 1's test now passes.

### Step 3: Guard the whole class of failure

Add a test asserting the invariant directly: for a document with substantial
article text, `parse()` must never return `Some(article)` with
`article.length == 0`. Parameterize it over several wrapper classes —
`entry-header`, `post-header`, `article-header`, `page-header`,
`entry-content`, `site-header` — with real content inside each.

Note honestly in a comment which of these are *expected* to be removed (a
genuine `site-header` containing only nav links SHOULD be removed) versus
which must be preserved (a wrapper containing the article body). If a case is
genuinely ambiguous, assert only the non-empty invariant.

**Verify**: `cargo test --lib post_processor` → all pass.

### Step 4: Corpus regression

**Verify**: `cargo test --test mozilla_test_suite` → passes. The
`KNOWN_METADATA_DIVERGENCES` / `KNOWN_CONTENT_DIVERGENCES` lists must not
grow. If a case moves from divergent to passing, that's a win — say so. If
any case regresses, STOP.

Also run the full `cargo test`.

## Test plan

Steps 1, 3 (regression + invariant), and the Mozilla corpus as the gate.
Model the tests on the existing `#[cfg(test)]` tests in
`src/post_processor.rs`.

## Done criteria

- [ ] A test reproducing the `entry-header` content loss exists and passes
- [ ] `parse()` never returns `Some` with `length == 0` for the tested
      documents
- [ ] `cargo test` exits 0
- [ ] Mozilla suite passes with divergence lists not grown
- [ ] `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
      exit 0
- [ ] `plans/README.md` status row updated

## STOP conditions

- Option A causes any Mozilla corpus case to regress (junk that used to be
  removed now survives) — report which cases and fall back to Option B.
- Removing the call reveals another pass that depends on it having run.
- More than 3 existing unit tests need their expectations changed — report
  before editing them; a test asserting the buggy deletion is itself suspect,
  but 3+ suggests the behaviour is load-bearing somewhere.

## Maintenance notes

- Plan 006 (DOM-only element removal) supersedes this fix by deleting all the
  regex removers. This plan is the urgent targeted subset — do not let it
  block 006, and do not expand it into 006.
- The root lesson: substring class matching plus non-greedy `.*?</tag>` is
  never safe on HTML. `src/cleaner.rs:56-60` already documented this hazard
  for `widget`; the same reasoning was never applied to `header`.
- Reviewers: any future keyword added to a removal list must be checked
  against the "could this substring appear in a container that holds the
  article?" question.
