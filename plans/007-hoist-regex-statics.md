# Plan 007: Hoist per-call regex compilation into Lazy statics

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- src/post_processor.rs src/cleaner.rs src/metadata.rs`
> IMPORTANT: if plan 006 (DOM-only element removal) has already landed, most
> of the `post_processor.rs`/`cleaner.rs` targets below were DELETED — skip
> any function that no longer exists and do only what remains.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none (but coordinate with plan 006 — see drift note)
- **Category**: perf
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

Several hot functions compile regexes inside loops on every call. A single
`prep_article` invocation (runs once per parsed article) recompiles ~60
regexes whose patterns are fixed at compile time: `remove_unwanted_elements`
compiles 12, `remove_share_elements` ~24 (tags × keywords × class/id),
`remove_navigation_elements` ~32, plus `remove_nav_like_sections` and
`remove_blocks_for_tag` in `cleaner.rs`, and one-shot sites in `metadata.rs`.
Regex compilation is orders of magnitude more expensive than matching. The
repo already uses `once_cell::sync::Lazy` statics extensively (e.g.
`clean_styles` at `src/post_processor.rs:226-235` does it right) — this plan
just finishes the job. It also removes the per-call
`Regex::new(...).unwrap()` latent-panic pattern flagged in the audit.

## Current state

Verified per-call compilation sites at `c7622fd`:

- `src/post_processor.rs:292` — loop over 12 `(name, pattern)` tuples,
  `Regex::new(pattern).unwrap()` per iteration, in
  `remove_unwanted_elements`.
- `src/post_processor.rs:302-320` — `remove_share_elements`: nested
  tags×keywords loops, TWO `Regex::new(&format!(...)).unwrap()` per inner
  iteration:

```rust
    for tag in &tags {
        for keyword in &keywords {
            let class_pattern =
                format!(r#"(?is)<{tag}\b[^>]*?class="[^"]*?{keyword}[^"]*?"[^>]*?>.*?</{tag}>"#);
            let re = Regex::new(&class_pattern).unwrap();
            result = re.replace_all(&result, "").to_string();
            let id_pattern = ...;
            let re = Regex::new(&id_pattern).unwrap();
            result = re.replace_all(&result, "").to_string();
        }
    }
```

- `src/post_processor.rs:325-350` — `remove_navigation_elements`: same shape,
  4 tags × 4 keywords × 2.
- `src/cleaner.rs:62-75` — `remove_nav_like_sections`: same shape (the outer
  `NAV_REGEX` at `:50` is already static — only the loop bodies compile).
- `src/cleaner.rs:130-132` — `remove_blocks_for_tag`:
  `Regex::new(&format!(r"(?is)<{tag}\b[^>]*?>.*?</{tag}>")).unwrap()` per call.
- `src/metadata.rs:32, 252, 256, 1425, 1428, 1437` — one-shot
  `Regex::new(...)` per document parse (lower priority, same fix).

Exemplar of the target pattern, already in this file:

```rust
// src/post_processor.rs:226-229
    static STYLE_DOUBLE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"(?i)\s+style\s*=\s*"[^"]*""#).unwrap());
```

For the tag×keyword matrices, the target is a
`static NAME: Lazy<Vec<Regex>> = Lazy::new(|| { ...build all combinations... })`
— iterate the prebuilt vector in the function body.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Build   | `cargo build` | exit 0 |
| Tests   | `cargo test` | all pass, zero output diffs |
| Bench (optional) | `cargo bench` | prep/clean benches improve or hold |

## Scope

**In scope**:
- `src/post_processor.rs`, `src/cleaner.rs`, `src/metadata.rs` — ONLY the
  mechanical hoist of `Regex::new` calls whose pattern inputs are
  compile-time-fixed (including fixed tag/keyword matrices). No pattern text
  may change.

**Out of scope**:
- Any pattern rewrite, keyword addition/removal, or semantic change.
- `Selector::parse(...)` call sites (same idea, different type — note them in
  your report if you see per-call ones, but don't change them here).
- Functions deleted by plan 006 if it landed first.

## Git workflow

- Branch: `perf/hoist-regex-statics`
- Commit style: `perf(post_processor): precompile removal regexes as statics`
  (one commit per file is fine).
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Hoist the fixed-list loops in post_processor.rs

For `remove_unwanted_elements`: replace the `(name, pattern)` tuple array +
in-loop compile with `static UNWANTED_TAG_REGEXES: Lazy<Vec<Regex>>` built
from the same 12 pattern strings, unchanged byte-for-byte.

For `remove_share_elements` and `remove_navigation_elements`: build
`static SHARE_REGEXES: Lazy<Vec<Regex>>` / `static NAV_REGEXES: Lazy<Vec<Regex>>`
that materialize the full tags×keywords×(class|id) matrix once. Keep the
`format!` template strings identical.

**Verify**: `cargo build` → exit 0; `cargo test --lib post_processor` → pass.

### Step 2: Hoist cleaner.rs sites

Same treatment for the loop in `remove_nav_like_sections` (keep the existing
`widget`-exclusion comment attached to the keyword list) and for
`remove_blocks_for_tag` (5 fixed tags — a `Lazy<Vec<(&str, Regex)>>` keyed by
tag, or a small `match`).

**Verify**: `cargo build` → exit 0; `cargo test --lib cleaner` → pass.

### Step 3: Hoist metadata.rs one-shots

Convert the `Regex::new` sites at `src/metadata.rs:32, 252, 256, 1425, 1428,
1437` to file-local `Lazy<Regex>` statics named for their purpose (read the
surrounding function to pick names matching existing statics like
`ITEMPROP_NAME_SELECTOR` at `:917`).

**Verify**: `cargo build` → exit 0; `cargo test --lib metadata` → pass.

### Step 4: Confirm no per-call compilation remains in scope

**Verify**:
`grep -n "Regex::new" src/post_processor.rs src/cleaner.rs src/metadata.rs`
→ every remaining hit is inside a `Lazy::new(...)` closure (visually confirm
each; list any justified exceptions in the report).

## Test plan

No new tests — output must be bit-identical, which the existing 154 lib +
117 markdown tests plus the Mozilla suite enforce. Optionally run
`cargo bench` before/after and paste the delta into the PR description.

## Done criteria

- [ ] `cargo test` exits 0 with zero expectation changes
- [ ] All `Regex::new` in the three files live inside `Lazy` initializers
- [ ] Pattern strings unchanged (`git diff` shows moves, not edits, of
      pattern literals)
- [ ] `plans/README.md` status row updated

## STOP conditions

- Any test output changes — a pattern was altered during the move; diff the
  literal.
- Plan 006 landed and deleted a target function mid-execution — re-run the
  drift check, skip deleted functions, note it in the report.

## Maintenance notes

- Statics with `.unwrap()` inside `Lazy` panic at first use if a pattern is
  invalid — acceptable for compile-time-fixed patterns, and now it happens
  once, not per call. New dynamic patterns (built from user input) must NOT
  use this pattern; none exist today.
- If plan 006 executes after this one, it will delete some of these statics —
  that's fine, wasted-work-wise this plan is cheap.
