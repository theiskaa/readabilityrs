# Plan 025: Stop post-processing from collapsing whitespace inside `<pre>`/`<code>`

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on. On
> any STOP condition, stop and report. When done, update this plan's row in
> `plans/README.md` and `plans/ROADMAP.md` — unless a reviewer dispatched you
> and said they maintain the index.
>
> **Drift check (run first)**: `git diff --stat 8889569..HEAD -- src/post_processor.rs`

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW (output formatting only; no scoring or extraction logic touched)
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `8889569`, 2026-08-10
- **Source**: [issue #24](https://github.com/theiskaa/readabilityrs/issues/24)
  by @sak96, reproduced locally

## Why this matters

`prep_article` runs `normalize_whitespace` over the serialized article HTML.
That pass collapses every run of two-or-more spaces to one and every run of
three-or-more newlines to two — across the *whole* document, including the
inside of code listings, where whitespace **is** the content.

Reproduced at `8889569` on a Hugo/chroma-shaped page (the structure the
reporting article uses):

```
<pre tabindex="0" class="chroma"><code class="language-rust">fn main() {
    let x = 1;
        deeper();
}</code></pre>
```

extracts as

```
fn main() {
 let x = 1;
 deeper();
}
```

Every indentation level flattens to a single space, so nesting is
unrecoverable. The damage propagates: `content`, `text_content`, and
`markdown_content` are all derived from the same string, so the fenced code
block in the Markdown output — the whole point of the LLM-ready path — carries
the flattened listing too. Setting `clean_whitespace(false)` avoids it only by
disabling the pass wholesale, which gives up prose normalization everywhere.

## Current state

- `src/post_processor.rs:237` — `normalize_whitespace(html)` applied
  `MULTI_NEWLINE` (`\n{3,}` → `\n\n`) and `MULTI_SPACE` (` {2,}` → ` `) to the
  entire string with no notion of element boundaries.
- `src/post_processor.rs:193-197` — `prep_article` calls it whenever
  `clean_whitespace` is on (default `true`, `src/options.rs:232`).
- `src/readability.rs:178-183` — `prep_article` output feeds
  `cleaner::clean_article_content`, whose result becomes `content`,
  `text_content`, and the input to `standardize_all` → `html_to_markdown`.
- The other regex passes in the module (`remove_unwanted_elements`,
  `remove_share_elements`, `remove_navigation_elements`,
  `remove_empty_paragraphs`) match on tag syntax; `<` inside a code listing
  arrives entity-escaped, so they do not fire on listing text.

## Approach

Split the input on preformatted elements and apply the collapsing regexes only
to the stretches between them.

Rejected alternative — the placeholder/substitute approach in the reference
commit (`sak96/readabilityrs@b8aba64`): it extracts `<pre>` matches, swaps in
`\x00PRE_BLOCK_n\x00` sentinels via `replacen(full_match, …, 1)`, collapses,
then substitutes back. `replacen` searches for the *text* of the match, not its
offset, so two identical listings restore in the wrong order and a listing
whose text also occurs earlier in the document corrupts that earlier
occurrence; it clones the whole document once per match (quadratic); and a
non-greedy `.*?` cannot see nesting, so an inner `</code>` closes the outer
block early. A split-and-map pass has none of those failure modes.

## Steps

1. Add `map_outside_preformatted(html, rewrite)` in a new `src/preformatted.rs`:
   scan for `<`, treat a `<pre>`/`<code>` element or a comment as an opaque
   span copied through verbatim, and hand every other stretch to `rewrite`.
2. Add `block_end` to find the matching close tag with a depth
   counter (an inner `<code>` must not close an outer `<code>` early), and
   `starts_with_tag_name` so `<precondition>` is not mistaken for `<pre>`.
   When the close tag is missing, preserve the remainder — preserving too much
   is recoverable, mangling a listing is not.
3. Skip comments in **both** scanners. `element_to_html`
   (`src/content_extractor.rs:1050`) emits comment bodies verbatim whenever
   `sanitize_content` is false (the default), so a comment is the one channel
   into `prep_article` carrying unescaped `<`. `<pre><!-- </pre> -->…</pre>`
   would otherwise close the listing early and re-break this very issue, and a
   commented-out `<code>` open tag would silently disable normalization for the
   rest of the document.
4. Rewrite `normalize_whitespace` to run its two regexes through that helper.
   Applying them per stretch rather than once globally is equivalent: a run of
   spaces or newlines cannot span an element boundary.
5. Route `cleanup_after_title_removal`'s whitespace loop through the same
   helper. `src/readability.rs:184-189` runs it *after* `prep_article` when
   `remove_title_from_content` is on, and its `\n\s*\n\s*\n` and `\n[ \t]+\n`
   patterns destroy blank lines and whitespace-only lines inside listings.
   Leaving it global would fix the default path and leave that option broken.
6. Unit tests in `src/preformatted.rs`: prose still collapses; `<pre>`
   indentation and blank lines survive; inline `<code>` spacing survives;
   repeated identical blocks each survive in place; nested `<code>`;
   unterminated block; `<precondition>`/`<codex>` still normalize; uppercase
   `<PRE>` and `</code >` are recognized; close and open tags inside comments
   are ignored; title removal preserves indentation.
7. Integration test `tests/code_whitespace_tests.rs` asserting through the public
   API that both `content` and `markdown_content` keep indentation, including
   under `remove_title_from_content: true`.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Unit | `cargo test --lib preformatted post_processor` | all pass |
| Regression | `cargo test --test code_whitespace_tests` | all pass |
| Corpus | `cargo test --test mozilla_test_suite` | unchanged baseline |
| Full | `cargo test` | all pass |
| Lint/format | `cargo clippy --all-targets -- -D warnings` && `cargo fmt --check` | exit 0 |

## Scope

**In scope**: `src/post_processor.rs`, a new `src/preformatted.rs`, and their
tests; a new `tests/code_whitespace_tests.rs`.

**Out of scope**:

- `clean_styles`. Its attribute regexes (`\s+style\s*=\s*"[^"]*"` and friends)
  would happily match *text* like `align="center"` inside a listing, but the
  serializer at `src/content_extractor.rs:1045` escapes text through
  `v_htmlescape`, which turns both quote characters into entities — so the
  patterns cannot fire on listing text today. Latent, not reachable; leaving it
  global also keeps `style` stripping working on the highlight `<span>`s inside
  a chroma block, which is the behavior the corpus expects. Track separately if
  the serializer's escaping ever changes.
- Migrating the module off regexes onto the DOM (plan 006/009 territory).

## STOP conditions

- Mozilla corpus score moves off its baseline in either direction.
- Any existing test expectation has to be edited to accommodate the change.

## Outcome

Committed on branch `bugfix/preserve-code-whitespace`, not yet merged. Both
stated STOP conditions held: the Mozilla corpus is unchanged and
no pre-existing expectation was touched. Verified end-to-end on a chroma-shaped
fixture (`<div class="highlight"><pre class="chroma"><code class="language-…">`
with per-line `<span>` wrappers) — indentation is intact in `content` and in
the fenced block in `markdown_content`.

The splitter lives in **`src/preformatted.rs`**, not inline in
`post_processor.rs`: it is a closed set of helpers behind one `pub(crate)`
entry point, and plan 026's fix for `replace_brs` needs it from `cleaner.rs`.

Steps 3 and 5 came out of review, not the original plan: the first draft
protected only `normalize_whitespace` and trusted comment bodies. Both gaps
were reproduced end-to-end before being fixed, and both regression tests were
confirmed to fail against the un-fixed code.

Two further review rounds hardened it: an unterminated comment now counts as
no comment (treating it as opaque swallowed the rest of the document once
`remove_unwanted_elements` ate a `-->`), `/` was dropped from the tag-name
terminator set so `<pre/>` cannot open a never-closing block, and the
empty-wrapper loop in `cleanup_after_title_removal` was routed through the
splitter too. The callback returns `String` rather than filling an out-param:
both shapes benchmarked identically (1.01x/0.99x/0.99x across a typical
article, a 40k-inline-`<code>` document, and a no-code document), so the
simpler signature wins. The per-chunk regex overhead on code-heavy input is
real (~3.6x on 100k code spans) but inherent to the fix and still linear.

A pre-merge review round added the last three commits. `comment_len` rescanned
to end-of-input from every `<!--`, so a run of unterminated comments made the
pass quadratic — 1.0 s on 160 KB, reachable end-to-end because
`remove_unwanted_elements` can eat a `-->` and leave the `<!--` behind. A failed
search proves no `-->` lies ahead at all, so it now latches; 160 KB dropped to
0.6 ms with byte-identical output. The same round caught two vacuous tests:
`test_inner_close_tag_does_not_end_outer_block` wrapped its fixture in a `<pre>`
that made the whole span opaque, leaving `block_end`'s depth counter untested,
and nothing at all covered routing the empty-wrapper loop through the splitter.
Both now die under mutation (`if depth == 0` → `if true`; the loop reverted to a
global application).

Verified not to be problems, so that a future reader does not re-derive them:
no UTF-8 boundary panic is reachable (every index lands just past an ASCII
`<` or `>`, and `starts_with_tag_name` compares `&[u8]`); both scanners
advance their cursor unconditionally, so neither can stall; and the pass stays
linear on adversarial nesting. Attribute values cannot smuggle a `<` — unlike
comments, they go through `v_htmlescape` at `content_extractor.rs:1024`.
