# Plan 013: Dependency hygiene — v_htmlescape pin, thiserror 2.x, dev-advisory bump

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on. On
> any STOP condition, stop and report. When done, update this plan's row in
> `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- Cargo.toml Cargo.lock src/error.rs`
> On drift, re-verify the excerpts below before proceeding.

## Status

- **Priority**: P3
- **Effort**: S
- **Risk**: LOW-MED (escaping swap has output-correctness stakes; tests cover)
- **Depends on**: none (plan 003's CI helps)
- **Category**: migration
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

Three small hygiene items: (1) `v_htmlescape = "=0.15.8"` (`Cargo.toml:23`)
is exact-pinned with no recorded reason — the introducing commit `b434758`
("Add escape the text") added pin and usage together, no comment; an
undocumented `=` pin blocks even patch updates and confuses maintainers.
(2) `thiserror = "1.0"` (`Cargo.toml:19`) has a 2.x line; usage is a single
derive on one enum (`src/error.rs`), so migration is near-free. (3) `cargo
audit` flags RUSTSEC-2026-0204 in `crossbeam-epoch 0.9.18`, reachable only
through `criterion` (dev-dep) — a lockfile bump clears the audit noise (skip
this item if plan 003 Step 5 already did it).

## Current state

- `Cargo.toml:19` `thiserror = "1.0"`; `Cargo.toml:23`
  `v_htmlescape = "=0.15.8"`.
- `v_htmlescape` is used in exactly two call sites, both in
  `src/content_extractor.rs`: attribute values (`:920`, `escape(value)`) and
  text nodes (`:941`, `escape(&text.text)`). Regression tests exist:
  `test_attribute_values_are_escaped` (`src/content_extractor.rs:972`) and a
  `test_html_escape`-style test near `:1114`.
- `thiserror` is used only in `src/error.rs` (one enum, `ReadabilityError`,
  `#[derive(Error, ...)]` with `#[error("...")]` attributes).
- `Cargo.lock` resolves crossbeam-epoch 0.9.18 via
  criterion → rayon → rayon-core → crossbeam-deque → crossbeam-epoch.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Tests   | `cargo test` | all pass |
| Audit (if installed) | `cargo audit` | no advisories after Step 3 |
| Build   | `cargo build` | exit 0 |

## Scope

**In scope**: `Cargo.toml`, `Cargo.lock`, `src/error.rs` (only if thiserror
2.x needs attribute tweaks).

**Out of scope**: replacing `v_htmlescape` with a different escaping
implementation (investigated below; only do the RELAX, not a swap — a swap
is its own decision); bumping `scraper`/`ego-tree`/`regex` (no findings
against them).

## Git workflow

- Branch: `chore/dependency-hygiene`
- One commit per item; style `chore(deps): …`.
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Investigate and relax the v_htmlescape pin

1. Check for a stated reason: `git log --all --grep="v_htmlescape"` and
   `git show b434758` — read the diff/message for any hint the pin was
   deliberate (e.g. a regression in a later version).
2. Check what newer 0.15.x versions exist: `cargo add v_htmlescape --dry-run`
   or crates.io. At planning, 0.15.8 was current-ish; the pin may be
   vacuous today.
3. If no documented reason exists: change to `v_htmlescape = "0.15"`, run
   `cargo update -p v_htmlescape`, and rely on the two escape regression
   tests.
4. Record the outcome in the commit message ("pin had no recorded rationale;
   relaxed to caret; escape tests green").

**Verify**: `cargo test` → all pass, especially
`cargo test --lib content_extractor` escape tests.

### Step 2: thiserror 1.x → 2.x

Change `Cargo.toml` to `thiserror = "2.0"`, `cargo build`. thiserror 2.0's
breaking changes rarely affect simple `#[error("…")]` enums; if compile
errors appear in `src/error.rs`, apply the compiler's guidance (typical: none
needed). 

**Verify**: `cargo build` → exit 0; `cargo test` → pass;
`grep -A1 'name = "thiserror"' Cargo.lock` → 2.x.

### Step 3: Clear the dev-only advisory (skip if already done by plan 003)

`grep -A2 'name = "crossbeam-epoch"' Cargo.lock` — if < 0.9.20, run
`cargo update -p crossbeam-epoch`.

**Verify**: version ≥ 0.9.20 in Cargo.lock; `cargo test` → pass;
`cargo audit` (if installed) → clean.

## Test plan

No new tests. The two existing escape regression tests are the safety net
for Step 1; the full suite gates all three steps.

## Done criteria

- [ ] `grep -n "v_htmlescape" Cargo.toml` → shows `"0.15"` (caret), OR the
      exact pin retained WITH a `# pinned because: …` comment if Step 1
      found a real reason
- [ ] thiserror resolves to 2.x
- [ ] crossbeam-epoch ≥ 0.9.20
- [ ] `cargo test` exits 0
- [ ] `plans/README.md` status row updated

## STOP conditions

- Step 1 investigation reveals the pin WAS deliberate (regression in newer
  versions) — keep the pin, add the explanatory comment, report.
- thiserror 2.x migration requires anything beyond `src/error.rs` — report;
  something unusual is depending on 1.x internals.
- Escape tests fail after relaxing the pin — restore the pin, add the
  comment documenting the incompatibility, report which version broke.

## Maintenance notes

- Policy going forward: exact pins (`=x.y.z`) require an adjacent comment
  stating why. Reviewers enforce.
- Deferred deliberately: replacing v_htmlescape entirely (two call sites; a
  hand-rolled escaper or `html-escape` crate would work) — only worth it if
  the crate goes unmaintained AND an incompatibility appears.
