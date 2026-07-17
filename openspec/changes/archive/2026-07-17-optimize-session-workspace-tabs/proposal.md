## Why

The current session workspace is chat-only in the main content area, so developers must leave the active conversation context to inspect files, review changes, read diagnostics, or use a shell. A tabbed, session-scoped workspace brings those related workflows together while preserving state across tab switches and keeping both desktop and Web preview behavior coherent.

## What Changes

- Add a main-content `SessionTabs` workspace with Chat, Changes, Documents, Files, Terminal, Shell, Logs, and Report tabs.
- Lazy-mount each tab on first activation and keep mounted tab state alive with CSS visibility switching; reset the mounted set and active tab when the selected session changes.
- Keep the chat composer visible only while Chat is active, and show localized icon, label, active state, accessibility text, and badge metadata in the tab bar.
- Provide full-size file browsing, file preview, Git status/diff review, project document viewing, session tool-call history, filtered session diagnostics, a PTY-backed interactive shell, and session-level reporting.
- Keep the existing right information panel as a compact Agent Info / Files / Changes overview while the main tabs provide the detailed workflows.
- Extend the frontend agent service boundary and both Tauri and Web/mock adapters with matching session workspace operations.
- Add native Rust commands for bounded project inspection, Git inspection, unified-log querying/export, and managed PTY shell sessions.
- Add deterministic Web/mock data for every tab and a clearly identified simulated shell so the browser preview and Playwright suite remain useful.
- Add synchronized Simplified Chinese and English strings and theme-aware presentation for both `futuristic` and `minimal` styles.

## Capabilities

### New Capabilities

- `session-workspace-tabs`: Session-scoped tab container behavior plus Chat, Documents, Terminal, and Report presentation requirements.
- `session-project-inspection`: Safe project file browsing, file preview, Git status, and unified/split diff behavior.
- `session-shell`: Managed interactive PTY shell lifecycle, I/O, resize, cleanup, and Web simulation behavior.
- `session-log-viewer`: Session-filtered unified diagnostics browsing, filtering, search, and safe export behavior.

### Modified Capabilities

- `main-layout-ui`: Expand the chat-first center panel into a tabbed session workspace while retaining the compact right information panel and Chat-only composer behavior.
- `unified-log-management`: Make already-redacted session diagnostics queryable and exportable through the service boundary without introducing a second SQLite log store.

## Impact

- Frontend: new session-tab components and types, refactoring of the oversized main layout, synchronized i18n resources, theme-aware xterm integration, and additional component/unit/E2E coverage.
- Service boundary: new typed methods in `agent-service.ts` with matching implementations in `tauri-agent-client.ts` and `web-agent-client.ts`; React components continue to avoid direct Tauri calls.
- Desktop runtime: new Rust modules and Tauri commands for project/Git inspection, unified-log reads/exports, and PTY process management with cleanup and path confinement.
- Web runtime: deterministic mock project, diff, log, report, and simulated-shell behavior without local filesystem or process access.
- Dependencies: a maintained xterm frontend package, its fit/resize support, a Markdown renderer, and a cross-platform Rust PTY implementation.
- Persistence: no new log database or feature-local log file; existing SQLite sessions/messages and the unified log directory remain the sources of truth.
