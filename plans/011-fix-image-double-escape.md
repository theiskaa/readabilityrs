# Plan 011: Stop double-escaping `&` in standardized image URLs

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on. On
> any STOP condition, stop and report. When done, update this plan's row in
> `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- src/elements/images.rs src/readability.rs`
> On drift, re-verify the excerpts below before proceeding.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

`standardize_images` (part of the markdown-output standardization pipeline)
rewrites lazy-loaded image attributes. It runs on HTML that has ALREADY been
serialized with entity escaping (`Readability::parse` calls
`elements::standardize_all(&cleaned_html, …)` at `src/readability.rs:223`,
and `cleaned_html` was produced by `element_to_html`, which escapes attribute
values with `v_htmlescape`). The regex captures therefore contain `&amp;`,
not raw `&` — but `escape_attr` in `src/elements/images.rs` re-escapes `&` →
`&amp;`, turning `?a=1&amp;b=2` into `?a=1&amp;amp;b=2`. Image URLs with
query strings come out broken in the markdown output.

## Current state

- `src/elements/images.rs:49-77` — captured `data-src`/`srcset` values are
  written back through `escape_attr`:

```rust
// src/elements/images.rs:54-60
        if (src.is_empty() || is_placeholder_src(&src)) && !data_src.is_empty() {
            if src.is_empty() {
                result = result.replacen("<img", &format!("<img src=\"{}\"", escape_attr(&data_src)), 1);
            } else {
                result = replace_src_attr(&result, &src, &escape_attr(&data_src));
            }
        }
        // ... srcset path also calls escape_attr(&best) at :71
```

```rust
// src/elements/images.rs:127-132
fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
```

- Pipeline context: values arriving here CANNOT contain raw `"`, `<`, `>`,
  or `&` when called from `parse()` (upstream escaping guarantees it; also
  the capture regexes use `[^"]*` so `"` is impossible regardless). Raw `&`
  IS possible in the documented standalone use
  (`elements::standardize_all` on arbitrary HTML, README ~line 65) — where an
  unescaped `&` in an attribute is tolerated by all HTML5 parsers as long as
  it doesn't form an ambiguous entity.
- Tests for this module live in `src/elements/images.rs` `#[cfg(test)]` and
  `tests/markdown_tests.rs` (117 tests).

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Unit tests | `cargo test --lib images` | pass |
| Markdown suite | `cargo test --test markdown_tests` | all pass |
| Full | `cargo test` | all pass |

## Scope

**In scope**: `src/elements/images.rs` only.

**Out of scope**: `element_to_html`'s escaping (correct as-is); other
`elements/*` modules (audit found no equivalent double-escape there, but if
you spot one, report — don't fix here).

## Git workflow

- Branch: `bugfix/image-url-double-escape`
- Commit style: `fix(elements): avoid double-escaping ampersands in
  standardized image URLs`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Make `escape_attr` entity-aware

Replace the blind `&` escape so an `&` that already begins a character
reference is left alone:

```rust
use once_cell::sync::Lazy;
use regex::Regex;

/// Escape attribute-unsafe chars WITHOUT re-escaping existing entities:
/// `&` is escaped only when it does not already start a character reference.
static BARE_AMP_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"&(?![a-zA-Z][a-zA-Z0-9]{1,31};|#[0-9]{1,7};|#[xX][0-9a-fA-F]{1,6};)").unwrap()
});

fn escape_attr(s: &str) -> String {
    let s = BARE_AMP_RE.replace_all(s, "&amp;").to_string();
    s.replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}
```

NOTE: the `regex` crate does NOT support look-ahead (`(?!…)`). The snippet
above will NOT compile as written — implement the same semantics manually:
scan for `&`, and check whether the following bytes match an entity pattern
(`[a-zA-Z][a-zA-Z0-9]*;`, `#[0-9]+;`, or `#[xX][0-9a-fA-F]+;` within a
reasonable length, say 32 chars); escape only bare `&`. A small hand-rolled
loop over `char_indices` is the expected shape — model precision on the
existing entity decoder in `src/utils.rs:9-61`, which does the inverse walk.

**Verify**: `cargo build` → exit 0.

### Step 2: Unit tests

Add to `src/elements/images.rs` tests:
1. `escape_attr("a?x=1&amp;y=2")` → unchanged (`&amp;` preserved, not
   doubled).
2. `escape_attr("a?x=1&y=2")` → `a?x=1&amp;y=2` (bare `&` still escaped —
   standalone-use path).
3. `escape_attr("&#38; &#x26; &notanentity")` → numeric refs preserved; the
   trailing `&notanentity` case: `&notanentity` DOES match the alpha entity
   pattern shape (letters then... no semicolon) — it has no `;`, so it must
   be escaped. Make the test reflect that: without a terminating `;` within
   bounds, escape.
4. End-to-end: run `standardize_images` (or `standardize_all`) on an
   `<img data-src="https://cdn.example.com/i.jpg?w=100&amp;h=50">` fixture
   with a placeholder src, assert the output `src` contains `&amp;` exactly
   once per separator (no `&amp;amp;`).

**Verify**: `cargo test --lib images` → pass.

### Step 3: Full regression

**Verify**: `cargo test` → all pass; `cargo test --test markdown_tests` → all
117 pass (if any markdown expectation contains `&amp;amp;`, it was asserting
the BUG — fix the expectation and call it out in the commit message).

## Test plan

Step 2's four tests; existing markdown suite as the regression gate.

## Done criteria

- [ ] `cargo test` exits 0
- [ ] New tests prove `&amp;` is not doubled and bare `&` still escapes
- [ ] Any expectation that encoded the bug is corrected and documented in the
      commit message
- [ ] `git status` shows only `src/elements/images.rs` and possibly
      `tests/markdown_tests.rs` modified
- [ ] `plans/README.md` status row updated

## STOP conditions

- More than 3 markdown-suite expectations contain `&amp;amp;` — the bug is
  load-bearing in the corpus; report scope before mass-editing.
- You find the same double-escape in other `elements/*` files — report the
  list; fixing them here without tests is out of scope.

## Maintenance notes

- Root cause is the string-level standardization pipeline operating on
  already-escaped HTML. Plan 009's deferred follow-up (tree-native
  standardization) eliminates the entire class; this fix is the targeted
  interim.
- Reviewers: check the entity-detection bounds (max lengths) — an unbounded
  scan on a `&` followed by megabytes of alphanumerics would be a perf trap.
