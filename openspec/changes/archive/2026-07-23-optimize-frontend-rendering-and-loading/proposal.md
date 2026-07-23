## Why

Prompt Hook inventories and session log streams currently render every loaded item as a React DOM subtree, which degrades scrolling and interaction responsiveness as data grows. At the same time, heavy settings and workspace modules are included in the initial frontend bundle even when users never open them, increasing startup parsing and execution cost in both desktop and Web runtimes.

## What Changes

- Window Prompt Hook cards when the filtered inventory exceeds 500 items, while preserving filtering, sorting, grouping, selection, and card operations.
- Virtualize loaded Agent log entries so DOM growth remains bounded as users page through large streams.
- Add timestamp-based log navigation that can reveal a loaded entry or fetch bounded additional pages before reporting that the timestamp is outside the available range.
- Dynamically import heavy settings and workspace modules, including Agent configuration, Prompt Hooks/editor surfaces, the Loop task board, and non-default session tabs.
- Preserve the existing first-activation mount and keep-alive semantics after a lazy module has loaded.
- Add loading, failure, accessibility, and automated regression coverage for virtualized and lazy-loaded surfaces.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `frontend-runtime-architecture`: Require route- and feature-level code splitting with bounded loading and recovery states in both desktop and Web runtimes.
- `settings-center-ui`: Lazy-load unvisited settings page modules while keeping visited stateful pages mounted.
- `main-layout-ui`: Lazy-load the Loop task-board destination without discarding mounted session workspace state.
- `session-workspace-tabs`: Lazy-load non-default tab modules while retaining existing first-activation and per-session keep-alive behavior.
- `settings-prompt-hooks-ui`: Window large filtered Prompt Hook inventories above the defined threshold without changing operations or visible ordering.
- `session-log-viewer`: Virtualize loaded log rows and support timestamp-based navigation across bounded paginated results.

## Impact

- Frontend code in settings page registration/shell, Prompt Hook card lists, the main workspace destination switcher, session tab registration, and the session log viewer.
- Frontend tests and Playwright coverage for large datasets, lazy-loading boundaries, state preservation, keyboard access, and timestamp navigation.
- Adds `@tanstack/react-virtual` as a rendering dependency used only for list windowing; React built-in `lazy` and `Suspense` handle module loading.
- No service contract, Tauri command, SQLite schema, native logging, or runtime-adapter boundary changes are required. The behavior applies identically to Tauri desktop and browser Web/mock runtimes.
