# Roadmap

Living status board for the readabilityrs improvement work. Glance here for
**what's done, what's in flight, what's next**. Detailed plans, deviation notes,
and dependency reasoning live in [`README.md`](README.md); this file is the
dashboard on top of it.

**How to keep this current**
- When a plan lands, flip its status to `SHIPPED` and add the merge SHA.
- When you start one, set `IN PROGRESS`; when it stalls, `BLOCKED` + one-line reason.
- Log every status change under [Changelog](#changelog) with a date.
- New work → add a row. A finding that gets its own plan moves from
  [Open findings](#open-findings-without-a-plan) into the plan table.

**Status legend**

| Status | Meaning |
|---|---|
| `SHIPPED` | Merged to `main` and pushed to `origin` |
| `IN PROGRESS` | Actively being executed |
| `BLOCKED` | Stopped on a dependency or unresolved decision (reason given) |
| `TODO` | Planned, not started |
| `REJECTED` | Considered and dropped (rationale in `README.md`) |

**As of 2026-08-09:** `main` == `origin/main` == `c4f0842`. The execute-loop
stack (024 `cb98bc3`, 013 `b112b2a`, 012 `cafb70e`, 015 `9f91e8d`, 014
`c4f0842`) is merged and **pushed**. Branch `advisor/remaining-plans` deleted.
Backup tag `pre-merge-backup` reverts to the original pre-stack base. Snapshot:
**21 shipped · 1 blocked · 3 todo** (both remaining are design-heavy, held per
owner decision — see below).

---

## Plan status

Ordered by status, then plan number. `P` = priority (P0 highest).

### Shipped (21)

| # | P | Title | Merge |
|---|---|-------|-------|
| 025 | P1 | Preserve `<pre>`/`<code>` whitespace in post-processing (issue #24) | pending |
| 024 | P1 | Escape link/image/media destinations in markdown output (injection) | `cb98bc3` |
| 014 | P3 | Document dev workflow in CONTRIBUTING.md (CLAUDE.md left untracked) | `c4f0842` |
| 013 | P3 | Dependency hygiene (relax v_htmlescape pin, thiserror 2.0) | `b112b2a` *(branch, unmerged)* |
| 012 | P3 | once_cell → std LazyLock; declare MSRV **1.83** (not 1.80 — see note) | `cafb70e` *(branch, unmerged)* |
| 015 | P3 | Split 2171-line metadata.rs into 6 submodules (pure move, byte-verified) | `9f91e8d` *(branch, unmerged)* |
| 021 | P0 | Stop nav-wrapper removal deleting article bodies | `3b8f337` |
| 022 | P0 | Remove `<script>`/`<style>` at DOM level (`</script >` bypass) | `e1789cf`,`0eb0a65` |
| 001 | P1 | Fix byline lowercase-slice panic (remote DoS) | `00de913` |
| 002 | P1 | Make Mozilla test suite assert by default (baseline 119/130) | `6d197cc` |
| 003 | P1 | CI + clean fmt/clippy baseline | `59d5b25`,`6060fba`,`89e1b52`,`bcd8657` |
| 004 | P1 | Sanitization contract + opt-in `sanitize_content` | `d3c817f` |
| 020 | P1 | Wire up / deprecate the 4 dead public options | `2eb21aa`,`dfd56f3` |
| 023 | P1 | Bound markdown table column widths (panic + 265× blowup) | `f392a7b` |
| 005 | P2 | NodeId element identity + NaN-safe sort | `98c6fdf`,`08cf31f` |
| 007 | P2 | Hoist per-call regex compilation into Lazy statics (39.7s→25.6s) | `13204b8` |
| 008 | P2 | Bound DOM recursion depth (stack-overflow DoS; limit 256) | `d522421` |
| 016 | P2 | Complete `is_probably_readerable` (corpus 128→130/130) | `d840b82` |
| 010 | P2 | Fix README example + compile README as doctests | `a57130f`,`335397e` |
| 011 | P2 | Stop double-escaping `&` in standardized image URLs | `9355750` |
| 019 | P2 | Entity-aware escape fix for math.rs + code_blocks.rs | `08b9088` |

### Blocked (1)

| # | P | Title | Reason |
|---|---|-------|--------|
| 006 | P2 | Consolidate element removal onto the DOM path | STOP hit: correct removal deletes real article bodies (`aclu`, `mercurial`) because the keyword rules were only survivable while the old regex remover was broken. Needs **006b** (re-derive removal rules) first. Work preserved on local branch `refactor/dom-only-element-removal` (`c1568da`), not pushed. |

### Todo (8)

| # | P | Title | Depends on |
|---|---|-------|-----------|
| 014 | P3 | Document dev workflow (CONTRIBUTING) + CLAUDE.md — *partly superseded* | — |
| 015 | P3 | Split the 2127-line `metadata.rs` module | after 001, 007 |
| 017 | P3 | SPIKE: LLM-ready-Markdown entry point design | — |
| 018 | P3 | SPIKE: CLI binary design + feature-gated prototype | 017 (soft) |
| 026 | P1 | Code loss in `replace_brs` + the comment channel (found reviewing 025) | 025 |
| 009 | P3 | Collapse serialize→re-parse round-trips into one tree pass | **006** + 002 |

---

## Needs a new plan

Work that is known and wanted but doesn't have a written plan yet.

| ID | Title | Why | Priority |
|---|---|---|---|
| 006b | Re-derive DOM removal rules so 006 can land | Route keyword removal through `should_remove_dom_node` (don't detach containers with substantial text or that hold the top candidate); match whole class tokens, not substrings; keep the unconditional tag list. Unblocks 006 → 009. | P2 |

---

## Open findings without a plan

From the 2026-08-04 post-merge bug hunt (all reproduced against `4430e24`).
Each needs a plan before execution. Full evidence in
[`README.md`](README.md#recorded-findings-without-their-own-plan-yet-post-merge-bug-hunt-2026-08-04).

| Finding | Location | Severity | Status |
|---|---|---|---|
| **Markdown-path injection** — link/image/media destinations + footnote defs emitted raw; `alt`/`href`/`title`/`src` could break out of `![]()`/`[]()` | `markdown/rules/{images,links,media}.rs`, `converter.rs` footnotes | MED (HIGH if rendered to HTML) | ✅ CLOSED (plan 024, `cb98bc3` — structural escaping always on; scheme-drop gated on `sanitize_urls`; media + footnote sinks caught in review) |
| `pick_best_srcset` splits on `,` first — corrupts CDN URLs containing commas (imgix/Cloudinary) | `elements/images.rs:115` | MED | OPEN |
| `element_to_html` drops attribute namespace prefixes — duplicate `href` on SVG `<a xlink:href …>`, dead `xlink:href` arm | `content_extractor.rs:980` (`:976` dead) | MED | OPEN |
| `IMG_TAG_RE` truncates at `>` inside an attribute value — deletes partial tag, leaks text | `elements/images.rs:11` | LOW | OPEN |
| Void-element end-tag alternation swallows content up to a planted `</input>` in a comment (comments emitted verbatim on default path) | `post_processor.rs:279`, `content_extractor.rs:1004` | LOW | OPEN |
| `cleaner::parse_element` can build a reversed byte range (latent; live only if `replace_brs` is exported) | `cleaner.rs:411` | LOW | OPEN |
| Mozilla content assertion still weak — ±100% band admits some empty content; tighten to ±15–20% | `tests/mozilla_test_suite.rs` | LOW (test quality) | PARTLY CLOSED (`3b8f337` added unconditional `> 0` guard) |
| `link_density_modifier` NaN/±∞ panic in `partial_cmp().unwrap()` | `content_extractor.rs:229`, `scoring.rs:129` | MED | ✅ CLOSED (`98c6fdf`, `08cf31f`) |

---

## Changelog

- **2026-08-10** — 025 shipped: `normalize_whitespace` was collapsing
  indentation and blank lines inside `<pre>`/`<code>` (issue #24), flattening
  every code listing in `content`, `text_content`, and `markdown_content`.
  Fixed with a split-and-map pass that runs the collapsing regexes only outside
  preformatted elements, extracted to `src/preformatted.rs`. Mozilla corpus
  unchanged; no existing expectation edited. Review of it opened **026**: two
  *pre-existing* paths (`replace_brs`, and comment bodies feeding the removal
  regexes) delete listing content outright — same family, worse damage.

- **2026-08-09** — CI hotfix `8889569` (pushed to main): CI runs Rust
  **1.97** `@stable`, whose clippy promoted `useless_borrows_in_formatting` to
  `-D warnings`. Three pre-existing debug `println!("{}", &x…)` in
  `tests/mozilla_test_suite.rs` (untouched this session) started failing the
  clippy gate. Dropped the redundant `&`. **Gotcha: local clippy (1.96) is
  older than CI's stable (1.97), so a green local `cargo clippy` does NOT
  guarantee a green CI clippy** — newer lints only fire on CI. A pinned-MSRV /
  pinned-clippy CI job (or bumping the local toolchain) would close the gap.
- **2026-08-09** — Plan **014** (dev docs) executed + committed `c4f0842`.
  Added a Development section to CONTRIBUTING.md (build/test/fmt/clippy commands,
  Mozilla-suite invocation, fixture prohibition, branch/commit conventions,
  source layout — reflects MSRV 1.83 and the metadata/ split). Reconciled scope:
  CLAUDE.md already exists and is left **untracked** (owner's file; not asked to
  track it). 20 shipped / 1 blocked / 2 todo. **Then per owner decision: the 5
  branch commits (024, 013, 012, 015, 014) merged to `main` + pushed.**
- **2026-08-09** — Plan **015** (split metadata.rs) executed + reviewed
  (2 agents) + committed `9f91e8d`. 2171-line `metadata.rs` → `src/metadata/`
  with mod.rs (440) + json_ld.rs (350) + byline.rs (1187) + title.rs (141) +
  image.rs (52) + language.rs (50). Bug-hunt byte-verified it as a faithful
  pure move (63-fn set identical, 225 lib / 33 metadata tests unchanged, no
  API growth, `pub(super)` discipline). Removed a stale `#[cfg(test)]` herald
  debug block per review. utils.rs migration (Step 3) deliberately skipped —
  helpers are shared/entangled. **Follow-ups (not blockers):** byline.rs is
  1187 lines (clean seam = extract its 18 tests to `byline/tests.rs`);
  pre-existing selector/regex `.unwrap()`/`.expect()` moved verbatim could be
  swept later. Note: first commit `5384e70` accidentally captured only the
  file deletion (a failed `git add` pathspec aborted staging the new dir) —
  amended to `9f91e8d` with the full split before moving on. 19 shipped / 1
  blocked / 3 todo.
- **2026-08-09** — Plan **012** (LazyLock migration) executed + reviewed
  (2 agents) + committed `cafb70e`. 13 files migrated `once_cell::sync::Lazy`
  → `std::sync::LazyLock`; `once_cell` dropped as a direct dep. **MSRV
  deviation:** plan said 1.80 (the LazyLock floor), but the bug-hunt review
  found the resolved tree already requires **1.83** (ICU4X 2.x via
  `url → idna → idna_adapter`; confirmed with `cargo metadata`). Declared
  `rust-version = "1.83"` instead — the honest, dependency-driven floor. Not
  toolchain-verified locally (no 1.83 toolchain; not installed). CI runs on
  `@stable` so it won't catch MSRV drift — a follow-up could add a pinned-MSRV
  CI job. 18 shipped / 1 blocked / 4 todo.
- **2026-08-09** — Plan **013** (dependency hygiene) executed + committed
  `b112b2a`: v_htmlescape pin relaxed `=0.15.8`→`0.15`, thiserror `1.0`→`2.0`
  (no `error.rs` change); crossbeam Step 3 already done by 003; `cargo audit`
  clean. Pure dep bump — review scaled to self-verify (no source logic).
  17 shipped / 1 blocked / 5 todo.
- **2026-08-09** — Started execute loop on branch `advisor/remaining-plans`.
  Plan **024** (markdown-path injection) written + executed + reviewed
  (code-quality + bug-hunt agents) + committed `cb98bc3`. Review caught two
  extra sinks the plan missed — media (iframe/video/audio) and footnote
  definitions — both fixed before commit. Markdown-injection finding CLOSED.
  16 shipped / 1 blocked / 6 todo.
- **2026-08-09** — Roadmap dashboard created. Confirmed full stack pushed:
  `main` == `origin/main` == `d840b82`. 15 shipped / 1 blocked / 7 todo.
- **2026-08-05** — 016 shipped (`d840b82`); corpus agreement 128→130/130.
  006 hit its STOP condition and was marked BLOCKED pending 006b.
- **2026-08-04** — Merged and pushed the P0/P1 safety core plus perf/quality
  plans: 005, 007, 008 shipped; 021, 022 (P0) shipped; 020, 023, 010, 011,
  019 shipped. Post-merge bug hunt recorded 8 findings (above).
- **2026-07-23** — Roadmap generated (23 plans) against `c7622fd`.
