# Plan 010: Fix the broken README example and make README snippets compile in CI

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on. On
> any STOP condition, stop and report. When done, update this plan's row in
> `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- README.md src/lib.rs`
> On drift, re-verify the excerpts below before proceeding.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: docs
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

The README's error-handling example — the first thing a new user copies —
does not compile: it imports `readabilityrs::error::ReadabilityError`, but
`error` is a private module (`src/lib.rs:121` declares `mod error;`; the type
is re-exported at the crate root by `src/lib.rs:133`). More broadly, README
code fences are plain ```` ```rust ```` blocks that `cargo test` never
compiles (Rust only doctests `///`/`//!` comments), so README examples rot
silently — this bug is the proof. Wiring the README into the crate docs via
`include_str!` makes every README snippet a doctest, so CI (plan 003) keeps
the front page of crates.io honest permanently.

## Current state

- `README.md:90` — `use readabilityrs::{Readability, error::ReadabilityError};`
  (broken import; correct is `readabilityrs::ReadabilityError`).
- `src/lib.rs:121` — `mod error;` (private);
  `src/lib.rs:133` — `pub use error::{ReadabilityError, Result};`.
- `src/lib.rs:1-114` — crate-level `//!` docs with their own `no_run`
  examples (those are fine and compile today).
- Other README rust fences: usage example (~lines 27-48), configuration
  (~lines 70-84), error handling (~89-97). None are compiled anywhere.
- Public API for reference (all at crate root): `Readability`, `Article`,
  `ReadabilityError`, `Result`, `ReadabilityOptions`, `MarkdownOptions`,
  `is_probably_readerable`, `ReaderableOptions` (`src/lib.rs:132-137`).

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Doctests | `cargo test --doc` | all pass |
| Full tests | `cargo test` | all pass |

## Scope

**In scope**: `README.md`, `src/lib.rs`.

**Out of scope**: rewriting README prose or structure (only make snippets
compile); the compatibility-claim sentence (plan 002 owns it); adding a
security section (plan 004 owns it — if 004 landed, don't disturb its
section).

## Git workflow

- Branch: `docs/readme-doctests`
- Commit style: `docs(readme): fix error-handling example and compile README
  snippets as doctests`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Fix the broken import

In `README.md`, change the error-handling example's import to
`use readabilityrs::{Readability, ReadabilityError};` and confirm the rest of
that snippet matches the current API (`Readability::new(html, Some(url),
None)?`, `parse()` returning `Option<Article>` — cross-check against
`src/lib.rs`'s own examples).

**Verify**: visual; compiled in Step 3.

### Step 2: Wire README into crate docs

At the very top of `src/lib.rs`, above the existing `//!` block, add:

```rust
#![doc = include_str!("../README.md")]
```

Caveat: this prepends the entire README to the crate docs, which ALSO still
contain the existing `//!` documentation — that duplication is ugly on
docs.rs. Decide by inspection: if the existing `//!` docs substantially
duplicate the README (they do overlap — both have usage + options examples),
REPLACE the overlapping `//!` sections with the include and keep only
crate-specific doc content that the README lacks. Keep the module-level docs
minimal rather than duplicated. Preserve any `#![...]` attributes order
requirements (inner attributes must precede items).

**Verify**: `cargo doc --no-deps` → exit 0; open
`target/doc/readabilityrs/index.html` optionally to sanity-check no doubled
sections.

### Step 3: Make every README fence pass as a doctest

Run `cargo test --doc`. Each README ```` ```rust ```` fence now compiles and
RUNS. Fix failures by:
- Adding `no_run` to fences that would do real work (none currently should —
  they parse inline strings; prefer keeping them runnable).
- Making snippets complete: doctests need items wrapped in an implicit main —
  statements are fine, but `?` needs a `Result` context. Use the hidden-line
  convention (`# fn main() -> Result<(), readabilityrs::ReadabilityError> {`
  … `# Ok(()) }` with `#`-prefixed lines) so README display stays clean.
  The usage example at README:42 uses `?` — it will need this treatment.

**Verify**: `cargo test --doc` → exit 0, and the doctest count INCREASED vs
before (report before/after counts).

## Test plan

The doctests ARE the tests. `cargo test` (full) must stay green.

## Done criteria

- [ ] `grep -n "error::ReadabilityError" README.md` → no matches
- [ ] `src/lib.rs` contains `#![doc = include_str!("../README.md")]`
- [ ] `cargo test --doc` exits 0 with more doctests than baseline
- [ ] `cargo test` exits 0
- [ ] `plans/README.md` status row updated

## STOP conditions

- A README snippet documents API that doesn't exist (beyond the known import
  bug) — that's a bigger staleness problem; report the mismatch list.
- `include_str!` doc duplication can't be resolved without a substantial
  `lib.rs` docs rewrite (>100 lines churn) — report and fall back to fixing
  Step 1 alone, leaving Steps 2-3 for a follow-up decision.

## Maintenance notes

- From now on, editing a README code fence can break `cargo test --doc` —
  that's the feature. Contributors should run doctests before pushing
  (plan 014 documents this).
- Version numbers in README (`readabilityrs = "0.1.3"`) are NOT compile-checked;
  release flow still must bump them manually.
