# Plan 005: Replace stringified element identity with native NodeId keys (kills O(N·K) full-tree scans)

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat c7622fd..HEAD -- src/content_extractor.rs`
> If the file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: MED
- **Depends on**: plans/002-asserting-mozilla-test-suite.md (regression net)
- **Category**: perf
- **Planned at**: commit `c7622fd`, 2026-07-23

## Why this matters

Candidate selection in `src/content_extractor.rs` identifies elements by
`format!("{:?}", element.id())` — a debug-formatted `ego_tree::NodeId` used as
a `String` HashMap key — and resolves an ID back to an element with
`find_element_by_id`, which walks the ENTIRE document (`select("*")`),
debug-formatting every element's id until one matches. The code itself admits
this ("This is a simplified approach - in production we'd need better element
tracking"). With ~14 call sites, several inside loops over all scored
candidates, resolution is O(N·K) string comparisons per extraction pass — and
`grab_article` runs up to 4 passes (flag-relaxation retries). `ego_tree::NodeId`
is `Copy + Eq + Hash` and `tree.get(id)` is O(1); the fast idiom already
exists in this repo (`src/cleaner.rs:88-99`). This plan swaps the identity
type, removing both the string formatting and the full-tree scans. It also
fixes a latent panic in the same code (`sort_by(partial_cmp().unwrap())` on
f64 scores).

## Current state

- `src/content_extractor.rs` — all changes live here.

The identity helpers:

```rust
// src/content_extractor.rs:954-965
fn get_element_id(element: &ElementRef) -> String {
    format!("{:?}", element.id())
}

/// Find an element by our generated ID
fn find_element_by_id<'a>(document: &'a Html, id: &str) -> Option<ElementRef<'a>> {
    // This is a simplified approach - in production we'd need better element tracking
    // For now, search for elements and match by generated ID
    let all_selector = Selector::parse("*").unwrap();
    document.select(&all_selector).find(|&elem| get_element_id(&elem) == id)
}
```

Representative consumers (verified by reading; the compiler will find the
rest once the types change):

```rust
// src/content_extractor.rs:213-219
fn apply_link_density_penalty(document: &Html, scores: &mut HashMap<String, f64>) {
    for (element_id, score) in scores.iter_mut() {
        if let Some(element) = find_element_by_id(document, element_id) {
            let penalty = (1.0 - dom_utils::get_link_density(element)).max(0.0);
            *score *= penalty;
        }
    }
}

// src/content_extractor.rs:227-235 (inside find_best_candidate)
    let mut sorted_scores: Vec<_> = scores.iter().collect();
    sorted_scores.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());   // <-- latent NaN panic

    let top_candidates: Vec<(String, f64)> = sorted_scores
        .iter()
        .take(options.nb_top_candidates)
        .map(|(id, score)| ((*id).clone(), **score))
        .collect();
```

Other `find_element_by_id`/`get_element_id` call sites are inside
`score_candidates` (builds the `HashMap<String, f64>`; see the score
propagation at lines ~195-209), `find_best_candidate` (line ~245),
`promote_shared_top_candidate_parent`, `promote_high_scoring_parents`, and
`extract_article_content` (lines ~289-636). `grep -n "find_element_by_id\|get_element_id" src/content_extractor.rs`
at planning time returns ~16 hits.

- The in-repo exemplar of the target idiom:

```rust
// src/cleaner.rs:88-91
    let body_id = doc.select(&BODY_SELECTOR).next().map(|e| e.id());
    let root_id = body_id.unwrap_or_else(|| doc.tree.root().id());
    let root_el = ElementRef::wrap(doc.tree.get(root_id)?)?;
```

- `ego_tree` is a direct dependency (`Cargo.toml:15`, `ego-tree = "0.10"`).
  `scraper::ElementRef::id()` returns `ego_tree::NodeId`.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Build   | `cargo build` | exit 0 |
| Tests   | `cargo test` | all pass |
| Mozilla suite | `cargo test --test mozilla_test_suite` (after plan 002) | pass, same divergence set as baseline |
| Bench (optional) | `cargo bench` | completes; compare candidate-selection timings before/after |

## Scope

**In scope**:
- `src/content_extractor.rs` only.

**Out of scope** (do NOT touch):
- `src/cleaner.rs` (already uses the right idiom).
- `src/scoring.rs`, `src/dom_utils.rs` — unless a signature they expose takes
  the String id (verify with grep; at planning time they do not).
- Serialization behavior (`element_to_html`) — no output change is expected
  from this plan.

## Git workflow

- Branch: `refactor/nodeid-element-identity`
- Commit style: `refactor(content_extractor): use ego_tree NodeId as element
  identity` (+ a separate small commit for the sort fix:
  `fix(content_extractor): use total_cmp when sorting candidate scores`).
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Record baseline

Run the Mozilla suite and save its output (pass/fail names). If plan 002
hasn't landed, run `cargo test --test mozilla_test_suite -- --ignored
--nocapture` and save the printed counts + failing names.

**Verify**: you have a baseline list to diff against in Step 5.

### Step 2: Swap the map key type

Change `HashMap<String, f64>` to `HashMap<ego_tree::NodeId, f64>` at every
score-map site in `src/content_extractor.rs` (`score_candidates` return type,
`apply_link_density_penalty`, `find_best_candidate`,
`extract_article_content`, the promote functions, and the
`top_candidates: Vec<(String, f64)>` → `Vec<(NodeId, f64)>`). Where the code
currently calls `get_element_id(&elem)` to produce a key, use `elem.id()`.
Add `use ego_tree::NodeId;` at the top (check existing imports first).

Let the compiler drive: `cargo build`, fix every type error, repeat. `NodeId`
is `Copy`, so `.clone()`s on keys become copies — drop the `.clone()` calls
the compiler flags as unnecessary if clippy complains later.

**Verify**: `cargo build` → may still fail on `find_element_by_id` callers;
that's Step 3.

### Step 3: Replace resolution with O(1) lookup

Replace the body of `find_element_by_id` with the direct lookup and change
its signature:

```rust
fn find_element_by_id(document: &Html, id: NodeId) -> Option<ElementRef<'_>> {
    document.tree.get(id).and_then(ElementRef::wrap)
}
```

Delete `get_element_id` entirely. Update all callers (they now pass `NodeId`
by value).

**Verify**: `cargo build` → exit 0;
`grep -n "get_element_id\|format!(\"{:?}\"" src/content_extractor.rs` → no
identity-related hits;
`grep -c "Selector::parse(\"\\*\")" src/content_extractor.rs` → 0.

### Step 4: Fix the NaN-unsafe sort

In `find_best_candidate`, replace

```rust
sorted_scores.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
```

with

```rust
sorted_scores.sort_by(|a, b| b.1.total_cmp(a.1));
```

`f64::total_cmp` is total ordering — no unwrap, no panic, identical order for
non-NaN values.

**Verify**: `cargo build` → exit 0.

### Step 5: Regression check

Run the full suite and the Mozilla suite; diff against Step 1's baseline.
The pass/fail sets must be IDENTICAL — this plan must not change extraction
results at all. HashMap iteration order was already nondeterministic with
String keys, so any order-sensitivity was pre-existing; if results differ
between runs, see STOP conditions.

**Verify**: `cargo test` → pass; Mozilla suite results == baseline.

## Test plan

No new unit tests required — this is a pure identity-representation change
guarded by the existing 154 lib tests, 117 markdown tests, and the Mozilla
suite (plan 002). If you want one cheap guard: a unit test asserting
`find_element_by_id(&doc, elem.id())` returns an element whose `id()` equals
the input, in the existing `mod tests` of `src/content_extractor.rs`.

## Done criteria

- [ ] `cargo build` and `cargo test` exit 0
- [ ] `grep -n "get_element_id" src/content_extractor.rs` → no matches
- [ ] `grep -n 'HashMap<String, f64>' src/content_extractor.rs` → no matches
- [ ] `grep -n "partial_cmp" src/content_extractor.rs` → no `.unwrap()` on a
      score comparison
- [ ] Mozilla suite pass/fail set identical to Step 1 baseline
- [ ] `git status` shows only `src/content_extractor.rs` (+ plans/README.md)
- [ ] `plans/README.md` status row updated

## STOP conditions

- Mozilla-suite results differ from baseline after the swap — the String keys
  may have been colliding or the code may depend on resolution failure
  somewhere; report the differing cases rather than tuning.
- Results differ BETWEEN two runs of the same build (nondeterminism surfaced
  by iteration order) — report; the fix (deterministic tie-break in sorting)
  is a separate decision.
- A consumer outside `src/content_extractor.rs` takes the String id (grep
  before starting: `grep -rn "get_element_id\|find_element_by_id" src/`).

## Maintenance notes

- After this lands, `scores` keys are only valid for the `Html` document they
  came from. The current code already only mixes documents in the retry loop
  (each attempt re-parses); if plan 009 later threads one tree end-to-end,
  NodeId identity composes cleanly with it.
- Reviewer focus: every former `.clone()` of an id, and the promote-functions'
  ancestor walks — confirm they still climb via `ElementRef` parents, not via
  re-resolution.
