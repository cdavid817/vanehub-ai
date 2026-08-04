## MODIFIED Requirements

### Requirement: Secure connector configuration
The desktop runtime SHALL store connector secrets in the operating-system credential store, SHALL store non-secret connector configuration and credential-reference metadata in SQLite, and SHALL apply credential edits as validated field-level patches.

#### Scenario: Save connector credentials
- **WHEN** a user saves valid connector configuration and secret fields
- **THEN** secret fields SHALL be written to the operating-system credential store and SHALL NOT be persisted as plaintext in SQLite or frontend storage
- **AND** non-secret fields SHALL be persisted separately from the secret payload

#### Scenario: Patch a configured connector
- **WHEN** a configured connector update supplies only a subset of editable fields
- **THEN** the native runtime SHALL preserve omitted stored fields, merge the supplied patch, and validate the complete candidate before changing persistence or runtime state

#### Scenario: Reject an incomplete credential candidate
- **WHEN** the merged connector candidate lacks a required field or is otherwise invalid
- **THEN** the system SHALL reject the update without replacing the stored credential, persisted configuration, or running connector

#### Scenario: Return configured secret field
- **WHEN** the frontend reloads a connector that has a stored secret
- **THEN** the service SHALL return only secret-presence metadata or a redacted placeholder and SHALL NOT return the stored secret

#### Scenario: Credential store unavailable
- **WHEN** the operating-system credential store is unavailable
- **THEN** the system SHALL reject secret persistence with a concise remediation error and SHALL NOT fall back to plaintext storage

#### Scenario: Clear connector credentials
- **WHEN** a user clears a connector configuration
- **THEN** the runtime SHALL stop that connector and remove its connector-owned credential entries and persisted credential references

### Requirement: Dedicated session binding
The system SHALL bind each `(connector id, external direct-chat id)` pair to one dedicated VaneHub session, and an existing binding SHALL execute from its persisted session configuration independently of current global routing defaults.

#### Scenario: First message creates binding
- **WHEN** a valid direct message has no existing binding
- **THEN** the router SHALL create a non-activating CLI session using the current IM routing defaults and persist the binding

#### Scenario: Later message reuses binding
- **WHEN** a valid direct message has an existing live binding
- **THEN** the router SHALL reuse that session, its persisted Agent and project configuration, and its provider runtime-session continuity

#### Scenario: Bound session was deleted
- **WHEN** an inbound message resolves to a binding whose session no longer exists
- **THEN** the router SHALL remove the stale binding and create a replacement session from the current defaults

#### Scenario: Routing defaults change
- **WHEN** the user changes the default Agent or project
- **THEN** existing bindings SHALL retain and execute through their existing sessions without comparing them to the new defaults
- **AND** new bindings SHALL use the new defaults

### Requirement: Per-chat serialized execution
The system SHALL run at most one Agent generation at a time for each external chat, SHALL preserve per-chat arrival order, and SHALL bound active and pending IM work across all chats.

#### Scenario: Queue message behind active generation
- **WHEN** a second message arrives for a chat with an active generation and per-chat and global capacity remain
- **THEN** the router SHALL enqueue it in arrival order and start it only after the active generation reaches a terminal state and global execution capacity is available

#### Scenario: Queue reaches capacity
- **WHEN** another message arrives after the per-chat queue reaches its configured bound
- **THEN** the connector SHALL return a localized busy response and SHALL NOT silently start or drop an untracked Agent generation

#### Scenario: Global IM capacity reaches its bound
- **WHEN** a new inbound message cannot reserve bounded global pending capacity
- **THEN** the connector SHALL return a localized busy response and SHALL NOT create a lane, completion waiter, or Agent generation for that message

#### Scenario: Different chats receive messages
- **WHEN** messages arrive for different bindings and global execution capacity remains
- **THEN** the runtime SHALL process them concurrently up to the configured native IM limit

#### Scenario: Chat lane becomes idle
- **WHEN** a chat lane has no active generation, queued message, worker, or reservation
- **THEN** the runtime SHALL remove that exact idle lane without removing a concurrently reused lane

## ADDED Requirements

### Requirement: Transactional connector lifecycle mutation
Connector configuration and lifecycle mutations SHALL be serialized per connector and SHALL either establish the validated requested state or restore the prior usable state.

#### Scenario: Save an enabled connector successfully
- **WHEN** a validated connector update is persisted and its replacement runtime starts successfully
- **THEN** only that connector SHALL use the new configuration and report the new lifecycle generation

#### Scenario: Connector update fails
- **WHEN** credential persistence, SQLite persistence, runtime replacement, or startup fails during a connector update
- **THEN** the native runtime SHALL restore the previous credential and configuration where possible, restart the previously enabled runtime, and record redacted primary and rollback outcomes

#### Scenario: Test an enabled connector
- **WHEN** a user tests an enabled connector configuration
- **THEN** the runtime SHALL use an isolated bounded test adapter without stopping, replacing, or registering over the enabled inbound runtime

#### Scenario: Concurrent operations target one connector
- **WHEN** two lifecycle mutations target the same connector concurrently
- **THEN** they SHALL execute in a deterministic serialized order while operations for other connectors remain responsive

### Requirement: Safe malformed-event handling
Each connector SHALL distinguish intentionally unsupported inbound events from payloads that fail protocol normalization and SHALL handle both with bounded, redacted behavior.

#### Scenario: Protocol payload cannot be normalized
- **WHEN** a platform frame cannot be normalized because required structure is missing or invalid
- **THEN** the connector SHALL emit a redacted safe-code diagnostic and apply its bounded acknowledgement or checkpoint policy without logging the raw frame, external identifiers, or message content

#### Scenario: Platform schema drift repeats
- **WHEN** equivalent malformed events repeat
- **THEN** the connector SHALL avoid both silent loss and an unbounded retry or diagnostic loop

### Requirement: Efficient connector maintenance
Recurring connector maintenance SHALL be incremental, bounded, and decoupled from the per-message hot path.

#### Scenario: Deduplication retention runs
- **WHEN** deduplication retention becomes due
- **THEN** the runtime SHALL remove expired records through startup or scheduled bounded maintenance rather than executing retention cleanup for every accepted message

#### Scenario: Reuse a valid access token
- **WHEN** Feishu or DingTalk sends multiple requests while its cached access token remains valid
- **THEN** the connector SHALL reuse the token and SHALL single-flight refresh it before expiry or after an authentication rejection

#### Scenario: Persist a WeChat reply context
- **WHEN** personal WeChat receives a reply context for a chat
- **THEN** the native runtime SHALL update that chat's secure context without reading and rewriting an unbounded all-chat credential payload

#### Scenario: Successful poll returns immediately
- **WHEN** a polling connector repeatedly receives successful empty responses faster than normal long-poll behavior
- **THEN** it SHALL apply bounded connector-appropriate pacing and remain responsive to shutdown

