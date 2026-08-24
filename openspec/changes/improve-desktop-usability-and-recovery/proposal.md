## Why

Several core desktop workflows are visually cramped at common window sizes, provide incomplete feedback, or leave the user without a recovery path after an inactive session fails. The next usability pass must make these surfaces responsive, actionable, and understandable while fixing documentation and update reliability defects.

## What Changes

- Add a polished, accessible multi-Agent creation dialog with responsive seat assignment and clear validation.
- Make the workspace, settings navigation, task board, and goal center responsive and improve their information hierarchy, spacing, truncation, and interaction affordances.
- Add an explicit session recovery action for recoverable failed or disconnected sessions, including a contextual right-click entry point.
- Provide a visible startup loading state, route the workspace help action to the user guide, and render guide content safely with working internal and external links.
- Correct task-board Windows path presentation, reposition transient session notifications, make update checks recoverable, and remove the obsolete preview label from About.

## Capabilities

### New Capabilities

- `desktop-usability`: Responsive desktop workspace layout, startup feedback, contextual help, and human-centered action feedback.

### Modified Capabilities

- `multi-agent-group-chat`: Define an accessible multi-Agent session creation flow.
- `session-recovery`: Allow a user to explicitly recover a recoverable failed or disconnected session.
- `agent-task-list`: Require task-board path rendering to use user-facing paths rather than platform namespace prefixes.
- `goal-management`: Require a responsive, actionable goal-center presentation.
- `settings-center-ui`: Require responsive selected-navigation rendering and remove obsolete About preview status from the user experience.
- `user-guide-documentation`: Require safe rendered guide content and valid navigable links.
- `signed-desktop-auto-update`: Require an actionable retry path after an update-check failure.

## Impact

- Affects both desktop and Web/mock UI rendering; native session and updater operations remain behind existing service interfaces.
- May add service methods only where session recovery requires an explicit runtime action; corresponding Tauri and Web adapters will remain contract-compatible.
- Affects React components, shared layout/style primitives, localized strings, desktop and browser tests, and user-guide content routing. No new UI framework or direct component-to-Tauri calls are introduced.
