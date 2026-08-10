# Plan 026: Two pre-existing paths that delete code-listing content

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on. On
> any STOP condition, stop and report. When done, update this plan's row in
> `plans/README.md` and `plans/ROADMAP.md`.
>
> **Drift check (run first)**: `git diff --stat 8889569..HEAD -- src/cleaner.rs src/content_extractor.rs src/post_processor.rs`

## Status

- **Priority**: P1 (finding A), P2 (finding B)
- **Effort**: M
- **Risk**: MED — finding A sits in the pre-extraction path, which feeds
  scoring; the Mozilla corpus is the guard rail
- **Depends on**: 025 (provides `src/preformatted.rs`)
- **Category**: bug (content loss)
- **Planned at**: commit `8889569`, 2026-08-10
- **Source**: adversarial review of plan 025. Both findings reproduced
  end-to-end and confirmed byte-identical at `HEAD`, so neither is a
  regression from 025.

## Why this matters

Plan 025 stopped the whitespace passes in `post_processor` from flattening
code listings. Two other paths still **delete** listing content outright, which
is strictly worse than flattening it. Neither is reachable by the splitter that
025 introduced: one runs before `prep_article`, the other inside a step the
splitter does not cover.

### Finding A — `replace_brs` shreds `<br>`-separated code (P1)

`src/cleaner.rs:399`, called from `src/content_extractor.rs:669` and `:710`,
i.e. **before** `prep_article`. It implements Mozilla's `_replaceBrs`
(`<br><br>` → `<p>`) over serialized HTML with no notion of `<pre>`.

Reproduced end-to-end, default options:

```
in:  <pre><code>fn a() {<br><br>    body();<br>}</code></pre>
md:  ```
     fn a() {
     ```
```

`body();` and `}` are gone. `<br>`-separated code inside `<pre>` is a real
legacy-CMS export pattern, not a crafted input.

### Finding B — comment bodies let the removal regexes eat the article (P2)

`element_to_html` (`src/content_extractor.rs:1047-1052`) emits `Node::Comment`
**verbatim** when `sanitize_content` is false — the default. A comment body is
therefore the only channel into `prep_article` carrying unescaped `<`; text and
attribute values both go through `v_htmlescape`. `remove_unwanted_elements`'
`(?is)<aside\b[^>]*?>.*?</aside>` then matches an `<aside>` written *inside* a
comment and consumes the `-->` along with it.

Reproduced end-to-end, default options:

```
in:  <pre><code>keep1\n<!-- <aside> -->\n    keep2\n</code></pre><aside>junk</aside>
md:  ```
     keep1
     ```                      <- keep2 lost, trailing content swallowed
```

Also reproduced with `<form>` in place of `<aside>`, and without any `<pre>`
involved at all (`<p>x <!-- <aside> --> y</p><aside>junk</aside><p>tail</p>`
loses everything after `x`).

## Current state

- `src/cleaner.rs:399` `replace_brs` → `:419` `parse_element` →
  `replace_brs_in_content`; all string-level, no `<pre>` awareness.
- `src/content_extractor.rs:669`, `:710` — the two `replace_brs` call sites.
- `src/content_extractor.rs:1047-1052` — the comment serialization arm.
- `src/post_processor.rs:252` `remove_unwanted_elements` — the tag-pair regexes
  that consume a `-->` (the `<aside>` pattern is at `:258`).
- `src/preformatted.rs` — `map_outside_preformatted`, added by plan 025 and
  already `pub(crate)`, is the tool for finding A.

## Approach

**Finding A**: route `replace_brs_in_content` through
`map_outside_preformatted`. This is exactly what the helper was extracted for;
no new machinery is needed.

**Finding B**: fix it at the source rather than teaching every regex about
comments. Two candidates, pick one after measuring corpus impact:
1. Escape `-->` when serializing a comment body, so a comment can never be
   terminated early by its own content.
2. Drop comments unconditionally instead of only when `sanitize_content` is on.
   Comments are never rendered, so nothing user-visible is lost, and this also
   removes the one remaining reachable trigger for the "unterminated comment"
   path in `src/preformatted.rs`.

Option 2 is simpler and closes the channel completely; option 1 is the smaller
behavior change. Do **not** wrap the individual removal regexes — that fixes
one symptom and leaves the channel open.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Unit | `cargo test --lib` | all pass |
| Regression | `cargo test --test code_whitespace_tests` | all pass |
| Corpus | `cargo test --test mozilla_test_suite` | unchanged baseline |
| Full | `cargo test` | all pass |
| Lint/format | `cargo clippy --all-targets -- -D warnings` && `cargo fmt --check` | exit 0 |

## Scope

**In scope**: `src/cleaner.rs`, `src/content_extractor.rs`, and their tests;
new cases in `tests/code_whitespace_tests.rs`.

**Out of scope**: migrating `post_processor` off regexes onto the DOM (plans
006/009); the `pub fn prep_article` direct-caller hazard noted below.

## STOP conditions

- The Mozilla corpus score moves in either direction. Finding A's fix changes
  what the scorer sees, so this is a live risk, not a formality.
- Any existing test expectation has to be edited.

## Noted, deliberately not planned

`clean_styles`' attribute regexes match *text* like `align="center"` inside a
listing. Calling `pub fn prep_article` directly with such input does delete it,
but the pipeline never produces it: `element_to_html:1024` escapes attribute
values and text through `v_htmlescape`, which entity-escapes both quote
characters. A direct-caller hazard on a public function, not a pipeline bug.
Revisit if that serializer's escaping changes.
