# Baseline: Current Production Call Sites (Task 0.3)

Recorded before any implementation change, against `main` at `c37caa4a`. Line numbers are the
positions observed at that commit and will drift as the change lands; the module and symbol names
are the durable part.

## Memory persistence boundary

| Concern | Location |
|---|---|
| Port definition (`save` / `list_all` / `delete` / `delete_all`) | `src-tauri/src/contexts/agent_runtime/application/ports.rs` — `AgentMemoryPort` |
| Markdown store implementation | `src-tauri/src/contexts/agent_runtime/infrastructure/memory_directory.rs` |
| **200-file scan cap** | `memory_directory.rs:24` — `const MAX_SCANNED_FILES: usize = 200` |
| Capped enumeration | `memory_directory.rs:83` — `scan()`, truncated at line 130 |
| **Reset reuses the capped scan** | `memory_directory.rs:179` — `delete_all()` iterates `self.scan()?` |
| Name-as-identity replacement write | `memory_directory.rs:343` — `save()`; saving under an existing name replaces that file |
| Memory document domain type | `src-tauri/src/contexts/agent_runtime/domain/memory_document.rs` |

`delete_all()` iterating a scan that is truncated to 200 is the concrete data-loss path the proposal
names: a reset on a directory holding more than 200 memories silently leaves the remainder behind.

## Unscoped `list_all` production readers

| Caller | Location |
|---|---|
| OnePiece system-prompt memory index | `agent_runtime/infrastructure/api_process_adapter/prompt.rs:352` |
| OnePiece memory-extraction instruction | `agent_runtime/infrastructure/api_process_adapter/generation.rs:622` |
| CLI prompt memory index | `agent_runtime/application/service.rs:2597` |
| Extraction "existing memories" manifest | `agent_runtime/application/service.rs:3993` |
| Settings management list | `agent_runtime/application/service.rs:1907` → `api.rs:734` → `commands/agent_runtime/list_agent_memories.rs` |
| Retrieval reconciliation | `bootstrap/retrieval.rs:899` |
| Runtime bootstrap wiring | `bootstrap/agent_runtime.rs:526` |

Every one of these is a policy-free read. They are the call sites Task 13.6 must eliminate or
convert to snapshot-scoped resolution.

## Custom-instruction assembly

| Concern | Location |
|---|---|
| Cross-context port | `agent_runtime/application/ports.rs` — `AgentPersonalizationPort` |
| Adapter over `desktop` settings | `agent_runtime/infrastructure/personalization_gateway.rs` — `RuntimeAgentPersonalizationAdapter` |
| Settings domain fields | `contexts/desktop/domain/settings.rs` |
| OnePiece system-prompt section | `agent_runtime/infrastructure/api_process_adapter/prompt.rs` |
| Value object + block rendering | `agent_runtime/application/models.rs` — `PersonalizationSettings`, `custom_instructions_block()`, `safe_fallback()` |

## CLI prompt assembly (the ordering contract this change must preserve)

`src-tauri/src/contexts/agent_runtime/application/service.rs:2572-2627`

Current order, assembled **after** Prompt Hook rendering so hook templates still see the user's
original message:

```
custom_instructions  ->  memory_section  ->  Prompt-Hook-assembled content
```

Failure behavior already matches the change's intent on the instruction side: a personalization
lookup failure degrades to `PersonalizationSettings::safe_fallback()` and logs, rather than
blocking delivery. The memory side currently degrades to "no memory section" on a `list_all`
failure. Neither path is fail-closed with respect to *policy*, because there is no policy to fail
closed against yet.

## Session creation

| Concern | Location |
|---|---|
| Application service | `contexts/sessions/application/service.rs` |
| Ports | `contexts/sessions/application/ports.rs` |
| SQLite repository | `contexts/sessions/infrastructure/sqlite_repository.rs` |

`personalizationMode` does not exist on the session record today; Group 8 adds it.

## Tauri commands and frontend adapters

| Concern | Location |
|---|---|
| List memories command | `src-tauri/src/commands/agent_runtime/list_agent_memories.rs` |
| Delete memory command | `src-tauri/src/commands/agent_runtime/delete_agent_memory.rs` |
| Reset memories command | `src-tauri/src/commands/agent_runtime/reset_agent_memories.rs` |
| Frontend service contract | `src/services/agent-memory-service.ts` — `listAllMemories`, `deleteAgentMemory` |
| Tauri adapter | `src/services/tauri-agent-client.ts:375`, `:397` |
| Web/mock adapter | `src/services/web-agent-client.ts` |
| Settings page | `src/settings/pages/personalization-page.tsx` |
| Instructions section (blur-based drafts) | `src/settings/pages/personalization/custom-instructions-section.tsx` |
| Memory section (flat full-body list) | `src/settings/pages/personalization/agent-memory-section.tsx:44`, `:49` |

## SQLite migration registry

`src-tauri/src/platform/database/migrations/mod.rs` — highest version on `main` is
`(81, "cli-parameter-profiles")`. The registry enforces a dense version history, so this change
takes **82**.

**Risk recorded, not resolved:** two unmerged local worktree branches already define a version 82
(`cli-version-catalogs`, `extension-platform-gate-degradations`). All worktrees share one
`%APPDATA%\ai.vanehub.app\vanehub.sqlite`, so running two of those branches against the same
database produces the "version recorded but schema differs" contamination class. This is a
pre-existing cross-branch condition resolved at merge time, not something this change can pick a
different number to avoid — the density check rejects skipping to 83.

## Compatibility boundary that must NOT change

`openspec/specs/agent-context-compaction/spec.md` is an explicit non-goal. OnePiece keeps its
VaneHub-native compaction semantics; every CLI keeps its own internal compaction and its native
memory/instruction files.

## Baseline test results (Task 0.5)

Captured on this worktree at `c37caa4a` before any source change:

| Suite | Command | Result |
|---|---|---|
| Frontend unit/component | `npm run test` | 302 files, **1402 passed**, 0 failed |
| Rust workspace | `cargo test --workspace` | **3793 passed**, 0 failed, 15 ignored |
| Change artifacts | `openspec validate add-unified-personalization-governance --strict` | valid (after the Task 0.2 fixes) |
| Canonical specs | `openspec validate --specs --strict` | 136 passed, 0 failed |

`cargo test` must be run with `all_proxy` unset; the ambient `socks5://127.0.0.1:9999` proxy makes
loopback-bound test clients fail with a bogus 502.
