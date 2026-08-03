## Why

The selected-Agent Skill view currently uses immediate-action checkboxes and vertically stacked Assigned/Available lists, making assignment look like passive selection state and forcing excessive scrolling as the catalog grows. The interaction should make the Agent-to-Skill relationship, action direction, pending state, and failure ownership obvious without mixing lifecycle administration into the assignment task.

## What Changes

- Replace the selected-Agent checkbox rows with explicit Assign and Remove actions that communicate an immediate mutation rather than a local selection.
- Present Assigned and Available Skills as a responsive selection board: parallel columns on wide layouts and ordered sections on narrow layouts.
- Keep assignment status, global enabled/paused state, preview, pending state, and row-level errors visible while removing edit/delete lifecycle distractions from selected-Agent views.
- Preserve the existing compact All Skills lifecycle inventory, filters, stable Agent navigation, granular CLI/API binding operations, and mount-path disclosure.
- Add synchronized localization and responsive/accessibility coverage for both CLI mount assignment and API prompt assignment.
- Document the distinction between All Skills, Unassigned, global enablement, and per-Agent assignment in the equivalent English and Simplified Chinese user guides.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `settings-skill-management-ui`: Add a responsive, explicit Agent-to-Skill selection board with unambiguous assignment actions and focused row controls.

## Impact

- Affects the shared React settings UI in both Tauri desktop and Web/mock runtimes.
- Primarily changes `SkillsPage`, the selected-Agent Skill list components, shared localization, and frontend/Playwright tests.
- Adds equivalent Skill-management chapters to the English and Simplified Chinese user guides.
- Reuses existing `AgentService` granular CLI/API bind and unbind methods; no Rust command, database schema, dependency, or runtime adapter boundary changes are required.
