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

The modules follow the measured regions:

| Module | Content | Approx |
|---|---|---:|
| `mod.rs` | struct, three impls, constants, type aliases, re-exports | ~500 |
| `native_tools.rs` | skill reads, registered tools, shell, code intelligence, remember/recall/search | ~1,412 |
| `execution.rs` | `execute_with_code_intelligence` and the skill-tool dispatch it drives | ~1,100 |
| `compaction.rs` | compaction, optimization, context snapshots | ~808 |
| `prompt.rs` | tool catalog, system prompt, personalization, memory formatting | ~440 |
| `invocation.rs` | accounting lifecycle, wire format | ~416 |
| `interactive.rs` | permissions, ask-user, plan exit, approval waiting | ~370 |
| `generation.rs` | generation entry, `GenerationOptions`, summarization, streaming, child turns | ~583 |
| `sinks.rs` | `EvidenceCountingSink`, `EvidenceToolCounts` | ~85 |

`native_tools.rs` and `execution.rs` exceed 1,500 and near it respectively. Splitting them further would mean cutting mid-concern to hit a number, which trades a real boundary for an arbitrary one.

### Visibility widens by the minimum, and the widening is the review surface

Every item a sibling module calls needs `pub(super)` or `pub(crate)`. That is the only class of change here that is not a move, so it is the thing to review. The rule: nothing becomes more visible than `pub(super)` unless it was already `pub(crate)` before the split, and any exception is recorded with its reason rather than applied silently.

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

## Migration Plan

No deployment or data migration. Compile-time reorganization; a revert is a plain `git revert`. The change is committable module by module, each leaving `cargo test` green, so it can pause at any module boundary.
