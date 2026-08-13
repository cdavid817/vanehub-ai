# im-connector-management

## Purpose

This capability defines the five built-in IM connector management behaviors.
## Requirements

Wall time: 0.9 seconds
Output:

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

### Requirement: Stable IM connector wire identities
Native and frontend IM services SHALL exchange the stable connector ids `feishu`, `telegram`, `dingtalk`, `wecom`, and `weixin` for descriptors, configuration, health events, and command inputs.

#### Scenario: Serialize DingTalk and WeCom views
- **WHEN** the native service returns DingTalk or WeCom connector data
- **THEN** every nested connector kind SHALL serialize as `dingtalk` or `wecom` respectively
- **AND** the frontend contract SHALL parse the complete connector list without an invalid-value error

#### Scenario: Deserialize stable command ids
- **WHEN** the frontend submits `dingtalk` or `wecom` to an IM command
- **THEN** the native command boundary SHALL deserialize it to the matching connector kind

