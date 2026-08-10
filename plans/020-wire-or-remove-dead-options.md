# Plan 020: Wire up or remove the four public options the library never reads

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on. On
> any STOP condition, stop and report. When done, update this plan's row in
> `plans/README.md` — unless a reviewer dispatched you and said they maintain
> the index.
>
> **Drift check (run first)**: `git diff --stat 4430e24..HEAD -- src/options.rs src/error.rs src/lib.rs src/content_extractor.rs`
> On drift, re-verify the excerpts below before proceeding.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED (touches the public API of a published crate)
- **Depends on**: none
- **Category**: bug / api
- **Planned at**: commit `4430e24`, 2026-08-04 (found by post-merge quality review)

## Why this matters

`readabilityrs` is published on crates.io. Four options on the public
`ReadabilityOptions` builder are declared, documented, and settable — and are
never read by any code path. Verified at `4430e24` with
`grep -rn "<option>" src/`:

| Option | References outside `src/options.rs` |
|---|---|
| `max_elems_to_parse` | only `src/error.rs:120` (a doc comment) and `:131` (a doctest) |
| `classes_to_preserve` | **none** |
| `keep_classes` | only `src/lib.rs:54` (a doctest that advertises it) |
| `allowed_video_regex` | **none** |

For contrast, `sanitize_content` — added recently — IS wired, read at
`src/content_extractor.rs:651` and `:692`. That is what a live option looks
like.

The worst of the four is `max_elems_to_parse`. `src/error.rs:120` documents it
as "a safety mechanism to prevent processing" oversized documents, and
`ReadabilityError::MaxElementsExceeded` exists as a variant. A user parsing
untrusted HTML who sets `max_elems_to_parse(10_000)` reasonably believes they
have bounded the work done on a hostile page. They have not: the value is
stored and ignored, no element counting happens anywhere, and
`MaxElementsExceeded` is never constructed. That is a false security
assurance in a library whose entire input is untrusted.

Separately, 6 of the 7 `ReadabilityError` variants are never constructed
anywhere in `src/` (`ParseError`, `InvalidDocument`, `JsonLdError`,
`MaxElementsExceeded`, `NoContentFound`, `Other`), so consumers who `match` on
them write unreachable arms.

## Current state

- `src/options.rs` — `ReadabilityOptions` struct (~line 60) with the four
  fields; `ReadabilityOptionsBuilder` (~line 240) with a setter for each;
  `build()` (~line 358) copies them across. All present and functional as
  *storage*.
- `src/error.rs:120-131` — doc comment + doctest for `MaxElementsExceeded`
  referencing `max_elems_to_parse`; the doctest ends with the comment
  `// Would trigger MaxElementsExceeded if implemented`, which is an
  admission in published documentation.
- `src/lib.rs:54` — crate-level doctest calls `.keep_classes(true)`.
- `src/content_extractor.rs` — `grab_article(document, options)` is the entry
  point and already receives `&ReadabilityOptions`; `try_extract_with_flags`
  and `extract_article_content` thread it further. This is where an element
  cap would go.
- Repo conventions: options are plain fields + a builder setter; `///` docs on
  public items; `thiserror` for errors; no `unwrap()`/`expect()` in library
  code.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Build | `cargo build` | exit 0 |
| Tests | `cargo test` | all pass |
| Lint | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Format | `cargo fmt --check` | exit 0 |
| Docs | `cargo doc --no-deps` | exit 0 |

## Scope

**In scope**:
- `src/content_extractor.rs` — implement the `max_elems_to_parse` guard
- `src/options.rs` — doc corrections; deprecation attributes
- `src/error.rs` — remove the "if implemented" comment; construct
  `MaxElementsExceeded`
- `src/lib.rs` — fix the doctest if `keep_classes` is deprecated
- `README.md` — only if it documents a deprecated option

**Out of scope**:
- Implementing `classes_to_preserve` / `keep_classes` / `allowed_video_regex`
  behaviour. Deprecate them (Step 3); implementing class preservation is its
  own design problem and belongs in a separate plan.
- Removing any public item outright — that is a breaking change and is the
  maintainer's release decision, not the executor's. Deprecate only.
- Any change to `sanitize_content` (correctly wired already).

## Git workflow

- Branch: `fix/dead-options`
- Commits (single-line conventional, no body, no trailer):
  1. `feat(extractor): enforce max_elems_to_parse limit`
  2. `docs(options): deprecate options that have no effect`
- Do NOT push or open a PR.

## Steps

### Step 1: Implement the `max_elems_to_parse` guard

In `src/content_extractor.rs`, at the top of `grab_article` (before any
scoring work), count the elements in the document and bail if the limit is
exceeded:

```rust
if options.max_elems_to_parse > 0 {
    let count = document.tree.nodes().filter(|n| n.value().is_element()).count();
    if count > options.max_elems_to_parse {
        return Err(ReadabilityError::MaxElementsExceeded {
            // match the variant's actual shape — read src/error.rs first
        });
    }
}
```

Read `src/error.rs` for the exact variant shape (it may carry fields) and
`src/options.rs` for the documented default. **`0` must mean unlimited** —
confirm the current default value and preserve today's behaviour for anyone
who never sets the option. If the default is a non-zero number, then enabling
this guard changes default behaviour for large documents: that is a STOP
condition; report the default and ask.

Counting elements is O(n) over an already-parsed tree, once per
`grab_article` call — negligible against the scoring passes.

**Verify**: `cargo build` → exit 0.

### Step 2: Test the guard

Add to the `#[cfg(test)] mod tests` in `src/content_extractor.rs`:
1. A document with ~50 elements and `max_elems_to_parse(10)` → `parse()`
   returns the error (or `None`, depending on how `Readability::parse`
   surfaces `Err` — read `src/readability.rs:160-256` and assert on what the
   public API actually returns).
2. The same document with `max_elems_to_parse(0)` (default) → parses
   normally.
3. The same document with `max_elems_to_parse(10_000)` → parses normally.

**Verify**: `cargo test --lib content_extractor` → new tests pass.

### Step 3: Deprecate the three that remain unimplemented

In `src/options.rs`, mark the fields AND their builder setters:

```rust
#[deprecated(since = "0.1.4", note = "has no effect; not implemented — see plans/020")]
```

Update each `///` doc to state plainly that the option currently has no
effect. Do NOT delete them (breaking change — maintainer's call).

Deprecation warnings will fire on the crate's own doctest at `src/lib.rs:54`
and anywhere else they're used: fix those call sites (remove
`.keep_classes(true)` from the doctest) rather than suppressing the warning
globally. If `#[deprecated]` on a struct field causes warnings inside
`options.rs` itself (e.g. in `Default` or `build()`), add a narrowly-scoped
`#[allow(deprecated)]` on just those functions with a one-line comment.

**Verify**: `cargo clippy --all-targets -- -D warnings` → exit 0;
`cargo doc --no-deps` → exit 0.

### Step 4: Fix the misleading error docs

- `src/error.rs` — delete the `// Would trigger MaxElementsExceeded if
  implemented` comment from the doctest; after Step 1 it IS implemented, so
  make the doctest reflect real behaviour.
- Audit the other 5 never-constructed variants (`ParseError`,
  `InvalidDocument`, `JsonLdError`, `NoContentFound`, `Other`). Do NOT remove
  them. Instead, report in your final message which remain unconstructed
  after this plan, so the maintainer can decide in a future breaking release.

**Verify**: `cargo test --doc` → passes;
`grep -n "if implemented" src/` → no matches.

## Test plan

Step 2's three cases. Regression gates: full `cargo test` (168 lib + 117
markdown + 2 asserting Mozilla + 24 doctests at time of writing), plus
`cargo test --test mozilla_test_suite` must keep its current pass/divergence
set — the element guard must not trip on any real corpus page.

## Done criteria

- [ ] `max_elems_to_parse` is read in `src/content_extractor.rs` and enforced
- [ ] `0` still means unlimited; default behaviour unchanged
- [ ] `MaxElementsExceeded` is actually constructed somewhere
- [ ] The three unimplemented options carry `#[deprecated]` and honest docs
- [ ] `grep -n "if implemented" src/` → no matches
- [ ] `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check` all exit 0
- [ ] Mozilla suite pass/divergence set unchanged
- [ ] `plans/README.md` status row updated

## STOP conditions

- The default value of `max_elems_to_parse` is non-zero — enforcing it would
  change behaviour for existing users on large documents. Report the default
  and stop.
- Any Mozilla corpus page trips the new guard at the default setting.
- Deprecating the three options cascades into more than ~5 `#[allow(deprecated)]`
  sites — report rather than scattering allows.

## Maintenance notes

- The real lesson is process: a builder option with no reader is invisible in
  review. When adding an option, the same PR must contain the code that reads
  it, plus a test that fails when it is ignored.
- The five still-unconstructed `ReadabilityError` variants are a semver
  cleanup for the next breaking release — deliberately left alone here.
- If class preservation (`keep_classes` / `classes_to_preserve`) is ever
  implemented, it belongs in the serializer alongside the
  `sanitize_content` attribute filter in `element_to_html`, since that is
  already the one place attributes are decided.
