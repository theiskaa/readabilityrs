# Plan 003: Add CI and bring fmt/clippy to a clean, enforced baseline

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- src/ .github/`
> This plan touches many files mechanically (formatting). If `.github/` already
> exists, STOP — CI may have been added since planning.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none (but merge after 002 if possible, so CI gates the
  asserting suite from day one)
- **Category**: dx
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

The repo has no CI at all — no `.github/` directory exists, yet PRs are merged
regularly (see merge commits for #19, #22, #23 in `git log`). Nothing enforces
build, tests, formatting, or lints. Concretely, at planning time:
`cargo fmt --check` FAILS (unformatted code in `src/cleaner.rs` among others),
and `cargo clippy --all-targets` reports ~11 warnings. For a crate published
to crates.io, a broken build or an extraction regression can ship undetected.
This plan cleans the baseline and adds a single workflow that keeps it clean.

## Current state

- No `.github/` directory.
- `cargo fmt --check` exits 1 (reflow diffs in `src/cleaner.rs` lines ~61,
  70, 107, 584, 605 and possibly elsewhere). No `rustfmt.toml` — default
  style is in effect and is fine; do not add a config.
- `cargo clippy --all-targets` warnings (verified at planning):
  - `src/content_extractor.rs:59` — `sort_by` → `sort_by_key`
  - `src/content_extractor.rs:1070`, `:1073` — `.len() > 0` → `!is_empty()`
  - `src/metadata.rs:1828` — length comparison to zero
  - `src/metadata.rs:2118` — `.clone()` on `ElementRef` (which is `Copy`)
  - `src/elements/footnotes.rs:39` — manual loop counter
  - `src/markdown/rules/headings.rs:44`, `links.rs:84`,
    `lists.rs:47/54/62` — field assignment outside `Default::default()`
- Tests: `cargo test` passes (~30s debug). The Mozilla integration suite may
  or may not be `#[ignore]`d depending on whether plan 002 landed first —
  handle both cases in Step 4.
- Toolchain: repo has no `rust-toolchain.toml`; CI should use `stable`.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Format  | `cargo fmt` then `cargo fmt --check` | second exits 0 |
| Lint    | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Tests   | `cargo test` | all pass |
| Build   | `cargo build` | exit 0 |

## Scope

**In scope**:
- `.github/workflows/ci.yml` (create)
- Formatting-only changes across `src/`, `tests/`, `benches/*.rs` produced by
  `cargo fmt`
- Minimal mechanical edits to fix the clippy warnings listed above
- `Cargo.lock` only if `cargo update -p crossbeam-epoch` is included (Step 5)

**Out of scope** (do NOT touch):
- Any behavioral change. Clippy fixes must be semantics-preserving; if a fix
  would change behavior (e.g. the `sort_by_key` change altering ordering
  stability), prefer `#[allow]` with a one-line comment and report it.
- `benches/*.js`, `benches/package.json` — the Node comparison harness is not
  part of Rust CI.
- Adding rustfmt/clippy config files.

## Git workflow

- Branch: `feature/ci-baseline`
- Commits: separate `style(fmt): apply cargo fmt` from
  `fix(clippy): resolve clippy warnings` from `ci: add GitHub Actions workflow`
  — reviewers need the mechanical diff isolated.
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Apply formatting

Run `cargo fmt`. Commit the result alone.

**Verify**: `cargo fmt --check` → exit 0; `cargo test` → still passes.

### Step 2: Fix the clippy warnings

Address each warning listed in Current state with the idiomatic fix clippy
suggests. For `src/metadata.rs:2118`, replace `.clone()` with a plain copy
(dereference/`*`). For the `sort_by` at `src/content_extractor.rs:59`
(`attempts.sort_by(|a, b| b.text_length.cmp(&a.text_length))`), the
`sort_by_key` equivalent is `attempts.sort_by_key(|a| std::cmp::Reverse(a.text_length))`
— behavior-identical.

**Verify**: `cargo clippy --all-targets -- -D warnings` → exit 0;
`cargo test` → passes.

### Step 3: Create the workflow

Create `.github/workflows/ci.yml`:

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - name: Format
        run: cargo fmt --check
      - name: Clippy
        run: cargo clippy --all-targets -- -D warnings
      - name: Build
        run: cargo build --verbose
      - name: Tests
        run: cargo test --verbose
```

**Verify**: `ruby -ryaml -e 'YAML.load_file(".github/workflows/ci.yml")'` (or
`python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml'))"`
if pyyaml is available; otherwise visually confirm indentation) → no error.

### Step 4: Ensure the Mozilla suite is covered

- If plan 002 has landed (check: `grep -c "#\[ignore\]" tests/mozilla_test_suite.rs`
  → 1 means landed, 2+ means not), the suite already runs under `cargo test` —
  nothing to add.
- If 002 has NOT landed, add a CI step after Tests:
  `run: cargo test --test mozilla_test_suite -- --ignored --nocapture`
  with a YAML comment `# non-asserting until plan 002 lands` so the gap is
  visible.

**Verify**: workflow file contains either the default `cargo test` note or the
explicit `--ignored` step.

### Step 5 (optional but recommended): Clear the dev-only advisory

`cargo audit` (if installed) reports RUSTSEC-2026-0204 in `crossbeam-epoch
0.9.18`, reachable only via `criterion` (dev-dependency). Run
`cargo update -p crossbeam-epoch` to pull ≥0.9.20; commit `Cargo.lock` as
`chore(deps): update crossbeam-epoch past RUSTSEC-2026-0204`.

**Verify**: `grep -A2 'name = "crossbeam-epoch"' Cargo.lock` → version ≥ 0.9.20;
`cargo test` → passes.

## Test plan

No new tests. The plan's deliverable is that the existing suite becomes
enforced. Sanity: after Step 3, deliberately introduce a formatting error in a
scratch commit, confirm `cargo fmt --check` fails, then drop the commit.

## Done criteria

- [ ] `cargo fmt --check` exits 0
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0
- [ ] `cargo test` exits 0
- [ ] `.github/workflows/ci.yml` exists and includes fmt, clippy, build, test
- [ ] No behavioral diffs: `cargo test` pass/fail set identical before/after
      Steps 1-2
- [ ] `plans/README.md` status row updated

## STOP conditions

- `.github/` already exists at execution time.
- Any clippy fix changes test results.
- `cargo fmt` produces a diff so large it suggests a non-default rustfmt was
  previously in use (>2000 changed lines) — report before committing.

## Maintenance notes

- Once merged, enable branch protection requiring the `test` job on `main`
  (repo-settings change, human task — outside executor scope).
- Future plans (005, 006, 009, 012) rely on this gate; if CI is red, fix CI
  first.
- Deferred: MSRV job (added by plan 012 when `rust-version` is declared);
  benchmark regression tracking (criterion output is not CI-gated).
