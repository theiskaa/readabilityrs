# Plan 017: SPIKE — design the "HTML → LLM-ready Markdown" entry point and positioning

> **Executor instructions**: This is a DESIGN SPIKE, not a build plan. The
> deliverable is a written design document + a small proof-of-concept, NOT
> shipped features. Follow the steps, produce the deliverable file, and
> update this plan's row in `plans/README.md`. STOP conditions apply as
> usual.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- src/elements/ src/markdown/ src/lib.rs README.md`

## Status

- **Priority**: P3
- **Effort**: M (spike scope: 0.5-1 day)
- **Risk**: LOW (no production code changes)
- **Depends on**: none
- **Category**: direction
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

The repo has quietly built something more valuable than a reader-view port:
the standardization pipeline in `src/elements/` normalizes vendor-specific
HTML — syntax-highlighted code from Prism/Shiki/rehype/WordPress/GitHub
(`code_blocks.rs` + `languages.rs` with ~250 language mappings), lazy-loaded
images (`images.rs`), heading permalinks (`headings.rs`), CMS footnote
formats (`footnotes.rs`), and rendered MathJax/KaTeX math (`math.rs`) — into
canonical forms, then `src/markdown/` converts to configurable Markdown.
That depth is far beyond reader-view needs; it is exactly the cleanup an
LLM/RAG ingestion pipeline wants, and no other Readability port offers it.
Today this capability is buried: reaching it requires knowing to call
`elements::standardize_all` + `markdown::html_to_markdown` manually
(README mentions it in one sentence at ~line 65) or flipping
`output_markdown(true)`. This spike designs the one-call entry point and the
positioning, WITHOUT committing to implementation.

## Current state

- Public pieces already exported: `elements::standardize_all(html, title)`
  (`src/elements/mod.rs:12`), `markdown::html_to_markdown(html, &opts)`
  (`src/markdown/mod.rs:12`), `MarkdownOptions` (re-exported at crate root,
  `src/lib.rs:134`), `Article::markdown_content` behind
  `ReadabilityOptions::output_markdown` (`src/options.rs:184`,
  `src/readability.rs:216-230`).
- `MarkdownOptions` (`src/markdown/options.rs`) covers formatting style
  (heading style, bullets, fences, emphasis, link style) — nothing
  LLM-specific (no "strip images", "max length", "include title as H1",
  "frontmatter" concepts).
- 117 markdown tests + a "130-page quality audit" exist
  (`tests/markdown_tests.rs`, commit `4e69a22`).
- Audience evidence: benches compare against `@mozilla/readability` (JS);
  README positions against reader view, not ingestion.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Tests   | `cargo test` | pass (spike must not break anything) |
| PoC run | `cargo test --lib <poc test name>` or a `#[ignore]`d test | demonstrates the flow |

## Scope

**In scope** (deliverables):
- `plans/017-spike-OUTPUT.md` — the design doc (see Step list for required
  sections)
- Optionally a `#[cfg(test)]`/`#[ignore]` proof-of-concept function showing
  the proposed API shape compiles against current internals — NOT exported

**Out of scope**: shipping any new public API, README rewrite, new options,
new dependencies. Those become follow-up plans informed by this spike.

## Git workflow

- Branch: `spike/llm-markdown-entrypoint`
- Single commit: `docs(spike): design LLM-markdown entry point`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Inventory what the pipeline already handles

Read `src/elements/*.rs` and `src/markdown/options.rs` fully. In the design
doc, table out: normalization performed, vendor patterns covered, and the
gaps that matter for LLM ingestion specifically (candidates to evaluate:
tables→markdown fidelity, image alt-text preservation vs stripping, link
handling for token budgets, frontmatter/metadata emission, whitespace
guarantees, deterministic output).

### Step 2: Design the API surface

Propose (with rust signatures, doc comments, and a worked example) ONE
primary entry point, e.g.:

```rust
/// Extract the article from `html` and return it as clean Markdown
/// suitable for LLM ingestion. Equivalent to Readability::parse() with
/// markdown output enabled, returning just the markdown (+ metadata).
pub fn extract_markdown(html: &str, url: Option<&str>, options: Option<ExtractMarkdownOptions>) -> Result<MarkdownArticle>
```

Decide and justify in the doc: does it wrap `Readability::parse` (extraction
included) or `standardize_all + html_to_markdown` (no extraction)? Probably
both tiers, named distinctly. What does `MarkdownArticle` carry (markdown,
title, byline, lang, published_time — the RAG-relevant metadata)? Which
LLM-specific options earn a place (evaluate: `strip_images`,
`include_title_heading`, `yaml_frontmatter`, `max_chars` — recommend a
minimal set, reject the rest with reasons)? Do defaults differ from the
reader-view path (e.g. reference-style links off, setext headings off)?

### Step 3: Proof of concept

As an `#[ignore]`d test or private function, implement the happy path of the
chosen tier-1 API using ONLY existing internals, run it on 2-3 pages from
`tests/test-pages/` (e.g. a code-heavy and an image-heavy case), and paste
representative output into the design doc. Note any quality problems found
(these seed future plans).

**Verify**: `cargo test` still green; the PoC test runs with `-- --ignored`.

### Step 4: Positioning section

In the design doc: the README/docs.rs story ("readability + LLM-ready
markdown in one call"), the comparison hook vs other ports (name the
concrete differentiators from Step 1's table — code-block language recovery,
math, footnotes), and what DIR-03/DIR-04 (WASM, CLI — see plans/README.md
rejected/deferred notes) would add on top if ever pursued. Also list open
questions for the maintainer (naming, whether `MarkdownArticle` merges into
`Article`, semver implications).

## Test plan

Spike: no permanent tests. The PoC must not weaken existing suites.

## Done criteria

- [ ] `plans/017-spike-OUTPUT.md` exists with: capability inventory table,
      proposed API (signatures + example), option decisions with rationale,
      PoC output samples, positioning section, open questions
- [ ] PoC compiles; `cargo test` unaffected
- [ ] A recommended follow-up plan list (1-3 items, each S/M/L-estimated)
      closes the doc
- [ ] `plans/README.md` status row updated

## STOP conditions

- The PoC reveals the markdown path panics or produces garbage on corpus
  pages — that's a bug finding that outranks the spike; file it in the
  report and stop.
- The design requires breaking `Article`/`MarkdownOptions` semver — flag
  prominently; the maintainer decides.

## Maintenance notes

- The spike doc is input to the maintainer's roadmap decision; nothing here
  commits the project. If accepted, the follow-up plans go through the
  normal plans/ flow.
