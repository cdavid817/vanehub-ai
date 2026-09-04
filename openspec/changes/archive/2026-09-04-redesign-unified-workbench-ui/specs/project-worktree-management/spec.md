# project-worktree-management Specification Delta

## ADDED Requirements

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
