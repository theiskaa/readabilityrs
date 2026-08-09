# Plan 002: Make the Mozilla test suite assert and run by default

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- tests/mozilla_test_suite.rs README.md`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: none
- **Category**: tests
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

The 130-page Mozilla Readability corpus in `tests/test-pages/` is the
project's flagship verification asset — the README's headline claim is
"passes 122 of 130". But the harness (`tests/mozilla_test_suite.rs`) is
`#[test] #[ignore]` on both of its tests, contains **zero assert calls**
(it only `println!`s pass/fail counts and returns Ok), and never compares the
extracted article body — only title/byline/excerpt/site_name. So `cargo test`
skips it, and even `cargo test -- --ignored` cannot fail. Any regression in
the ~10k lines of extraction code ships silently. This plan turns the corpus
into an enforced regression net. Several later plans (005, 006, 008, 009,
012) are refactors that depend on this net existing.

## Current state

- `tests/mozilla_test_suite.rs` — 274 lines. Structure:
  - `TestCase::load` (lines ~34-59) reads per-directory `source.html`,
    optional `expected.html`, and `expected-metadata.json` (serde camelCase
    struct `ExpectedMetadata` with fields title, byline, dir, lang, excerpt,
    site_name, published_time, readerable).
  - `load_test_cases()` (lines ~61-84) iterates `tests/test-pages/*/`,
    sorted by name. There are 130 directories.
  - `strings_match` (lines ~87-97) whitespace-normalizes both sides before
    comparing.
  - `#[test] #[ignore] fn test_mozilla_suite_metadata()` (line 100-101):
    loops all cases, counts pass/fail with `println!`, compares ONLY title,
    byline, excerpt, site_name via `strings_match`. **No asserts anywhere**
    (`grep -c assert tests/mozilla_test_suite.rs` → 0).
  - `#[test] #[ignore] fn` second test (~line 211): a debug printer for a
    single page — prints content previews, asserts nothing.
- `README.md:8` claims "passes 122 of 130 cases (93.8%)"; `README.md`
  around lines 130-132 explains the 8 divergences are intentional
  ("arguable improvements": better byline/excerpt choices).
- Baseline behavior at planning time (run it yourself to reproduce):
  `cargo test --test mozilla_test_suite -- --ignored --nocapture` prints
  per-case ✅/❌ and a final count. **Record the actual failing-case names
  from that output before changing anything** — those become the explicit
  exception list.
- Repo conventions: integration tests live in `tests/`, plain `#[test]`
  functions, no test framework beyond libtest. Conventional-commit messages.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Current suite (baseline) | `cargo test --test mozilla_test_suite -- --ignored --nocapture` | prints per-case results + counts; exit 0 (it cannot fail today) |
| Full tests | `cargo test` | all pass |
| New suite after this plan | `cargo test --test mozilla_test_suite` | runs by default, exit 0 |

## Scope

**In scope**:
- `tests/mozilla_test_suite.rs` (rewrite)
- `README.md` (only the sentence describing how to reproduce the pass count —
  do not otherwise rewrite the README; plan 010 owns README fixes)

**Out of scope** (do NOT touch):
- `tests/test-pages/**` — fixtures are Mozilla's corpus; never edit them to
  make tests pass.
- Any file under `src/` — if a case fails unexpectedly, that is a STOP
  condition, not a license to change extraction logic.
- `tests/markdown_tests.rs`.

## Git workflow

- Branch: `feature/asserting-mozilla-suite`
- Commit style: conventional commits, e.g.
  `test(mozilla): make suite assert and run by default`.
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Capture the baseline exception list

Run `cargo test --test mozilla_test_suite -- --ignored --nocapture` and write
down the exact directory names of every ❌ case. Expect ~8 (README says 8; if
the count differs wildly — see STOP conditions).

**Verify**: you have a concrete list of failing case names, e.g. saved in the
commit message or a comment.

### Step 2: Restructure the metadata test to assert

Rewrite `test_mozilla_suite_metadata` so that:

1. Remove `#[ignore]` — it runs on plain `cargo test`.
2. Add a constant at the top of the file:
   ```rust
   /// Cases where readabilityrs intentionally diverges from Mozilla's expected
   /// metadata (better byline/excerpt choices — see README "Compatibility").
   /// A case listed here is ALLOWED to mismatch; it is still parsed and must
   /// not panic. Do not add to this list to silence a new regression.
   const KNOWN_METADATA_DIVERGENCES: &[&str] = &[ /* names from Step 1 */ ];
   ```
3. Keep the existing loop and `strings_match` comparisons, but collect
   failures into a `Vec<String>` of formatted mismatch descriptions,
   skipping cases in `KNOWN_METADATA_DIVERGENCES`.
4. End with:
   ```rust
   assert!(
       failures.is_empty(),
       "{} case(s) regressed:\n{}",
       failures.len(),
       failures.join("\n")
   );
   ```
5. Also assert the corpus loaded: `assert_eq!(test_cases.len(), 130, ...)` —
   protects against the loader silently finding zero directories (today it
   just prints "No test cases found. Skipping.").

**Verify**: `cargo test --test mozilla_test_suite` → runs (not ignored),
passes, and reports the expected test count.

### Step 3: Add a body-content assertion tier

The suite currently never checks extracted article bodies. Byte-identical
HTML comparison against Mozilla's `expected.html` will NOT pass (different
serializers), so use a robust similarity floor instead:

1. Add a helper that extracts normalized text from an HTML string using the
   same approach as the library (`scraper::Html::parse_fragment(html)` then
   `root_element().text().collect::<String>()`, whitespace-normalized via
   `split_whitespace().collect::<Vec<_>>().join(" ")`).
2. New `#[test] fn test_mozilla_suite_content()`: for every case that has an
   `expected.html` AND expects `readerable: true`, parse with default
   options, require `article.content` to be `Some`, and compare
   normalized-text lengths: computed `len` must be within a tolerance band of
   expected `len` — start with `actual >= expected / 2 && actual <= expected * 2`.
3. Run it. If specific cases fall outside the band, list them in a second
   constant `KNOWN_CONTENT_DIVERGENCES: &[&str]` with the same "do not grow
   this list" comment. If MORE than ~15 cases fall outside the band, the
   tolerance is wrong for this codebase — STOP and report the distribution
   instead of hand-tuning further.
4. Add the same `assert!(failures.is_empty(), ...)` pattern.

This is deliberately a coarse floor, not parity: its job is to catch "body
extraction broke / went empty / grabbed the whole page", which today nothing
catches.

**Verify**: `cargo test --test mozilla_test_suite` → both tests pass;
`grep -c "assert" tests/mozilla_test_suite.rs` → ≥ 3.

### Step 4: Convert the debug-printer test

The second `#[ignore]` test (single-page debug printer) stays `#[ignore]` but
gets a reason string: `#[ignore = "manual debugging helper, prints only"]`.

**Verify**: `cargo test --test mozilla_test_suite -- --ignored` still runs it
without failing.

### Step 5: Update the README reproduction sentence

In README's compatibility section, state the reproduction command
(`cargo test --test mozilla_test_suite`) and that the known divergences are
listed by name in the test file. Do not change the 122/130 number unless
Step 1's baseline contradicts it — if it does, update the number to match
reality and say so in your report.

**Verify**: `grep -n "mozilla_test_suite" README.md` → at least one hit.

## Test plan

This plan IS the test plan. Net-new enforced coverage:
- 130-case metadata regression gate (title/byline/excerpt/site_name).
- 130-case content-extraction floor (non-empty + length band).
- Corpus-integrity assertion (130 directories load).

## Done criteria

- [ ] `cargo test` exits 0 and now includes the Mozilla suite (not ignored)
- [ ] `grep -c "assert" tests/mozilla_test_suite.rs` ≥ 3
- [ ] `KNOWN_METADATA_DIVERGENCES` exists, is non-empty, and every entry is a
      real directory name under `tests/test-pages/`
- [ ] Deliberately breaking extraction (e.g. locally editing a fixture copy —
      revert after) makes the suite FAIL — verify the gate actually gates,
      then restore
- [ ] `git status` shows only in-scope files modified
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:

- Step 1's baseline shows a failure count differing from README's 8 by more
  than 3 in either direction — the README claim or the harness may already be
  stale; report the actual list.
- Step 3's tolerance band excludes more than ~15 cases.
- Making the suite pass would require editing anything under
  `tests/test-pages/` or `src/`.
- Suite runtime by default exceeds ~3 minutes in release-of-debug (`cargo
  test`); report timing so the operator can decide whether it belongs behind
  a feature or in CI-only.

## Maintenance notes

- The two divergence lists are the suite's honesty mechanism. Reviewers must
  push back on any PR that grows them without a README-level justification.
- Plan 003 (CI) should run this suite on every PR once it exists.
- Deferred: byte-level golden comparison of serialized output (would need a
  canonical serializer; revisit after plan 009 consolidates serialization).
