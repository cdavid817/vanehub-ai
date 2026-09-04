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

### Requirement: Loop run worktree isolation
The desktop runtime SHALL create a dedicated Git worktree and branch for every started Loop run before creating role sessions or modifying project files.

#### Scenario: Prepare Loop worktree
- **WHEN** a valid Loop run starts for a local Git project and base branch
- **THEN** the workspaces context SHALL create a collision-safe Loop branch and sibling worktree through the guarded project operation boundary
- **AND** the run SHALL persist the canonical project path, worktree path, worktree name, and branch

#### Scenario: Reject existing Loop target
- **WHEN** a proposed Loop worktree path or branch conflicts with an existing target
- **THEN** preparation SHALL fail before role-session creation or file mutation
- **AND** concise failure context and detailed redacted diagnostics SHALL remain available

#### Scenario: Use Loop worktree as role root
- **WHEN** Loop worktree preparation succeeds
- **THEN** all Worker and Verifier sessions and verification commands for that run SHALL use the canonical worktree as their bounded root

### Requirement: Loop worktree review retention
The first-phase system SHALL preserve a Loop worktree after success, failure, cancellation, rejection, or restart recovery until a user manages it outside this capability.

#### Scenario: Run reaches terminal state
- **WHEN** a Loop run becomes succeeded, failed, or cancelled
- **THEN** the runtime SHALL retain the worktree and expose its path for review
- **AND** it SHALL NOT automatically execute `git worktree remove`, delete the branch, merge, or commit

### Requirement: First-class Projects and Workspaces destination
The workbench SHALL provide a first-class Projects and Workspaces destination that aggregates known local projects, local worktrees, and remote SSH workspaces through existing frontend service boundaries.

#### Scenario: Open the destination
- **WHEN** the user activates Projects and Workspaces
- **THEN** the page SHALL show bounded recent, favorite, all, and needs-attention workspace views
- **AND** it SHALL not require creating a Session before a known workspace can be inspected

#### Scenario: Select a workspace
- **WHEN** the user selects a local project, worktree, or remote workspace
- **THEN** the detail surface SHALL show safe identity, availability, Git context when known, trust, recent Sessions, active Runs, and permitted actions

#### Scenario: No workspaces exist
- **WHEN** no local or remote workspace history is available
- **THEN** the page SHALL explain how workspaces are discovered and provide a supported browse or connection action

### Requirement: Persistent workspace trust presentation
Remote and otherwise trust-gated workspaces SHALL expose their current trust classification and its consequences in list, detail, and session-creation entry points.

#### Scenario: Render an untrusted remote workspace
- **WHEN** a remote workspace is untrusted or trust is unknown
- **THEN** the page SHALL show a non-color-only trust warning and restrict actions according to the existing permission contract

#### Scenario: Trust is revoked
- **WHEN** a previously trusted workspace becomes revoked or its host identity changes
- **THEN** the detail SHALL show the canonical new state and SHALL not continue to present privileged actions as available

#### Scenario: Open create Session
- **WHEN** the user starts Session creation from a trust-gated workspace
- **THEN** the wizard SHALL carry the stable workspace identity and show the trust consequence before final confirmation

### Requirement: Workspace contextual quick actions
A workspace detail SHALL expose state-aware actions to continue or create a Session, open an ordinary Shell, create or inspect a worktree, reconnect remote access, and repair or remove unavailable history when supported.

#### Scenario: Continue recent work
- **WHEN** a usable recent Session exists for the workspace
- **THEN** the primary action SHALL open that Session without creating a duplicate

#### Scenario: Create a Session
- **WHEN** the user starts a new Session from a workspace
- **THEN** the create-session wizard SHALL be prefilled with the validated workspace and allow review before creation

#### Scenario: Action is unsupported
- **WHEN** a non-Git, disconnected, untrusted, or otherwise ineligible workspace cannot perform an action
- **THEN** the action SHALL be absent or disabled with an accessible explanation
- **AND** no mutation request SHALL be sent

### Requirement: Unavailable workspace recovery
The Projects and Workspaces destination SHALL distinguish missing local paths, disconnected remote workspaces, stale history, permission restrictions, and empty state and provide bounded remediation.

#### Scenario: Local path is missing
- **WHEN** a known local project path no longer resolves
- **THEN** the row SHALL remain visible as unavailable and offer supported relocate, remove-history, or retry actions
- **AND** it SHALL not silently disappear

#### Scenario: Remote connection is offline
- **WHEN** an SSH workspace cannot be reached
- **THEN** the page SHALL retain safe host and workspace identity, show disconnected state, and offer reconnect or settings navigation

#### Scenario: Recovery fails
- **WHEN** a retry, relocation, or reconnect action fails
- **THEN** the error SHALL remain local to the workspace detail and preserve the loaded list

### Requirement: Workspace relationship summary
Workspace detail SHALL provide bounded links to authoritative Sessions, Runs, Work Items, Goals, Loops, and Evaluations associated through existing stable identities.

#### Scenario: Inspect related work
- **WHEN** relationship data is available
- **THEN** the detail SHALL show counts and recent safe summaries with EvidenceLink navigation
- **AND** it SHALL not copy full transcripts, logs, diffs, or evaluation artifacts

#### Scenario: Relationship is unavailable
- **WHEN** an owning service cannot provide a relation or the user lacks access
- **THEN** the corresponding section SHALL show unavailable or restricted status without fabricated zero counts

### Requirement: Responsive workspace management
Projects and Workspaces SHALL remain fully operable as a list-detail layout on wide screens and a list-then-detail flow on compact screens.

#### Scenario: Open compact detail
- **WHEN** the user selects a workspace at compact width
- **THEN** the detail SHALL replace or overlay the list with an obvious return action
- **AND** selection and scroll position SHALL be preserved on return

#### Scenario: Operate with keyboard
- **WHEN** the user navigates workspace rows and actions by keyboard
- **THEN** focus, selection, status, and action menus SHALL remain visible and correctly associated

