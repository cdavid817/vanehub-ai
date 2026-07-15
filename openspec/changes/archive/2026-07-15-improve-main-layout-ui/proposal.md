## Why

The workspace shell currently mixes navigation, session status, canvas content, tool shortcuts, and runtime details in a layout that becomes cramped as session count and panel content grow. This change improves the main layout so developers can scan agent sessions, keep the composer stable, and collapse secondary details without losing local UI state.

## What Changes

- Add a dedicated main layout UI contract for the three-panel workspace shell used by both Tauri desktop and browser Web runtimes.
- Remove the six tool shortcuts from the sidebar bottom area while keeping the bottom utility row for Settings, visual style switching, and Help.
- Add agent-type visual markers to session cards so Codex, Claude Code, OpenCode, Gemini, and future agents can be distinguished by icon and color.
- Add sidebar session list view modes:
  - Activity mode groups sessions into needs-input, pending-verification, in-progress, and inactive buckets with per-bucket counts and priority ordering.
  - Group mode groups sessions by owning folder and supports folder expand/collapse state.
- Give the sidebar session list its own internal scrolling region so long lists do not scroll the whole app shell.
- Make the central canvas area flex with available panel size instead of relying on a fixed minimum height.
- Keep the bottom composer/input area at a fixed usable size within the main content frame.
- Add a collapsible information panel that preserves mounted state while transitioning between expanded and collapsed states.
- Replace the information panel's five tabs with three keep-alive tabs: Agent Info, Files, and Changes.
- Add an Agent Info progress summary with overall percent and completed, in-progress, and pending task counts.
- Give the information panel content its own internal scrolling region.
- Normalize the three-panel grid to 220px / 1fr / 300px when expanded and 220px / 1fr / 0px when the information panel is collapsed, using a 200ms CSS transition.

## Capabilities

### New Capabilities

- `main-layout-ui`: Defines the workspace shell layout, sidebar session organization, central content sizing, collapsible information panel behavior, keep-alive panel tabs, internal scrolling rules, and responsive three-panel proportions.

### Modified Capabilities

- `settings-center-ui`: Keeps Settings as the sidebar utility entry while the six removed tool shortcuts are handled through the settings center.

## Impact

- Affects both Tauri desktop and browser Web runtimes because both render the shared React workspace UI.
- Primary implementation target is `src/main-layout/main-layout.tsx`.
- No backend, Rust, SQLite, Tauri command, service adapter, or agent runtime boundary changes are expected.
- No new npm or Rust dependencies are expected.
- Frontend/backend isolation remains unchanged: React UI must continue to avoid direct Tauri `invoke()` calls and must use existing service boundaries when runtime data is needed.
