## Context

VaneHub already stores remote-workspace snapshots, manages SSH connection profiles and credentials, and exposes a local PTY Shell through the Agent service boundary. The SSH connection test is intentionally limited to TCP reachability, remote sessions do not retain the selected SSH profile id, and `WorkspaceShellApplicationService` rejects remote workspaces. Shell output is currently streamed to xterm and kept only for the live process; no FTS schema exists.

The change crosses React, Tauri/Web adapters, Rust application and infrastructure layers, secure credential storage, network security, SQLite, and background lifecycle work. Terminal content is user data rather than diagnostic data, so it requires a storage and retention boundary distinct from unified logs.

## Goals / Non-Goals

**Goals:**

- Open authenticated SSH PTY channels for remote-workspace Shell tabs.
- Reuse one authenticated SSH transport for multiple independent PTY and exec channels.
- Preserve safe endpoint identity by binding sessions to a profile revision while retaining workspace snapshots.
- Provide explicit command templates, quick execution, structured history, and searchable output.
- Keep UI streaming responsive when persistence is slow or unavailable.
- Preserve service parity with deterministic Web/mock behavior.
- Verify host identity and keep credentials and raw user content out of diagnostic logs.

**Non-Goals:**

- Running Codex, Claude Code, Gemini, or OpenCode as remote Agent runtimes.
- SFTP, remote Files/Documents, remote Git inspection, remote worktrees, jump hosts, or agent forwarding.
- Reconstructing commands from raw terminal keystrokes.
- Transparently restoring a remote interactive process after its SSH channel is lost.
- Persisting private-key contents or storing command secrets in templates.

## Decisions

### Decision: Extend the Shell boundary with a Rust-owned remote runtime

React continues to call `AgentService` operations. The Tauri adapter maps those operations to Rust commands; the Web adapter simulates them. Rust selects the local PTY runtime or remote SSH runtime after loading the session workspace and binding.

```text
ShellTab / command drawer / output search
                   |
             AgentService
            /            \
   Tauri adapter       Web adapter
          |                 |
 Workspace application   deterministic mock
      /          \
 local PTY     remote SSH coordinator
```

The existing Shell input, resize, kill, and event contract remains the common lifecycle surface. Remote-specific profile trust, rebind, template, run, and search operations are additive service methods.

Alternative: invoke SSH directly from React. Rejected because credentials, host trust, SQLite, and process/network lifecycle belong to the native boundary.

### Decision: Bind sessions to a profile revision and keep snapshots

Remote sessions gain nullable `remote_ssh_connection_id` and `remote_ssh_connection_revision` fields. A selected or newly saved SSH profile populates both fields while the existing host, port, user, path, display name, and URI remain the historical snapshot.

Opening a remote Terminal requires a live profile whose id and revision match the binding and whose endpoint matches the snapshot. Profile edits increment the revision, making existing sessions require explicit rebind. Profile deletion leaves snapshots readable and clears or invalidates the operational binding.

Alternative: find credentials by matching host, port, and user. Rejected because multiple profiles can share endpoints with different credentials and a profile edit could silently redirect an old session.

### Decision: Pool authenticated transports, not terminal channels

The pool key is `(connection_id, revision)`. Each entry has a connection state, a single-flight connection future, lease count, last-used timestamp, and failure state. Each Shell opens a distinct PTY channel and each quick command opens a distinct exec channel on the shared transport.

Profile mutation drains the prior revision; deletion and shutdown close matching transports. Idle zero-lease entries are evicted after a bounded interval, and the pool enforces a maximum size. Keepalive detects dead transports. A transport failure marks dependent channels failed; reopening creates a new channel but does not claim to restore the old process.

Alternative: start a system `ssh` process for every Terminal. Rejected because portable cross-platform multiplexing, password credentials, host-key prompts, and lifecycle ownership would depend on host-specific OpenSSH configuration.

### Decision: Use `russh` 0.62.4 with the `ring` crypto backend

Pin `russh` to `=0.62.4` with default features disabled and the `ring` backend enabled. Its Tokio-native client exposes a server-key callback, password and public-key authentication, concurrent session channels, PTY and shell requests, exec requests, window changes, split async channel I/O, keepalive, and explicit disconnect. This matches the existing Tokio runtime and the transport/channel pool model without adding a system OpenSSH dependency or a separately packaged C SSH library.

The adapter SHALL configure a curated modern algorithm set rather than enabling legacy algorithms merely because the library supports them. Dependency updates require the existing supply-chain review and all Windows native verification gates.

Alternative: `ssh2`/libssh2. It provides the required channel primitives and a mature client implementation, but adds C-library and crypto-backend build/link complexity to the Windows desktop package and its shared session handle is less natural for the planned async pool. Alternative: a system `ssh` process. Rejected because portable password authentication, host trust prompts, and multiplexing would depend on machine configuration.

### Decision: Require explicit host-key trust

The native runtime verifies the server key before authentication. A new or changed key produces a bounded fingerprint challenge exposed through the service boundary. Trust is recorded for the connection endpoint only after explicit user confirmation. A changed previously trusted key blocks connection until separately confirmed.

Diagnostics contain safe fingerprint and endpoint classifications, never passwords, credential references, private-key contents, or raw protocol frames.

Alternative: accept unknown keys automatically. Rejected because it defeats SSH server authentication.

### Decision: Quick execution is separate from interactive input

xterm `onData` contains keys and control sequences and can include password input, so it is not a reliable command-history source. Templates support two explicit actions:

- Insert writes the selected template into the active PTY without recording a run.
- Quick execute opens an exec channel, records an immutable command snapshot and structured lifecycle, and captures bounded stdout/stderr and exit status.

Templates can be global, connection-scoped, or workspace-scoped. Secret-like values are rejected from saved templates; future secure variables are outside this change.

Alternative: append a sentinel to commands typed in the PTY. Rejected because quoting, shell differences, full-screen programs, and user-controlled output make boundaries unsafe.

### Decision: Store normalized output in a dedicated bounded content store

Remote PTY and quick-command output is sent to the UI first and copied to a bounded capture queue. A background writer strips terminal control sequences, preserves UTF-8 boundaries, batches chunks in transactions, and writes ordered records. If the queue cannot accept data, the UI stream continues and one searchable gap marker is recorded when storage recovers.

`terminal_output_chunks` owns metadata and normalized text. An FTS5 index supports query snippets and filters by session, connection, Terminal, run, and time. The first version does not persist ANSI replay streams; live reattach can retain a bounded in-memory transcript.

Capture has an explicit enabled state, a fixed default retention window, a global capacity ceiling, per-session deletion, and scheduled maintenance. FTS and content rows are deleted transactionally.

Alternative: write every reader chunk synchronously. Rejected because database contention would block an interactive channel. Alternative: use unified log files. Rejected because searchable Terminal content is user data and current logging policy redacts and bounds diagnostics.

The first implementation uses these reviewed bounds: 8 pooled transports; 15-second connect, 30-second drain, 30-second keepalive, and 5-minute zero-lease idle timeouts; a 256-chunk capture queue with 32 KiB chunks and 32-chunk batches; a 1 MiB live transcript; 30-day output retention with a 512 MiB global ceiling; and search pages of 50 by default and 100 maximum with 512-byte query and cursor limits.

### Decision: Web mode simulates semantics without security claims

The Web adapter provides deterministic connection, channel, template, run, capture, and search fixtures through the same service interface. It labels the Terminal simulated, never performs network I/O, and never persists real credentials or claims host-key verification.

## Risks / Trade-offs

- [SSH library behavior or platform incompatibility] → Isolate it behind native ports, add adapter contract tests, and verify Windows first.
- [Host-key prompt races] → Deduplicate challenges per endpoint and reject authentication until trust persistence succeeds.
- [Profile edits strand sessions] → Keep snapshots visible and provide an explicit rebind flow rather than silently changing targets.
- [Large output grows SQLite and FTS rapidly] → Batch writes, normalize content, enforce age and capacity limits, and expose purge controls.
- [Terminal output contains secrets] → Never capture input, make capture visible/configurable, reject secrets in templates, and provide deletion; diagnostics remain redacted.
- [Persistence backpressure disrupts interaction] → Use a bounded non-blocking capture queue and gap markers while prioritizing the UI stream.
- [Transport loss destroys interactive state] → Report failure honestly; document tmux/screen integration as future work.
- [FTS tokenization performs poorly for paths or CJK] → Use a supported substring-oriented tokenizer after migration-time capability verification and cover multilingual/path queries.

## Migration Plan

1. Add nullable session binding columns and additive SSH profile revision/host-trust metadata.
2. Add command-template, command-run, output-chunk, capture-settings, and FTS tables plus indexes and triggers.
3. Existing local and remote sessions remain readable; existing remote sessions start unbound and require selecting an SSH profile before Terminal use.
4. Add service contracts and deterministic Web implementations before wiring React controls.
5. Add the native SSH adapter, pool, host verification, remote PTY, exec runtime, and shutdown/maintenance hooks.
6. Rollback leaves additive tables and nullable columns unused; local Shell behavior remains available. Persisted host trust and credentials remain removable through SSH profile deletion.

## Open Questions

- Decide whether encrypted private keys are rejected in the first version or supported with a separately stored passphrase.
