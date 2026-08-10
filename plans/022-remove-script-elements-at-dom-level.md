# Plan 022: Remove `<script>`/`<style>` at the DOM level — the regex misses `</script >`

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on. On
> any STOP condition, stop and report. When done, update this plan's row in
> `plans/README.md` — unless a reviewer dispatched you and said they maintain
> the index.
>
> **Drift check (run first)**: `git diff --stat 4430e24..HEAD -- src/cleaner.rs src/content_extractor.rs`

## Status

- **Priority**: P0 — security; defeats a shipped mitigation
- **Effort**: M
- **Risk**: MED
- **Depends on**: none (builds on the `sanitize_content` option added by plan 004)
- **Category**: security
- **Planned at**: commit `4430e24`, 2026-08-04 (found by post-merge bug hunt, reproduced)

## Why this matters

`<script>` elements survive into `article.content`, **including when
`sanitize_content(true)` is set**. Reproduced at `4430e24`:

```
input:  <article>…4 paragraphs…<script>alert(1)</script >\n</article>
output (default):        contains "<script" = true, contains "alert(1)" = true
output (sanitize=true):  contains "<script" = true, contains "alert(1)" = true
```

A single space before the `>` of the closing tag defeats script removal
entirely. The HTML tokenizer accepts `</script >`, `</script\n>` and
`</script\t>` as valid end tags; the removal regex does not.

Two independent defects combine:

1. **`src/cleaner.rs:451`** — script removal is a regex,
   `(?i)<script\b[^>]*>[\s\S]*?</script>`, requiring the literal `</script>`
   with no whitespace. It runs on raw text before parsing.
2. **`src/content_extractor.rs`** — `element_to_html` serializes whatever
   element it reaches, and the `sanitize` branch added by plan 004 only drops
   `on*` attributes and dangerous `href`/`src` values. It never drops a whole
   element. Script *text* is escaped, but ordinary JavaScript contains no
   `&`, `<` or `>`, so it round-trips intact.

This directly undermines plan 004: the option is documented as a partial
mitigation for consumers rendering `article.content`, and a `<script>` block
is the most severe thing it should have stopped.

## Current state

- `src/cleaner.rs:451` — the script regex, inside `prep_document`, alongside
  five other regexes compiled there (`:451-475`).
- `src/content_extractor.rs` — `element_to_html(element, sanitize)`:
  - `:894-899` `is_event_handler_attr`
  - `:909-939` `is_dangerous_url`
  - `:971-979` the `if sanitize { … }` attribute-filtering branch
  - `:990-1008` the recursive child walk that serializes every element
  - `:1001` text escaping; `:1004` comment emission (`<!--{}-->`, body verbatim)
- Plan 004's option lives at `src/options.rs` (`sanitize_content: bool`,
  default `false`) and is read at `src/content_extractor.rs:651` and `:692`.
- The DOM removal idiom to follow is `clean_conditionally_tag` in
  `src/cleaner.rs` — two-phase: collect `NodeId`s under an immutable borrow,
  then `detach()` under a mutable borrow.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Build | `cargo build` | exit 0 |
| Tests | `cargo test` | all pass |
| Mozilla suite | `cargo test --test mozilla_test_suite` | passes; divergence lists unchanged |
| Lint/format | `cargo clippy --all-targets -- -D warnings` && `cargo fmt --check` | exit 0 |

## Scope

**In scope**:
- `src/cleaner.rs` — replace regex script/style removal with a DOM pass
- `src/content_extractor.rs` — element denylist in `element_to_html`
- Tests in both

**Out of scope**:
- The other regex removers (plan 006 owns those).
- Changing the default value of `sanitize_content` — it stays `false`.
- Adding a sanitizer dependency.

## Git workflow

- Branch: `bugfix/script-survives-sanitization`
- Commits (single-line, no body, no trailer):
  1. `fix(cleaner): remove script and style elements on the parsed DOM`
  2. `fix(extractor): drop unsafe elements when sanitize_content is enabled`
- Do NOT push or open a PR.

## Steps

### Step 1: Write the failing tests first

Add tests covering all the end-tag spellings the tokenizer accepts. In
`src/content_extractor.rs` tests (model on
`test_sanitize_content_strips_event_handlers_and_dangerous_schemes`), build a
full article page with 4 substantial paragraphs plus a script, once per
variant:

- `</script>` (baseline — may already pass)
- `</script >` (space)
- `</script\n>` (newline)
- `</script\t>` (tab)
- uppercase `</SCRIPT >`

For each, assert `article.content` contains neither `<script` nor the script
body — with default options AND with `sanitize_content(true)`.

Run and record which variants fail before the fix.

**Verify**: `cargo test --lib content_extractor` → the whitespace variants
fail.

### Step 2: Remove script/style on the DOM

In `src/cleaner.rs`, replace the regex-based script removal with a DOM pass
that runs after parsing. Detach every `script`, `style`, `noscript` and
`template` element using the two-phase collect-then-detach idiom from
`clean_conditionally_tag`.

Careful: `prep_document` currently operates on a `&str` before the main
parse. Read it and decide the cleanest insertion point — either parse there
and return serialized output, or move script removal into the existing DOM
cleaning stage (`remove_conditionally_dom`). Prefer whichever adds NO
additional full parse of the document. State which you chose and why.

NOTE on `<noscript>`: the repo deliberately unwraps `<noscript>` around
lazy-loaded images (see the README's extraction description). Removing
`noscript` outright may regress image extraction. Check whether that
unwrapping happens BEFORE your new removal; if it does not, exclude
`noscript` from the denylist and say so.

**Verify**: `cargo build` → exit 0; Step 1's tests pass for the default path.

### Step 3: Element denylist in the serializer

In `element_to_html`, when `sanitize` is true, return `String::new()` for a
denylist of tag names — at minimum `script`, `style`, `iframe`, `object`,
`embed`, `form`, `noscript`, `template`. This is defence in depth: even if a
future cleaning path misses one, the sanitized serializer never emits it.

Put the predicate in a small private `fn is_unsafe_element(tag: &str) -> bool`
next to the existing `is_event_handler_attr` / `is_dangerous_url`, so it is
unit-testable. Match case-insensitively (html5ever lowercases HTML tag names,
but do not rely on it for foreign content).

Also handle comments: `:1004` emits `<!--{}-->` with the body verbatim, so a
comment containing `-->` can break out. When `sanitize` is true, either skip
comments entirely or strip `-->` from the body. Skipping is simpler and
loses nothing.

**Verify**: `cargo build` → exit 0; Step 1's tests pass for the
`sanitize_content(true)` path too.

### Step 4: Unit-test the predicate directly

Add tests for `is_unsafe_element`: each denylisted tag returns true; `p`,
`div`, `article`, `img`, `a` return false; matching is case-insensitive.

**Verify**: `cargo test --lib content_extractor` → all pass.

### Step 5: Regression

**Verify**: `cargo test` → all pass. `cargo test --test mozilla_test_suite`
→ passes with divergence lists NOT grown. Removing `script`/`style` should
never reduce extracted article text; if a corpus case regresses, STOP and
report which.

## Test plan

Steps 1 and 4. The Mozilla corpus is the regression gate. Net-new: 5 end-tag
spelling variants × 2 option settings, plus direct predicate tests.

## Done criteria

- [ ] `<script>` with `</script>`, `</script >`, `</script\n>`, `</script\t>`
      and `</SCRIPT >` is absent from `article.content` under BOTH default and
      `sanitize_content(true)`
- [ ] Script removal happens on the parsed DOM, not by regex on raw text
- [ ] `is_unsafe_element` exists, is unit-tested, and is applied when
      `sanitize` is true
- [ ] Comments cannot break out of `<!-- -->` when `sanitize` is true
- [ ] `cargo test` exits 0; Mozilla divergence lists unchanged
- [ ] `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` exit 0
- [ ] `plans/README.md` status row updated

## STOP conditions

- Removing `noscript` regresses lazy-image extraction (see the Step 2 note) —
  exclude it and report.
- Any Mozilla case loses article text after DOM-level script/style removal.
- Moving script removal into the DOM stage would require adding a full extra
  parse of the document — report; that trade-off is the maintainer's call and
  interacts with plan 009.

## Maintenance notes

- After this, `sanitize_content(true)` means: no unsafe elements, no `on*`
  handlers, no dangerous URL schemes. Update the option's doc comment in
  `src/options.rs` and the Security sections in `README.md` / `src/lib.rs` to
  say so — they currently describe only attribute-level filtering.
- It is still NOT a full sanitizer. Keep the `ammonia` recommendation.
- The general lesson matches plan 021: removing HTML elements with regex is
  unsound. Every remaining regex remover (plan 006) is a latent instance of
  this same bug.
