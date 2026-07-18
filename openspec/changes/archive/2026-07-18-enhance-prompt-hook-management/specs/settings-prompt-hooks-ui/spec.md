## ADDED Requirements

### Requirement: Service-backed Prompt Hooks settings page
The system SHALL provide a service-backed Prompt Hooks settings page in the settings center.

#### Scenario: Open Prompt Hooks page
- **WHEN** a user opens the Prompt Hooks settings page
- **THEN** the page SHALL load Prompt Hooks, supported CLI agents, summary statistics, and recent trace summaries through the frontend Agent service boundary
- **AND** React components SHALL NOT call Tauri `invoke()` directly

#### Scenario: Preserve loaded data during refresh
- **WHEN** Prompt Hook settings data refreshes while previous data is available
- **THEN** the page SHALL keep the previous data visible with refreshing state instead of replacing the page with a blank panel

### Requirement: Prompt Hook filtering and grouping
The Prompt Hooks settings page SHALL support filtering and grouping by operational metadata.

#### Scenario: Filter hooks
- **WHEN** the user searches or selects a category, source, enabled state, or CLI binding filter
- **THEN** the page SHALL show only matching Prompt Hooks
- **AND** it SHALL preserve stable grouping and order

#### Scenario: Display category groups
- **WHEN** Prompt Hooks are listed
- **THEN** the page SHALL make the categories `bootstrap`, `callback`, `dynamic`, `law`, `navigation`, `routing`, and `static` scannable through localized labels

### Requirement: Prompt Hook card controls
Each Prompt Hook item SHALL expose controls appropriate to its source and governance flags.

#### Scenario: Toggle hook enabled state
- **WHEN** a user toggles an enabled control for a disableable Prompt Hook
- **THEN** the page SHALL submit the change through the Agent service and refresh affected hook data

#### Scenario: Disable immutable toggle
- **WHEN** a Prompt Hook has `disableable=false`
- **THEN** the page SHALL show the enabled state as locked and SHALL NOT submit a disable request from the control

#### Scenario: Update CLI bindings
- **WHEN** a user changes the CLI binding checkboxes on a Prompt Hook
- **THEN** the page SHALL submit the stable agent id binding set through the Agent service

### Requirement: User Prompt Hook dialogs
The Prompt Hooks settings page SHALL provide dialogs for custom Prompt Hook creation, editing, deletion confirmation, and preview.

#### Scenario: Create custom hook
- **WHEN** a user submits a valid custom Prompt Hook form
- **THEN** the page SHALL call the Agent service to create the hook
- **AND** user-visible validation labels and errors SHALL be localized

#### Scenario: Edit custom hook
- **WHEN** a user edits a user-created Prompt Hook
- **THEN** the page SHALL allow editable metadata, template body, enabled state, and CLI bindings
- **AND** it SHALL prevent changing immutable identity fields in a way the service rejects

#### Scenario: Preview hook content
- **WHEN** a user explicitly opens a Prompt Hook preview
- **THEN** the page SHALL request rendered content through the service boundary and show it in a bounded preview dialog

### Requirement: Prompt Hook trace display
The Prompt Hooks settings page SHALL show safe trace summaries by default and full content only after explicit preview.

#### Scenario: Show trace summaries
- **WHEN** recent Prompt Hook traces are available
- **THEN** the page SHALL display hook id, status, content hash, token estimate, timestamp, and skip reason when present
- **AND** it SHALL NOT show full rendered content in the default trace list

#### Scenario: Explicit trace content preview
- **WHEN** the user explicitly requests content preview from a trace or hook
- **THEN** the page SHALL show the rendered content returned by the service in a bounded dialog

### Requirement: Prompt Hooks visual and localization consistency
The Prompt Hooks settings page SHALL follow the shared settings visual system and i18n contract.

#### Scenario: Render in both visual styles
- **WHEN** the active visual style is `futuristic` or `minimal`
- **THEN** the Prompt Hooks page SHALL use shared settings primitives, semantic tokens, compact panels, stable controls, and icons consistent with the rest of the settings center

#### Scenario: Localize Prompt Hooks page
- **WHEN** the Prompt Hooks page renders in Simplified Chinese or English
- **THEN** page title, description, filters, categories, source labels, stage labels, statuses, actions, dialogs, validation messages, empty states, and trace labels SHALL use synchronized locale resources
