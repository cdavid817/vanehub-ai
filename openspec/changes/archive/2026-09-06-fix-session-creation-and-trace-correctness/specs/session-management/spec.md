## ADDED Requirements

### Requirement: Request-attributed workspace inspection during session creation
Workspace inspection results consumed by session creation SHALL be attributed to the request and the normalized path that produced them. A result, an error, or a derived capability from a superseded inspection SHALL NOT be applied to a different path, and abandoning or reopening the creation surface SHALL invalidate inspections still in flight. Attribution SHALL be enforced by the consumer rather than by assuming the transport cancels work.

#### Scenario: Inspections complete out of order
- **WHEN** the user inspects path A, then inspects path B, and the response for B arrives before the response for A
- **THEN** the surface SHALL retain B's inspection result and B's derived Git capability
- **AND** A's late result SHALL be discarded rather than replacing B's

#### Scenario: Late inspection error arrives for an abandoned path
- **WHEN** an inspection for a path the user has already replaced fails after the replacement
- **THEN** the surface SHALL NOT present that failure against the current path
- **AND** the current path's own state SHALL remain unchanged

#### Scenario: Creation surface is reopened
- **WHEN** the creation surface is closed and reopened while an inspection is still in flight
- **THEN** the reopened surface SHALL start with no inherited inspection, worktree, or derived capability state
- **AND** the in-flight result SHALL NOT populate the new surface

### Requirement: Session creation completion is consistent and recoverable
Persisting a created session and recording the completion of its workspace operation SHALL have a declared consistency boundary. When a session has been persisted, its operation completion SHALL be durably recorded or remain recoverable; it SHALL NOT be silently discarded. A client SHALL be able to reconcile a creation whose operation result was not delivered without creating a second session.

#### Scenario: Operation completion fails after the session is persisted
- **WHEN** the session record is persisted but recording the operation's completion fails
- **THEN** the failure SHALL be surfaced or retained as recoverable state rather than discarded
- **AND** the operation SHALL NOT remain indefinitely unfinished with no path to completion

#### Scenario: Completion is retried for an already-created session
- **WHEN** completion is retried for a creation whose session already exists
- **THEN** the retry SHALL complete the existing operation
- **AND** it SHALL NOT create a second session for the same creation request

#### Scenario: Client reconciles an undelivered creation result
- **WHEN** a client cannot obtain the result of a creation it has already submitted
- **THEN** it SHALL be able to reconcile the outcome by the operation's stable id
- **AND** reconciliation SHALL be distinguishable from resubmitting the creation

### Requirement: Canonical session read failure after a successful creation is recoverable
A creation whose operation succeeded but whose canonical session read failed SHALL be a distinct, recoverable state. The client SHALL NOT mark such a creation as handled, and SHALL retry the read rather than requiring the user to create the session again.

#### Scenario: Canonical read fails transiently after operation success
- **WHEN** the creation operation reports success and reading the canonical session then fails
- **THEN** the creation SHALL remain in a recoverable state that identifies the operation and the created session
- **AND** the client SHALL retry reading the session rather than resubmitting the creation

#### Scenario: Recovered read succeeds
- **WHEN** a retried canonical session read succeeds
- **THEN** the creation SHALL complete and deliver that session
- **AND** the operation SHALL be marked handled only once the result has been delivered

#### Scenario: User leaves the creation surface before the result resolves
- **WHEN** an asynchronous creation result resolves after the user has left or reopened the creation surface
- **THEN** the stale result SHALL NOT navigate the user or populate the current surface
- **AND** the created session SHALL remain reachable through ordinary session listing

## MODIFIED Requirements

### Requirement: Session creation input
The system SHALL create sessions from a service-level input that includes stable agent id, interaction mode, selected project path, and optional worktree request. The native and Web boundaries SHALL accept a declared `api` mode as well as existing supported modes and SHALL validate the selected Agent's identity, declared mode, and readiness before persisting the session. The condition that enables submission and the validation performed at submission SHALL derive from one normalized result for the active workspace mode, so that a field belonging to an inactive workspace mode cannot block a submission the user was invited to make.

#### Scenario: Create session for selected agent
- **WHEN** the user creates a session for Claude Code, Gemini CLI, Codex CLI, or OpenCode using a declared CLI mode
- **THEN** the created session SHALL store the selected stable agent id rather than matching by display name

#### Scenario: Create session for OnePiece
- **WHEN** the user selects a ready OnePiece and submits a local Single-Agent session
- **THEN** the frontend SHALL submit `agentId = onepiece` and `interactionMode = api`
- **AND** the created session SHALL persist those stable values

#### Scenario: Reject unsupported agent
- **WHEN** session creation receives an unknown Agent id or a mode that the selected Agent does not declare
- **THEN** the system SHALL reject the request without creating a session

#### Scenario: Reject a non-ready API Agent
- **WHEN** session creation receives an API Agent whose availability is not selectable
- **THEN** the native or Web boundary SHALL reject the request with a safe readiness reason
- **AND** it SHALL NOT contact the provider or create a session

#### Scenario: Create session uses selected folder
- **WHEN** the user creates a session without worktree creation
- **THEN** the created session SHALL use the selected project folder as the effective folder

#### Scenario: Reject remote OnePiece session
- **WHEN** session creation combines `agentId = onepiece` with a remote workspace request
- **THEN** the frontend SHALL prevent submission or the service SHALL reject it
- **AND** the system SHALL explain that first-version OnePiece sessions require a local project or local worktree

#### Scenario: Preserve Web runtime behavior
- **WHEN** the app runs in Web mode
- **THEN** the Web adapter SHALL accept the same supported session creation input and return equivalent mock session metadata
- **AND** it SHALL enforce equivalent Agent mode, readiness, and workspace restrictions

#### Scenario: Inactive workspace mode retains an incomplete draft
- **WHEN** the user leaves an incomplete draft belonging to one workspace mode, switches to another workspace mode, and completes every field that mode requires
- **THEN** submission SHALL proceed
- **AND** validation for the inactive mode's draft SHALL NOT reject the request

#### Scenario: Submission is refused while the control invites it
- **WHEN** any validation would refuse a submission
- **THEN** the control that invites submission SHALL reflect that refusal
- **AND** the system SHALL NOT present an enabled submission that silently fails against a field the active workspace mode does not display
