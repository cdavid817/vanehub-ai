## Why

The workspace activity bar has grown to ten entries: Sessions, Loops, Scheduled Tasks, Task Board, Goals, Evaluations, System Activity, Mission Control, Settings, and Help. Most of them are implementation surfaces rather than distinct user intents:

- Mission Control ("attention inbox", "recent", run monitoring) and System Activity (unread badge, severity timeline) both answer "what needs me right now", so the attention badge sits on a secondary surface and the two disagree about where the user should look.
- Loops, Scheduled Tasks, and Goals are three shapes of the same intent — unattended execution — yet Scheduled Tasks is the only entry that opens a dialog instead of a destination.
- Task Board overlaps Mission Control's run list, Evaluations is a diagnostic tool, and Help already exists as a settings page (`onHelp` calls `onOpenSettings("help")`).

Mainstream peers (Claude Desktop, Codex app, Conductor, VS Code defaults) keep three to five primary entries and push everything else one level down. A first-time user of VaneHub currently cannot tell which icon is the main path.

## What Changes

- Reduce the activity bar to four entries: **Sessions**, **Inbox**, **Automations** in the top group and **Settings** in the bottom group. Help moves to the settings sidebar's bottom group.
- **Inbox** hosts the existing Mission Control overview and the System Activity feed as one surface with three sections (Needs attention / Running / Recently finished). It is the only activity-bar entry that renders an unread badge. Task Board becomes a List / Board view toggle inside Inbox rather than a destination.
- **Automations** hosts Loops, Scheduled Tasks, and Goals as in-page tabs. Scheduled Tasks becomes a page-level surface instead of a dialog; its form, validation, and service-boundary behavior are unchanged.
- **Evaluations** moves to a new settings page (`evaluation`) under the Agents group. System Activity's export, rebuild, and health controls move to the existing `observability` settings page.
- Collapse `WorkspaceDestination` to `sessions | inbox | automations` with an optional sub-view path segment (`/workspace/inbox/recent`, `/workspace/automations/loops`). Legacy paths (`/workspace/loops`, `/workspace/mission-control`, `/workspace/system-activity`, `/workspace/goals`, `/workspace/work-board`, `/workspace/evaluations`) redirect to their new hosts.
- Bind `Mod+1` … `Mod+4` to the four entries. The existing top-bar search exposes Board, Evaluations, and each Automations tab as navigable entries so demoted surfaces stay reachable by keyboard.
- Replace the eleven callback props of `WorkspaceActivityBar` with a declarative `items` list so the bar renders from configuration.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `main-layout-ui`: Redefine the activity bar entries, badge ownership, keyboard shortcuts, destination sub-views, and legacy route redirects.
- `scheduled-task-management`: Host scheduled-task management as an Automations tab instead of a standalone dialog while preserving the form, validation, refresh, and error-handling contract.
- `settings-center-ui`: Insert the Agent Evaluation page after CLI Parameters, pin Help in the sidebar bottom group, and host the System activity export, rebuild, and health controls on the Observability page.
- `frontend-runtime-architecture`: Name Sessions, Inbox, and Automations (with sub-views) as the routed destinations and require retired paths and remembered locations to redirect.
- `loop-management-ui`: Reach the Loop Center through the Loops tab of Automations rather than a Loops activity entry.

Mission Control, System Activity, Loops, Goals, Task Board, and Evaluations keep their existing behavioral specs; only their hosting surface changes.

## Impact

- Frontend only: `src/main-layout/workspace-route.ts`, `src/main-layout/workspace-activity-bar.tsx`, `src/main-layout/main-layout.tsx`, `src/main-layout/use-main-layout-model.ts`, a new `src/inbox/` composition over `mission-control/` and `system-activity/`, a new `src/automations/` shell over `loop-center/`, `scheduled-task-*`, and `goal-center/`, plus `src/settings/settings-pages.ts` and `settings-page-types.ts`.
- No Tauri command, service interface, runtime adapter, database, or dependency changes. Lazy loading of each hosted surface is preserved.
- Locale resources gain `layout.activityBar.inbox` / `.automations` and lose the retired entry keys in every registered locale.
- Persisted workspace location (`vanehub.workspace.location.v1`) needs no migration: unknown destinations already fall back to Sessions, and legacy paths redirect on parse.
