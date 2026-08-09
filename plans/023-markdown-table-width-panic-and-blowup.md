# Plan 023: Fix markdown table column widths — panic above 64 KiB and ~265× output amplification

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on. On
> any STOP condition, stop and report. When done, update this plan's row in
> `plans/README.md` — unless a reviewer dispatched you and said they maintain
> the index.
>
> **Drift check (run first)**: `git diff --stat 4430e24..HEAD -- src/markdown/rules/tables.rs`

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW (output formatting only, markdown path)
- **Depends on**: none
- **Category**: bug (DoS)
- **Planned at**: commit `4430e24`, 2026-08-04 (found by post-merge bug hunt, reproduced)

## Why this matters

Markdown table rendering pads every cell to the width of the widest cell in
its column. Two failure modes, both reproduced at `4430e24` with
`output_markdown(true)`:

1. **Panic.** `src/markdown/rules/tables.rs:74` and `:90` use the computed
   width as a runtime format width (`format!(" {:<width$} |", …)`). Rust's
   formatting machinery caps width at `u16::MAX`; beyond that it panics with
   *"Formatting argument out of range"*. Measured: a 65,530-byte cell →
   `Ok`; a 65,540-byte cell → **panic** at `tables.rs:74`. This is a library,
   so the panic unwinds into caller code.
2. **Memory amplification.** Every cell in the column is padded to the widest
   cell's length. Measured: **input 69,731 bytes → `markdown_content`
   18,183,449 bytes** (~265×) from one 60,000-byte cell plus 300 trivial
   rows. Scaling rows scales the blow-up linearly; a few MB of input yields
   gigabytes.

Both are reachable from any crawled page containing a large table cell — no
crafting required beyond a big cell, which real pages (embedded base64, long
code samples, pasted logs) do contain.

## Current state

- `src/markdown/rules/tables.rs:57-65` — `col_widths` computed as the max
  `escape_pipe(cell).len()` per column (byte length, unbounded).
- `:74` — header cells: `format!(" {:<width$} |", escape_pipe(h), width = w)`
- `:81` — separator row: `"-".repeat(*w)`
- `:90` — data cells: same `{:<width$}` pattern
- Indexing itself is sound (`num_cols` is the max of header/row lengths and
  the row loop guards `i < num_cols`) — the defect is the *value* of `w`, not
  an out-of-bounds access.
- Tests for the markdown path live in `tests/markdown_tests.rs` (117 tests)
  and in `#[cfg(test)]` modules under `src/markdown/`.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Build | `cargo build` | exit 0 |
| Markdown tests | `cargo test --test markdown_tests` | all pass |
| Full | `cargo test` | all pass |
| Lint/format | `cargo clippy --all-targets -- -D warnings` && `cargo fmt --check` | exit 0 |

## Scope

**In scope**: `src/markdown/rules/tables.rs` and its tests; `tests/markdown_tests.rs`
only if an existing expectation depends on exact padding.

**Out of scope**: other markdown rules; the escaping/injection issues in
links and images (tracked separately); the standardization pipeline.

## Git workflow

- Branch: `bugfix/markdown-table-width`
- Commit: `fix(markdown): bound table column widths to prevent panic and output blowup`
- Do NOT push or open a PR.

## Steps

### Step 1: Write the failing tests first

In `src/markdown/rules/tables.rs`'s test module (or
`tests/markdown_tests.rs` if that is where table tests live — check both and
follow the existing convention):

1. **Panic repro**: a 2-column table whose one data cell is 65,540 `'A'`s,
   converted via the markdown path. Must not panic.
2. **Amplification repro**: a table with one 60,000-byte cell plus ~300
   trivial rows. Assert the output length is bounded — e.g. no more than ~3×
   the input length. Pick the exact bound after Step 2 so it reflects the
   implemented behaviour, but it must be a small constant factor, not 265×.

Run them and confirm #1 panics and #2 blows up before the fix. Report the
observed numbers.

**Verify**: test 1 panics; test 2's output length is enormous.

### Step 2: Bound the column width

Add a cap and clamp each column width to it:

```rust
/// Upper bound on markdown table column padding. Pipe tables do not require
/// aligned columns to be valid; padding beyond this only inflates output and,
/// past u16::MAX, panics the formatting machinery.
const MAX_COL_WIDTH: usize = 200;
```

Clamp when computing `col_widths` (`:57-65`) with `.min(MAX_COL_WIDTH)`.
Cells longer than the cap are simply not padded — emit them at natural
length. Do NOT truncate cell content; that would lose data. The `{:<width$}`
format only pads, never truncates, so a longer cell passes through intact
once the width is clamped.

This fixes both defects at once: the width can never reach `u16::MAX`, and
padding can never exceed 200 bytes per cell.

**Verify**: `cargo build` → exit 0; both Step 1 tests pass.

### Step 3: Confirm rendering is still correct

Markdown pipe tables are valid without alignment padding. Add a test that a
small ordinary table still renders with its previous shape (header row,
separator row with at least three dashes per column, data rows, pipes in the
right places), so the cap doesn't change everyday output.

Check that the separator row (`"-".repeat(*w)`) still emits at least 3 dashes
per column — the markdown minimum — even for very narrow columns. If the
existing code can emit fewer than 3 for a 1-character column, fix that with a
`.max(3)` and note it.

**Verify**: `cargo test --test markdown_tests` → all 117 pass.

### Step 4: Regression

**Verify**: `cargo test` → all pass. If any existing markdown expectation
encoded the old unbounded padding, update it and list the changes in your
report.

## Test plan

Steps 1 and 3. Existing 117 markdown tests are the regression gate.

## Done criteria

- [ ] A 65,540-byte table cell converts without panicking
- [ ] Output length for the amplification case is within a small constant
      factor of input (state the measured ratio before and after)
- [ ] Ordinary tables render unchanged in shape; separator rows have ≥3
      dashes
- [ ] `cargo test` exits 0
- [ ] `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` exit 0
- [ ] `plans/README.md` status row updated

## STOP conditions

- More than 5 existing markdown expectations change — report before editing;
  that suggests tests are asserting exact padding widely and the cap value
  needs discussion.
- Clamping alone does not stop the panic (i.e. a panic persists) — report the
  backtrace; the width may be reaching the formatter by another path.

## Maintenance notes

- Any future use of a runtime-computed `{:width$}` / `{:.prec$}` on
  attacker-influenced data has this same `u16::MAX` hazard. Reviewers should
  treat computed format widths as untrusted-input-derived.
- Dropping alignment padding altogether (emitting `| a | b |`) would be
  simpler still and equally valid markdown — a reasonable follow-up if the
  padding is not valued.
