## 1. Layout Shell

- [x] 1.1 Refactor `src/main-layout/main-layout.tsx` to own local UI state for sidebar view mode, expanded folder groups, active info tab, and info panel collapsed state.
- [x] 1.2 Change the workspace grid to expanded columns `220px / 1fr / 300px` and collapsed columns `220px / 1fr / 0px` with a 200ms Tailwind CSS transition.
- [x] 1.3 Ensure the workspace grid is a `min-h-0` flex child between `TopBar` and `StatusBar` so all three panels share the same available height and bottom alignment.
- [x] 1.4 Keep the implementation frontend-only with no changes to service interfaces, runtime adapters, Tauri commands, Rust code, SQLite schema, or dependencies.

## 2. Sidebar

- [x] 2.1 Remove the six sidebar tool shortcuts while keeping Settings, visual style switching, and Help pinned at the sidebar bottom.
- [x] 2.2 Add agent-type marker metadata for known agents including Codex, Claude Code, OpenCode, and Gemini, with neutral fallback behavior for unknown types.
- [x] 2.3 Render an agent icon/color marker to the left of each session card title.
- [x] 2.4 Add activity and group view controls for the session list.
- [x] 2.5 Implement activity view groups in priority order: needs-input, pending-verification, in-progress, inactive, with visible counts.
- [x] 2.6 Implement folder group view with expandable and collapsible folder sections.
- [x] 2.7 Put session list content inside an internal `overflow-y-auto` region so long lists scroll within the sidebar.

## 3. Main Content

- [x] 3.1 Remove fixed canvas minimum-height behavior from the main content area and make the canvas fill remaining space with `flex-1 min-h-0`.
- [x] 3.2 Keep the composer/input area non-shrinking and fully contained inside the main content panel.
- [x] 3.3 Verify the main content panel expands smoothly when the information panel collapses.

## 4. Information Panel

- [x] 4.1 Add a collapse control that hides the information panel column without unmounting the panel subtree.
- [x] 4.2 Add a right-edge expand control that restores the information panel after collapse.
- [x] 4.3 Replace the old five-tab panel with Agent Info, Files, and Changes tabs.
- [x] 4.4 Render all three tab contents in keep-alive mode and toggle visibility without unmounting inactive tab content.
- [x] 4.5 Add an Agent Info progress summary with progress percentage and completed, in-progress, and pending task counts.
- [x] 4.6 Put information panel body content inside an internal `overflow-y-auto` region.

## 5. Verification

- [x] 5.1 Run `openspec validate "improve-main-layout-ui" --strict`.
- [x] 5.2 Run `npm run build`.
- [x] 5.3 Run `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; $env:CARGO_NET_OFFLINE="true"; cargo check --manifest-path src-tauri\Cargo.toml`.
- [x] 5.4 Perform a browser/Web visual smoke check for expanded/collapsed panel behavior, sidebar mode switching, internal scroll regions, and info tab state preservation.
- [x] 5.5 Verify settings pages use independent page scrolling while preserving fixed top navigation and left menu layout.
