## Context

The native index separates Tree-sitter parsing and FTS5 persistence from external Embedding, and the first implementation added explicit local and semantic workspace modes. Workspace identity is still created through a manual settings action even though every local OnePiece session already carries its project folder. The revised change combines mode selection with session-driven workspace discovery while preserving workspace-specific security boundaries and management.

The change crosses session orchestration, retrieval configuration, SQLite storage, the background worker, frontend service contracts, both runtime adapters, settings UI, current-session UI, and localization. Session creation must remain available even when indexing is slow, unavailable, or waiting for semantic consent.

## Goals / Non-Goals

**Goals:**

- Make `disabled`, `local`, and `semantic` a clear OnePiece default policy for newly discovered local session projects.
- Automatically register or reuse a workspace after creating a local OnePiece session and index it asynchronously.
- Keep local Tree-sitter + FTS5 retrieval complete and provider-free.
- Preserve per-workspace overrides, parsed data, vectors, privacy confirmation, and stable canonical identity.
- Show local readiness independently from optional semantic enhancement.
- Preserve Tauri/Web adapter parity and deterministic tests.

**Non-Goals:**

- Indexing SSH or other remote filesystems.
- Integrating an on-device Embedding model.
- Blocking session creation until indexing completes.
- Adding a separate exact-symbol tool; `search_code` continues to return typed locations.
- Deleting retained index data when the effective policy is disabled.

## Decisions

### Use a three-state automatic policy plus workspace overrides

Persist a OnePiece automatic code-index policy with `disabled`, `local`, and `semantic` values. It is explicitly labeled as the default for session projects rather than a hidden global override. A newly auto-discovered workspace inherits that policy; an existing workspace retains its explicit configuration. Users can later set a workspace override to disabled, local, or semantic.

The safe default for a fresh installation is `disabled`. Existing registered workspaces keep their enabled state and mode during migration. Changing the automatic policy does not silently rewrite existing workspace privacy decisions; the UI can offer an explicit apply-to-existing operation later without making it part of this change.

Alternative considered: one global mode that forcibly changes every workspace. It is simpler but conflicts with large-repository exclusions, mixed privacy needs, and the existing workspace isolation model.

### Keep the workspace registry but remove it as a prerequisite

The workspace record remains the persistence and security boundary for canonical root, stable ID, selected roots, languages, exclusions, generations, confirmations, and status. The user-facing workflow creates or reuses this record automatically from session creation.

Registration is idempotent by canonical root. Multiple sessions for the same folder reuse one record and one generation-safe work queue. A Git worktree uses its actual session folder because its checked-out content can differ from the parent repository.

### Trigger indexing from native session orchestration

After a local session for stable agent ID `onepiece` is committed successfully, native orchestration emits or handles a workspace-opened event. The retrieval context canonicalizes the project folder, upserts the workspace when the automatic policy is not disabled, and schedules reconciliation. The session operation completes independently; indexing failure is reported through index status and unified logging rather than failing session creation.

React does not chain `createSession()` to `registerCodeIndexWorkspace()`. This avoids lost registration when the window closes and prevents frontend code from becoming the filesystem orchestration layer. The Web adapter simulates the same observable behavior after session creation.

Remote sessions do not auto-register because the current indexer has no bounded remote filesystem source. Sessions without a project folder are unaffected.

### Persist explicit workspace mode and mode-aware processing

`CodeIndexMode::{Local, Semantic}` and the `index_mode` SQLite column remain the workspace execution mode. Disabled continues to be represented by workspace enablement, while the global automatic policy uses the three-state union because it controls whether discovery creates or enables work.

After reconciliation, an enabled local workspace enters local-ready with no pending or estimated Embedding work. Switching modes increments generation, invalidates stale work, and preserves manifests, chunks, symbols, FTS rows, and vectors.

### Separate base-index readiness from semantic enhancement

Status must not imply that Tree-sitter data is unusable merely because semantic configuration is absent. The service contract exposes enough state for the UI to present two channels:

- Local index: disabled, scanning, parsing, ready, degraded, or unavailable.
- Semantic enhancement: not applicable, unconfigured, awaiting confirmation, embedding, ready, or degraded.

The existing aggregate phase may remain for compatibility, but local file/chunk counts and search availability are authoritative once parsing completes. In semantic mode without a configured model, FTS5 search remains available while the semantic channel reports unconfigured. External calls still require workspace-specific confirmation.

### Make search mode-aware

`CodeSearchService` loads the workspace mode before ranking. Local mode executes FTS5 only and returns keyword hits without a degradation marker. Semantic mode attempts vector and keyword fusion; a missing or temporarily unavailable vector channel returns usable keyword hits with the semantic channel state visible.

### Present policy, effective mode, and progress in context

CLI Parameter Management contains a dedicated OnePiece page for the three-state automatic policy and Embedding source/model. Its parameter cards, control widths, and information hierarchy follow the same visual structure as the managed CLI pages. It does not show index status or rebuild controls. Agent Configuration remains responsible only for OnePiece provider profiles.

The session information panel contains a code-index tab when the active session is a local OnePiece session. It resolves the effective worktree or project path, displays only that workspace record, and exposes its progress, counts, failures, configuration, confirmation, refresh, rebuild, disable, and delete actions. Workspace index information is not shown as a global settings dashboard and follows session switching.

### Keep workspace deletion off the UI and command threads

Deleting a workspace can cascade through its file, chunk, symbol, FTS, vector, and audit rows. The Tauri delete command is therefore asynchronous and moves the synchronous SQLite operation to the blocking thread pool. The information panel closes the delete confirmation immediately, keeps the deletion promise in the background, prevents duplicate workspace actions while it is pending, and reports failures in the panel after refreshing authoritative state.

Alternative considered: awaiting deletion while the confirmation dialog remains modal. Although the JavaScript API is promise-based, that design keeps the session UI unnecessarily locked and a synchronous native command can still occupy Tauri's command executor during a large cascade.

## Risks / Trade-offs

- [Automatic indexing surprises users] -> Fresh installs default to disabled; enabling semantic still requires per-workspace external-send confirmation.
- [Session creation becomes slow or fragile] -> Commit the session first and enqueue indexing asynchronously; report failure only in index status and unified logs.
- [Duplicate sessions create duplicate indexes] -> Canonical-root upsert and generation-safe scheduling reuse one workspace record.
- [Global changes overwrite workspace privacy choices] -> Treat the global value as an automatic-discovery default and preserve explicit workspace overrides.
- [Semantic configuration obscures local readiness] -> Expose local and semantic channel states separately and keep FTS5 usable after parsing.
- [Remote paths escape the local security boundary] -> Do not auto-register remote sessions until a bounded remote index source exists.
- [Worktree content crosses branches] -> Scope by the actual session/worktree folder rather than rebasing to the parent repository.

## Migration Plan

1. Retain the existing workspace `index_mode` migration and mode-aware indexing behavior.
2. Add the automatic policy to the centralized retrieval configuration with a safe `disabled` default.
3. Preserve all existing workspace rows as explicit configurations; do not rewrite their enabled state or mode.
4. Add idempotent native session-to-index orchestration and Web/mock parity.
5. Extend status contracts before exposing the policy and session-scoped index pane.
6. Remove global workspace management UI after automatic discovery and session-scoped management are available.
7. Rollback leaves workspace rows and parsed data readable; older builds ignore the additive automatic-policy and status fields.

## Open Questions

None. The policy applies to new local OnePiece session projects, while existing workspace configuration remains authoritative.
