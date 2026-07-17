## ADDED Requirements

### Requirement: Five built-in IM connectors
The system SHALL provide independently configurable built-in connectors with stable ids `feishu`, `telegram`, `dingtalk`, `wecom`, and `weixin`.

#### Scenario: List supported connectors
- **WHEN** the IM service lists connector descriptors
- **THEN** it SHALL return all five connector ids with localized display metadata, configuration fields, capabilities, and whether the connector is experimental

#### Scenario: Keep personal WeChat experimental
- **WHEN** the `weixin` connector descriptor or status is displayed
- **THEN** the system SHALL identify it as experimental without marking the other four connectors experimental

### Requirement: First-version direct-message scope
Each connector SHALL accept text direct messages and SHALL exclude group messages and non-text content from Agent execution in the first version.

#### Scenario: Receive direct text
- **WHEN** an enabled connector receives a valid text direct message
- **THEN** it SHALL normalize the platform event and submit the text to the shared inbound router

#### Scenario: Ignore group message
- **WHEN** a connector receives a group message
- **THEN** it SHALL acknowledge or consume the platform event without creating a VaneHub message or Agent generation

#### Scenario: Reject unsupported content
- **WHEN** a connector receives an image, file, voice, card, or other unsupported content without usable text
- **THEN** it SHALL not start an Agent generation and SHALL handle the event according to the connector's bounded unsupported-content behavior

### Requirement: Global IM routing configuration
The system SHALL require a valid default Agent id and project path for new IM session bindings.

#### Scenario: Save valid routing defaults
- **WHEN** a user selects a registered available CLI Agent and a valid project directory
- **THEN** the native runtime SHALL persist those defaults for future IM-created sessions

#### Scenario: Reject invalid Agent
- **WHEN** a routing update contains an unknown Agent id or an Agent without CLI interaction support
- **THEN** the system SHALL reject the update without launching an Agent or changing the previous defaults

#### Scenario: Reject invalid project
- **WHEN** a routing update contains a missing or invalid project directory
- **THEN** the system SHALL reject the update without changing the previous defaults

### Requirement: Secure connector configuration
The desktop runtime SHALL store connector secrets in the operating-system credential store and SHALL store only non-secret configuration and credential-reference metadata in SQLite.

#### Scenario: Save connector credentials
- **WHEN** a user saves valid connector credentials
- **THEN** secret fields SHALL be written to the operating-system credential store and SHALL NOT be persisted as plaintext in SQLite or frontend storage

#### Scenario: Return configured secret field
- **WHEN** the frontend reloads a connector that has a stored secret
- **THEN** the service SHALL return only secret-presence metadata or a redacted placeholder and SHALL NOT return the stored secret

#### Scenario: Credential store unavailable
- **WHEN** the operating-system credential store is unavailable
- **THEN** the system SHALL reject secret persistence with a concise remediation error and SHALL NOT fall back to plaintext storage

#### Scenario: Clear connector credentials
- **WHEN** a user clears a connector configuration
- **THEN** the runtime SHALL stop that connector and remove its connector-owned credential entries and persisted credential references

### Requirement: Connector-specific authorization
The system SHALL support each platform's required authentication flow while presenting common configuration and status semantics.

#### Scenario: Configure credential-based connector
- **WHEN** a user configures Feishu, Telegram, DingTalk, or WeCom with its required credential fields
- **THEN** the connector SHALL validate required fields before it can be enabled

#### Scenario: Authorize personal WeChat
- **WHEN** a user starts personal WeChat authorization
- **THEN** the service SHALL return a short-lived QR authorization result without persisting or logging the QR payload

#### Scenario: Personal WeChat authorization expires
- **WHEN** personal WeChat authorization or session credentials expire
- **THEN** the connector SHALL enter `authorization-expired`, stop receiving messages, and require reauthorization

### Requirement: Explicit connector lifecycle
Each connector SHALL expose `unconfigured`, `disabled`, `connecting`, `connected`, `reconnecting`, `authorization-expired`, and `error` states as applicable.

#### Scenario: Enable configured connector
- **WHEN** a user enables a correctly configured connector
- **THEN** the native runtime SHALL start its inbound lifecycle asynchronously and update status without blocking settings navigation

#### Scenario: Disable connector
- **WHEN** a user disables a running connector
- **THEN** the native runtime SHALL stop new inbound work, close its connection or polling loop, and retain configuration for later re-enablement

#### Scenario: Restart after configuration change
- **WHEN** saved configuration changes for an enabled connector
- **THEN** the runtime SHALL restart only that connector using the new configuration

#### Scenario: Test connector configuration
- **WHEN** a user requests a connection test
- **THEN** the runtime SHALL perform a bounded authentication or connectivity check without starting a persistent inbound loop or sending a user-visible IM message

### Requirement: Connector reconnection behavior
Enabled connectors SHALL recover from transient connection failures with bounded retry behavior and SHALL stop automatic retries for authentication failures.

#### Scenario: Retry transient failure
- **WHEN** an active connection fails with a transient network or platform error
- **THEN** the connector SHALL enter `reconnecting` and retry with bounded exponential backoff and jitter

#### Scenario: Stop retrying invalid credentials
- **WHEN** a connector receives a definitive authentication failure
- **THEN** it SHALL enter `error` or `authorization-expired` and SHALL wait for a credential change or explicit retry

#### Scenario: Apply network proxy
- **WHEN** VaneHub has a supported network proxy configured before a connector starts
- **THEN** the connector's new HTTP and WebSocket traffic SHALL use the VaneHub-managed proxy and bypass policy

### Requirement: Durable inbound deduplication
The system SHALL prevent a repeated platform event from starting more than one Agent execution.

#### Scenario: Process new platform event
- **WHEN** the router receives a previously unseen connector event id
- **THEN** it SHALL durably record the event id before scheduling its Agent work

#### Scenario: Receive repeated platform event
- **WHEN** the same connector event id is received again
- **THEN** the system SHALL acknowledge or consume it without creating another user message, assistant message, session, or Agent process

### Requirement: Dedicated session binding
The system SHALL bind each `(connector id, external direct-chat id)` pair to one dedicated VaneHub session.

#### Scenario: First message creates binding
- **WHEN** a valid direct message has no existing binding
- **THEN** the router SHALL create a non-activating CLI session using the current IM routing defaults and persist the binding

#### Scenario: Later message reuses binding
- **WHEN** a valid direct message has an existing live binding
- **THEN** the router SHALL reuse that session and its provider runtime-session continuity

#### Scenario: Bound session was deleted
- **WHEN** an inbound message resolves to a binding whose session no longer exists
- **THEN** the router SHALL remove the stale binding and create a replacement session from the current defaults

#### Scenario: Routing defaults change
- **WHEN** the user changes the default Agent or project
- **THEN** existing bindings SHALL retain their existing sessions and new bindings SHALL use the new defaults

### Requirement: Per-chat serialized execution
The system SHALL run at most one Agent generation at a time for each external chat while allowing different external chats to execute concurrently.

#### Scenario: Queue message behind active generation
- **WHEN** a second message arrives for a chat with an active generation and queue capacity remains
- **THEN** the router SHALL enqueue it in arrival order and start it only after the active generation reaches a terminal state

#### Scenario: Queue reaches capacity
- **WHEN** another message arrives after the per-chat queue reaches its configured bound
- **THEN** the connector SHALL return a localized busy response and SHALL NOT silently start or drop an untracked Agent generation

#### Scenario: Different chats receive messages
- **WHEN** messages arrive for different bindings
- **THEN** the runtime MAY process them concurrently subject to native runtime capacity

### Requirement: Shared Agent execution path
IM-originated messages SHALL use the same native chat execution, message persistence, Agent parsing, token accounting, cancellation state, and unified runtime logging behavior as desktop-originated chat messages.

#### Scenario: Submit IM message
- **WHEN** the inbound router submits a normalized text message to a bound session
- **THEN** the shared chat service SHALL persist the user and assistant messages and launch the session's configured Agent without routing through React

#### Scenario: Agent is unavailable
- **WHEN** the bound session's Agent is unavailable at execution time
- **THEN** the shared chat service SHALL fail concisely, persist the failed assistant state, and allow the connector to deliver a localized failure response

### Requirement: Final-only outbound delivery
The connector SHALL deliver an Agent response only after the assistant message reaches a terminal completed state.

#### Scenario: Agent response completes
- **WHEN** an IM-originated assistant message completes successfully
- **THEN** the outbound dispatcher SHALL send the final response to the originating connector and external chat

#### Scenario: Response exceeds platform limit
- **WHEN** a completed response exceeds a platform message-size limit
- **THEN** the adapter SHALL split the final response into ordered transport messages after completion

#### Scenario: Agent response fails
- **WHEN** an IM-originated assistant message fails or is cancelled
- **THEN** the connector SHALL send a concise localized failure result and SHALL NOT expose raw runtime diagnostics

#### Scenario: Outbound delivery fails
- **WHEN** final delivery fails after Agent completion
- **THEN** the assistant message SHALL remain completed in VaneHub and the connector SHALL record a redacted delivery failure without rerunning the Agent

### Requirement: Non-blocking platform acknowledgement
Connectors with acknowledgement deadlines SHALL decouple platform acknowledgement from Agent execution.

#### Scenario: Platform event requires acknowledgement
- **WHEN** a platform delivers an accepted event with a bounded acknowledgement deadline
- **THEN** the adapter SHALL acknowledge after durable deduplication and queue acceptance without waiting for Agent completion

### Requirement: Connector state persistence
The runtime SHALL persist only the minimum non-secret checkpoints required to resume polling and avoid repeated processing after restart.

#### Scenario: Restart enabled connector
- **WHEN** VaneHub restarts with an enabled, configured connector
- **THEN** the connector SHALL resume from its persisted non-secret checkpoint and credential references without replaying already confirmed work

#### Scenario: Maintain bounded dedup state
- **WHEN** deduplication records exceed their retention window
- **THEN** the native runtime SHALL remove expired records through bounded maintenance without deleting active bindings or message history

