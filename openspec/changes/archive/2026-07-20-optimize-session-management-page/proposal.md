## Why

The current workspace session sidebar supports single-session navigation and context actions, but it lacks efficient workflows for managing many sessions. Users need a faster way to filter, organize, and delete sessions while keeping the workspace visual system consistent across both registered styles.

## What Changes

- Add a batch-management mode to the session management surface with multi-select, selected-count feedback, select-visible behavior, cancel/exit behavior, and confirmed multi-session deletion.
- Add a session list presentation switch between list and categorized views.
- Add an Agent filter for Claude Code, OpenCode, Codex CLI, Gemini CLI, and All sessions.
- Adjust the session management surface using the cc-switch session manager as interaction reference while preserving VaneHub's existing layout, Tailwind tokens, lucide icon usage, and service-boundary architecture.
- Keep all new user-visible labels, buttons, confirmations, empty states, and accessible names synchronized in zh-CN and en.
- Verify both visual styles remain coherent and that session deletion, filtering, sorting, and selection behavior are covered by focused tests.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `main-layout-ui`: Session management UI gains batch mode, list/category presentation switching, Agent filtering, localized controls, and visual-system requirements.
- `session-management`: Session deletion behavior is extended to support UI-driven multi-session deletion while retaining service-boundary and active-session clearing semantics.

## Impact

- Frontend UI: `src/main-layout/session-sidebar.tsx`, `src/main-layout/main-layout.tsx`, `src/main-layout/use-main-layout-model.ts`, context/dialog components as needed, and focused tests.
- Frontend service boundary: use existing `deleteSession(sessionId)` unless implementation reveals a need for a dedicated service method; React components must still depend on `AgentService` only.
- Runtime adapters: no new Tauri command is expected for the first implementation; if a bulk service method is introduced, both Tauri and Web adapters must be updated together.
- i18n: update `src/i18n/locales/zh-CN.json` and `src/i18n/locales/en.json`.
- Desktop and Web runtimes are both affected because the session sidebar is shared by both runtime modes.
