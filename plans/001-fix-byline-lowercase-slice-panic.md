# Plan 001: Fix reachable panic in byline datetime stripping (byte index from lowercased string)

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- src/utils.rs`
> If the file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

`strip_trailing_datetime_clause` in `src/utils.rs` computes a byte index on a
**lowercased copy** of the byline text and then uses that index to slice the
**original** string. `String::to_lowercase()` can change byte length (e.g. the
Turkish dotted capital I, `İ` U+0130, is 2 bytes and lowercases to `i̇` which is
3 bytes), so the index can land on a non-char-boundary of the original string —
or past its end. Both cases panic. This function runs on byline text extracted
from untrusted web pages during `Readability::parse()`, so a crafted (or merely
Turkish) page can abort any process embedding this library. This is a
remotely-triggerable denial of service for library consumers.

## Current state

- `src/utils.rs` — byline cleaning helpers. The buggy function, as of `c7622fd`:

```rust
// src/utils.rs:285-301
fn strip_trailing_datetime_clause<'a>(text: &'a str, allow_strip: bool) -> Cow<'a, str> {
    if !allow_strip {
        return Cow::Borrowed(text);
    }

    let lower = text.to_lowercase();
    for separator in [" | ", " - ", " – ", " — ", " · "] {
        if let Some(idx) = lower.rfind(separator) {
            let tail = lower[idx + separator.len()..].trim();
            if looks_like_datetime_segment(tail) {
                return Cow::Owned(text[..idx].trim_end().to_string());   // <-- BUG: idx is an offset into `lower`, not `text`
            }
        }
    }

    Cow::Borrowed(text)
}
```

- Call chain (all in-repo, verified): `clean_byline_text` → `clean_byline_text_with_reason`
  (`src/utils.rs`, calls `strip_trailing_datetime_clause(&canonical, has_author_segment)`
  around `src/utils.rs:603`) → invoked on DOM-extracted byline text at
  `src/metadata.rs:422` inside `get_article_metadata`, which runs on every
  `Readability::parse()`.
- Note that the `tail` computation (`lower[idx + separator.len()..]`) slices
  `lower` with `lower`'s own index — that part is fine. Only the `text[..idx]`
  slice mixes coordinate systems.
- Repo conventions: helper functions in `utils.rs` are private `fn`s with
  `#[cfg(test)] mod tests` at the bottom of the same file. Unit tests use plain
  `assert_eq!` on the helper directly (see the existing tests at the bottom of
  `src/utils.rs`). Match that.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Build   | `cargo build` | exit 0 |
| Unit tests | `cargo test --lib` | all pass |
| Full tests | `cargo test` | all pass (154 lib + 117 markdown + doctests as of planning) |

## Scope

**In scope** (the only files you should modify):
- `src/utils.rs`

**Out of scope** (do NOT touch):
- `src/metadata.rs` — the caller is correct; the fix is local to the helper.
- Any other `to_lowercase` sites in the repo (fix only this one; if you spot
  the same pattern elsewhere, note it in your report instead of changing it).

## Git workflow

- Branch: `bugfix/byline-lowercase-slice-panic` (repo uses `bugfix/…`,
  `feature/…`, `refactor/…` branch prefixes).
- Commit style: conventional commits, e.g. `fix(utils): avoid slicing original
  string with lowercased-string index` (matches history like
  `fix(content_extractor): escape attribute values to prevent parsing issues`).
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Rewrite the function to search the original string

Replace the body so separators are located in the ORIGINAL `text`, walking
match positions from the right. All five separators are pure ASCII-or-fixed
strings that are unaffected by lowercasing, so searching `text` directly finds
the same separator positions; only the datetime-tail *predicate* needs
lowercased input, and `looks_like_datetime_segment` can be fed a lowercased
copy of the tail alone. Target shape:

```rust
fn strip_trailing_datetime_clause<'a>(text: &'a str, allow_strip: bool) -> Cow<'a, str> {
    if !allow_strip {
        return Cow::Borrowed(text);
    }

    for separator in [" | ", " - ", " – ", " — ", " · "] {
        if let Some(idx) = text.rfind(separator) {
            let tail = text[idx + separator.len()..].trim().to_lowercase();
            if looks_like_datetime_segment(&tail) {
                return Cow::Owned(text[..idx].trim_end().to_string());
            }
        }
    }

    Cow::Borrowed(text)
}
```

Check the signature of `looks_like_datetime_segment` (defined just above in
the same file): it takes `&str`. It internally lowercases or inspects chars —
read it before deciding whether the `.to_lowercase()` on the tail is needed.
If it already lowercases internally, pass the tail without lowercasing.

**Verify**: `cargo build` → exit 0.

### Step 2: Add regression tests

In the existing `#[cfg(test)] mod tests` in `src/utils.rs`, add tests that call
`clean_byline_text` (the public-to-the-crate entry) or
`strip_trailing_datetime_clause` directly:

1. A byline containing `İ` before a separator followed by a datetime tail,
   e.g. `"DİLARA ŞENKAYA | 12:30, 5 May 2024"` — must not panic, and must
   strip the datetime clause.
2. A byline where the lowercased form is longer than the original and the
   datetime tail check FAILS (so no strip happens) — must not panic and must
   return the input unchanged, e.g. `"İstanbul Correspondent | Senior Editor"`.
3. An ASCII behavior-preservation case, e.g. `"Jane Doe | 08:15, 3 June 2024"`
   strips to `"Jane Doe"` (adjust expected values to what
   `looks_like_datetime_segment` actually accepts — read it and pick a tail it
   returns `true` for).

**Verify**: `cargo test --lib utils` → new tests pass.

## Test plan

Covered by Step 2: panic-repro case, no-strip Unicode case, ASCII regression
case. Model the tests after the existing tests at the bottom of
`src/utils.rs`.

## Done criteria

- [ ] `cargo build` exits 0
- [ ] `cargo test` exits 0, including ≥3 new tests in `src/utils.rs`
- [ ] The expression `text[..idx]` where `idx` derives from `lower.rfind` no
      longer exists: `grep -n "lower.rfind" src/utils.rs` returns nothing
      inside `strip_trailing_datetime_clause`
- [ ] `git status` shows only `src/utils.rs` (and `plans/README.md`) modified

## STOP conditions

Stop and report back (do not improvise) if:

- The code at `src/utils.rs:285-301` doesn't match the excerpt above.
- `looks_like_datetime_segment` turns out to be case-sensitive in a way that
  makes the rewrite change ASCII behavior (existing utils tests fail).
- Fixing this requires touching any file other than `src/utils.rs`.

## Maintenance notes

- The same "index from transformed string used on original" hazard applies to
  any future `to_lowercase()` + `find`/`rfind` combination. Reviewers should
  scan for that pattern in new code.
- Deferred: a repo-wide sweep for the same pattern (none found during the
  audit outside this site, but the audit was standard-depth, not exhaustive).
