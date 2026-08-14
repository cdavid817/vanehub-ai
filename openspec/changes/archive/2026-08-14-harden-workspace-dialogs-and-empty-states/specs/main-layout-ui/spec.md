## MODIFIED Requirements

### Requirement: Create-session dialog
The main layout UI SHALL provide a create-session dialog with Agent mode selection, Agent choice for Single Agent sessions, project folder, project history, and optional Git worktree controls. The dialog SHALL obtain its modal behavior from the shared application dialog primitive.

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
- **AND** the message SHALL remain fully readable rather than being truncated to a single line
- **AND** it SHALL be announced to assistive technology

#### Scenario: Dismiss without creating
- **WHEN** the create-session dialog is open and no creation request is in flight
- **THEN** pressing Escape SHALL close it without creating a session
- **AND** focus SHALL return to the control that opened it

#### Scenario: Dismissal blocked while creating
- **WHEN** a creation request is in flight
- **THEN** Escape and backdrop dismissal SHALL NOT close the dialog

## ADDED Requirements

### Requirement: Workspace auxiliary dialog behavior
The scheduled-tasks dialog and the session batch-delete confirmation SHALL obtain dismissal, focus containment, and focus return from the shared application dialog primitive.

#### Scenario: Dismiss the scheduled-tasks dialog
- **WHEN** the scheduled-tasks dialog is open
- **THEN** pressing Escape SHALL close it and focus SHALL return to the activity bar control that opened it

#### Scenario: Confirm destructive batch deletion
- **WHEN** the batch-delete confirmation is open
- **THEN** focus SHALL be placed inside the confirmation and SHALL remain contained within it
- **AND** Escape SHALL cancel the deletion while a delete request is not in flight

### Requirement: In-application session category creation
Creating a session category from the session context menu SHALL use an in-application dialog.

#### Scenario: Create and assign in one step
- **WHEN** the user chooses to create a category for a session and submits a non-empty name
- **THEN** the system SHALL create the category, assign that session to it, and close the dialog

#### Scenario: Empty or rejected name
- **WHEN** the submitted category name is empty or the creation request fails
- **THEN** the dialog SHALL stay open and SHALL present the reason next to the field
