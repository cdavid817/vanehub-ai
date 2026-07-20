## ADDED Requirements

### Requirement: Session batch management mode
The workspace session sidebar SHALL provide an explicit batch-management mode for selecting multiple visible sessions and running a confirmed delete operation.

#### Scenario: Enter batch management
- **WHEN** the user activates the batch-management action from the session sidebar
- **THEN** the sidebar SHALL show selectable controls on visible session rows
- **AND** it SHALL show localized selected-count, select-visible, delete-selected, and exit-batch controls

#### Scenario: Toggle session selection
- **WHEN** batch-management mode is active and the user selects a session row checkbox or row selection affordance
- **THEN** the sidebar SHALL toggle that session id in the batch selection
- **AND** it SHALL NOT switch the active session as part of that toggle

#### Scenario: Select visible sessions
- **WHEN** batch-management mode is active and the user activates select-visible
- **THEN** the sidebar SHALL select every currently visible session in the active search, Agent filter, archive, and presentation state
- **AND** it SHALL update the selected-count control

#### Scenario: Confirm batch deletion
- **WHEN** batch-management mode is active and one or more sessions are selected
- **AND** the user activates delete-selected
- **THEN** the sidebar SHALL show a localized destructive confirmation that includes the number of selected sessions
- **AND** it SHALL call session deletion only after the user confirms

#### Scenario: Exit batch management
- **WHEN** the user exits batch-management mode
- **THEN** the sidebar SHALL hide selectable controls
- **AND** it SHALL clear the current batch selection
- **AND** normal session selection, context menu, and category drag behavior SHALL be restored

### Requirement: Session list presentation switch
The workspace session sidebar SHALL let users switch between a flat list presentation and a categorized presentation.

#### Scenario: Use list presentation
- **WHEN** the list presentation is selected
- **THEN** the sidebar SHALL render matching sessions as a flat scannable list while preserving pinned and archived indicators

#### Scenario: Use categorized presentation
- **WHEN** the categorized presentation is selected
- **THEN** the sidebar SHALL render matching sessions grouped by their user-defined category
- **AND** it SHALL include a localized uncategorized group for sessions without a category

#### Scenario: Preserve presentation while filtering
- **WHEN** search text or Agent filter changes
- **THEN** the sidebar SHALL keep the selected presentation mode
- **AND** it SHALL apply the new filter within that presentation

### Requirement: Session Agent filter
The workspace session sidebar SHALL provide an Agent filter for All, Claude Code, OpenCode, Codex CLI, and Gemini CLI sessions.

#### Scenario: Filter all sessions
- **WHEN** the user selects the All Agent filter
- **THEN** the sidebar SHALL include sessions for every Agent in the active session source

#### Scenario: Filter by managed CLI Agent
- **WHEN** the user selects Claude Code, OpenCode, Codex CLI, or Gemini CLI
- **THEN** the sidebar SHALL show only sessions whose stable `agentId` matches the selected managed CLI Agent id
- **AND** it SHALL NOT match by display name

#### Scenario: Filter archived sessions
- **WHEN** the archived session source is visible and an Agent filter is active
- **THEN** the sidebar SHALL filter archived sessions by the same stable Agent id semantics

### Requirement: Session management visual and localization parity
The optimized session management controls SHALL remain consistent with the workspace visual design system and synchronized zh-CN/en localization.

#### Scenario: Render localized session management controls
- **WHEN** the sidebar renders batch-management, presentation, Agent filter, and confirmation controls
- **THEN** every user-visible label, tooltip, accessible name, empty state, and destructive confirmation SHALL use the active locale

#### Scenario: Preserve visual styles
- **WHEN** the workspace renders in `futuristic` or `minimal` style
- **THEN** the optimized session management controls SHALL use existing semantic tokens, compact dimensions, stable spacing, and lucide or project icons without overlapping adjacent text
