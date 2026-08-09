## Why

Tree-sitter and FTS5 can provide useful on-device code retrieval without an Embedding provider, but the current workflow requires users to register a workspace manually before creating a session. OnePiece should let users choose a safe default indexing policy once, automatically discover local project folders from new sessions, and expose progress without making session creation wait for indexing.

## What Changes

- Add a OnePiece automatic code-index policy with `disabled`, `local`, and `semantic` choices for newly discovered session projects.
- Keep per-workspace configuration and overrides so large or sensitive projects can opt out or choose a different mode without changing every workspace.
- Automatically canonicalize, register, or reuse the local project folder after a OnePiece session is created, then enqueue indexing in the background without blocking session availability.
- Make local mode complete after Tree-sitter parsing and FTS5 persistence, report a ready local-search channel, and never enqueue or call Embedding.
- Keep semantic mode's provider/model binding, cost estimate, explicit external-send confirmation, throttled Embedding, and hybrid retrieval behavior while allowing local search to become ready first.
- Place the OnePiece retrieval policy and Embedding parameters under CLI Parameter Management instead of Agent Configuration.
- Replace the settings-level workspace dashboard with a session-scoped code-index pane that follows the active OnePiece session and its effective worktree/project folder.
- Show current-session indexing progress, file/chunk counts, pending work, estimated Embedding requests, failures, separate local/semantic readiness, and workspace management actions.
- Preserve parsed chunks and existing vectors when switching modes, disabling indexing, or reopening a previously indexed project.
- Mirror automatic registration and state transitions in the Web/mock adapter without filesystem or network access.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-code-indexing`: Define the OnePiece automatic indexing policy, session-driven workspace discovery, local/semantic workspace behavior, progress reporting, overrides, and runtime adapter parity.

## Impact

- Desktop runtime: session-creation orchestration, Rust code-index domain model, SQLite configuration and workspace metadata, background scheduling, status calculation, Embedding gates, and code search.
- Web runtime: deterministic session-driven workspace registration and local/semantic transitions without filesystem or network access.
- Frontend: shared service contracts, Tauri/Web adapters, OnePiece parameter management, session-scoped workspace index status and actions, conditional Embedding guidance, and localization.
- Security: automatic indexing remains limited to canonical local folders selected for OnePiece sessions; sensitive-file denial, redaction, workspace isolation, and per-workspace semantic confirmation remain mandatory.
- Adapter boundaries remain unchanged: React uses `AgentService`; Tauri commands and native orchestration own SQLite, filesystem, and background indexing behavior.
- No new third-party dependency is required.
