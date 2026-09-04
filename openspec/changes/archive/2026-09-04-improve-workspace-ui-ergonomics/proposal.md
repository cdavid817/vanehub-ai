## Why

Nine workspace usability defects were reported from daily desktop use. They fall into three groups. Some surfaces are visually dense and hard to scan: the create-session dialog, the unified work board, and the Goal Center present every control at the same weight. Some surfaces break at particular window widths: the session sidebar and the conversation column have no reliable separation, and a selected settings navigation entry can be clipped horizontally. And two behaviors are simply missing: the Help activity entry opens the About settings page instead of product documentation, and a session whose runtime has failed offers the user no way back — the list shows `failed` and nothing in the UI restores a usable state. The work board additionally renders raw Windows extended-length paths (`\\?\D:\...`) even though a display-normalization helper already exists and is used everywhere else.

## What Changes

- Restructure the create-session dialog into weighted sections so agent mode, participants, workspace, and session name are scannable, and give the multi-Agent seat editor per-seat identity rather than a stack of unlabeled selects.
- Guarantee horizontal separation between the session sidebar and the conversation column at every supported width, so the sidebar's rightmost content and its resize affordance are never overlapped by the conversation column.
- Point the Help activity entry at a documentation page that renders the bundled README in the user's language, instead of the About page.
- Apply the existing user-safe path display rule to every work board surface that shows a project path.
- Rework the work board's presentation: weighted card layout, clearer stage columns, and readable filter grouping.
- Rework the Goal Center's presentation: weighted goal list rows, clearer status and progress affordances, and a detail pane that separates identity from actions.
- Keep a selected settings navigation entry fully visible at every supported width by letting long labels truncate instead of overflowing the scroll container.
- Move notification toasts to the top center of the viewport, where they cover neither the session sidebar nor the composer's send control nor the information panel's tabs.
- Add an explicit session recovery action that clears a stuck runtime — cancelling any lingering generation lease and streaming messages and returning the session to an idle, usable state — reachable both from a failure banner in the session workspace and from the session list context menu.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `main-layout-ui`: Revise the workspace column separation rule, the Help activity entry destination, the create-session dialog structure, and add failure-recovery entry points to the session workspace and session list context menu.
- `notification-system`: Revise the toast viewport placement rule so toasts never overlay the session sidebar, the composer send control, or the information panel tabs.
- `settings-center-ui`: Add a documentation settings page that renders the bundled README, and require that a selected navigation entry stay fully visible at every supported width.
- `unified-todo-board`: Require user-safe path display on every board surface and revise the board's card and column presentation.
- `goal-management`: Add a Goal Center presentation requirement covering the goal list, status affordances, and detail pane.
- `session-runtime-management`: Add a runtime-neutral session recovery operation that releases a stuck generation and restores an idle lifecycle.

## Impact

- Affects the React workspace shell, settings center, work board, Goal Center, notification presentation, and the session runtime context in `src-tauri/`.
- Primarily changes `src/main-layout/`, `src/settings/`, `src/work-board/`, `src/goal-center/`, `src/notifications/`, `src/styles.css`, and the frontend service boundary in `src/services/`, with synchronized locale resources and UI tests.
- Adds one Tauri command for session recovery in `src-tauri/src/commands/agent_runtime/`, backed by the existing generation-cancellation and lifecycle ports; no database schema change and no new dependency.
- The new command must be reachable through `agent-service.ts` with contract-compatible `tauri-agent-client.ts` and `web-agent-client.ts` implementations; React components must not call `invoke()` directly.
- Requires Playwright coverage because navigation, dialogs, board interaction, and recovery are user-visible behavior changes.
