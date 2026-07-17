## Why

The workspace currently combines session navigation with global Settings and Help actions inside the session sidebar, leaving no stable top-level navigation surface and forcing the session list to remain visible even when users want more room for the active conversation. A compact activity bar and collapsible session sidebar will clarify navigation, improve workspace density, and reserve a discoverable location for future scheduled-task functionality without implementing that feature prematurely.

## What Changes

- Add a fixed icon-only activity bar at the far left of the workspace, with Session and Scheduled Tasks entries at the top and Settings and Help entries anchored at the bottom.
- Make the Session activity entry expand and collapse the existing session sidebar while preserving the sidebar's mounted UI state and allowing the main content area to consume the released width.
- Move the existing Settings and Help controls out of the session sidebar into the activity bar while preserving the existing Settings navigation behavior.
- Add a Scheduled Tasks placeholder entry that displays a localized coming-soon indication but does not add scheduling pages, routes, service APIs, persistence, or native runtime behavior.
- Provide active, hover, focus, tooltip, and accessible-name behavior for every icon-only activity entry in both supported visual styles and synchronized locales.
- Define responsive behavior so the activity bar remains available while the session sidebar does not make the chat workspace unusable at narrow widths.
- Apply the same workspace-shell behavior in both the Tauri desktop runtime and the browser/Web runtime.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `main-layout-ui`: Extend the workspace shell with an icon-only activity bar, relocatable utility actions, a state-preserving collapsible session sidebar, a non-functional Scheduled Tasks placeholder, and responsive layout behavior.
- `settings-center-ui`: Change the workspace entry point for the existing Settings center from the session sidebar utility row to the new activity bar without changing Settings routing, mounting, or service-backed behavior.

## Impact

- Frontend workspace layout and state composition in `src/main-layout/`, including the grid proportions and session-sidebar ownership in `main-layout.tsx`, `session-sidebar.tsx`, and shared workspace styles.
- Workspace localization resources, accessible labels/tooltips, component tests, and Playwright coverage for activity navigation and sidebar collapse/expand behavior.
- Existing `/settings` navigation remains intact; Help remains a workspace utility entry and Scheduled Tasks remains a presentation-only placeholder.
- No changes to Rust commands, SQLite, frontend service interfaces, Tauri/Web adapters, dependencies, or the frontend/backend isolation boundary are expected.
