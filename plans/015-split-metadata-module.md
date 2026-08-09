# Plan 015: Split the 2127-line metadata.rs god module

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on. On
> any STOP condition, stop and report. When done, update this plan's row in
> `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- src/metadata.rs src/utils.rs`
> Line numbers below WILL have drifted if plans 001/007 landed (they touch
> these files). The function inventory, not the line numbers, drives this
> plan — re-locate functions by name.

## Status

- **Priority**: P3
- **Effort**: M
- **Risk**: LOW (mechanical move, no logic change)
- **Depends on**: schedule AFTER plans 001 and 007 (they edit these files;
  moving code underneath them creates merge pain)
- **Category**: tech-debt
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

`src/metadata.rs` is 2127 lines — the repo's largest file, roughly double the
next — holding at least five separable concerns: JSON-LD parsing, meta-tag
extraction, a ~780-line byline/author heuristic cluster (~15 functions),
title extraction, language detection, and image extraction. Every
metadata-adjacent change lands in this one file, and the byline heuristics
(the most actively evolved area per git history: WaPo/Breitbart/CNET/Herald
Sun case tests) are buried inside it. A directory split cuts merge friction
and makes the heuristics reviewable. Additionally, most of `src/utils.rs`
(853 lines) is byline-cleaning helpers that belong with the byline module.

## Current state

Function inventory of `src/metadata.rs` at `c7622fd` (names are ground
truth; re-grep for current lines):

- JSON-LD: `get_json_ld` (:28), `extract_json_ld_image` (:189)
- Meta-tag orchestration: `get_article_metadata` (:250)
- Image: `extract_image_from_document` (:474)
- Byline cluster (~lines 520-1360): `DomBylineCandidate::new` (:524),
  `extract_byline_from_document` (:543), `extract_standfirst_caps_byline`
  (:824), `build_byline_text` (:856), `strip_intermediate_newline` (:890),
  `collect_byline_candidate_text` (:907), `collect_child_author_names`
  (:920), `element_has_semantic_name` (:965), `should_prefer_child_names`
  (:978), `looks_like_job_descriptor` (:1038), `should_prefer_dom_byline`
  (:1090), `should_prefer_caps_standfirst` (:1169), `looks_like_caps_author`
  (:1184), `contains_caps_noise_token` (:1203), `parent_byline_text` (:1217),
  `element_has_byline_keyword` (:1236), `element_has_explicit_byline_marker`
  (:1250), `is_priority_dom_candidate` (:1256), `ancestor_has_keyword`
  (:1260), `is_ignorable_byline_context` (:1284), `is_noise_byline_context`
  (:1324), plus `ITEMPROP_NAME_SELECTOR` static (:917)
- Language: `extract_language_from_document` (:1363)
- Title: `extract_title_from_document` (:1407) + `word_count` helper (:1419)
- `#[cfg(test)] mod tests` from ~:1490 — ~45 tests, most byline-focused
- There is also a `#[cfg(test)]`-gated debug `eprintln!` block around :426 —
  move it with `get_article_metadata`, or delete it (it references one
  specific test article; deleting is fine, note it).

Consumers (keep these compiling unchanged): `src/readability.rs` calls
`get_json_ld` and `get_article_metadata` (see `src/readability.rs:161-167`);
`src/lib.rs` does NOT export metadata publicly (`mod metadata;` is private —
confirm with `grep -n "mod metadata" src/lib.rs`).

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Build   | `cargo build` | exit 0 |
| Tests   | `cargo test` | all pass, same count |
| Structure | `wc -l src/metadata/*.rs` | no file > ~900 lines |

## Scope

**In scope**: `src/metadata.rs` → `src/metadata/` directory
(`mod.rs`, `json_ld.rs`, `byline.rs`, `title.rs`, `language.rs`,
`image.rs`); moving byline-specific helpers from `src/utils.rs` into
`src/metadata/byline.rs` ONLY if their sole callers are byline code
(verify each with grep before moving).

**Out of scope**: ANY logic change, threshold tweak, or "improvement while
we're here" — this is a pure move. Do not rename functions. Do not change
which items are `pub`/private beyond the minimum `pub(crate)`/`pub(super)`
adjustments the module split forces.

## Git workflow

- Branch: `refactor/split-metadata-module`
- Use `git mv`-style history where possible; one commit for the split, one
  for the utils.rs migration. Style: `refactor(metadata): split into
  submodules`.
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Create the module skeleton

`mkdir src/metadata` … create `src/metadata/mod.rs` that declares the
submodules and re-exports the two crate-facing entry points so callers don't
change:

```rust
mod byline;
mod image;
mod json_ld;
mod language;
mod title;

pub use json_ld::get_json_ld;
// get_article_metadata stays in mod.rs (it orchestrates the others)
```

Move code by concern per the inventory: JSON-LD functions → `json_ld.rs`;
the whole byline cluster + its `Lazy` statics → `byline.rs`;
title+`word_count` → `title.rs`; language → `language.rs`; image →
`image.rs`; `get_article_metadata`, the `Metadata` struct (check where it's
defined — grep `struct Metadata`; it may live in metadata.rs or elsewhere)
and shared glue stay in `mod.rs`. Adjust visibility with `pub(super)` where
mod.rs calls into siblings.

**Verify**: `cargo build` → exit 0.

### Step 2: Move the tests with their subjects

Split the `mod tests` block: each test moves to the file whose functions it
exercises (byline tests → `byline.rs`, JSON-LD tests → `json_ld.rs`, etc.).
Integration-ish tests that span concerns stay in `mod.rs`.

**Verify**: `cargo test --lib metadata` → same test COUNT as before the
split (count first: `cargo test --lib metadata 2>&1 | tail -1`); `cargo
test` → all pass.

### Step 3: Migrate byline helpers from utils.rs

`grep -n "pub fn\|fn " src/utils.rs`, and for each byline-related helper
(`clean_byline_text*`, `strip_trailing_datetime_clause`,
`looks_like_datetime_segment`, `contains_author_like_segment`,
`remove_timestamp_lines`, `remove_social_handle_lines`,
`collapse_blank_lines_preserve_indent`, and similar — the cluster spans
roughly `src/utils.rs:79-711`), check its callers with grep. Move those
whose ONLY callers are metadata/byline code into `src/metadata/byline.rs`
(with their tests). Leave genuinely shared helpers (`unescape_html_entities`,
whitespace utilities, `looks_like_bracket_menu` — called from
readability.rs too) in utils.rs.

**Verify**: `cargo build` → exit 0; `cargo test` → all pass;
`wc -l src/utils.rs` → meaningfully smaller (report before/after).

## Test plan

No new tests. Invariant: identical test count and results before/after.

## Done criteria

- [ ] `src/metadata.rs` (single file) no longer exists; `src/metadata/`
      has mod.rs + 5 submodules
- [ ] `wc -l src/metadata/*.rs` → largest file < ~900 lines
- [ ] `cargo test` exits 0 with the SAME total test count as baseline
- [ ] `git diff` shows moves, not edits (spot-check: function bodies
      byte-identical)
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0 (if 003 landed)
- [ ] `plans/README.md` status row updated

## STOP conditions

- A helper you're moving has callers outside metadata that grep missed
  (compile error) — leave it in utils.rs and continue; report the list.
- The split forces making anything `pub` at the CRATE root that wasn't —
  stop; the public API must not grow from a refactor.
- Merge conflicts with in-flight plans 001/007 — coordinate ordering rather
  than resolving blind.

## Maintenance notes

- New byline heuristics (a frequent change category per git history) now go
  in `src/metadata/byline.rs`; reviewers should bounce byline code appearing
  anywhere else.
- Follow-up candidate (not planned): `utils.rs` post-shrink may merge into
  `dom_utils.rs`/a `text.rs` — decide after this lands.
