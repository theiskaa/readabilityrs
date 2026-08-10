# Plan 019: Apply the entity-aware escape fix to math.rs and code_blocks.rs

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on. On
> any STOP condition, stop and report. When done, update this plan's row in
> `plans/README.md` — unless a reviewer dispatched you and said they maintain
> the index.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- src/elements/math.rs src/elements/code_blocks.rs src/elements/images.rs`
> If `src/elements/images.rs` already contains an `is_entity_start` helper,
> plan 011 has landed and you should REUSE it (see Step 1). If it does not,
> plan 011 has not landed — STOP and report, because this plan depends on it.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: plans/011-fix-image-double-escape.md (MANDATORY — provides
  the shared helper this plan reuses)
- **Category**: bug
- **Planned at**: commit `c7622fd`, 2026-08-04 (found during execution of 011)

## Why this matters

Plan 011 fixed a double-escaping bug in `src/elements/images.rs`: the
standardization pipeline runs on **already entity-escaped** HTML (see
`src/readability.rs:223`, which calls `elements::standardize_all(&cleaned_html, …)`
on output from `element_to_html`), so blindly replacing `&` with `&amp;`
turns `?a=1&amp;b=2` into `?a=1&amp;amp;b=2`.

Two more sites in the same pipeline have the identical blind replace and were
left untouched by 011's scope. Verified at `c7622fd`:

```rust
// src/elements/math.rs — byte-identical to the pre-fix images.rs version
fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
```

```rust
// src/elements/code_blocks.rs — same pattern, no quote handling
fn html_escape_code(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
```

Consequence: math content (MathJax/KaTeX `alttext`/annotation values) and
fenced code blocks containing `&amp;`, `&lt;`, `&gt;` or any other character
reference get their ampersands doubled in the Markdown output. Code blocks are
the worse case — source code is *full* of `&&`, `&amp;` in embedded HTML, and
entity-looking text — so a snippet round-tripped through the markdown path can
come out visibly corrupted.

## Current state

- After plan 011 lands, `src/elements/images.rs` contains:
  - `const ENTITY_LOOKAHEAD_LIMIT: usize = 32;`
  - `fn escape_attr(s: &str) -> String` — entity-aware, escapes `"`, `<`, `>`,
    and bare `&` only
  - `fn is_entity_start(s: &str) -> bool` — returns true when the string
    (which starts with `&`) begins a terminated character reference
    (`&name;`, `&#123;`, `&#x1F;`) within the lookahead bound
  These are private to `images.rs`.
- `src/elements/mod.rs` declares the sibling modules (`code_blocks`,
  `footnotes`, `headings`, `images`, `languages`, `math`) and exposes
  `standardize_all`.
- Repo conventions: private helpers with `#[cfg(test)] mod tests` at the
  bottom of each module; no `unwrap()`/`expect()` in library code; no
  decorative comments.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Build | `cargo build` | exit 0 |
| Unit tests | `cargo test --lib elements` | pass |
| Markdown suite | `cargo test --test markdown_tests` | all pass |
| Full | `cargo test` | all pass |

## Scope

**In scope**:
- `src/elements/mod.rs` — host the shared helper (see Step 1)
- `src/elements/images.rs` — switch to the shared helper
- `src/elements/math.rs` — switch to the shared helper
- `src/elements/code_blocks.rs` — switch to the shared helper
- `tests/markdown_tests.rs` — ONLY if an expectation encoded the bug

**Out of scope**:
- `element_to_html`'s escaping in `src/content_extractor.rs` — correct as-is.
- Any behavior change beyond ampersand handling. `"`/`<`/`>` escaping must
  stay exactly as each call site has it today (note `html_escape_code` does
  NOT escape `"` — preserve that difference).

## Git workflow

- Branch: `bugfix/double-escape-math-code-blocks`
- Commit: single line, `fix(elements): reuse entity-aware escaping in math and code blocks`
- Do NOT push or open a PR.

## Steps

### Step 1: Promote the helper to a shared location

Move `ENTITY_LOOKAHEAD_LIMIT`, `is_entity_start`, and an `escape_attr`-style
core out of `src/elements/images.rs` into `src/elements/mod.rs` as
`pub(crate)` (or `pub(super)`) items. Because `html_escape_code` must NOT
escape `"`, expose the core as two functions rather than one:

```rust
/// True when `s` (which must start with `&`) begins a terminated character
/// reference within the lookahead bound.
pub(crate) fn is_entity_start(s: &str) -> bool { /* moved verbatim */ }

/// Escape `&` (bare only), `<`, `>` — and `"` when `escape_quotes` is true.
pub(crate) fn escape_html_preserving_entities(s: &str, escape_quotes: bool) -> String { /* … */ }
```

Move the helper's existing unit tests along with it.

**Verify**: `cargo build` → exit 0; `cargo test --lib elements` → pass.

### Step 2: Switch the three call sites

- `images.rs::escape_attr` → delegate to
  `escape_html_preserving_entities(s, true)`.
- `math.rs::escape_attr` → same delegation (identical semantics to before,
  minus the double-escape).
- `code_blocks.rs::html_escape_code` → `escape_html_preserving_entities(s, false)`
  — **quotes must remain unescaped** to preserve current behavior.

Delete the now-duplicated private implementations.

**Verify**: `cargo build` → exit 0;
`grep -n "replace('&', \"&amp;\")" src/elements/` → no matches.

### Step 3: Tests

Add to each of `math.rs` and `code_blocks.rs` test modules:
1. Already-escaped input is not doubled: a value containing `&amp;` stays
   `&amp;` (not `&amp;amp;`).
2. Bare `&` is still escaped.
3. For `code_blocks.rs` specifically: a code snippet containing `a && b`,
   `x < y`, and a literal `"quoted"` — assert `&&` becomes `&amp;&amp;`,
   `<` becomes `&lt;`, and the double quotes are **left as-is**.
4. Multi-byte UTF-8 input does not panic.

**Verify**: `cargo test --lib elements` → pass; `cargo test` → full suite
passes.

### Step 4: Regression baseline

**Verify**: `cargo test --test markdown_tests` → all pass. If any expectation
contained `&amp;amp;`, it encoded the bug — fix it and say so in your report.
Also confirm the Mozilla suite is unmoved (if it still shows two `#[ignore]`d
tests, run `cargo test --test mozilla_test_suite -- --ignored --nocapture`
and expect `119 passed, 11 failed`).

## Test plan

Step 3's cases per module, plus the 117-test markdown suite as the regression
gate. Reuse the battery shape from `images.rs`'s tests.

## Done criteria

- [ ] `grep -rn "replace('&', \"&amp;\")" src/elements/` → no matches
- [ ] One shared helper; no duplicated entity-detection logic across modules
- [ ] `cargo test` exits 0
- [ ] `code_blocks.rs` still leaves `"` unescaped (explicit test)
- [ ] Markdown suite unchanged (or documented expectation fixes)
- [ ] `plans/README.md` status row updated

## STOP conditions

- `src/elements/images.rs` has no `is_entity_start` — plan 011 hasn't landed;
  this plan depends on it.
- More than 3 markdown expectations contain `&amp;amp;` — report before
  mass-editing.
- Making `code_blocks.rs` share the helper would change quote handling — the
  `escape_quotes` flag exists precisely to prevent that; if it can't be
  preserved, stop and report.

## Maintenance notes

- After this, there is exactly ONE ampersand-escaping implementation in
  `src/elements/`. Reviewers should reject new blind `replace('&', "&amp;")`
  calls in that directory.
- Plan 009's deferred follow-up (tree-native standardization) removes the
  need for string-level escaping here entirely; this is the interim fix.
