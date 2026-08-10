# Plan 018: SPIKE — design a thin CLI binary over the library

> **Executor instructions**: This is a DESIGN SPIKE with a small prototype.
> The deliverable is a design doc + a feature-gated prototype binary that is
> NOT wired into default builds. Follow the steps, honor STOP conditions,
> update this plan's row in `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- Cargo.toml src/lib.rs src/article.rs`

## Status

- **Priority**: P3
- **Effort**: M (spike scope: 0.5-1 day)
- **Risk**: LOW
- **Depends on**: plans/017-spike-llm-markdown-entrypoint.md (soft — the CLI
  surfaces whatever 017 designs; readable independently)
- **Category**: direction
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

The extraction workflow already exists in ad-hoc command-line form: the
benches drive file-based extraction via shell + Node scripts
(`benches/run_benchmarks.sh`, `benches/benchmark.js`, `compare.js`). A
first-party `readability` CLI (stdin/file/URL? → HTML/text/markdown/JSON on
stdout) would make the crate usable from shell pipelines and by non-Rust
users — every `Article` field the CLI would emit already exists
(`src/article.rs:62-153`: content, text_content, markdown_content, title,
byline, excerpt, length, lang, published_time; `Article` already derives
`Serialize`, so `--format json` is nearly free via the existing `serde_json`
dependency). This spike decides the interface and packaging WITHOUT
committing the repo to a binary it must maintain.

## Current state

- `Cargo.toml` defines only `[lib]` + one `[[bench]]`; no `[[bin]]`, no CLI
  dependency (`Cargo.toml:13-35`).
- `Article` is `Serialize` (`src/article.rs:61` area — confirm the derive)
  and `serde_json` is already a direct dependency (`Cargo.toml:22`).
- Library API: `Readability::new(html: &str, url: Option<&str>, options:
  Option<ReadabilityOptions>)` → `parse() -> Option<Article>`
  (`src/lib.rs:132-137` exports; `src/readability.rs:160`).
- Markdown output requires `output_markdown(true)`
  (`src/options.rs:184,343`).
- No network fetching exists anywhere in the crate (input is always a
  string) — a URL-fetching CLI mode would add an HTTP dependency; that is a
  major scope decision for the doc, not the prototype.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Prototype build | `cargo build --features cli --bin readability` (shape per Step 2) | exit 0 |
| Default build unaffected | `cargo build` | exit 0, no bin built |
| Tests | `cargo test` | all pass |

## Scope

**In scope** (deliverables):
- `plans/018-spike-OUTPUT.md` — design doc
- Prototype: `src/bin/readability.rs` (or `src/cli.rs` + `[[bin]]`) behind a
  `cli` cargo feature, argument parsing kept dependency-light for the spike
  (`std::env::args` is fine for the prototype; the doc evaluates `clap` vs
  `lexopt` vs hand-rolled for the real thing)

**Out of scope**: publishing decisions, `--url` fetching implementation
(design-doc topic only), shell completions, man pages, adding `clap` as a
non-optional dependency.

## Git workflow

- Branch: `spike/cli-binary`
- Commit style: `docs(spike): design CLI binary + feature-gated prototype`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Design doc — interface

In `plans/018-spike-OUTPUT.md`, specify: invocation forms (`readability
article.html`, `curl … | readability -`), flags (`--format
html|text|markdown|json` mapping to Article fields, `--base-url`,
`--char-threshold`, `--markdown` toggles mapping to existing
ReadabilityOptions builder methods — enumerate the exact mapping table),
exit codes (0 extracted, 1 no article found, 2 usage/IO error), and behavior
on `parse() == None`. Address: dependency policy (feature-gated `clap` vs
zero-dep), binary naming/crate packaging (same crate `[[bin]]` +
`required-features = ["cli"]` vs a separate `readabilityrs-cli` crate — the
doc must weigh the "cargo install readabilityrs" install-weight tradeoff),
and whether `--url` fetching is in v1 (recommendation with reasoning; note
it drags in TLS/HTTP deps).

### Step 2: Prototype

Implement the minimal loop behind the feature gate: read file-or-stdin,
parse, print the selected format. Cargo.toml sketch:

```toml
[features]
cli = []

[[bin]]
name = "readability"
required-features = ["cli"]
```

Use `std::env::args` parsing for the prototype (flags: `--format`,
`--base-url` only). No new dependencies.

**Verify**:
`cargo build` → exit 0 with NO bin target built;
`cargo run --features cli --bin readability -- tests/test-pages/001/source.html --format text | head -5`
→ prints extracted text;
`echo $?` → 0.

### Step 3: Validate against the corpus

Run the prototype over 5+ corpus pages incl. one where extraction fails
(find a `"readerable": false` case in `tests/test-pages/*/expected-metadata.json`)
to confirm exit-code behavior. Paste transcripts into the design doc.

**Verify**: transcripts in doc; `cargo test` unaffected.

### Step 4: Recommendation

Close the doc with: ship / don't-ship recommendation, packaging choice,
effort estimate for the production version (incl. tests via `assert_cmd` or
plain `std::process::Command` integration tests), and how it composes with
plan 017's entry point (the CLI's `--format markdown` should be 017's tier-1
call if that lands first).

## Test plan

Spike: prototype has no permanent tests; the production follow-up plan (if
accepted) must include CLI integration tests. `cargo test` must stay green
throughout (the feature gate guarantees the lib is untouched).

## Done criteria

- [ ] `plans/018-spike-OUTPUT.md` exists with interface spec, option-mapping
      table, packaging analysis, corpus transcripts, recommendation
- [ ] Prototype builds ONLY with `--features cli`; default `cargo build` and
      `cargo test` unchanged
- [ ] `plans/README.md` status row updated

## STOP conditions

- The prototype requires ANY new dependency — stop; that contradicts the
  spike's zero-dep constraint and the doc should discuss it instead.
- `[[bin]]` + `required-features` interacts badly with the existing
  `[[bench]]`/docs.rs build (verify `cargo doc --no-deps` still works) —
  report.

## Maintenance notes

- If shipped, the CLI becomes a semver-visible surface (flags are API);
  the design doc's exit-code and flag contracts are the compatibility
  promise. Keep the lib the source of truth — the CLI must stay a thin shim.
