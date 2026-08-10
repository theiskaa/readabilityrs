# Plan 012: Migrate once_cell → std LazyLock and declare an MSRV

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on. On
> any STOP condition, stop and report. When done, update this plan's row in
> `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- Cargo.toml src/`
> This is a repo-wide mechanical migration; drift in individual files is
> expected and fine — the pattern, not the line numbers, is what matters.

## Status

- **Priority**: P3
- **Effort**: M (mechanical, ~104 sites)
- **Risk**: LOW
- **Depends on**: plans/003-ci-and-lint-baseline.md (CI should gate this)
- **Category**: migration
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

`once_cell = "1.19"` (`Cargo.toml:16`) is used exclusively as
`once_cell::sync::Lazy` — ~104 `Lazy::new` sites across ~10 files — which
`std::sync::LazyLock` (stable since Rust 1.80, August 2024) replaces
one-for-one. Dropping the dependency shrinks the tree and compile graph. The
migration also forces declaring `rust-version` in Cargo.toml, which this
published crate currently lacks entirely — consumers today get no MSRV
signal at all.

## Current state

- Every usage is the same pattern (verified — no `OnceCell`, no `race`, no
  `unsync` usage):

```rust
use once_cell::sync::Lazy;
static COMMENT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"…").unwrap());
```

- Files with `use once_cell::sync::Lazy`: find them with
  `grep -rln "once_cell" src/` (at planning: constants.rs, metadata.rs,
  utils.rs, post_processor.rs, cleaner.rs, elements/*, and others — trust
  the grep, not this list).
- `Cargo.toml` has NO `rust-version` field.
- Target replacement is a pure rename: `std::sync::LazyLock<T>` with
  `LazyLock::new(...)` — identical `Deref` semantics for this pattern.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Build   | `cargo build` | exit 0 |
| Tests   | `cargo test` | all pass |
| MSRV check (if installed) | `cargo +1.80.0 build` (requires rustup toolchain) | exit 0; SKIP if toolchain absent — note it |

## Scope

**In scope**: every `once_cell` usage in `src/`; `Cargo.toml`
(remove dep, add `rust-version`); `Cargo.lock` (regenerated).

**Out of scope**: any other refactor while touching these files; benches/
tests unless they use once_cell (grep to confirm; at planning they don't).

## Git workflow

- Branch: `refactor/lazylock-migration`
- Commit style: `refactor: replace once_cell::sync::Lazy with std LazyLock`
  + `chore: declare rust-version 1.80`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Mechanical replace

For each file from `grep -rln "once_cell" src/`:
- `use once_cell::sync::Lazy;` → `use std::sync::LazyLock;`
- `Lazy<` → `LazyLock<`; `Lazy::new` → `LazyLock::new`.

Do it file-by-file with exact-string edits, not a blind repo-wide sed (some
files may import Lazy inside functions or alongside other items — read each
import line you touch).

**Verify**: `cargo build` → exit 0 after each file batch;
`grep -rn "once_cell" src/` → no matches when done.

### Step 2: Drop the dependency, declare MSRV

In `Cargo.toml`: delete the `once_cell` line from `[dependencies]`; add
`rust-version = "1.80"` in `[package]`. Run `cargo build` to refresh
`Cargo.lock`. Note: `once_cell` may remain in `Cargo.lock` as a TRANSITIVE
dep of other crates — that's expected; only the direct dependency matters.

**Verify**: `cargo build` → exit 0;
`grep -n "once_cell" Cargo.toml` → no matches;
`grep -n "rust-version" Cargo.toml` → 1 match.

### Step 3: Full regression + optional MSRV proof

`cargo test` → all pass. If `rustup toolchain list` shows a 1.80.x toolchain
(or you can install one non-interactively), build with it to prove the MSRV
claim; otherwise state in the report that MSRV was declared but not
toolchain-verified.

**Verify**: `cargo test` → exit 0.

## Test plan

No new tests — statics behave identically; the full suite is the gate.

## Done criteria

- [ ] `grep -rn "once_cell" src/ Cargo.toml` → zero matches
- [ ] `rust-version = "1.80"` present in Cargo.toml
- [ ] `cargo test` exits 0
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0 (if plan 003 landed)
- [ ] `plans/README.md` status row updated

## STOP conditions

- Any usage turns out NOT to be the `sync::Lazy` pattern (e.g. `OnceCell` or
  `Lazy::force`) — report the site; those need case-by-case mapping.
- The team/operator has expressed an MSRV constraint below 1.80 anywhere
  (README, CI matrix) — none found at planning, but check before committing.

## Maintenance notes

- Future contributors should reach for `std::sync::LazyLock` /
  `std::sync::OnceLock`; reject new `once_cell` imports in review.
- If a CI MSRV job is wanted later, add a matrix entry pinned to the declared
  `rust-version` (extends plan 003's workflow).
