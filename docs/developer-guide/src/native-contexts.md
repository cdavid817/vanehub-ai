# Native bounded contexts

Native code is organized by ownership rather than by UI page.

| Context | Owns |
| --- | --- |
| `agent_runtime` | Agent catalog, provider execution, terminal sessions, loops, Multi-Agent coordination |
| `sessions` | Sessions, messages, categories, chat configuration, export, usage |
| `workspaces` | Projects, worktrees, bounded file/Git queries, PTY shells |
| `tooling` | CLI, MCP, SDK, extensions, plugins, Skills, Prompt Hooks |
| `communications` | Connector configuration, credentials, routing, inbound delivery |
| `desktop` | Settings, paths, startup, window, tray, and floating lifecycle |
| `operations` | Observable operations and unified diagnostic/operation logging contracts |
| `retrieval` | Agent-memory and workspace-code indexing, Tree-sitter parsing, FTS/vector search, embedding confirmation, and code-index audit metadata |

A context publishes an `api.rs` facade for in-process consumers. Other contexts must not reach into its repository or infrastructure modules. Bootstrap modules compose concrete dependencies at the application edge.

Tauri commands are transport adapters, not business services. Cross-command error values are mapped to safe strings or explicit transport error DTOs.

## Retrieval and workspace code

`retrieval` owns the persistent code-index workspace identity, configuration, file manifests, chunks, symbols, vectors, and bounded local audit records. It consumes workspace roots at the composition edge but does not import the `workspaces` repository. `agent_runtime` consumes only the typed code-retrieval port and supplies the current session folder; models cannot provide a workspace id or folder to `search_code`.

The native worker performs metadata-first reconciliation and reads or parses only new or changed files. Tree-sitter grammars, chunking queries, and redaction policy share a version marker. Workspace-code embedding is gated by an explicit confirmation tied to workspace id, generation, provider profile, and model. FTS remains workspace-scoped and available before confirmation; vectors from another workspace or model are never candidates.

Native diagnostics use the unified logging port and contain only safe ids, phases, counts, durations, model ids, and reason categories. Normalized relative paths remain only in the bounded SQLite audit table. Raw code, search queries, credentials, detected secret values, absolute paths, and provider bodies are excluded from code-index diagnostics and telemetry.

For the full implemented context and command inventory, read [`src-tauri/ARCHITECTURE.md`](../reference/native-architecture.md) alongside the generated [native API reference](native-api-reference.md).
