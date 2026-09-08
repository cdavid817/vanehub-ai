## Context

The activity bar is rendered by `WorkspaceActivityBar` with one prop per entry and one `xxxVisited` flag per destination in `main-layout.tsx`. `WorkspaceDestination` is a flat union of seven values persisted to localStorage and mirrored in the URL. Each destination is a self-contained lazily loaded feature directory.

The consolidation is a routing and composition change. It must not touch the hosted features' service contracts, fixtures, or backend commands.

## Goals / Non-Goals

Goals:

- Four primary entries whose labels describe intent (work with an agent, see what needs me, run unattended, configure) rather than implementation.
- Exactly one badge-bearing entry.
- Every retired entry remains reachable through a sub-view, a settings page, or top-bar search — nothing is removed.
- Legacy URLs and persisted locations keep working.

Non-Goals:

- Redesigning the internals of Mission Control, System Activity, Loop Center, Goal Center, Task Board, or Evaluation Center.
- Changing the session sidebar, information panel, or settings center layout beyond adding the Help entry and the Evaluation page.
- Backend or persistence changes.

## Decisions

### Destination model

```ts
export type WorkspaceDestination = "sessions" | "inbox" | "automations";
export type InboxView = "attention" | "running" | "recent";
export type AutomationsView = "loops" | "scheduled" | "goals";
```

`WorkspaceLocation` gains an optional `view: string | null`. Paths are `/workspace/<destination>[/<view>]`; the sessions destination keeps its existing `/workspace/sessions/<id|new>` shape. `parseWorkspaceLocation` maps legacy destinations:

| Legacy path | New location |
| --- | --- |
| `/workspace/mission-control` | `inbox/attention` |
| `/workspace/system-activity` | `inbox/attention` |
| `/workspace/work-board` | `inbox/attention` with board view mode |
| `/workspace/loops` | `automations/loops` |
| `/workspace/goals` | `automations/goals` |
| `/workspace/evaluations` | settings page `evaluation` |

Unknown values continue to fall back to `sessions`.

### Inbox composition

`src/inbox/inbox.tsx` renders three sections from the existing data sources:

- **Needs attention** — Mission Control's attention-sorted runs plus System Activity items whose severity is `warning` or `critical` and are unread.
- **Running** — Mission Control runs in non-terminal states.
- **Recently finished** — Mission Control's recent section.

System Activity items are rendered through the existing `activity-presentation-registry` as inbox rows; the timeline view remains available as a secondary "Activity log" disclosure inside Inbox for users who want the raw feed. `event-coalescer.ts` is reused unchanged. The unread count that previously fed the System Activity badge now feeds the Inbox badge; Mission Control's attention count is added to it.

Task Board appears as a List / Board toggle in the Inbox header. Board mode renders the existing `work-board` component; the toggle is persisted with the other layout preferences.

### Automations composition

`src/automations/automations.tsx` is a thin tab shell: Loops (`loop-center`), Scheduled (`scheduled-task-*` rendered inline instead of inside a dialog), Goals (`goal-center`). Each tab lazy-loads on first visit exactly as the destinations do today. The scheduled-task form keeps its dialog-era focus and validation behavior; only the container changes.

### Activity bar rendering

`WorkspaceActivityBar` takes `items: ActivityItem[]` and `utilityItems: ActivityItem[]`:

```ts
interface ActivityItem {
  id: WorkspaceDestination | "settings";
  icon: LucideIcon;
  label: string;
  shortcut: string;      // "Mod+1"
  badge?: number;
  onSelect: () => void;
  ariaControls?: string;
}
```

The Sessions item keeps its `aria-expanded` sidebar toggle behavior. Badges render only when `badge > 0`; the design allows a badge on any item but this change wires one only for Inbox.

### Keyboard shortcuts

`Mod+1..4` are registered at the workspace shell level and ignored while a text input or the composer has focus. Tooltips display the shortcut.

### Settings additions

- `help` moves from an activity-bar callback to a bottom-group entry in `settings-sidebar.tsx`.
- New `evaluation` settings page wraps `evaluation-center` unchanged.
- `observability` gains a "System activity" section hosting export, rebuild, and health-panel controls moved from `system-activity-controls.tsx` and `system-activity-health-panel.tsx`.

## Risks / Trade-offs

- Users who relied on the System Activity timeline as a first-class page get it one click deeper, inside Inbox. Mitigation: the Activity log disclosure and top-bar search entry.
- Goals is a grouping container over loops, board items, sessions, and runs; hosting it as an Automations tab is the least disruptive move now, but it may later be better expressed as a "group by goal" filter on Inbox and Automations. This change keeps the Goal Center intact so that decision can be made separately.
- Merging two badge sources could double-count an item that is both a Mission Control attention run and a System Activity event. The coalescer already keys on run id; the Inbox count de-duplicates by that key.

## Migration Plan

1. Land routing and the new activity bar behind the same build; no feature flag is needed because legacy paths redirect.
2. Existing E2E flows that navigate by clicking retired entries are rewritten to use the new entries or top-bar search.
3. Locale keys for retired entries are removed only after the E2E rewrite lands, so a partial merge never ships an untranslated icon.
