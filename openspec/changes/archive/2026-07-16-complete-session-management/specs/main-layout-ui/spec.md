## MODIFIED Requirements

### Requirement: Sidebar session organization
The sidebar SHALL support service-backed session navigation without the six bottom tool shortcuts and SHALL provide activity, folder, pinned, archived session organization, and dialog-based session creation.

#### Scenario: Preserve sidebar utility row
- **WHEN** the workspace sidebar is rendered
- **THEN** the sidebar SHALL keep bottom Settings and Help controls while omitting the six previous tool shortcuts and any visual style switching control

#### Scenario: Show agent marker on session cards
- **WHEN** a session card is rendered
- **THEN** the card SHALL show an agent-type marker to the left of the title using a distinct icon and color for known agent types including Codex, Claude Code, OpenCode, and Gemini

#### Scenario: Open create-session dialog from new action
- **WHEN** the user activates the sidebar new-session action
- **THEN** the sidebar SHALL open a create-session dialog rather than immediately creating a session

#### Scenario: Create session from dialog
- **WHEN** the user submits a valid create-session dialog
- **THEN** the UI SHALL create a session through the frontend agent service and make the created session available for selection

#### Scenario: Select session card
- **WHEN** the user selects a session card
- **THEN** the sidebar SHALL switch the active session through the frontend agent service and visually mark that card as selected

#### Scenario: Switch to activity view
- **WHEN** the user selects the activity view mode
- **THEN** the sidebar SHALL group sessions into needs-input, pending-verification, in-progress, and inactive categories with a visible count for each category

#### Scenario: Sort activity groups by priority
- **WHEN** the sidebar renders the activity view mode
- **THEN** the categories SHALL appear in priority order: needs-input, pending-verification, in-progress, inactive

#### Scenario: Show pinned sessions
- **WHEN** one or more sessions are pinned
- **THEN** the sidebar SHALL render pinned sessions in a dedicated pinned area before the normal activity or folder groups

#### Scenario: Switch to folder group view
- **WHEN** the user selects the group view mode
- **THEN** the sidebar SHALL group sessions by their owning folder

#### Scenario: Toggle folder expansion
- **WHEN** the user toggles a folder group in group view
- **THEN** the sidebar SHALL expand or collapse that folder's session cards without changing the selected session

#### Scenario: Open archived view
- **WHEN** the user opens the archived session view
- **THEN** the sidebar SHALL show archived sessions from the frontend agent service and indicate the archived session count

#### Scenario: Use context actions
- **WHEN** the user opens a session card context menu
- **THEN** the sidebar SHALL provide actions to rename, pin or unpin, archive or restore, and delete the session according to the session's current state

#### Scenario: Prevent browser context menu
- **WHEN** the user opens the custom session context menu in browser or desktop WebView mode
- **THEN** the sidebar SHALL prevent the browser default context menu from appearing over the custom menu

#### Scenario: Confirm destructive session deletion
- **WHEN** the user chooses to delete a session
- **THEN** the sidebar SHALL ask for confirmation before calling the delete operation

#### Scenario: Scroll long session lists internally
- **WHEN** the session list content exceeds the sidebar height
- **THEN** the session list SHALL scroll inside the sidebar without scrolling the whole workspace shell

## ADDED Requirements

### Requirement: Create-session dialog
The main layout UI SHALL provide a create-session dialog with Agent, project folder, project history, and optional Git worktree controls.

#### Scenario: Select Agent
- **WHEN** the create-session dialog opens
- **THEN** it SHALL let the user choose among Claude Code, Gemini CLI, Codex, and OpenCode using stable agent ids

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
- **THEN** the dialog SHALL allow normal session creation and SHALL hide or disable worktree controls

#### Scenario: Submit concise failures
- **WHEN** project inspection, folder selection, or session creation fails
- **THEN** the dialog SHALL show a concise error message without rendering raw stdout or stderr
