# Plan 014: Document the dev workflow (CONTRIBUTING) and add CLAUDE.md

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on. On
> any STOP condition, stop and report. When done, update this plan's row in
> `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- CONTRIBUTING.md`
> Also check whether CLAUDE.md or AGENTS.md now exists (`ls CLAUDE.md
> AGENTS.md 2>/dev/null`) — if so, STOP and reconcile instead of overwriting.

## Status

- **Priority**: P3
- **Effort**: S
- **Risk**: LOW
- **Depends on**: best written AFTER plans 002/003 land (the commands it
  documents change); if they haven't landed, document TODAY's reality and
  note the pending changes
- **Category**: dx
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

`CONTRIBUTING.md` explains GitHub mechanics (fork, branch, PR) but never
names a single actual command — step 4 is literally "Test out your changes.
Make sure that they work as you intended." It doesn't mention `cargo test`,
`cargo fmt`, `cargo clippy`, or the critical caveat that the Mozilla
integration suite needs `-- --ignored` to run at all (pre-plan-002).
There is also no CLAUDE.md/AGENTS.md, so coding agents (which will execute
the other plans in this directory) have no repo briefing. Both gaps make
wrong-by-default contributions the norm, especially with no CI (plan 003).

## Current state

- `CONTRIBUTING.md` — 6-step PR walkthrough, no commands (see step list
  around "Here's how to contribute and submit your pull request").
- No `CLAUDE.md`, no `AGENTS.md`, no `.cursorrules` etc.
- Verified command reality at planning time (adjust to what's true when you
  execute — that's why this plan runs late):
  - `cargo test` — 154 lib + 117 markdown tests + doctests, ~30s debug.
  - `cargo test --test mozilla_test_suite -- --ignored --nocapture` — the
    130-page suite (non-asserting pre-002; asserting and default-run
    post-002).
  - `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` —
    failing pre-003, enforced post-003.
  - `cargo bench` — criterion benches; `benches/` also has a Node.js
    comparison harness (`npm install` inside `benches/`, `node benchmark.js`)
    — mention it exists, don't document it deeply.
- Repo facts for the briefing: Rust port of Mozilla Readability.js;
  edition 2021; core flow `Readability::new(html, url, options)` →
  `parse()` → `Article`; source layout: `readability.rs` (orchestration),
  `content_extractor.rs` (scoring/candidate selection),
  `cleaner.rs`/`post_processor.rs` (cleanup), `metadata.rs`
  (JSON-LD/meta/byline), `elements/` + `markdown/` (markdown output path),
  `constants.rs` (regexes/flags), tests in `tests/` with the Mozilla corpus
  under `tests/test-pages/` (130 fixture dirs — never hand-edit fixtures).

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Verify docs' claims | run every command you document | matches what you wrote |

## Scope

**In scope**: `CONTRIBUTING.md` (add a Development section; don't rewrite
the existing GitHub-mechanics prose), `CLAUDE.md` (create).

**Out of scope**: README (plans 002/004/010 own pieces of it); CI config;
issue/PR templates (nice-to-have, not planned).

## Git workflow

- Branch: `docs/dev-workflow`
- Commit style: `docs(contributing): document build/test/lint workflow` and
  `docs: add CLAUDE.md agent briefing`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Add a "Development" section to CONTRIBUTING.md

Insert after the "Opening a Pull Request" section: build/test/fmt/clippy
commands (as verified live), the Mozilla-suite invocation and what it
covers, the fixture-editing prohibition, and the branch-prefix convention
observed in history (`feature/…`, `bugfix/…`, `refactor/…`) plus
conventional-commit message style with a real example from `git log`.

**Verify**: every command in the section executed verbatim from a clean
checkout state, output matches the description.

### Step 2: Create CLAUDE.md

Keep it under ~60 lines. Contents: one-paragraph project description; the
command table; source-layout map (the file roles from Current state); the
three invariants agents must not violate (never edit `tests/test-pages/`
fixtures to make tests pass; extraction-behavior changes require the Mozilla
suite green with divergence lists unchanged or explicitly justified; output
HTML is intentionally unsanitized by default — see plan 004's contract);
pointer to `plans/README.md` for in-flight work.

**Verify**: `cargo test` still passes (docs-only change — trivially true;
run it anyway to confirm clean state); a fresh reader can run every command
in CLAUDE.md top to bottom.

## Test plan

Docs-only; the verification IS running each documented command.

## Done criteria

- [ ] CONTRIBUTING.md names concrete commands for build, test, integration
      suite, fmt, clippy
- [ ] CLAUDE.md exists, ≤ ~60 lines, contains the fixture prohibition and
      the suite-green invariant
- [ ] Every documented command was executed and matches
- [ ] `plans/README.md` status row updated

## STOP conditions

- CLAUDE.md or AGENTS.md already exists (drift check).
- A documented command's behavior contradicts what plans 002/003 were
  supposed to establish (e.g. suite still non-asserting after 002 marked
  DONE) — that's a reconciliation problem; report it.

## Maintenance notes

- CLAUDE.md rots fast; whoever changes the test invocation or adds a
  workspace member must update it in the same PR. Reviewers enforce.
