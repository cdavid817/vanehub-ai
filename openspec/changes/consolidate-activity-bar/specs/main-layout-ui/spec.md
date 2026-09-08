## MODIFIED Requirements

### Requirement: Workspace activity bar
The workspace shell SHALL render a persistent icon-only activity bar at the far left of the workspace body in both the Tauri desktop frontend and browser Web runtime, with exactly four entries.

#### Scenario: Render activity entries
- **WHEN** the workspace activity bar renders
- **THEN** it SHALL show Sessions, Inbox, and Automations entries in a top group
- **AND** it SHALL show a Settings entry anchored in a bottom group
- **AND** it SHALL NOT show Loops, Scheduled Tasks, Task Board, Goals, Evaluations, System Activity, Mission Control, or Help entries
- **AND** the entries SHALL display icons without visible text labels

#### Scenario: Identify icon-only entries
- **WHEN** an activity-bar entry is available to pointer, keyboard, or assistive-technology users
- **THEN** it SHALL provide a synchronized accessible name and tooltip in every registered locale
- **AND** the tooltip SHALL include the entry's keyboard shortcut
- **AND** it SHALL expose stable hover, focus, and active styling without shifting adjacent controls

#### Scenario: Render from configuration
- **WHEN** the activity bar is given its entry configuration
- **THEN** it SHALL render entries from that configuration rather than from per-entry props
- **AND** the Sessions entry SHALL keep its sidebar expand/collapse semantics and `aria-expanded` state

#### Scenario: Single badge owner
- **WHEN** unread attention items exist
- **THEN** the Inbox entry SHALL render an unread badge
- **AND** no other activity-bar entry SHALL render a badge

#### Scenario: Open settings from activity bar
- **WHEN** the user activates the Settings activity entry
- **THEN** the system SHALL open the existing settings center without requiring a runtime-specific backend call
- **AND** the settings sidebar SHALL expose Help in its bottom group

#### Scenario: Open Inbox from activity bar
- **WHEN** the user activates the Inbox activity entry
- **THEN** the workspace SHALL open the Inbox as the active workspace destination on its Needs attention view
- **AND** it SHALL preserve mounted session workspace state for later return

#### Scenario: Open Automations from activity bar
- **WHEN** the user activates the Automations activity entry
- **THEN** the workspace SHALL open Automations as the active workspace destination on its most recently used tab, defaulting to Loops
- **AND** it SHALL preserve mounted session workspace state for later return

#### Scenario: Return to sessions from activity bar
- **WHEN** the user activates the Sessions activity entry while Inbox or Automations is active
- **THEN** the workspace SHALL restore the session workspace without losing its selected session and mounted tab state

#### Scenario: Navigate entries by keyboard shortcut
- **WHEN** the user presses `Mod+1`, `Mod+2`, `Mod+3`, or `Mod+4` while no text input or composer has focus
- **THEN** the workspace SHALL activate Sessions, Inbox, Automations, or Settings respectively
- **AND** the same chord SHALL be ignored while a text input or the composer has focus

#### Scenario: Open Loops from activity bar
- **WHEN** the user wants Loops from the activity bar
- **THEN** the activity bar SHALL NOT expose a dedicated Loops entry
- **AND** activating Automations and its Loops tab SHALL open the Loop Center with mounted session workspace state preserved for later return

#### Scenario: Open scheduled tasks from activity bar
- **WHEN** the user wants scheduled tasks from the activity bar
- **THEN** the activity bar SHALL NOT expose a dedicated Scheduled Tasks entry
- **AND** activating Automations and its Scheduled tab SHALL render scheduled-task management inline as page content rather than as a dialog
- **AND** it SHALL NOT show a coming-soon placeholder

#### Scenario: Preserve future help entry
- **WHEN** the activity bar renders its bottom group
- **THEN** it SHALL render only the Settings entry there
- **AND** Help SHALL remain reachable from the settings sidebar bottom group without introducing a new Help destination

## ADDED Requirements

### Requirement: Inbox destination
The workspace SHALL provide an Inbox destination that hosts the Agent Mission Control overview and the System Activity feed as one attention surface.

#### Scenario: Render Inbox sections
- **WHEN** the Inbox destination renders
- **THEN** it SHALL show Needs attention, Running, and Recently finished sections
- **AND** Needs attention SHALL include Mission Control attention runs and unread System Activity items of warning or critical severity
- **AND** each row SHALL expose the same owning-surface navigation and control actions the hosted surface already defines

#### Scenario: Count unread items once
- **WHEN** an item is both a Mission Control attention run and a System Activity event for the same run
- **THEN** the Inbox badge SHALL count it once

#### Scenario: Reach the activity log
- **WHEN** the user expands the Activity log disclosure inside Inbox
- **THEN** the existing System Activity timeline SHALL render with its search and severity filters
- **AND** its export, rebuild, and health controls SHALL NOT appear there because they live on the observability settings page

#### Scenario: Toggle list and board view
- **WHEN** the user switches the Inbox view toggle to Board
- **THEN** the existing unified task board SHALL render in place of the list sections
- **AND** the selected view SHALL persist with other layout preferences

### Requirement: Automations destination
The workspace SHALL provide an Automations destination that hosts Loops, Scheduled Tasks, and Goals as in-page tabs.

#### Scenario: Render Automations tabs
- **WHEN** the Automations destination renders
- **THEN** it SHALL show Loops, Scheduled, and Goals tabs
- **AND** each tab SHALL lazy-load its hosted surface on first visit and keep it mounted afterwards

#### Scenario: Deep-link to a tab
- **WHEN** the workspace location is `/workspace/automations/<tab>`
- **THEN** the matching tab SHALL be active on render

### Requirement: Destination sub-views and legacy route redirects
Workspace locations SHALL support an optional sub-view segment and SHALL redirect retired destinations to their new hosts.

#### Scenario: Parse a sub-view path
- **WHEN** the pathname is `/workspace/<destination>/<view>` for the inbox or automations destination
- **THEN** the parsed location SHALL carry both the destination and the view
- **AND** an unknown view SHALL fall back to the destination's default view

#### Scenario: Redirect a retired destination
- **WHEN** the pathname or persisted location names `loops`, `goals`, `work-board`, `mission-control`, or `system-activity`
- **THEN** the location SHALL resolve to the corresponding Inbox or Automations view without an intermediate blank panel

#### Scenario: Redirect evaluations to settings
- **WHEN** the pathname or persisted location names `evaluations`
- **THEN** the workspace SHALL open the settings center on the evaluation page and restore the session workspace behind it

#### Scenario: Reach demoted surfaces from search
- **WHEN** the user searches from the top bar
- **THEN** Task Board, Evaluations, and each Automations tab SHALL appear as navigable entries
