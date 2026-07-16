## Context

VaneHub already has durable session records, sidebar session actions, active-session selection, and session-owned message persistence. The remaining gap is that a session is still mostly metadata: desktop `send_message` produces a preview response, lifecycle state is not consistently tied to generation state, and UI/Web behavior has small contract drift from the session specification.

This change completes sessions as the unit that owns active Agent execution, persisted messages, cancellation, status display, and diagnostics while preserving the existing service boundary.

The merged scope also completes session creation as a project-first workflow. New sessions must collect the working folder, Agent, optional title, and optional Git worktree intent up front. Clowder AI is used as a reference for worktree safety principles: create worktrees outside the source project, construct Git commands in backend code, and reject unsafe names or target paths before running Git.

## Goals / Non-Goals

**Goals:**

- Keep session CRUD and listing behavior intact while fixing Web/default-title and hard-coded UI metadata drift.
- Replace immediate new-session creation with a create-session dialog that supports Agent selection, project history, directory browsing, and optional Git worktree creation.
- Persist project history from folders selected during session creation.
- Detect whether a selected folder is a Git repository and disable worktree controls for non-Git folders while allowing normal sessions.
- Make the active session the source of truth for active Agent id, interaction mode, lifecycle state, messages, and generation controls.
- Add session-scoped runtime state in the desktop layer so generation, cancellation, failure, and completion update the owning session coherently.
- Replace desktop preview-only responses with real CLI-backed execution for supported, installed CLI Agents, with deterministic fallback errors when the selected runtime is unavailable or unsupported.
- Persist user messages, assistant messages, status transitions, token/thinking/tool events when available, and diagnostics by session.
- Write session runtime stdout/stderr/diagnostics through the unified logging service.
- Keep Web/mock behavior contract-compatible without requiring SQLite or local CLI access.
- Keep Web/mock behavior contract-compatible for project history and worktree metadata without executing native Git commands.

**Non-Goals:**

- Full protocol support for every Agent CLI feature.
- Long-lived terminal UI embedding or PTY rendering.
- Remote sync across devices.
- Folder entity management beyond the existing `folder` session field.
- Scanning arbitrary recent directories outside VaneHub history.
- Full worktree lifecycle management such as list, prune, remove, or sync.
- Worktree branch checkout from arbitrary existing branches.
- Replacing the existing Agent registry, CLI detection, or workflow-state model.

## Decisions

### 1. Session runtime state is owned by the Rust layer

The desktop runtime will track active generation state by `session_id` in Rust and persist observable lifecycle state back to SQLite. React remains a consumer of `AgentService` state and events.

Alternative considered: keep generation state in React Query. That would be easier to prototype but would lose desktop restart persistence and violate the boundary for process state.

### 2. Keep `sessions.lifecycle_state` as the user-visible session lifecycle

The existing `lifecycle_state` column remains the primary user-visible state. Generation commands update it to `starting`, `running`, `idle`, `failed`, or `stopped` as the session runtime changes.

Alternative considered: add a separate `session_runtime_state` table immediately. A table may become useful for richer process metadata, but the current lifecycle field is enough for the first complete contract and avoids duplicated state.

### 3. Agent execution starts with a generic CLI adapter and explicit unsupported fallback

The Rust runtime will route by stable Agent id to an adapter. The first implementation will provide a generic CLI adapter that executes a real installed CLI when enough launch metadata is available, validates CLI availability before execution, and returns structured failure if execution is unavailable or unsupported. Agent-specific parsers can then improve output handling without changing the frontend contract.

Alternative considered: keep desktop preview responses while the runtime contract is introduced. That would leave sessions incomplete because a generation could appear successful without any Agent runtime execution.

### 4. Stream events and persisted messages remain separate

`ChatStreamEvent` continues to carry incremental UI updates while `messages` remains the durable transcript. Completion/failure/cancellation updates both the message status and the owning session lifecycle.

Alternative considered: rely only on final persisted messages. That would remove the interactive streaming experience and make stop/cancel feedback worse.

### 5. Unified logging receives runtime diagnostics

Session runtime stdout, stderr, command metadata, exit status, and failure diagnostics will be written through the unified logging service with session id and Agent id context. UI message lists keep user-facing content; logs keep operational diagnostics.

Alternative considered: store all raw output in message metadata. That risks exposing noisy or sensitive diagnostics in the chat UI and bypasses retention/redaction rules.

### 6. Web/mock mirrors the contract, not the implementation

The Web adapter will not emulate process management deeply. It will provide the same session lifecycle transitions, message ownership, cancellation behavior, and default-title behavior using in-memory timers.

Alternative considered: leave Web as a loose demo. That would continue contract drift and weaken frontend tests.

### 7. Archiving a running session stops it first

Archiving a running session will request generation cancellation for that session, mark active assistant output as cancelled, then archive the session and clear active session selection. This keeps archive reversible as organization while preventing hidden background generation.

Alternative considered: prevent archiving while running. That is safer but forces extra user steps and still requires backend cleanup for races.

### 8. UI supports one active generation, runtime supports per-session isolation

The first UI version will only allow generation controls for the active session. The Rust runtime map will still be keyed by `session_id` so backend isolation is correct and future multi-session generation does not require a data model rewrite.

Alternative considered: expose concurrent generation controls for many visible sessions. That is more complex than needed for completing the current session contract.

### 9. User-facing errors stay concise

The chat UI will show short errors such as "Codex CLI unavailable" or "command failed". Detailed stdout, stderr, command diagnostics, and exit status go to unified logs with session and Agent context.

Alternative considered: render raw diagnostics in chat messages. That creates noisy UI and increases the chance of exposing sensitive command output.

### 10. Create-session dialog owns user intent; service owns effects

The sidebar new action opens a dialog that collects stable Agent id, interaction mode, project path, optional title, and optional worktree request. React calls service methods such as `listKnownProjects`, `inspectProject`, `selectProjectDirectory`, and `createSession`; it does not call Tauri commands directly.

Alternative considered: create the worktree from the dialog through a separate command and then call `createSession`. A single `createSession(input)` with optional worktree details keeps transaction boundaries and diagnostics in the service/native layer.

### 11. Worktree defaults are deterministic and outside the project

When worktree creation is enabled, the worktree name is required. The target path is the selected project's parent directory plus `{projectName}-{worktreeName}`. The branch is `vanehub/{worktreeName}`. The worktree name rejects empty values, path separators, `..`, and control characters. The target path is rejected if it exists or is inside the selected project.

Alternative considered: create worktrees under the project directory. That complicates repository tooling and cleanup.

### 12. Git and project persistence stay native

Rust inspects selected paths, persists known project history in SQLite, creates worktrees with explicit `git` executable/argument values, and writes Git diagnostics through unified logging. Web/mock mode mirrors the contract in memory.

Alternative considered: run Git or path parsing from JavaScript. That violates the native runtime boundary and makes command safety harder to audit.

## Risks / Trade-offs

- Agent CLI invocation differs by tool and version -> Start with a generic CLI adapter plus explicit unsupported/failure fallback, then refine individual adapters behind stable service contracts.
- Long-running process cleanup can leak children -> Track active generation handles by session and clean them on complete, failure, cancellation, archive, and delete.
- Lifecycle state can drift if a command crashes unexpectedly -> Update session state in all terminal paths and reconcile active runtime map before returning session details.
- Logs may contain sensitive output -> Redact before persistence through unified logging.
- Web and desktop behavior may diverge -> Add shared service tests for Web and Rust tests for session/message lifecycle transitions.
- Worktree target collisions -> Reject before running Git and log any Git failure if the filesystem changes between validation and execution.
- Project history can become stale -> Inspect selected entries when used and update `last_opened_at` and Git status opportunistically.
- Large changes in `src-tauri/src/lib.rs` can worsen maintainability -> Prefer extracting session/chat/runtime helpers into focused Rust modules when implementation touches these areas.

## Migration Plan

1. Add any required SQLite fields or indexes additively; preserve existing `sessions` and `messages` rows.
2. Add known project history and optional session project/worktree metadata additively; older sessions load with null project/worktree metadata and their existing folder value.
3. Fix existing contract drift in Web adapter and UI placeholders.
4. Update session/message lifecycle helpers so send, complete, fail, cancel, archive, and delete keep session state coherent.
5. Introduce a session runtime adapter boundary in Rust and route desktop `send_message` through it.
6. Add create-session dialog, service contract, Web/mock project history, Tauri dialog picker, Git inspection, and worktree creation.
7. Write runtime and Git diagnostics through the unified logging service.
8. Validate with OpenSpec, frontend tests/build, Rust tests/checks, and targeted manual Web/mock chat verification.

Rollback is additive: if runtime execution is disabled, existing session CRUD and message listing remain valid and unsupported runtime sends return a structured failure instead of corrupting session state.

## Open Questions

- Which Agent should receive the first fully protocol-aware adapter after the generic CLI path?
- Should future runtime metadata expose process id and command path to the UI, or keep it only in logs?
- Which per-Agent parser should be implemented first after generic CLI execution is stable?
