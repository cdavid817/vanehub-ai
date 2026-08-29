# workbench-design-system-ui Specification Delta

## ADDED Requirements

### Requirement: Task-domain workbench navigation
The application SHALL organize primary business navigation into Sessions, Projects and Workspaces, Runs, Plan, and Quality while keeping Settings and Help in the global utility group.

#### Scenario: Render primary task domains
- **WHEN** the activity rail renders
- **THEN** it SHALL expose exactly the five business destinations with localized accessible names
- **AND** Loops and Schedules SHALL be reachable under Runs
- **AND** Board and Goals SHALL be reachable under Plan
- **AND** Agent Evaluation SHALL be reachable under Quality

#### Scenario: Use a narrow window
- **WHEN** the activity rail cannot show all optional labels
- **THEN** the five destination icons SHALL remain reachable without a hidden horizontal or vertical overflow trap
- **AND** tooltips and accessible names SHALL identify each destination

#### Scenario: Open a legacy destination
- **WHEN** a user opens a supported legacy Loops, Mission Control, Board, Goals, Evaluation, or Scheduled Tasks route
- **THEN** the router SHALL resolve the equivalent new task-domain route
- **AND** the current entity identity and safe filter state SHALL be preserved when valid

### Requirement: Shared destination layout primitives
Every first-class workbench destination SHALL compose the shared AppShell, DestinationLayout, PageHeader, Toolbar, Inspector, RuntimePanel, and asynchronous-state primitives appropriate to that destination instead of creating another independent shell.

#### Scenario: Render a management destination
- **WHEN** a collection or master-detail page renders
- **THEN** its title, bounded summary, one primary action, secondary action menu, toolbar, content, and optional inspector SHALL occupy the shared semantic regions

#### Scenario: Create a domain-specific page
- **WHEN** a feature requires a specialized board, timeline, table, or editor
- **THEN** the feature MAY supply domain content inside the shared regions
- **AND** it SHALL NOT duplicate global navigation, notification hosting, focus trapping, or pane-resize infrastructure

### Requirement: Container-responsive pane composition
The workbench SHALL choose inline, overlay-sheet, or single-surface pane composition from the available workbench container width while preserving a minimum readable main work surface.

#### Scenario: Render a wide workbench
- **WHEN** the available container width is at least the documented wide threshold
- **THEN** context navigation and the inspector MAY render inline beside the main work surface
- **AND** their widths SHALL be bounded and user-resizable

#### Scenario: Main content would become too narrow
- **WHEN** inline auxiliary panes would reduce the main surface below its minimum readable width
- **THEN** the workbench SHALL convert the inspector to a sheet before compressing the main surface
- **AND** it SHALL convert context navigation to a sheet if more width is still required

#### Scenario: Resize back to wide
- **WHEN** automatic responsive composition had converted a user-preferred inline pane to a sheet and sufficient width becomes available again
- **THEN** the workbench SHALL restore the user's prior preferred pane state without losing selection or draft state

### Requirement: Global command center
The workbench SHALL provide a keyboard-accessible command center that searches bounded safe summaries and executes context-aware navigation and UI commands without sending the query to an Agent or model provider.

#### Scenario: Open with keyboard
- **WHEN** the user presses the platform command-center shortcut
- **THEN** the command center SHALL open with focus in its query field
- **AND** Escape SHALL close it and return focus to the invoking context

#### Scenario: Search workbench entities
- **WHEN** the user enters a query
- **THEN** registered Session, Project, Run, Goal, Work Item, and Evaluation providers SHALL return bounded cancellable results
- **AND** a stale response SHALL NOT replace a newer query

#### Scenario: Run a contextual command
- **WHEN** the user selects an available command
- **THEN** the workbench SHALL execute that command through its owning UI or frontend service boundary
- **AND** unavailable commands SHALL be absent or disabled with an accessible explanation

#### Scenario: Protect sensitive data
- **WHEN** search results are assembled
- **THEN** they SHALL exclude prompts, responses, tool inputs, credentials, unrestricted paths, raw errors, and log bodies

### Requirement: Explicit page lifecycle policy
Every lazy destination and heavy settings page SHALL declare mounted retention, hidden-state suspension, focus refresh, and permitted background update behavior.

#### Scenario: Leave an ordinary page
- **WHEN** a page declares keepAlive never and the user navigates away
- **THEN** the page SHALL unmount and release page-owned polling, timers, observers, and subscriptions
- **AND** cached query data MAY remain available

#### Scenario: Hide a draft page
- **WHEN** a page declares draft-only retention and contains a permitted unsaved draft
- **THEN** the shell SHALL preserve or explicitly protect that draft according to the page contract
- **AND** secret values SHALL NOT be serialized into generic layout storage

#### Scenario: Run continues in background
- **WHEN** an Agent, Loop, evaluation, or scheduled operation continues while its page unmounts
- **THEN** service-owned execution SHALL continue
- **AND** returning to the page SHALL reconcile bounded canonical state

### Requirement: Unified asynchronous view states
Shared workbench surfaces SHALL distinguish initial loading, background refresh, no data, no filter matches, stale data, unavailable evidence, restricted evidence, retryable error, and terminal failure.

#### Scenario: Refresh loaded data
- **WHEN** a bounded refresh starts after content has loaded
- **THEN** the surface SHALL keep existing content visible and indicate refresh without replacing it with a blank loading view

#### Scenario: Mutation affects one entity
- **WHEN** a row, card, or detail action is pending
- **THEN** only that target and conflicting actions SHALL be disabled
- **AND** unrelated navigation and content SHALL remain operable

#### Scenario: Filter returns no matches
- **WHEN** data exists but active filters return no visible items
- **THEN** the surface SHALL render a filtered-empty state with clear-filter action rather than the first-run empty state

#### Scenario: Evidence is absent
- **WHEN** an owning service reports that a detail facet has no evidence
- **THEN** the UI SHALL render a localized unavailable state and SHALL NOT render fixture or placeholder content

### Requirement: Shared keyboard interaction models
Tabs, toolbars, menus, listboxes, trees, dialogs, sheets, grids, and drag alternatives SHALL use their corresponding documented keyboard interaction models consistently across destinations.

#### Scenario: Operate a tab list
- **WHEN** focus is in a workbench tab list
- **THEN** Arrow keys, Home, and End SHALL move focus according to orientation and activation policy
- **AND** only the active or current tab SHALL be in the normal tab sequence

#### Scenario: Operate a toolbar
- **WHEN** focus enters a grouped toolbar
- **THEN** the group SHALL use one normal Tab stop and directional navigation where appropriate
- **AND** disabled controls SHALL remain explainable without trapping focus

#### Scenario: Complete a drag-only task by keyboard
- **WHEN** a pointer user can drag an entity to change category, stage, or order
- **THEN** a keyboard and assistive-technology user SHALL have an equivalent menu, picker, or command path

#### Scenario: Close a modal surface
- **WHEN** a dialog or modal sheet closes
- **THEN** focus SHALL return to the control or logical object that opened it unless that object no longer exists

### Requirement: Semantic theme and localization parity
All new workbench primitives and states SHALL use semantic tokens and synchronized locale resources with equivalent structure and operability in futuristic and minimal themes.

#### Scenario: Render both themes
- **WHEN** a required page state renders in futuristic or minimal
- **THEN** content hierarchy, focus, status meaning, disabled behavior, target size, and responsive composition SHALL remain equivalent

#### Scenario: Render a supported locale
- **WHEN** new visible text, accessible names, tooltips, dates, recurrence labels, empty states, or errors render
- **THEN** they SHALL use the active locale and locale-aware formatting
- **AND** stable ids and command-like values MAY remain literal

#### Scenario: Express status
- **WHEN** success, running, warning, failure, blocked, or attention state is shown
- **THEN** the state SHALL be identifiable by text or accessible description and shape or icon in addition to color

### Requirement: Structural frontend performance budgets
The repository SHALL provide deterministic structural performance fixtures for large workbench histories and hidden-page resource use.

#### Scenario: Render one thousand list entities
- **WHEN** the Session, Run, Work Item, or Project large fixture is opened
- **THEN** the UI SHALL keep returned pages and rendered rows bounded through pagination or virtualization
- **AND** query count SHALL NOT grow once per visible entity

#### Scenario: Render long conversation history
- **WHEN** a conversation fixture contains five thousand dynamic-height messages
- **THEN** the UI SHALL not create one mounted MessageItem per history record
- **AND** streaming and prepend operations SHALL preserve the documented scroll anchor

#### Scenario: Hide a high-update page
- **WHEN** Mission Control, Evaluation, Logs, or another update-heavy page becomes hidden
- **THEN** its page-owned high-frequency timers and update batches SHALL satisfy the declared lifecycle budget

### Requirement: Workbench visual regression contract
Core workbench surfaces SHALL have deterministic visual regression coverage across registered themes, required locales, representative widths, and browser and desktop runtimes.

#### Scenario: Capture the core matrix
- **WHEN** visual regression runs
- **THEN** it SHALL cover Sessions, Runtime Panel, Inspector, Runs attention and detail, Loop action-required, Board, Evaluation comparison, Schedule editor, and Settings search states

#### Scenario: Review a changed baseline
- **WHEN** a visual baseline changes
- **THEN** the change SHALL be reviewed with a stated reason and SHALL NOT be updated solely to make the test pass

#### Scenario: Report desktop coverage
- **WHEN** native visual or smoke evidence is recorded
- **THEN** the report SHALL identify each operating system actually executed and SHALL NOT claim unexecuted platforms passed
