## Context

See proposal.md — Why for the region census and for why the ticket's phase-two plan no longer describes the file.

Three properties of the post-#169 file shape the split:

- The file is `api_process_adapter.rs` with a `api_process_adapter/tests.rs` child. Converting the parent to `api_process_adapter/mod.rs` puts parent and child in the same directory, which is where they already logically sit.
- `tests.rs` opens with `use super::*;`. After the split, `super` is `mod.rs`, so every item the tests reach must be re-exported there — this is the constraint that decides how much `mod.rs` must publish.
- `architecture.rs:1285` reads the adapter's source **text** by path and greps it for three behavior test names. #169 already repointed it once; whichever file ends up holding those tests must be what it reads.

## Goals / Non-Goals

**Goals:**

- One concern per file, with every external `use` path unchanged.
- Keep the diff mechanically checkable: a reviewer should be able to confirm "moves plus visibility" without re-reading 5,720 lines.
- Leave the residual honestly labelled rather than hidden behind a satisfied number.

**Non-Goals:**

- Decomposing `execute_with_code_intelligence`, or any other function. No function body changes.
- Reducing the `agent_runtime/infrastructure` subtree aggregate. This moves code within it; the aggregate is neutral apart from module boilerplate.
- Reaching "every file under 1,500 lines" as a headline. Two modules land near or above it, and the proposal says so.

## Decisions

### Cut on the census, not on the ticket's guess

The modules follow the measured regions. The **first** column below is the original estimate; the
**second** is the re-measured figure, taken by parsing every top-level item's span (declaration
plus its leading attribute/doc-comment block) and summing per module, rather than by reading
contiguous line ranges off the file. Four of the nine estimates were wrong, because a contiguous
range does not respect the item assignment in tasks.md §3:

| Module | Content | Estimated | Measured (moved lines) | With its import block |
|---|---|---:|---:|---:|
| `mod.rs` | struct, three impls, constants, type aliases, re-exports | ~500 | 431 | 579 |
| `native_tools.rs` | skill reads, registered tools, shell, code intelligence, remember/recall/search | ~1,412 | 1,391 | 1,478 |
| `execution.rs` | `execute_with_code_intelligence` and the skill-tool dispatch it drives | ~1,100 | 1,093 | 1,166 |
| `compaction.rs` | compaction, optimization, context snapshots | ~808 | **858** | 907 |
| `prompt.rs` | tool catalog, system prompt, personalization, memory formatting | ~440 | **363** | 388 |
| `invocation.rs` | accounting lifecycle, wire format | ~416 | 407 | 443 |
| `interactive.rs` | permissions, ask-user, plan exit, approval waiting | ~370 | **326** | 353 |
| `generation.rs` | generation entry, `GenerationOptions`, summarization, streaming, child turns | ~583 | 575 | 629 |
| `sinks.rs` | `EvidenceCountingSink`, `EvidenceToolCounts` | ~85 | **51** | 62 |

The four corrections, and why the estimate missed:

- `sinks.rs` 85 → 51. The 85 came from the proposal's "Constants, type aliases,
  `EvidenceCountingSink`" row, which bundles the constants and the two type aliases. tasks.md 3.9
  keeps both in `mod.rs`, so only the two structs and their two impls move.
- `prompt.rs` 440 → 363 and `compaction.rs` 808 → 858. The 440 was measured over a contiguous
  range that begins 67 lines early, at `turns_character_count`. Those 67 lines are
  `turns_character_count`, `value_character_count`, `should_compact`, and
  `compaction_notice_block` — the compaction predicates tasks.md 3.4 assigns to `compaction.rs`,
  whose own 808 was measured from a range that starts after them.
- `interactive.rs` 370 → 326. The 370 ran 37 lines past `plan_mode_denial` into
  `parse_optional_non_negative_integer_arg` and `non_negative_integer`, which parse tool-call
  arguments and belong with `native_tools.rs`.

tasks.md §3 also assigns no module to `failed_non_retryable`, `failed_configuration`, and
`failed_retryable`. They build `GenerationProcessEvent`s for four different modules and for
`mod.rs`'s own `AgentProcessGateway` impl, so they stay in `mod.rs` with the constants.

`native_tools.rs` stays just under 1,500 and `execution.rs` well under it, but only because
`execute_with_code_intelligence` is one item. Splitting either further would mean cutting
mid-concern to hit a number, which trades a real boundary for an arbitrary one.

### Visibility widens by the minimum, and the widening is the review surface

Every item a sibling module calls needs `pub(super)` or `pub(crate)`. That is the only class of change here that is not a move, so it is the thing to review. The rule: nothing becomes more visible than `pub(super)` unless it was already `pub(crate)` before the split, and any exception is recorded with its reason rather than applied silently.

**Outcome: the exception list is empty.** Every widening is `pub(super)`, which means "visible inside `api_process_adapter`" — the same boundary these items already sat behind as private items of one file. The twelve `pub(crate)` items after the split are the same twelve that were `pub(crate)` before it — `REQUEST_TIMEOUT`, `RuntimeAgentApiAdapter`, `GenerationOptions`, `WireFormat`, `ChildInvocationIdentity`, `begin_child_invocation`, `finish_child_invocation`, `context_snapshot_diagnostic`, `wire_format_for`, `summarize_turns`, `child_reply_turns`, `run_child_turn` — and none of them was widened or narrowed.

Three items also needed field- or method-level `pub(super)`, because a sibling reaches *through* them rather than only naming them:

- `WireFormat`'s eight function-pointer fields, invoked from `execution`, `compaction`, and `generation`. The struct's doc comment says those pointers "stay private"; `pub(super)` preserves exactly that, because the privacy boundary is still the adapter.
- `EvidenceToolCounts::{attempts, failures}`, read by `project_native_outcomes`, and `EvidenceCountingSink::{new, counts}`, called by `run_generation`.
- `AgentSkillToolLifecycle::{sink, tool_use}`, which `tests.rs` constructs directly.

### `mod.rs` re-exports for the tests, not for the world

`tests.rs`'s `use super::*;` means `mod.rs` must name everything the tests touch. That is a large surface, and it would be easy to let it become the module's public API by accident. Re-exports added solely to satisfy the test module are marked as such, so a later reader can tell "the tests need this" from "callers need this".

*Alternative considered — change `tests.rs` to import from the specific modules*: cleaner in principle, but it rewrites the test file, which #169 deliberately kept byte-identical. Keeping the glob preserves that guarantee for one more change.

### Verify by test-name inventory and a symbol census

Same control as #169: capture `cargo test --lib -- --list` before and after and require byte-identical output, unsorted included. Additionally, since this change moves production items rather than tests, capture the set of top-level item names before and after and require the multiset to be identical — a move that accidentally drops or duplicates an item shows up there and nowhere else.

## Risks / Trade-offs

- **A moved item silently changes meaning through a different `use` scope** → Each module gets an explicit import list rather than inheriting the parent's; if an item resolves differently, it fails to compile rather than resolving to the wrong thing.
- **Visibility creep turns internals into crate API** → The `pub(super)`-by-default rule plus a recorded exception list. `architecture.rs` already enforces context boundaries, so a widening that crosses a context fails the existing gate.
- **`tests.rs` stops compiling because `mod.rs` under-exports** → Expected and cheap to fix, but it must be fixed by adding a re-export, never by editing a test.
- **The subtree budget rises past what boilerplate explains** → Same stop-and-investigate rule as the earlier lanes; nine new modules cost roughly nine import blocks.
- **The residual 978-line function makes the change look incomplete** → It is incomplete, deliberately and in writing. The alternative is a relocation diff with a live-control-flow refactor buried in it.

## Residual after implementation

Nothing lands above 1,500 lines, but two modules are large enough to be recorded as
`[ARCH-NATIVE-006]` path budgets rather than left unremarked:

- `native_tools.rs`, 1,478 lines — 43 native tool implementations, the largest being
  `execute_tool_call_impl`'s 266-line dispatch.
- `execution.rs`, 1,166 lines — of which 978 are `execute_with_code_intelligence`, unchanged and
  awaiting its own change. Its text after the move is byte-identical to its text before, apart from
  the `pub(super)` qualifier on the declaration line.

The `agent_runtime/infrastructure` subtree rose from 58,072 to 58,357. All 285 lines are
scaffolding: 228 `use` lines, 22 `#[cfg(test)]` attributes on the test-only re-exports, 11 blank
separators, 8 module docs, 8 `mod` declarations, 4 comment lines heading `mod.rs`'s re-export
blocks, and 4 from rustfmt rewrapping two signatures that a `pub(super)` qualifier pushed past 100
columns. No body was duplicated.

## Migration Plan

No deployment or data migration. Compile-time reorganization; a revert is a plain `git revert`. The change is committable module by module, each leaving `cargo test` green, so it can pause at any module boundary.
