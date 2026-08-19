## Context

See proposal.md — Why for the measurements, the corrected premise, and the evidence that `src-tauri/tests/` is unreachable.

Four facts about the two files constrain the design:

- Both are already `#[cfg(test)] mod tests;` child module files. The only structural move still available is a further split into grandchild modules.
- Each file has a large block of shared test scaffolding that most of its tests depend on. In `agent_runtime/application/tests.rs` this is ~1,600 lines: `FakeWorld` and its ~25 port implementations, plus `FailingExecutionTelemetry`, `FakeMessageTerminalCompletions`, and the `agent()`/`api_agent()`/`chat_configuration()` builders. In `sessions/infrastructure/tests.rs` it is `Fixture`, `fixture()`, `session_record()`, `message_record()`, `correlated_message_record()`, `usage_record()`, and seven evidence/logging port doubles.
- A child module sees every private item of its ancestors. This is the property `extract-api-adapter-inline-tests` relied on, and it applies one level deeper here: `use super::*;` in `tests/<group>.rs` resolves both the items the parent defines and the private `use` bindings the parent declares.
- Neither `sessions/infrastructure` nor `agent_runtime/application` has a registered subtree budget, so only the two path budgets move.

## Goals / Non-Goals

**Goals:**

- Replace two undifferentiated multi-subject test buckets with subject-named modules, so a reviewer changing `usage_accounting.rs` finds its tests in `tests/usage_accounting.rs` rather than by searching 5,110 lines.
- Move every test body verbatim, with no visibility widening anywhere.
- Keep the diff mechanically auditable: each new file is one contiguous cut from the original, so "nothing but moves" is verifiable without reading 7,000 lines of test bodies.

**Non-Goals:**

- Any compile-time improvement. Explicitly not promised; see proposal.md.
- Relocating anything to `src-tauri/tests/`. Proven impossible; the goal is withdrawn.
- Rewriting, deduplicating, renaming, re-grouping, or re-ordering any test.
- Touching `tooling/skills/application/tests.rs`, or any production module.
- Splitting a group of tests that share one subject. That is the case the repository's `max-lines` test exemption correctly protects.

## Decisions

### Split by subject seam, along contiguous line ranges

Each new module is one uninterrupted range of the original file. No test moves past another, and no range is assembled from two places. This is what makes the claim "pure move" checkable: the reviewer confirms the ranges partition the file, and the test-name inventory confirms nothing was lost.

`sessions/infrastructure/tests.rs` (5,110 lines) partitions into seven subject ranges plus four retained blocks. Ranges are inclusive and start at an item's first attribute line, not its `fn` line:

| New module | Source range | Lines | Subject |
|---|---|---:|---|
| *(retained)* | 1–52 | 52 | imports and `Fixture` |
| `tests/usage_accounting.rs` | 53–838 | 786 | ledger idempotence, cursor epochs, projection semantics, accounting cardinality |
| *(retained)* | 839–1,227 | 389 | evidence/logging port doubles, `fixture()`, record builders, two tests among them |
| `tests/generation_lifecycle.rs` | 1,228–1,994 | 767 | generation claim/terminal atomicity, concurrency, crash-reopen partial writes |
| `tests/terminal_evidence.rs` | 1,995–2,311 | 317 | evidence ordering, bounding, quarantine of malformed payloads |
| `tests/recovery.rs` | 2,312–3,586 | 1,275 | candidate scan, claim revisions, publication, coordinator passes, file-backed recovery |
| *(retained)* | 3,587–3,832 | 246 | SSH binding, `usage_record()`, loop ownership, repository round-trips |
| `tests/search.rs` | 3,833–4,280 | 448 | FTS indexing, ranking, query plans |
| *(retained)* | 4,281–4,429 | 149 | active-session clearing, row mapping, transaction rollback |
| `tests/legacy_usage_retirement.rs` | 4,430–4,774 | 345 | post-cutover behavior of the retired usage table |
| `tests/configuration_and_seats.rs` | 4,775–5,110 | 336 | chat configuration mapping, seats, stable participants |

`agent_runtime/application/tests.rs` (4,628 lines) partitions into seven ranges plus one retained block:

| New module | Source range | Lines | Subject |
|---|---|---:|---|
| *(retained)* | 1–1,844 | 1,844 | imports, `FakeWorld` and its ~25 port impls, agent builders, two tests among them |
| `tests/onepiece_provider.rs` | 1,845–2,293 | 449 | OnePiece configuration, profiles, credentials, model discovery |
| `tests/embedding_models.rs` | 2,294–2,498 | 205 | embedding endpoint resolution and model listing |
| `tests/api_agent_management.rs` | 2,499–2,652 | 154 | API agent update/delete and credential handling |
| `tests/message_dispatch.rs` | 2,653–3,453 | 801 | launch, send, telemetry, tool lifecycle, streaming, completion accounting |
| `tests/loop_and_stream_failures.rs` | 3,454–3,755 | 302 | loop role generation, cancellation races, safe terminal errors |
| `tests/prompt_composition.rs` | 3,756–4,423 | 668 | custom instructions, memory injection ordering, extraction triggers |
| `tests/tool_approval_and_local_profiles.rs` | 4,424–4,628 | 205 | tool approval resolution, local/custom profile rules |

Each moved range gains a `use super::*;` header and each parent gains one `mod` declaration per child, so the only lines the split adds are boilerplate: 7 `mod` declarations plus 7 `use super::*;` per split file — 14 each, 28 across both. The post-split parents measure 843 and 1,851 lines.

### Shared scaffolding stays in the parent, and is not itself modularized

`FakeWorld` and the sessions fixtures remain in `tests.rs`. Each child module opens with `use super::*;` and sees them unchanged.

*Alternative rejected — move `FakeWorld` to `tests/world.rs`*: it would shrink the parent to a near-empty coordinator, but sibling modules cannot see a private item in a sibling, so `FakeWorld` and every type in its signatures would need `pub(super)`. That trades the change's strongest safety property — zero visibility edits — for a cosmetic line count. It also splits one cohesive test double away from nothing that benefits.

This leaves `agent_runtime/application/tests.rs` at roughly 1,850 lines, of which ~1,600 is the single `FakeWorld` double. That is one subject, and the `max-lines` test exemption's cohesion argument applies to it directly. Shrinking it further is a separate question about whether `FakeWorld` should be that large, which is a test-design change, not a file move.

### No visibility widening, and any need for one is a stop signal

The design predicts zero `pub`/`pub(super)` edits. If a moved test fails to compile without one, that means the item was reached by a path the analysis missed, and it must be recorded and reviewed rather than patched silently. In particular the three `pub(super)` items in `sqlite_repository` (`allocate_message_sequences`, `indexed_search_statement`, `compatibility_search_statement`) stay `pub(super)`: their consumers move from a child of `infrastructure` to a grandchild, which still lies inside `infrastructure`.

### Verify by test-name inventory, in both sorted and unsorted form

Capture `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --list` before and after and require byte-identical output.

- **Sorted** proves the *set* of tests is unchanged — nothing dropped, nothing added, nothing renamed.
- **Unsorted** additionally pins each test's reported module path, which is what changes when a test lands in the wrong module. Requiring only the sorted list would let a test silently move between groups.

*Alternative rejected — comparing test counts*: a dropped test plus an accidentally duplicated one nets to zero.

Note that the unsorted list *is* expected to differ in module path for every moved test — that is the change. The check is therefore run in two stages: the sorted list of bare test names must be byte-identical, and the full unsorted listing must differ only in the module-path prefix of moved tests, with the same names in the same relative order within each moved range.

### The architecture gate identifies test code by file name, and that had to be fixed

Splitting the tests made `provider_neutral_layers_do_not_select_concrete_cli_providers` fail on three of the new modules for "branching on built-in provider id codex-cli". The tests did not change: they have referenced `"codex-cli"` for as long as they have existed, and were exempt because the rule skipped paths matching `ends_with("tests.rs")`. Moving them into `tests/` left the same code failing a rule it was always meant to be outside of.

Two other rules — `runner_contracts_and_adapters_use_only_published_runtime_boundaries` and `concrete_runtime_dependencies_are_assembled_only_in_bootstrap` — carry their own copy of the same file-name predicate and therefore the same latent gap. They did not fire, which is luck rather than design.

The fix is one `is_test_source` helper, used by all three, that recognizes a test module in either shape: a `tests.rs`/`*_tests.rs` file, or a file inside a `tests/` directory. It is covered by its own fixture test.

This widens no rule's intent. Test code was already exempt from all three; the predicate only stops the exemption from evaporating when a test module is split. The alternative — leaving the rule as-is and contorting the test code to avoid a literal it is entitled to use — would encode a gate defect into the tests, and would leave the next person to split a test module hitting the same wall.

### Budgets are lowered, not removed

`sessions/infrastructure/tests.rs` and `agent_runtime/application/tests.rs` both survive the split as real files, so their `NATIVE_PATH_BUDGETS` entries stay and drop to the measured post-split counts. A removed entry would let them regrow unobserved, which is the failure `freeze-large-file-line-budgets` exists to prevent. The `tooling/skills/application/tests.rs` entry is left untouched along with the file.

Neither subtree has a registered subtree budget, so — unlike `extract-api-adapter-inline-tests` — there is no aggregate ceiling to raise for the `mod` declarations the split adds. The aggregate does grow by roughly the number of new files times its per-file header, and nothing bounds it. That is a pre-existing gap in the budget table, not one this change introduces; recording subtree budgets for these two directories is left to whoever next needs them.

## Risks / Trade-offs

- **The split drops, duplicates, or misplaces a test** → The two-stage test-name inventory is the specific control, and it is a task rather than a closing check.
- **A moved test needs a visibility widening** → Predicted not to happen; if it does, it is recorded and reviewed, not applied silently. No production item may become more visible.
- **The benefit is smaller than the ticket claimed** → Acknowledged in the proposal rather than papered over. If a reviewer judges reviewability alone insufficient, the correct outcome is to reject this change and lower the two budgets in place, not to restore the disproven rationale.
- **Merge conflicts with concurrent lanes** → `tooling/skills/application/tests.rs` is excluded precisely because three lanes are adding to it. The two files in scope are not targeted by any in-flight change.
- **Test bodies referencing line numbers or file paths** → None do; the moved code addresses items by name only.

## Migration Plan

No deployment, data migration, or schema change. The change is compile-time reorganization of test code; a revert is a plain `git revert`.
