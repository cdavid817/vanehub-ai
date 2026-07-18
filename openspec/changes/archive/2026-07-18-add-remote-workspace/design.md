## Context

VaneHub AI currently models session workspaces as local filesystem paths. A created session may point at a selected project folder, or at a Git worktree created from that project. React components consume this through `src/services/agent-service.ts`, while the desktop runtime persists session records in SQLite and performs local filesystem and Git operations in Rust.

Remote workspaces introduce a second target class: an SSH-addressable workspace that is not available to local filesystem APIs. The first version should make remote targets visible and durable without pretending that existing local tabs can read remote files. This keeps the data model ready for future remote execution while preserving the current service boundary.

Short-term shape:

```text
Create Session
├─ Local
│  ├─ projectPath -> inspect local path
│  └─ optional worktree -> git worktree add
└─ Remote
   ├─ host
   ├─ user?
   ├─ path
   ├─ displayName
   └─ uri = ssh://[user@]host/path
```

## Goals / Non-Goals

**Goals:**

- Add first-class remote workspace metadata to session records.
- Let users create a session in either local mode or remote mode.
- Persist and list known remote workspaces separately from local project history.
- Keep desktop and Web/mock adapters aligned through the shared service contract.
- Ensure local filesystem-backed tabs do not accidentally treat a remote URI as a local path.
- Document clear extension points for future SSH execution, remote file browsing, and remote Git inspection.

**Non-Goals:**

- Do not run Agent CLIs over SSH in this version.
- Do not implement SSH authentication, host key handling, connection tests, or credential storage.
- Do not implement SFTP/SSH-backed Files, Documents, Changes, or Shell tabs.
- Do not change IM connector routing to support remote workspaces.
- Do not replace the existing local project/worktree behavior.

## Decisions

### Decision: Represent remote workspaces as additive session metadata

Add an optional `remoteWorkspace` object to the shared `Session` contract instead of overloading `projectPath` or `worktreePath`.

Proposed fields:

```text
remoteWorkspace.host
remoteWorkspace.user?
remoteWorkspace.path
remoteWorkspace.displayName
remoteWorkspace.uri
```

`folder` remains the effective workspace label/path used by existing session grouping. For remote sessions, `folder` should be set to the stable remote URI. This avoids a larger sidebar and search rewrite in the first version, while `remoteWorkspace != null` gives every runtime a reliable way to avoid local filesystem access.

Alternative considered: store only `folder = ssh://...` without structured metadata. That is simpler initially, but weak for search, display, validation, and future SSH connection setup.

### Decision: Use mutually exclusive local and remote creation modes

`CreateSessionInput` should allow either:

- `projectPath`/`folder` with optional `worktree`
- `remoteWorkspace`

The backend and Web adapter should reject mixed `remoteWorkspace + worktree` requests before any Git command is executed. The UI should expose this as a segmented local/remote choice so users do not have to infer mode from path syntax.

Alternative considered: infer remote mode when the folder looks like `user@host:/path`. That creates ambiguous validation and makes Windows paths harder to handle safely.

### Decision: Keep remote history separate from local project history

Add a known-remote-workspace history list rather than mixing remote entries into `known_projects`.

Desktop storage can use a new SQLite table with `uri` as the primary key and columns for host, user, path, display name, and last opened timestamp. The Web adapter should keep an equivalent in-memory list.

Alternative considered: extend `known_projects` with a type discriminator. A separate table is less invasive and avoids changing local project history queries consumed by other pages such as IM routing.

### Decision: Treat local workspace tabs as unavailable for remote sessions

Before resolving a session root, native workspace commands should detect remote sessions and return an unavailable context or a concise unsupported error for local-process operations such as Shell. They must not pass `ssh://...` into local path canonicalization.

Expected first-version behavior:

```text
Chat       available
Files      unavailable: remote workspace unsupported
Documents  unavailable: remote workspace unsupported
Changes    unavailable: remote workspace unsupported
Shell      unsupported error
Logs       available for local session/runtime logs
Report     available from persisted messages
Terminal   available from persisted tool-use history
```

Alternative considered: hide unsupported tabs for remote sessions. Keeping the tab structure stable avoids special-case navigation and makes the limitation explicit.

### Decision: Keep remote execution as a future adapter layer

Future remote execution should be introduced behind a native runtime/service boundary, not inside React components. A later change can add a Rust-owned remote workspace service responsible for SSH process launch, connection testing, file reads, Git inspection, shell sessions, and redacted logging.

This first version should name the future boundary but not introduce incomplete SSH command execution.

## Short-Term Implementation

1. Extend shared frontend contracts:
   - Add `RemoteWorkspace` and `KnownRemoteWorkspace` types.
   - Add `remoteWorkspace: RemoteWorkspace | null` to `Session`.
   - Add `remoteWorkspace?: ... | null` to `CreateSessionInput`.
   - Add `listKnownRemoteWorkspaces()` to `AgentService`.

2. Update runtime adapters:
   - Tauri adapter invokes `list_known_remote_workspaces`.
   - Web adapter validates remote input, constructs a stable `ssh://[user@]host/path` URI, stores remote history, creates remote sessions, and includes remote fields in search.

3. Update Rust persistence:
   - Add a migration for remote session columns and a known remote workspace table.
   - Extend `Session`, `CreateSessionInput`, row loading, inserts, list/search queries, and export metadata.
   - Reject incomplete remote input and mixed remote/worktree input.

4. Update workspace command safety:
   - Make session root resolution remote-aware.
   - Return unavailable context for Files/Documents/Changes.
   - Return a concise unsupported error for Shell creation.

5. Update create-session UI:
   - Add local/remote mode selection.
   - Keep existing local project picker and worktree controls only in local mode.
   - Add remote host, optional user, path, and display name inputs in remote mode.
   - Load recent remote workspaces through `listKnownRemoteWorkspaces()`.
   - Add Chinese and English locale keys for every new visible label and error.

6. Add focused tests:
   - Contract conformance for new types.
   - Web adapter remote create/history/search/rejection tests.
   - Rust migration/session/search validation tests.
   - UI tests for switching local/remote mode and preserving existing local behavior.

## Optimization And Extension Points

- Remote connection profiles: promote host/user/auth metadata into reusable profiles with labels, default paths, and availability status.
- Connection testing: add an asynchronous operation for SSH reachability and host key diagnostics, writing detailed output through unified logs.
- Remote CLI launch: start Agent CLIs over SSH with explicit lifecycle operations and per-session log capture.
- Remote Files/Documents tabs: add bounded remote file listing and reading through a Rust-owned SSH/SFTP service with the same size, depth, and path escape limits as local tabs.
- Remote Git status/diff: run Git inspection remotely and normalize results to the existing `GitStatusResult` and `GitDiffResult` frontend contracts.
- Remote Shell: create an SSH-backed shell session behind the existing shell service contract, preserving resize/input/event semantics.
- Credential handling: introduce secure storage and redaction rules before any password, token, key path, or agent-forwarding setting is persisted or logged.
- URI normalization: centralize escaping and display formatting for IPv6 hosts, non-root paths, and future non-SSH remote schemes.
- IM routing support: evaluate remote workspaces for connector default routing only after remote execution semantics are available.

## Risks / Trade-offs

- Remote sessions may look actionable before remote execution exists -> The UI and service errors must clearly mark local tabs and shell as unsupported for remote workspaces.
- Storing `folder` as a remote URI may confuse local-only call sites -> Every filesystem boundary must branch on `remoteWorkspace` before reading `folder`.
- The first-version data model may not cover all SSH variants -> Keep metadata additive and avoid credential-specific fields until a security design exists.
- Search/index behavior can drift between desktop and Web mock -> Add equivalent Web and Rust tests for remote host, path, display name, and URI matches.
- Known remote workspace history could leak sensitive host/path names -> Apply the same local persistence expectations as known projects now, and revisit redaction/privacy before credential support.

## Migration Plan

- Add a new SQLite migration after the current latest migration.
- Add nullable remote workspace columns to `sessions`, so existing sessions remain valid.
- Create a separate `known_remote_workspaces` table keyed by URI.
- Keep rollback simple: removing the feature code leaves existing local sessions unaffected because new columns are nullable and local queries continue to use existing fields.
- During implementation, seed no remote entries by default.

## Open Questions

- Should the UI require a display name, or is the generated `host:path-name` label enough for the first version?
- Should `remoteWorkspace.path` require an absolute POSIX path, or allow relative paths for container-like targets?
- Should the first version show remote metadata in the right-side session info panel, or is create/sidebar/search visibility sufficient?
- What exact unsupported message should Shell return so it is clear but not noisy?
