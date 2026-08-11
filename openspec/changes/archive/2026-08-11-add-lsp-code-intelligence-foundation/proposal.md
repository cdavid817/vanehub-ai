## Why

VaneHub AI's native API Agent can search and read code, but it cannot ask language tooling for definitions, references, inferred types, hover documentation, or current diagnostics. The newly merged Tree-sitter workspace index supplies fast structural retrieval and stable workspace metadata, making this the right point to add complementary live semantic intelligence without coupling language-server processes to the retrieval lifecycle.

## What Changes

- Add a native LSP server-management capability for trusted local workspaces, initially supporting `rust-analyzer` and `typescript-language-server` through bounded JSON-RPC 2.0 over stdio.
- Discover or accept configured server executables, negotiate server capabilities, manage per-project-root processes, restart crashes with bounded backoff, close idle servers, and perform protocol-aware shutdown during desktop exit.
- Add disk-authoritative document leases with version tracking, negotiated full/incremental synchronization, request cancellation, bounded concurrency, and diagnostics caching.
- Expose four read-only native API Agent tools: `find_definition`, `find_references`, `get_hover`, and `get_diagnostics`, with implicit current-workspace scope and explicit warming, timeout, unavailable, stale, and truncation metadata.
- Make the read-only LSP tools available in both normal and Plan Mode generations while preserving Plan Mode's mutation restrictions.
- Publish successful Agent file mutations through a native workspace-mutation boundary so LSP documents are invalidated immediately and an enabled Tree-sitter code index can run targeted reconciliation.
- Add desktop settings for the master switch, per-language enablement, executable discovery/override, bounded initialization options, trusted workspaces, server testing, and runtime status. Add contract-compatible deterministic Web/mock behavior without launching processes or reading the filesystem.
- Route LSP lifecycle and failure diagnostics through unified logging with bounded, redacted metadata; raw protocol messages, source content, diagnostics text, stderr, environment values, and private absolute paths are not persisted.
- Defer additional languages, rename/completion/code actions, filesystem watching, remote workspaces, OS-level process sandboxing, and persistent LSP enrichment of the Tree-sitter index to later changes.

## Capabilities

### New Capabilities

- `lsp-server-management`: Configuration, trust, discovery, process pooling, JSON-RPC transport, document synchronization, health, status, and bounded shutdown for local language servers.
- `lsp-code-intelligence`: Workspace-scoped semantic query contracts, normalized results, diagnostics snapshots, Agent tool availability, and Web/mock parity.

### Modified Capabilities

- `agent-tool-execution`: Add the four conditional read-only LSP tools, their execution limits, workspace isolation, cancellation, and visible tool-result behavior.
- `agent-chat-configuration`: Retain the read-only LSP tools in Plan Mode while rejecting future or unadvertised mutating LSP operations.
- `workspace-code-indexing`: Trigger targeted Tree-sitter reconciliation after successful Agent file mutations without making code indexing a prerequisite for LSP.
- `settings-center-ui`: Add service-backed LSP configuration, trust, discovery, test, and server-status surfaces.
- `unified-log-management`: Define redacted native logging requirements for LSP server lifecycle, protocol failures, restarts, timeouts, and shutdown.

## Impact

- Desktop runtime: new Rust `code_intelligence` context, SQLite-backed configuration, managed child processes, stdio JSON-RPC transport, document and diagnostics state, Tauri commands, bootstrap adapters, shutdown integration, and Agent Runtime ports/tools.
- Frontend: new LSP contracts and settings/status UI through `AgentService`; `tauri-agent-client.ts` and `web-agent-client.ts` remain interface-compatible, and React components do not call Tauri directly.
- Existing code indexing: receives best-effort targeted mutation notifications through an API boundary; its configuration, workspace lifecycle, local/semantic modes, search behavior, and persistence ownership remain unchanged.
- Dependencies: add a reviewed LSP type dependency and any narrowly scoped protocol/process support selected by the design tasks; no frontend state-management or UI framework changes.
- Security: configured language servers are trusted local executables, not an OS sandbox. Workspace trust and application-side URI/result validation limit automatic activation and Agent-visible output, but do not claim to restrict the child process's operating-system file access.
