## Context

The existing session model already exposes `runtimeSessionId`, project path, worktree path, folder path, categories, and lifecycle state through the frontend Agent service boundary. Existing specs require runtime session id persistence and Web/mock parity, but the user-visible workflow still needs reliable capture on create/start, resume on reopen, and a project-oriented sidebar layout.

`cc-switch` groups session history by provider and project directory and keeps group expansion state in local storage. VaneHub should reuse that information architecture while keeping the implementation inside its existing React state/service boundary and Rust-owned persistence model.

## Goals / Non-Goals

**Goals:**

- Ensure CLI-backed sessions persist the provider resume id as soon as it is known and use it for later terminal reopen.
- Keep Tauri-specific resume invocation construction in Rust and expose only session metadata through frontend services.
- Add a bounded draggable session sidebar width without breaking collapse/expand behavior.
- Add a project grouping presentation for session navigation using existing session workspace metadata.
- Keep Web/mock behavior deterministic and compatible with the same TypeScript contracts.

**Non-Goals:**

- Add a new session category system or replace existing user-defined categories.
- Parse historical provider transcript files outside the current Agent Terminal runtime.
- Add new dependencies or direct Tauri calls in React components.

## Decisions

1. Treat `runtimeSessionId` as the resume id.

   The current service contract and database schema already carry `runtimeSessionId`; adding a separate resume field would duplicate semantics. Providers that distinguish display session ids from resume tokens should map the provider resume token into this field before persistence.

2. Capture resume ids in both event-driven and start-result paths.

   Terminal events can report a runtime session id after process startup, while some providers or mocks can know a deterministic id when the terminal session object is returned. The implementation should persist any newly observed id and refresh session state so later reads expose it.

3. Build resume invocations only in the native runtime.

   React components will continue to call `openAgentTerminal(sessionId, size)` through the Agent service. Rust owns provider-specific resume flags and launch wrappers, which avoids exposing command-token logic to the frontend.

4. Derive project groups in frontend presentation code.

   The list service already returns all metadata needed for grouping. Grouping in the sidebar model keeps SQLite queries stable and lets both desktop and Web/mock runtimes render the same groups from the same `Session` contract.

5. Persist sidebar UI preferences in browser local storage.

   Sidebar width and expanded project groups are local UI preferences, not durable domain data. Local storage matches existing frontend patterns for presentation state and works in both Tauri WebView and browser mode.

## Risks / Trade-offs

- Provider output does not include a resume id -> the session remains fresh-start only until a provider reports one.
- A stale resume id can fail after provider-side deletion -> native runtime should fall back to a clear user-facing failure rather than silently starting an unrelated fresh session.
- Very long project paths can overwhelm the sidebar -> project labels should use a basename plus tooltip/full path treatment and stable truncation.
- Resizable panels can damage responsive layouts -> clamp width and disable dragging while the sidebar is collapsed.

## Migration Plan

- No schema migration is expected if `runtime_session_id` already exists in session persistence.
- Existing sessions with null `runtimeSessionId` remain valid and start fresh.
- Existing local category data remains unchanged; project grouping is only a presentation mode.
