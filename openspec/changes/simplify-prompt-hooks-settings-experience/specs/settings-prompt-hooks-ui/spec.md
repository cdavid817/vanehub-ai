## MODIFIED Requirements

### Requirement: Service-backed Prompt Hooks settings page
The system SHALL provide a service-backed Prompt Hooks settings page in the settings center and SHALL load data for each task-oriented view through the frontend Agent service boundary.

#### Scenario: Open Prompt Hooks page
- **WHEN** a user opens the Prompt Hooks settings page
- **THEN** the page SHALL open the Hook management view and load Prompt Hooks, supported CLI agents, and summary statistics through the frontend Agent service boundary
- **AND** it SHALL present the statistics as a compact inventory summary rather than a separate grid of metric cards
- **AND** React components SHALL NOT call Tauri `invoke()` directly

#### Scenario: Open runtime records
- **WHEN** a user opens the Prompt Hooks runtime-records view
- **THEN** the page SHALL load recent safe trace summaries through the frontend Agent service boundary
- **AND** it SHALL expose assembled-prompt preview from that diagnostics-oriented view

#### Scenario: Preserve loaded data during refresh
- **WHEN** Prompt Hook settings data refreshes while previous data is available
- **THEN** the page SHALL keep the previous data visible with refreshing state instead of replacing the page with a blank panel

### Requirement: Prompt Hook filtering and grouping
The Prompt Hooks management view SHALL support compact primary filtering, progressive access to additional filters, and category grouping by operational metadata.

#### Scenario: Filter hooks
- **WHEN** the user searches or selects an enabled-state or CLI-binding filter
- **THEN** the page SHALL show only matching Prompt Hooks
- **AND** search, enabled state, and CLI binding SHALL remain directly accessible without opening an additional-filter surface

#### Scenario: Use additional filters
- **WHEN** the user opens additional filters and selects a source, stage, or category criterion
- **THEN** the page SHALL apply the criteria together with the primary filters
- **AND** it SHALL show that additional filters are active and provide one action to clear them

#### Scenario: Display category groups
- **WHEN** Prompt Hooks are listed
- **THEN** the page SHALL group matching Hooks under localized labels for `bootstrap`, `callback`, `dynamic`, `law`, `navigation`, `routing`, and `static`
- **AND** each group SHALL expose its matching item count and an accessible expand or collapse control
- **AND** Hooks within each group SHALL preserve stable execution order

### Requirement: Prompt Hook card controls
Each Prompt Hook item SHALL use a compact summary row and SHALL expose controls appropriate to its source and governance flags without displaying all configuration fields in every inventory item.

#### Scenario: Scan a Hook row
- **WHEN** a Prompt Hook appears in the management inventory
- **THEN** its compact row SHALL identify its name, source, enabled state, live version or unpublished state, and CLI-binding count
- **AND** hash, token estimate, full CLI checkboxes, and governance metadata SHALL not occupy the default row

#### Scenario: Open Hook details
- **WHEN** a user activates a Hook row
- **THEN** the page SHALL open one bounded detail surface for that Hook
- **AND** the surface SHALL expose overview, CLI binding, content and publication, and version-history information allowed by the Hook source and governance flags

#### Scenario: Toggle hook enabled state
- **WHEN** a user toggles an enabled control for a disableable Prompt Hook
- **THEN** the page SHALL submit the change through the Agent service and refresh affected hook data

#### Scenario: Disable immutable toggle
- **WHEN** a Prompt Hook has `disableable=false`
- **THEN** the page SHALL show the enabled state as locked and SHALL NOT submit a disable request from the control

#### Scenario: Update CLI bindings
- **WHEN** a user changes the CLI binding checkboxes in the Hook detail surface
- **THEN** the page SHALL submit the stable agent id binding set through the Agent service

### Requirement: User Prompt Hook dialogs
The Prompt Hooks settings page SHALL provide one coherent detail workflow for custom Prompt Hook creation, editing, deletion confirmation, content preview, and publication lifecycle actions.

#### Scenario: Create custom hook
- **WHEN** a user opens the create action and submits a valid custom Prompt Hook form
- **THEN** the page SHALL call the Agent service to create the Hook
- **AND** user-visible validation labels and errors SHALL be localized

#### Scenario: Edit custom hook
- **WHEN** a user opens a user-created Prompt Hook
- **THEN** one detail workflow SHALL allow navigation among editable metadata and bindings, template draft and publication controls, and version history
- **AND** the inventory SHALL NOT expose separate edit and advanced entry points for the same Hook
- **AND** the workflow SHALL prevent changing immutable identity fields in a way the service rejects

#### Scenario: Preview hook content
- **WHEN** a user explicitly opens a Prompt Hook preview
- **THEN** the page SHALL request rendered content through the service boundary and show it in a bounded preview dialog

#### Scenario: Delete custom hook
- **WHEN** a user requests deletion from the detail workflow or its overflow actions
- **THEN** the page SHALL show a localized confirmation before submitting deletion through the Agent service

### Requirement: Prompt Hook trace display
The Prompt Hooks runtime-records view SHALL show safe trace summaries by default and full content only after explicit preview.

#### Scenario: Keep diagnostics separate from management
- **WHEN** a user is viewing the Hook management inventory
- **THEN** recent Hook traces and assembled-prompt diagnostics SHALL not be appended below the inventory
- **AND** the page SHALL provide a clear navigation control to the runtime-records view

#### Scenario: Show trace summaries
- **WHEN** recent Prompt Hook traces are available in the runtime-records view
- **THEN** the page SHALL display hook id, status, content hash, token estimate, timestamp, and skip reason when present
- **AND** it SHALL NOT show full rendered content in the default trace list

#### Scenario: Explicit trace content preview
- **WHEN** the user explicitly requests content preview from a trace or hook
- **THEN** the page SHALL show the rendered content returned by the service in a bounded dialog

### Requirement: Prompt Hooks visual and localization consistency
The Prompt Hooks settings page SHALL follow the shared settings visual system and i18n contract while using progressive disclosure across compact, detail, and diagnostics surfaces.

#### Scenario: Render in both visual styles
- **WHEN** the active visual style is `futuristic` or `minimal`
- **THEN** the Prompt Hooks page SHALL use shared settings primitives, semantic tokens, compact panels, stable controls, and icons consistent with the rest of the settings center

#### Scenario: Adapt the detail workflow
- **WHEN** the Hook detail workflow is opened at a desktop or narrow viewport
- **THEN** it SHALL remain bounded, keyboard accessible, and usable without horizontal page overflow
- **AND** primary save or publish state SHALL remain visible without duplicating action entry points

#### Scenario: Localize Prompt Hooks page
- **WHEN** the Prompt Hooks page renders in Simplified Chinese or English
- **THEN** view navigation, compact summaries, filters, category groups, source labels, stage labels, statuses, actions, detail sections, dialogs, validation messages, empty states, and trace labels SHALL use synchronized locale resources

### Requirement: Windowed large Prompt Hook inventories
The Prompt Hooks settings page SHALL use measured row windowing when the filtered inventory contains more than 500 hooks and SHALL preserve ordinary document-flow rendering at or below 500 hooks.

#### Scenario: Render a small or medium inventory
- **WHEN** the filtered Prompt Hook inventory contains 500 or fewer hooks
- **THEN** the page SHALL render the category-grouped compact rows without virtualization

#### Scenario: Render a large inventory
- **WHEN** the filtered Prompt Hook inventory contains more than 500 hooks
- **THEN** the page SHALL mount only viewport-visible grouped rows plus no more than four overscan rows before and after the visible range
- **AND** it SHALL preserve category grouping, execution order, and stable hook ids

#### Scenario: Use responsive columns
- **WHEN** a windowed Prompt Hook inventory crosses responsive layout boundaries
- **THEN** the page SHALL remeasure virtual rows
- **AND** no Hook or category heading SHALL be omitted, duplicated, clipped, or reordered

#### Scenario: Change filters or grouping
- **WHEN** the user changes a Prompt Hook filter, search term, sort, or group expansion state
- **THEN** the virtualized collection SHALL update from the resulting ordered rows
- **AND** the collection SHALL return to its start without retaining stale virtual indices

#### Scenario: Operate an offscreen hook
- **WHEN** the user scrolls a large inventory until a previously unmounted Hook becomes visible
- **THEN** its compact row SHALL expose the same enablement, detail, preview, and overflow operations as a non-virtual row

#### Scenario: Navigate a windowed inventory accessibly
- **WHEN** keyboard or assistive-technology users traverse a large Prompt Hook inventory
- **THEN** rendered rows SHALL expose their position and total collection size
- **AND** scrolling SHALL make subsequent Hooks available without trapping focus

### Requirement: Prompt Hook lifecycle controls
The Prompt Hooks settings page SHALL provide localized draft, publish, version-history, and rollback controls inside the unified detail workflow for user-created Hooks.

#### Scenario: Show draft state
- **WHEN** a user Hook has unpublished changes
- **THEN** its compact row and detail workflow SHALL distinguish the draft revision from the active published version
- **AND** the inventory SHALL continue to reflect whether a live published version exists

#### Scenario: Save and publish from one workflow
- **WHEN** a user edits Hook metadata or template content
- **THEN** the detail workflow SHALL make the distinction between saving a draft and publishing it explicit
- **AND** it SHALL show the current live version and unpublished draft state near the publication actions

#### Scenario: Publish a draft
- **WHEN** a user confirms publication of a valid draft
- **THEN** the page SHALL publish through the Agent service, refresh the Hook and version history, and identify the newly active version

#### Scenario: Roll back from version history
- **WHEN** a user confirms rollback to a historical version
- **THEN** the page SHALL request rollback through the Agent service and identify the new published version
- **AND** it SHALL show that any unrelated draft remains unpublished

#### Scenario: Protect built-in Hooks
- **WHEN** a backend-owned built-in Hook is displayed
- **THEN** draft, publish, and rollback mutation controls SHALL not be offered
