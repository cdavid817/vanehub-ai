## ADDED Requirements

### Requirement: Connector lifecycle independent from session routing
A configured IM connector SHALL be allowed to start, reconnect, stop, and report health without requiring a default Agent, project path, or existing session binding.

#### Scenario: Start connector without a binding
- **WHEN** a user enables a valid configured connector and no session binding exists
- **THEN** the connector SHALL start its inbound lifecycle and report health normally

#### Scenario: Receive ordinary text without a binding
- **WHEN** an enabled connector receives direct text from an external chat that has no binding and the text is not a valid pairing command
- **THEN** the system SHALL return a concise localized pairing instruction and SHALL NOT create a session, persist the text as a VaneHub chat message, or launch an Agent

### Requirement: Short-lived session pairing
The system SHALL pair an external direct chat to an existing VaneHub session through a connector-scoped, session-scoped, short-lived, single-use code whose plaintext is neither persisted nor logged.

#### Scenario: Begin pairing
- **WHEN** a user starts pairing from an eligible existing session and connected connector
- **THEN** the service SHALL return a new pairing code and expiry without exposing any external identity

#### Scenario: Complete pairing from IM
- **WHEN** the intended external direct chat sends the valid unexpired pairing command through the selected connector
- **THEN** the system SHALL atomically consume the code, bind that external chat to the selected session, acknowledge success, and SHALL NOT execute the pairing command as an Agent prompt

#### Scenario: Reject invalid pairing code
- **WHEN** an external chat submits an unknown, expired, already consumed, or connector-mismatched pairing code
- **THEN** the system SHALL reject it without creating or changing a binding and without revealing the target session

#### Scenario: Pairing expires
- **WHEN** a pairing code reaches its expiry without being consumed
- **THEN** the code SHALL no longer authorize a binding and the session SHALL remain unbound

### Requirement: Binding lifecycle controls
The system SHALL let the user inspect, pause, resume, and remove a session's IM binding without deleting the session or global connector configuration.

#### Scenario: Pause a binding
- **WHEN** the user pauses an active binding
- **THEN** new ordinary messages from that external chat SHALL not execute the Agent until the binding is resumed

#### Scenario: Remove a binding
- **WHEN** the user removes a binding
- **THEN** the external chat and session SHALL become unbound while both the VaneHub session and connector configuration remain available

#### Scenario: Connector becomes unavailable
- **WHEN** a bound connector is disabled, reconnecting, authorization-expired, or in error
- **THEN** the binding SHALL remain persisted and expose the connector condition without silently moving traffic to another connector or session

### Requirement: Opt-in completion notifications
The system SHALL support an opt-in per-binding completion notification without mirroring arbitrary desktop conversation content to IM.

#### Scenario: Notify for a desktop-started completion
- **WHEN** completion notifications are enabled for a binding and a qualifying desktop-started Agent execution reaches a terminal state
- **THEN** the connector SHALL deliver a concise localized status notification to the bound external chat without including prompt text, response text, raw diagnostics, or secrets

#### Scenario: Notifications are disabled
- **WHEN** completion notifications are disabled or the binding is paused
- **THEN** desktop-started Agent executions SHALL NOT produce outbound IM notifications for that binding

#### Scenario: Reply to an IM-originated turn
- **WHEN** an IM-originated Agent execution completes
- **THEN** the existing final-response delivery behavior SHALL reply to its originating external chat independently of the desktop completion-notification preference

## MODIFIED Requirements

### Requirement: Dedicated session binding
The system SHALL bind an external direct chat to an existing VaneHub session, SHALL allow at most one active external-chat binding per session in the first version, and SHALL allow each `(connector id, external direct-chat id)` pair to target at most one session.

#### Scenario: First message creates binding
- **WHEN** a valid pairing command is the first accepted message from an unbound external direct chat
- **THEN** the router SHALL bind that chat to the existing session identified by the pairing intent without creating or activating another session

#### Scenario: Pair an existing session
- **WHEN** a valid pairing command identifies an eligible session that has no active binding
- **THEN** the router SHALL persist a binding to that session without creating or activating another session

#### Scenario: Later message reuses binding
- **WHEN** a valid direct message has an existing active binding
- **THEN** the router SHALL reuse that session, its persisted Agent and effective project or worktree configuration, and its provider runtime-session continuity

#### Scenario: Bound session was deleted
- **WHEN** an inbound message resolves to a binding whose session no longer exists
- **THEN** the router SHALL remove the stale binding, return a safe pairing instruction, and SHALL NOT create a replacement session automatically

#### Scenario: External chat is already bound
- **WHEN** a pairing command targets an external chat that is already bound to another live session
- **THEN** the system SHALL require explicit desktop confirmation before replacing the existing binding

#### Scenario: Session already has an active binding
- **WHEN** the user attempts to pair a session that already has an active external-chat binding
- **THEN** the system SHALL require the existing binding to be removed or explicitly replaced before completing the new binding

#### Scenario: Routing defaults change
- **WHEN** legacy global routing defaults are changed or removed
- **THEN** existing bindings SHALL retain their sessions and new external chats SHALL still require explicit session pairing

#### Scenario: Preserve legacy binding
- **WHEN** an existing IM-created session and binding are loaded after the migration
- **THEN** the binding SHALL remain usable with its persisted session configuration and SHALL be manageable through the new binding lifecycle controls

## REMOVED Requirements

### Requirement: Global IM routing configuration
**Reason**: Connector connectivity is application-scoped while Agent and workspace selection are session-scoped; one mutable global route is ambiguous for concurrent projects and worktrees.

**Migration**: Existing routing settings MAY remain stored for rollback compatibility but SHALL no longer gate connector startup or create sessions for unbound external chats. Existing session bindings continue using their persisted session configuration.
