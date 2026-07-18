# project-worktree-management Specification

## Purpose
Defines known project history, project folder selection, Git repository detection, optional Git worktree creation, and worktree diagnostics for session startup.
## Requirements
### Requirement: Known project history
The system SHALL maintain a history of project folders selected during session creation.

#### Scenario: Record selected project
- **WHEN** a session is created with a selected project folder
- **THEN** the system SHALL persist the canonical project path with a display name, last opened timestamp, and last known Git status

#### Scenario: List known projects
- **WHEN** the create-session dialog requests known projects
- **THEN** the system SHALL return previously selected project folders ordered by most recently opened first

#### Scenario: Preserve Web runtime behavior
- **WHEN** the app runs in Web mode
- **THEN** the Web adapter SHALL expose the same known-project history contract without requiring SQLite

### Requirement: Project folder selection
The system SHALL allow the user to select a session project folder before creating a session.

#### Scenario: Select folder from history
- **WHEN** the user selects a known project in the create-session dialog
- **THEN** the system SHALL inspect that folder and use it as the proposed session folder

#### Scenario: Select folder from native picker
- **WHEN** the user chooses to browse for a folder in desktop mode
- **THEN** the system SHALL open a native directory picker through the service adapter and return the selected path without exposing unrestricted filesystem APIs to React components

#### Scenario: Cancel folder picker
- **WHEN** the user cancels the directory picker
- **THEN** the create-session dialog SHALL remain open without changing the currently selected project folder

### Requirement: Project Git inspection
The system SHALL inspect whether a selected project folder belongs to a Git repository.

#### Scenario: Inspect Git project
- **WHEN** the selected folder is inside a Git repository
- **THEN** the system SHALL return Git status metadata that enables worktree creation controls

#### Scenario: Inspect non-Git project
- **WHEN** the selected folder is not inside a Git repository
- **THEN** the system SHALL return non-Git project metadata and the UI SHALL hide or disable worktree creation controls

#### Scenario: Inspection does not launch agents
- **WHEN** the system inspects a selected project folder
- **THEN** the inspection SHALL NOT launch an Agent or start an interactive session

### Requirement: Optional Git worktree creation
The system SHALL create a Git worktree during session creation when the user enables worktree creation for a Git project.

#### Scenario: Enable worktree for Git project
- **WHEN** the selected project is a Git repository
- **THEN** the create-session dialog SHALL allow the user to enable worktree creation and enter a worktree name

#### Scenario: Require worktree name
- **WHEN** worktree creation is enabled and the worktree name is empty or unsafe
- **THEN** the system SHALL reject session creation before executing a Git command

#### Scenario: Create default worktree path
- **WHEN** the user creates a worktree named `feature-a` for project folder `C:\code\app`
- **THEN** the default worktree path SHALL be `C:\code\app-feature-a`

#### Scenario: Create default worktree branch
- **WHEN** the user creates a worktree named `feature-a`
- **THEN** the default worktree branch SHALL be `vanehub/feature-a`

#### Scenario: Reject existing target path
- **WHEN** the resolved worktree target path already exists
- **THEN** the system SHALL reject worktree creation before executing `git worktree add`

#### Scenario: Use worktree as session folder
- **WHEN** worktree creation succeeds during session creation
- **THEN** the created session SHALL use the worktree path as its effective folder

#### Scenario: Allow non-Git normal session
- **WHEN** the selected project is not a Git repository
- **THEN** the user SHALL still be able to create a normal session using the selected folder

### Requirement: Worktree command diagnostics
The system SHALL keep worktree command output out of React UI while preserving diagnostics in unified logs.

#### Scenario: Git worktree command fails
- **WHEN** `git worktree add` fails during session creation
- **THEN** the UI SHALL receive a concise failure message and the native runtime SHALL write detailed stdout, stderr, and diagnostics through the unified logging service

#### Scenario: Git executable unavailable
- **WHEN** Git cannot be executed in desktop mode
- **THEN** the UI SHALL receive a concise unavailable message and the native runtime SHALL write the detailed failure through the unified logging service

### Requirement: Remote workspace history
The system SHALL maintain a history of remote workspaces used during session creation.

#### Scenario: Record remote workspace
- **WHEN** a session is created with a remote workspace target
- **THEN** the system SHALL persist host, optional user, path, display name, URI, and last opened timestamp

#### Scenario: List remote workspaces
- **WHEN** the create-session dialog requests known remote workspaces
- **THEN** the system SHALL return previously used remote workspaces ordered by most recently opened first

#### Scenario: Preserve local project history
- **WHEN** local project history is requested
- **THEN** remote workspace entries SHALL NOT be mixed into the local project history list

