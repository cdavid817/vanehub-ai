## Why

Cross-session memory is stored as rows in `agent_memories` whose only payload is a bare `content` blob. A memory has no name, no description, and no type, so nothing in the system can address one — and a record the model cannot address is a record it cannot update, merge, or retract. The store is therefore INSERT-only by construction (`memory_repository.rs` says so in its own comment), which means the pool grows monotonically while `format_memory_section` injects it newest-first under a 4,000-character budget. Past roughly fifty entries the tail is unreachable: the feature behaves as a FIFO window over recent extractions rather than as memory.

Giving each memory a file, a stable name, a one-line description, and a type turns it into an addressable unit. That single change is what unlocks deduplication, correction, semantic organization, and direct user editing — none of which are reachable from the current schema.

## What Changes

- Memories move from `agent_memories` rows to Markdown files in a host-level memory directory under the app data directory, one memory per file, with `name` / `description` / `type` frontmatter. **BREAKING**: `agent_memories` stops being the source of truth for memory content.
- A closed four-value memory type taxonomy (`user`, `feedback`, `project`, `reference`) is introduced. A missing or unrecognized type degrades gracefully rather than failing the read, so migrated and hand-written files keep working.
- The directory carries a `MEMORY.md` index file — one line per memory, pointer only, never memory content. This change establishes and maintains the index; consuming it as the injected surface is deferred to `add-two-tier-memory-recall`.
- `remember` keeps its tool name and its position in the tool catalog, and gains typed arguments. It writes a memory file plus its index entry instead of inserting a row. Keeping the name and ordering avoids invalidating the prompt-cache prefix and the fixed catalog-length assertions.
- OnePiece's generic file tools gain the memory directory as an auto-approved read and write scope, so the model can read an existing memory, correct it, or delete it — the operations the row store made impossible. Writes outside that directory are unaffected.
- Automatic extraction changes shape: instead of returning one opaque string to insert, it returns a bounded list of create / update / delete actions against named files, which the Rust layer validates and applies. Extraction remains a single non-agentic model call with no tool access, so the sandbox is a property of the construction rather than of the permission layer.
- CLI-wrapped agents keep their existing prompt-injection read path and their existing post-turn extraction write path unchanged in behavior; only the sink becomes the memory directory. Their prompts are not told about the directory, because several of them ship their own memory systems and a second set of persistence instructions would conflict.
- Existing rows migrate to files deterministically, without a model call: description is derived from the leading sentence, type is left absent, provenance (`agent_id`, `folder`, `source`, `created_at`) is preserved as frontmatter. The model can improve these in place afterwards, which it now can do.
- The retrieval index for `agent_memory` keys on the memory file's directory-relative path instead of a row id, and reconciles against a directory scan so that files deleted outside the app stop being recallable.

Affects both runtimes. The desktop/Tauri runtime gains the real file-backed store; the Web/mock runtime keeps contract and event parity through its adapter without a filesystem, as it does today for the row store.

## Capabilities

### New Capabilities

None. This change re-substrates an existing capability rather than introducing one.

### Modified Capabilities

- `agent-cross-session-memory`: memories become typed, named, addressable files rather than opaque rows; the model gains update and delete paths it did not have; automatic extraction returns structured actions instead of one string; injection continues to draw from the same shared host-level pool, now enumerated from the directory.
- `retrieval-vector-search`: the `agent_memory` source's document identity moves from a row id to a directory-relative file path, and reconciliation reads a directory scan rather than a table snapshot, so out-of-band file deletion revokes recall.

## Impact

- `src-tauri/src/contexts/agent_runtime/infrastructure/memory_repository.rs` is replaced by a directory-backed implementation; `memory_schema.rs` retains `agent_memories` only as the migration source.
- `src-tauri/src/contexts/agent_runtime/application/models.rs`: `AgentMemory` gains name, description, and type; `format_memory_section` enumerates the directory.
- `src-tauri/src/contexts/agent_runtime/infrastructure/memory_extraction_gateway.rs`: return type moves from `Option<String>` to a validated action list.
- `src-tauri/src/contexts/agent_runtime/application/tool_catalog.rs`: `remember` argument schema only. Tool names and catalog ordering are unchanged.
- Tool permission mapping: the memory directory becomes an auto-approved write scope for OnePiece's file tools.
- `src-tauri/src/contexts/retrieval/`: `agent_memory` document identity and reconciliation source.
- Frontend service boundary is preserved. `list_agent_memories`, `delete_agent_memory`, and `reset_agent_memories` keep their command names; their payloads gain the new metadata fields, and both the Tauri adapter and the Web/mock adapter change together.
- The memory directory is host-level and therefore shared across git worktrees, exactly as the SQLite database already is. Unlike the database it has no migration version to collide on, so concurrent branches degrade to redundant files rather than to startup failure.
- Frontmatter parsing, directory scanning, and prompt assembly follow the established `contexts/tooling/skills` implementation rather than introducing a second set of primitives.
