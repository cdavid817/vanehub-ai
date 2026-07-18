## Context

VaneHub AI already persists sessions, messages, active-session state, session runtime lifecycle, provider runtime session ids, and archived status through the Rust/Tauri SQLite layer. The current sidebar supports pinned, activity, folder, archived, and context-menu actions, but it does not provide full historical search, user-defined categories, export, automatic archival, crash reconciliation, Mermaid rendering, or file references from the chat composer.

This change is cross-cutting: session persistence and scheduled work belong in Rust, UI interactions belong in React, and every operation must pass through the frontend service boundary with matching Tauri and Web/mock adapters.

## Goals / Non-Goals

**Goals:**
- Let users search sessions by title, project metadata, and persisted message content.
- Add one-category-per-session organization with categorized sidebar grouping and drag/drop assignment.
- Export a session transcript and metadata as JSON or Markdown into a user-selected desktop directory.
- Run Rust-owned automatic archival at startup and then hourly, with a configurable threshold defaulting to 10 days.
- Reconcile orphan active sessions/messages after crashes without discarding partial assistant content or provider resume metadata.
- Render Mermaid fenced code blocks safely in chat messages.
- Let users reference active-session project files from the chat composer with `@` and submit bounded file content as message context.
- Preserve Web/mock behavior with deterministic in-memory equivalents for preview and tests.

**Non-Goals:**
- Multi-category tagging, nested categories, category sharing, or category sync.
- Full-text search engine integration beyond SQLite-backed queries for this version.
- Resuming a process that died with the application; provider resume metadata is preserved for the next user-initiated message.
- Mermaid editing tools, diagram export, or rendering Mermaid in arbitrary raw HTML.
- Referencing files outside the active session root or attaching binary/oversized files.

## Decisions

### Store categories natively and link sessions by nullable category id

Add a `session_categories` table with stable ids, names, sort order, and timestamps, plus a nullable `sessions.category_id`. This keeps category membership durable, queryable, and easy to expose through existing session listing contracts.

Alternative considered: encode categories in a JSON settings blob. That would avoid a migration but would make search, filtering, deletion, and consistency checks harder.

### Keep collapse state in the frontend

Category membership is durable, but expanded/collapsed UI state can stay in React state or existing UI preferences unless a later settings requirement asks for cross-restart persistence.

Alternative considered: persist collapsed state in SQLite. That adds native state churn for a purely visual preference and is not required by the current workflow.

### Implement search in Rust with additive service APIs

Desktop search should query SQLite using session metadata and message content, returning bounded result rows that include matched session metadata and lightweight match context. React calls an `AgentService` search method; `tauri-agent-client.ts` invokes the native command; `web-agent-client.ts` filters deterministic mock data.

Alternative considered: fetch all sessions/messages into React and filter there. That breaks scale expectations, bypasses native ownership of persisted data, and would not work well for large histories.

### Export through native commands

The desktop frontend requests export through the service boundary, selects format and directory through service-owned adapter behavior, and Rust writes the file. JSON should be structured and versioned. Markdown should be readable and include metadata, messages, thinking, tool-use blocks, status, errors, token usage, and file references.

Alternative considered: build and download files in React. That conflicts with desktop filesystem ownership and would not support native save-directory behavior consistently.

### Run automatic archival and recovery in Rust

Startup recovery and automatic archival both reconcile persisted runtime state. They should run after database initialization in the native runtime. Automatic archival runs once at startup and then every hour while enabled. It archives sessions whose `updated_at` is older than the configured threshold, excluding pinned, archived, `starting`, and `running` sessions. Recovery scans persisted `starting`/`running` sessions and unfinished assistant messages when no live generation handle exists, marks orphan messages `failed`, sets sessions `failed`, preserves partial content and `runtime_session_id`, and logs diagnostics.

Alternative considered: run archival in React using timers. That would only work while the UI is mounted and would mix runtime maintenance with presentation code.

### Represent file references as message metadata and injected send context

The composer should resolve `@` file references through existing session file listing/reading APIs. `SendMessageInput` should gain additive file-reference metadata or context payloads. Rust validates references remain under the session root, rejects binary or oversized files, persists safe reference metadata with the user message, and injects bounded file content into the provider prompt.

Alternative considered: paste file contents directly into the text input. That hides provenance, weakens export/history fidelity, and makes limits difficult to communicate.

### Render Mermaid only from fenced code blocks

Chat rendering should detect Markdown fenced code blocks with language `mermaid`, render them in an isolated component, and fall back to source text plus a localized error when parsing/rendering fails. Raw embedded HTML remains disabled.

Alternative considered: pre-render Mermaid in native code. Mermaid is a frontend rendering concern and should not expand the Rust runtime surface.

## Risks / Trade-offs

- Search over large message tables could become slow -> use bounded results, indexed metadata, and avoid unbounded message excerpts.
- Automatic archival may hide sessions unexpectedly -> default only archives non-pinned inactive sessions, provide settings, and log archival results.
- Recovery may classify an interrupted provider job as failed even if the provider completed remotely -> preserve `runtime_session_id` so the next message can resume provider context where supported.
- Drag-and-drop can be hard to use accessibly -> keep context-menu move actions as a complete non-drag path.
- File references can expose sensitive local content to provider CLIs -> require explicit user-selected references, enforce root and size checks, and show visible chips before send.
- Mermaid rendering can add bundle weight or failure modes -> lazy-load the renderer if practical and always preserve source fallback.

## Migration Plan

- Add SQLite migrations for `session_categories`, `sessions.category_id`, message metadata for file references if not already available, and archival settings defaults.
- Backfill existing sessions with `category_id = NULL`; the UI displays them under the localized uncategorized group.
- Add service methods and Tauri commands without removing existing session operations.
- Start the archival scheduler and recovery pass after database migration and unified logging initialization.
- Keep Web/mock data deterministic and resettable in tests.
- Rollback is data-compatible: sessions with category ids remain valid even if category UI is absent, and archived sessions can still be restored through existing operations.

## Open Questions

- Should users be able to rename, delete, and manually reorder categories in the first implementation? The current proposal requires creation and assignment; deletion and ordering can be included if product scope allows.
- Should search snippets show exact message excerpts or only indicate that message content matched? Exact snippets are more useful but need careful truncation and redaction expectations.
