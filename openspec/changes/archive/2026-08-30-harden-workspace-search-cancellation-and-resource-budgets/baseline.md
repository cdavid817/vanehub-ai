# Baseline: what the four behaviours actually were

Recorded so the change can be judged against what was there rather than against a description of it.
Every claim below names a revision and a symbol, so a reader can run `git show` and disagree.

The pre-change revision is `bece2e3c` — the merge base of this branch. Read a file from it with:

```
git show bece2e3c:src-tauri/src/contexts/workspaces/infrastructure/path_search.rs
```

**These behaviours no longer exist, so no test can characterize them.** A test asserting "the walk
collects two thousand candidates" would have to be deleted in the same commit that fixed it, which
is a test that never protected anything. What is recorded here instead is the measurement, its
source, and the gate that now fails if the behaviour returns — the gates live in
`src-tauri/src/contexts/workspaces/infrastructure/structural_performance_tests.rs` and they measure
the same four quantities.

## 1. Full candidate collection and a full sort

`path_search.rs` at `bece2e3c`:

- `walk_workspace_paths` returned `(Vec<Candidate>, Option<&'static str>)` and pushed every match
  into that vector up to `MATCH_COLLECTION_LIMIT = 2_000`.
- `search_session_paths` then called `candidates.sort_by(...)` over the whole vector and only
  afterwards took `limit` entries.

The constant's own doc comment said it: *"Bounded independently of the page size: ranking needs the
whole candidate set to order it, so this is the memory the search costs regardless of how few rows
the caller asked for."* A ten-row Quick Open held two thousand candidates.

| Quantity | Before | After |
| --- | --- | --- |
| Candidates retained for a 10-row page over 2 000 matches | up to 2 000 | ≤ 11 |
| Ordering cost | sort of the whole set | bounded selection, same order |

Gate: `ranking_retains_one_page_however_many_entries_match`.

## 2. Work performed beyond the result count

`content_search.rs` at `bece2e3c` began with:

```rust
let (candidates, walk_partial) = walk_workspace_paths(&root, "")?;
```

An empty query matches everything, so this materialized the candidate vector for the *entire*
workspace before a single file was opened — and then iterated it opening files until the match cap
was reached. The two costs were unrelated: the memory was decided by the workspace, the answer by
the caller's limit.

Path search had the same shape in a milder form: `MATCH_COLLECTION_LIMIT` was consulted instead of
the caller's `limit`, so asking for five rows did the work for two thousand.

| Quantity | Before | After |
| --- | --- | --- |
| Files opened for a 5-match content search over 400 matching files | up to 400 | ≤ 6 |
| Candidate vector built before the first file is opened | whole workspace | none |

Gate: `a_result_cap_stops_opening_files_rather_than_trimming_the_answer`.

## 3. Dependency-directory traversal

Three walks had their own exclusion list and a fourth had none.

- `path_search.rs` and `content_search.rs` consulted `is_excluded_directory(&name)`.
- The mention-candidate search consulted a second constant with a different list.
- `session_queries.rs::collect_documents` skipped `name.starts_with('.')` **and nothing else**. It
  descended into `node_modules`, `target`, `dist`, `vendor`, and every other generated tree, bounded
  only by `DOCUMENT_DEPTH_LIMIT = 6` and `DOCUMENT_LIMIT = 300`.

The observable consequence was a file findable by name and not by content, and a vendored `README.md`
in the Documents tab that no other surface would offer.

After: one `WorkspaceIgnorePolicy`, consulted by all four, with an architecture test —
`the_default_workspace_exclusions_have_exactly_one_owner` — that fails if a second list appears.

## 4. Blocking work launched without admission

`api.rs` at `bece2e3c`:

- `search_workspace_paths` called `self.inspection.search_paths(...)` directly. No registration, no
  cancellation token, no ceiling. A reader holding a key down started one filesystem walk per repeat.
- `search_workspace_content` registered a search id but acquired nothing. Registration was
  id-only — `self.searches.finish(&search_id)` — so a slow search finishing after a fast one had
  replaced it removed the *replacement's* registration.
- `list_session_documents` was synchronous, with neither a registration nor a ceiling, and it was a
  recursive walk of an entire project.

| Quantity | Before | After |
| --- | --- | --- |
| Concurrent inspection walks, globally | unbounded | 4 |
| Concurrent inspection walks, per workspace | unbounded | 2 |
| Refusal when the ceiling is reached | none — work started | `Unavailable` / `inspection_busy` |

Gates: the admission suite in `application/inspection_admission.rs`, and
`inspection_admission_is_acquired_only_by_the_published_workspace_api` in `tests/architecture.rs`.

## What this baseline does not claim

- **No timing.** Nothing here is a millisecond measurement, and none of the gates assert one. A
  duration would fail on a busy machine and pass on a fast one with a quadratic walk in it.
- **No remote numbers.** The remote helper's walk happens on a machine this suite does not have, and
  its bounds are asserted through the request it is sent rather than through what it spent.
