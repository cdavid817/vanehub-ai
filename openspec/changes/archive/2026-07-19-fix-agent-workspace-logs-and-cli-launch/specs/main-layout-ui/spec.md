## MODIFIED Requirements

### Requirement: Main content Agent workspace
The main content panel SHALL render a Workspace-first area for active single-Agent CLI sessions while keeping the panel responsive within the workspace shell.

#### Scenario: Workspace tab is user-facing
- **WHEN** the session tab navigation renders for the former Agent Terminal surface
- **THEN** the tab SHALL be named Workspace / 工作区
- **AND** the surface SHALL continue to host the selected Agent CLI terminal interaction

#### Scenario: Workspace terminal composer
- **WHEN** a Workspace terminal session is attached
- **THEN** the workspace SHALL provide a bottom multiline composer below the terminal viewport
- **AND** pressing Enter in the composer SHALL send the entered text followed by Enter to the current Agent CLI terminal
- **AND** pressing Shift+Enter SHALL insert a new line without submitting
- **AND** the composer SHALL be disabled when no terminal process is attached

#### Scenario: Agent Terminal flexes with panel height
- **WHEN** the workspace panel height changes
- **THEN** the Agent Terminal area SHALL flex to fill the available main content space without a fixed minimum height forcing overflow

#### Scenario: Agent Terminal scrolls internally
- **WHEN** terminal content exceeds the available terminal viewport
- **THEN** the terminal SHALL scroll or buffer inside the main content panel without scrolling the whole workspace shell

#### Scenario: Main content expands after panel collapse
- **WHEN** the information panel is collapsed
- **THEN** the main content panel SHALL smoothly expand to occupy the space released by the information panel

#### Scenario: Agent Terminal renders for active session
- **WHEN** an active single-Agent CLI session is selected
- **THEN** the main content panel SHALL render the Agent Terminal for that active session instead of the previous chat message list and composer

#### Scenario: Session-page chat selectors removed
- **WHEN** the Agent Terminal main content renders
- **THEN** the page SHALL NOT render model, provider, permission, reasoning, thinking, streaming, or prompt-composer controls for that terminal

### Requirement: Information panel tabs
The information panel SHALL provide keep-alive tabs for Basic Info, Files, Changes, and Logs.

#### Scenario: Information panel tab set
- **WHEN** the information panel renders for an active session
- **THEN** the panel SHALL show tabs named Basic Info, Files, Changes, and Logs
- **AND** the previous Agent Info tab content SHALL remain available under Basic Info

#### Scenario: Switch tabs without unmounting content
- **WHEN** the user switches between information panel tabs
- **THEN** all tab contents SHALL remain mounted while only the selected tab content is visible

#### Scenario: Show agent progress summary
- **WHEN** the Basic Info tab is visible
- **THEN** the tab SHALL show an independent progress bar with overall completion percentage and completed, in-progress, and pending task counts

#### Scenario: Compact terminal logs are visible
- **WHEN** the user opens the Logs tab in the information panel
- **THEN** the panel SHALL show recent session log entries for Agent terminal diagnostics
- **AND** startup and startup-failure records SHALL be visible without opening the detailed Logs workspace tab
