## Purpose

Defines explicit session opt-in and safe Feishu direct-message delivery for single-Agent and multi-Agent VaneHub sessions.

## ADDED Requirements

### Requirement: Explicit per-session Feishu opt-in
Each session SHALL keep Feishu IM access disabled until a user explicitly enables IM for that session, and the enabled state SHALL persist independently for each session.

#### Scenario: New session defaults to disabled
- **WHEN** a user creates or first opens a session that has no stored IM enablement state
- **THEN** Feishu IM access for that session SHALL be disabled
- **AND** no Feishu chat SHALL be allowed to pair with or deliver ordinary messages to that session

#### Scenario: User enables IM access
- **WHEN** the user enables IM access for an eligible session
- **THEN** the session SHALL become eligible for Feishu pairing without enabling IM for any other session

#### Scenario: User disables IM access
- **WHEN** the user disables IM access for a session with an active Feishu binding
- **THEN** the binding SHALL remain safely persisted but paused
- **AND** inbound messages SHALL NOT start Agent work until the user re-enables IM access

### Requirement: Single-Agent Feishu delivery
An enabled single-Agent session with an active Feishu binding SHALL route accepted Feishu text through the same persisted Agent and project configuration used by desktop-originated turns.

#### Scenario: Deliver direct text to a single Agent
- **WHEN** a unique Feishu direct-message event reaches an enabled bound single-Agent session
- **THEN** the system SHALL append one user turn to that session and invoke its Agent once
- **AND** the completed final response SHALL be delivered to the originating Feishu chat

#### Scenario: Reject delivery while disabled
- **WHEN** a Feishu direct-message event targets a bound session whose IM access is disabled
- **THEN** the event SHALL NOT append a chat message or invoke an Agent
- **AND** the connector SHALL return a concise localized disabled-state response

### Requirement: Multi-Agent Feishu delivery
An enabled multi-Agent session with an active Feishu binding SHALL apply the session's existing human mention, current-turn, and Agent handoff rules to accepted Feishu text.

#### Scenario: Route an explicit seat mention
- **WHEN** a Feishu message begins with a valid mention of an active seat
- **THEN** the message SHALL be routed to that seat using its stable participant identity
- **AND** any subsequent Agent handoff SHALL follow the session's bounded serial handoff rules

#### Scenario: Route text without a seat mention
- **WHEN** an accepted Feishu message contains no valid line-leading seat mention
- **THEN** it SHALL route to the seat that most recently held the turn
- **AND** when no seat has held the turn it SHALL route to the first active seat

#### Scenario: Reject an unavailable mentioned seat
- **WHEN** a Feishu message names a missing, removed, or unavailable seat
- **THEN** the system SHALL NOT silently choose another seat
- **AND** it SHALL return a concise safe response identifying the valid active seat labels

### Requirement: Feishu transport safety and compatibility
Feishu delivery SHALL preserve durable deduplication, bounded platform acknowledgement, safe output chunking, redacted diagnostics, and connector lifecycle behavior required by the shared IM service.

#### Scenario: Receive a repeated Feishu event
- **WHEN** Feishu retries an event whose stable event id was already accepted
- **THEN** the system SHALL acknowledge it without appending another message, invoking another Agent, or sending another final response

#### Scenario: Response exceeds the Feishu message limit
- **WHEN** a completed Agent response exceeds the active Feishu text-message limit
- **THEN** the connector SHALL split it into ordered messages without splitting a valid encoded character

#### Scenario: Feishu transport is unavailable
- **WHEN** an accepted IM turn completes while Feishu delivery is temporarily unavailable
- **THEN** the VaneHub assistant message SHALL retain its terminal state
- **AND** delivery failure diagnostics SHALL use safe codes without recording message content, external identities, or credentials

