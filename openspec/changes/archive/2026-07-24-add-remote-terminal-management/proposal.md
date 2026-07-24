## Why

Remote workspaces and reusable SSH profiles already exist, but remote sessions still cannot open an authenticated interactive Shell. Developers also lack a safe way to reuse SSH transports, run repeatable remote commands, and search prior Terminal output without relying on diagnostic log files.

## What Changes

- Add an authenticated SSH remote Terminal runtime for desktop remote-workspace sessions, including host-key verification, password and key authentication, independent PTY channels, and a bounded reusable connection pool.
- Bind remote sessions to an SSH connection profile and profile revision while preserving the existing immutable remote-workspace snapshot for historical display.
- Add reusable command templates with global, connection, and workspace scopes, plus explicit insert and quick-execute actions and structured command-run history.
- Capture bounded normalized remote Terminal and quick-command output in SQLite, index it with FTS5, and provide paginated full-text search, retention, capacity, and explicit deletion controls.
- Keep raw interactive input out of command history and persistent diagnostics; Terminal content persistence remains separate from the unified diagnostic log service.
- Preserve Web/mock service parity through deterministic simulated remote Terminal, template, run-history, capture, and search behavior without creating network connections or storing real credentials.
- Keep remote Agent CLI execution, SFTP/file browsing, remote Git inspection, jump hosts, agent forwarding, and automatic interactive-process restoration out of scope.

## Capabilities

### New Capabilities

- `remote-terminal-runtime`: Covers authenticated SSH transport pooling, host-key verification, remote PTY channel lifecycle, reconnection behavior, events, and Web simulation.
- `terminal-command-management`: Covers reusable command templates, scopes, explicit insertion and quick execution, structured run history, and secret-safe command handling.
- `terminal-output-search`: Covers bounded SQLite output capture, normalized searchable content, FTS5 queries, pagination, retention, capacity limits, capture gaps, and deletion.

### Modified Capabilities

- `session-management`: Remote sessions retain their workspace snapshot while gaining an explicit SSH profile and revision binding with safe edit, delete, and rebind behavior.
- `session-shell`: The Shell service supports remote SSH-backed sessions while preserving input, resize, cleanup, status, and diagnostic boundaries.
- `ssh-connection-management`: SSH profiles support authenticated runtime use, host-key trust state, profile revisions, and pooled-connection invalidation in addition to TCP reachability.
- `unified-log-management`: Remote Terminal lifecycle diagnostics use unified redacted logs while raw commands and Terminal content remain in their dedicated user-content stores.

## Impact

- Desktop runtime: adds a Rust-owned SSH client/runtime boundary, authenticated connection pool, host-key verification, remote PTY and exec channels, SQLite repositories, FTS5 schema, cleanup jobs, and Tauri commands.
- Frontend: extends the existing Agent service boundary and matching Tauri/Web adapters; React continues to avoid direct `invoke()` calls.
- UI: extends the existing Shell tab with remote state, connection trust/rebind feedback, command templates, run history, output search, capture status, and cleanup controls.
- Persistence: adds additive session binding columns and Terminal template, run, output-chunk, search-index, and retention metadata tables; credentials remain in native secure storage.
- Dependencies: requires an approved cross-platform Rust SSH implementation capable of multiplexing authenticated channels; no alternative frontend state, UI, or database technology is introduced.
- Security and privacy: introduces persistent user Terminal content, requiring bounded capture, redaction-aware diagnostics, host-key verification, configurable capture, and explicit purge behavior.
