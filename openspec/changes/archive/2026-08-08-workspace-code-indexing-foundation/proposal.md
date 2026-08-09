## Why

VaneHub AI's retrieval pipeline currently indexes only the host-wide agent memory pool, so agents cannot semantically locate code without repeatedly scanning the active workspace. A workspace code index must establish strict workspace isolation, safe file admission, incremental file-level reconciliation, and structured source locations before real-time watching or advanced local embedding can be added safely.

## What Changes

- Add per-workspace code-index configuration with a stable workspace identity, enablement, selected roots and languages, user exclusion patterns, and a maximum file size.
- Add mandatory sensitive-file denial and code-secret redaction before any content is persisted for retrieval or sent to an embedding provider.
- Add Tree-sitter parsing and symbol-aware chunking for the initially supported language set, backed by a persistent file manifest, chunk metadata, grammar/index version, and workspace-scoped status.
- Generalize reconciliation and embedding queues by source kind while preserving the existing host-wide semantics of agent memory.
- Add selective file reconciliation for created, changed, renamed, and deleted paths, with a periodic manifest audit as a recovery path for missed changes.
- Add provider-aware cross-batch throttling, cooperative cancellation, bounded failures, and per-workspace progress and cost accounting.
- Add a `search_code` agent tool that implicitly uses the current session workspace and returns typed file, line, language, and symbol metadata. Existing `read_file` remains the context-expansion mechanism.
- Extend the OnePiece retrieval settings and workspace UI through the existing frontend service boundary, with matching Tauri and Web/mock contracts.
- Retain index data when a workspace closes or indexing is disabled; provide explicit rebuild and delete actions.
- Defer filesystem watching and debounce, concurrent workspace indexing, public `search_symbols`, call-graph expansion, and local embedding runtimes to follow-up changes.

## Capabilities

### New Capabilities
- `workspace-code-indexing`: Workspace identity and configuration, safe file admission, Tree-sitter file/chunk manifests, scoped code search, lifecycle, progress, and retention behavior.

### Modified Capabilities
- `retrieval-vector-search`: Generalize indexing, embedding queues, status, model invalidation, and hybrid candidate queries to support source-specific scope semantics without changing the shared agent-memory pool.

## Impact

- Desktop runtime: new Rust Tree-sitter grammar dependencies, SQLite migrations, workspace file adapters, source-aware retrieval services, worker scheduling, Tauri commands, and unified redacted diagnostics.
- Agent runtime: a workspace-bound `search_code` tool and typed code retrieval port; the existing `recall` tool and global memory behavior remain unchanged.
- Frontend and Web runtime: workspace index configuration, progress and management contracts added to `agent-service.ts`, `tauri-agent-client.ts`, and `web-agent-client.ts`; React components continue to avoid direct Tauri calls.
- Persistence: new workspace/file/symbol metadata and source-aware retrieval fields or companion tables; existing retrieval rows remain compatible.
- Security: code may be sent to the configured external embedding provider only after mandatory file filtering and content redaction, and persisted diagnostics must not contain code, raw queries, credentials, or private file paths.
