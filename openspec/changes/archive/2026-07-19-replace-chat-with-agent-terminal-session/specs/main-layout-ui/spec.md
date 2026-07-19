## MODIFIED Requirements

### Requirement: Flexible main content area
The main content panel SHALL render an Agent Terminal-first workspace area for active single-Agent CLI sessions while keeping the panel responsive within the workspace shell.

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

### Requirement: Create-session dialog
The main layout UI SHALL provide a create-session dialog with Agent mode selection, Agent choice for Single Agent sessions, project folder, project history, and optional Git worktree controls.

#### Scenario: Select session mode
- **WHEN** the create-session dialog opens
- **THEN** it SHALL present Single Agent and Multi Agent mode choices
- **AND** Single Agent SHALL be the enabled first-version mode

#### Scenario: Multi Agent is disabled
- **WHEN** the user views the Multi Agent mode choice
- **THEN** it SHALL be marked as coming soon or disabled
- **AND** the user SHALL NOT be able to submit a Multi Agent session

#### Scenario: Select Agent
- **WHEN** Single Agent mode is active
- **THEN** the dialog SHALL let the user choose among Claude Code, Gemini CLI, Codex, and OpenCode using stable agent ids

#### Scenario: Show project history
- **WHEN** the create-session dialog opens
- **THEN** it SHALL show recently selected project folders from the frontend agent service

#### Scenario: Browse project folder
- **WHEN** the user chooses to browse for a project folder
- **THEN** the dialog SHALL request folder selection through the frontend agent service

#### Scenario: Show worktree controls for Git project
- **WHEN** the selected project folder is a Git repository
- **THEN** the dialog SHALL show an optional worktree checkbox and a worktree name field when the checkbox is enabled

#### Scenario: Disable worktree controls for non-Git project
- **WHEN** the selected project folder is not a Git repository
- **THEN** the dialog SHALL allow normal Single Agent session creation and SHALL hide or disable worktree controls

#### Scenario: Submit concise failures
- **WHEN** project inspection, folder selection, or session creation fails
- **THEN** the dialog SHALL show a concise error message without rendering raw stdout or stderr

### Requirement: Agent Terminal and Shell tab separation
The workspace shell SHALL keep the Agent Terminal experience separate from the ordinary project Shell tab.

#### Scenario: Keep ordinary Shell tab
- **WHEN** an active session is selected
- **THEN** the workspace SHALL keep the existing ordinary Shell tab available for project shell commands
- **AND** that Shell tab SHALL NOT inject Agent CLI parameters or automatically launch the selected Agent CLI

#### Scenario: Agent Terminal owns Agent CLI interaction
- **WHEN** the user interacts with the selected Agent CLI
- **THEN** that interaction SHALL occur through the Agent Terminal surface
- **AND** it SHALL use the selected session's stable agent id
