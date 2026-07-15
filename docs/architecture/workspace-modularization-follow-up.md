# Workspace Modularization

The current implementation moves workspace demo data behind runtime service adapters and verifies the Web adapter does not depend on Tauri APIs. `MainLayout` consumes a `WorkspaceSnapshot` through `workspaceService`.

The workspace shell has also been split into file-level modules:

- `conversation-sidebar.tsx`: workspace navigation/sidebar.
- `flow-canvas.tsx`: conversation/workflow content, graph/chat view, and composer controls.
- `info-panel.tsx`: runtime/detail panel.
- `status-bar.tsx`: bottom runtime status surface.
- `top-bar.tsx`: top application bar.

The split preserves the existing DOM structure and styling while removing static workspace data from `src/main-layout/main-layout.tsx`.
