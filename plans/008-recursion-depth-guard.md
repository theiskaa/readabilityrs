# Plan 008: Bound DOM recursion depth (stack-overflow DoS on hostile input)

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- src/content_extractor.rs src/markdown/converter.rs src/metadata.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: LOW-MED
- **Depends on**: plans/002-asserting-mozilla-test-suite.md (recommended)
- **Category**: bug (DoS hardening)
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

Three tree traversals recurse one stack frame per DOM nesting level with no
bound: the HTML serializer `element_to_html`, the markdown converter, and a
byline text collector. html5ever (via `scraper`) happily builds a tree from
tens of thousands of nested `<div>`s, so a hostile page can drive recursion
until the stack overflows — which in Rust **aborts the process** (not
catchable). For a library embedded in servers/crawlers ingesting untrusted
pages, that's a denial of service. The fix is a depth guard: beyond a
generous limit (real pages nest well under 100 levels), stop descending and
degrade gracefully instead of dying.

## Current state

Recursive sites (verified at `c7622fd`):

1. `src/content_extractor.rs:901-951` — `element_to_html(element:
   ElementRef) -> String`; recurses at `:934`
   (`let child_html = element_to_html(child_elem);`). Called from `:651` and
   `:692`. NOTE: plans 004 and 005 also touch this function — if they landed,
   the signature may already carry extra parameters; the guard composes with
   both (add the depth parameter alongside).
2. `src/markdown/converter.rs` — `convert_children` (~line 33) and
   `convert_element` (~line 65) are mutually recursive over the DOM.
3. `src/metadata.rs:856-888` — `build_byline_text` with inner
   `fn append_children_text(element: &ElementRef, out: &mut String)`
   recursing at `:877`.

There is no existing depth constant, and `ReadabilityOptions` has
`max_elems_to_parse` (breadth guard, `src/options.rs:77`) but nothing for
depth.

Repo conventions: shared constants live in `src/constants.rs` (bitflags +
`Lazy` statics). Unit tests per-module in `#[cfg(test)] mod tests`.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Build   | `cargo build` | exit 0 |
| Tests   | `cargo test` | all pass |
| Overflow repro (before fix) | `cargo test --lib depth -- --nocapture` (the new test) | see Step 1 |

## Scope

**In scope**:
- `src/constants.rs` — one new `pub const MAX_DOM_DEPTH: usize = 512;`
- `src/content_extractor.rs`, `src/markdown/converter.rs`,
  `src/metadata.rs` — threading a `depth: usize` parameter and the guard
- Unit tests in the touched modules

**Out of scope**:
- Making the limit configurable via `ReadabilityOptions` (defer until a user
  asks; note in maintenance).
- Iterative rewrites of the traversals — a depth guard is sufficient and far
  lower-risk; do NOT convert to explicit-stack iteration in this plan.
- Any other recursive function not listed (report extras you notice).

## Git workflow

- Branch: `bugfix/dom-recursion-depth-guard`
- Commit style: `fix(extractor): bound DOM recursion depth to prevent stack
  overflow on hostile input`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Write the failing repro first

In `src/content_extractor.rs` tests, add (temporarily `#[ignore]`d until the
fix is in):

```rust
#[test]
fn test_deeply_nested_html_does_not_overflow() {
    let depth = 50_000;
    let mut html = String::from("<html><body><article>");
    for _ in 0..depth { html.push_str("<div>"); }
    html.push_str("<p>Substantial paragraph text long enough to be scored as content. Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt.</p>");
    for _ in 0..depth { html.push_str("</div>"); }
    html.push_str("</article></body></html>");
    let r = crate::readability::Readability::new(&html, None, None).unwrap();
    let _ = r.parse(); // must return (Some or None), not abort
}
```

Adjust the constructor call to the crate's actual internal API if needed
(this is a lib test — check how other tests in the file construct documents;
`test_attribute_values_are_escaped` at `:972` is the model). Run it WITHOUT
`#[ignore]` once to confirm it aborts or overflows (run under
`cargo test -- test_deeply_nested` and observe the crash), then re-add
`#[ignore]` while you implement. If it does NOT crash at 50k, try 200k; if it
still doesn't crash, record the observation and continue — the guard is still
correct defense-in-depth, but say so in your report.

**Verify**: crash reproduced (or explicitly recorded as not reproducible at
200k on this platform).

### Step 2: Add the constant

`src/constants.rs`:

```rust
/// Maximum DOM depth traversed by recursive walkers. Real-world pages nest
/// well under 100 levels; beyond this we stop descending instead of
/// overflowing the stack on adversarial input.
pub const MAX_DOM_DEPTH: usize = 512;
```

**Verify**: `cargo build` → exit 0.

### Step 3: Guard `element_to_html`

Add a `depth: usize` parameter (alongside whatever parameters plans 004/005
added, if landed). At function entry:
`if depth > crate::constants::MAX_DOM_DEPTH { return String::new(); }` —
consistent with the existing early-return for invisible elements at `:903`.
Recursion passes `depth + 1`; the two top-level call sites (`:651`, `:692`)
pass `0`.

**Verify**: `cargo build` → exit 0.

### Step 4: Guard the markdown converter

Same pattern in `src/markdown/converter.rs`: thread `depth` through
`convert_element`/`convert_children` (read the file first — `ConversionState`
in `src/markdown/state.rs` may be a cleaner carrier for the depth counter if
it's already threaded through every call; choose whichever touches fewer
signatures and say which you chose). Beyond the limit, emit nothing for that
subtree.

**Verify**: `cargo build` → exit 0; `cargo test --test markdown_tests` → all
117 pass.

### Step 5: Guard `append_children_text`

`src/metadata.rs:857` — add the depth parameter to the inner function;
beyond the limit, return without appending.

**Verify**: `cargo build` → exit 0; `cargo test --lib metadata` → pass.

### Step 6: Activate the repro

Remove `#[ignore]` from Step 1's test. Add a companion test asserting
CONTENT SURVIVES moderate nesting (e.g. 100 levels): parse succeeds and the
paragraph text appears in `article.content` — this pins the limit being
generous, not just present.

**Verify**: `cargo test` → all pass including both new tests. The deep test
should complete in seconds; if it takes minutes, note it (html5ever parse
time, not our recursion — acceptable but worth recording).

## Test plan

Step 1 (abort repro turned regression test) + Step 6 (moderate-nesting
survival) + full existing suites. The Mozilla corpus (plan 002) guards
against the limit accidentally truncating real pages.

## Done criteria

- [ ] `cargo test` exits 0, including the 50k-nesting test
- [ ] `grep -n "MAX_DOM_DEPTH" src/` shows the constant used in all three
      modules
- [ ] Mozilla + markdown suites unchanged vs baseline
- [ ] `git status` shows only in-scope files modified
- [ ] `plans/README.md` status row updated

## STOP conditions

- Threading `depth` through the markdown converter requires touching more
  than ~6 function signatures — report; the `ConversionState` carrier is
  probably the right move and may deserve its own micro-plan.
- Any Mozilla-suite case changes result (a real page hit the limit — the
  limit is too low; report before raising it).
- The repro crashes even WITH the guard (the overflow is inside
  html5ever/scraper parsing itself, not our walkers) — that's an upstream
  issue this plan cannot fix; report with the stack trace.

## Maintenance notes

- If a legitimate page ever exceeds 512 levels, raising the constant is a
  one-line change; making it a `ReadabilityOptions` field is the deferred
  follow-up.
- Plan 009 (single-parse pipeline) may replace these walkers; the guard must
  survive that refactor — it's cheap insurance either way.
- Reviewers: check the guard returns EMPTY (drop subtree) rather than
  panicking or truncating mid-tag — partial output must stay well-formed.
