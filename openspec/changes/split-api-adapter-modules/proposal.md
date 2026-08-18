## Why

`extract-api-adapter-inline-tests` took `api_process_adapter.rs` from 13,927 lines to 5,720 by moving the test module out. What remains is 5,720 lines of production code in one file — still the largest production file in the native crate, and still holding eleven unrelated concerns behind one `use` path.

The optimization ticket's phase-two plan was written against the pre-split file and no longer describes it. It proposes a `noop_ports.rs` for the Noop and Unavailable port implementations — those were `#[cfg(test)]` scaffolding and left with the tests in #169, so that module would be empty. More importantly, it names none of the three regions that actually dominate the file:

| Region | Lines |
|---|---:|
| Native tool execution — skill reads, registered tools, shell background/output/kill, code intelligence, remember/recall/search | ~1,412 |
| `execute_with_code_intelligence` — **a single function** | ~978 |
| Context compaction and optimization | ~808 |
| Invocation accounting and wire format | ~416 |
| Tool catalog, system prompt, personalization, memory formatting | ~440 |
| `RuntimeAgentApiAdapter` and its three impls | ~390 |
| Permissions and interactive tools (ask-user, plan exit, approval) | ~370 |
| Summarization, streaming, child turns, memory extraction | ~343 |
| Generation entry and `GenerationOptions` | ~240 |
| Skill tool dispatch and lifecycle | ~125 |
| Constants, type aliases, `EvidenceCountingSink` | ~85 |

## What Changes

- Convert `api_process_adapter.rs` into a directory module, moving each region above into its own file, with `mod.rs` re-exporting so every external `use` path stays byte-identical.
- Keep the existing `api_process_adapter/tests.rs` child module working against the new layout.
- Lower the recorded path budget as the file shrinks.
- **`execute_with_code_intelligence` is moved, not decomposed.** See below.
- **No production behavior changes.** No function body, signature, trait implementation, or control flow is modified. Items move and gain the minimum visibility their new module requires.

## The 978-line function is deliberately left intact

The ticket's target was "each file under 1,500 lines". Moving `execute_with_code_intelligence` into its own module satisfies that number while improving nothing — the function is exactly as hard to read at the top of a 978-line file as it was in the middle of a 5,720-line one.

Decomposing it is the real work, and it is not a move: it means finding seams in live control flow that handles tool dispatch, code-intelligence fallback, and error propagation. That belongs in its own change, with its own review and its own risk budget, not smuggled into a relocation whose entire safety argument rests on "nothing changed but the file it lives in".

This change therefore reports it as a known residual rather than hiding it behind a satisfied line budget.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is a pure code-organization refactor with no externally observable behavior change: no Tauri command, SQLite schema, adapter contract, or runtime behavior is affected in either the desktop or Web runtime. The change sets `skip_specs: true`.

## Impact

- `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs` — becomes `api_process_adapter/mod.rs` plus the new region modules.
- `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter/tests.rs` — unchanged in content; its `use super::*;` must still resolve every item it exercises.
- `src-tauri/tests/architecture.rs` — the `api_process_adapter.rs` path budget becomes satisfied by absence and its entry is removed; the `agent_runtime/infrastructure` subtree budget of 58,072 continues to bind. There is also a guard at `architecture.rs:1285` that greps the adapter's source text by path, which #169 already had to repoint once — it will need repointing again.
- No frontend file is touched.
