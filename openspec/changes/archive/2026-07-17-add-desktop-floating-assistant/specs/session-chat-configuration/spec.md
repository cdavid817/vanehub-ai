## ADDED Requirements

### Requirement: Session-level chat configuration persistence
The system SHALL persist validated chat preferences per session and SHALL compose them with the session's authoritative stable agent id and interaction mode to produce the effective `ChatConfig`.

#### Scenario: Save configuration from the main chat
- **WHEN** a user changes provider, model, permission, reasoning, streaming, thinking, or long-context preferences for the active session
- **THEN** the frontend SHALL save the validated preferences through `AgentService` and the active session SHALL retain them across window and application restarts

#### Scenario: Keep session identity authoritative
- **WHEN** the system composes an effective configuration
- **THEN** `agentId` and `interactionMode` SHALL come from the referenced session's stable persisted fields rather than an independently writable configuration snapshot

#### Scenario: Reject invalid configuration
- **WHEN** a configuration contains an unsupported provider/model combination, permission mode, reasoning depth, or value type
- **THEN** the service SHALL reject or normalize it before it can reach CLI launch argument construction

### Requirement: Backward-compatible configuration defaults
The system SHALL keep existing sessions usable when they do not yet contain a persisted chat-configuration snapshot.

#### Scenario: Load an existing session without a snapshot
- **WHEN** an existing session is opened after the additive migration
- **THEN** the service SHALL derive a valid effective configuration from the session's agent, interaction mode, supported model catalog, existing CLI profile, and defined defaults

#### Scenario: Persist the first explicit update
- **WHEN** a user explicitly changes a derived preference for a session without a snapshot
- **THEN** the service SHALL persist the normalized snapshot without changing the session id, agent id, interaction mode, or history

#### Scenario: Delete a configured session
- **WHEN** a session is deleted
- **THEN** its persisted chat configuration SHALL be removed with the session and SHALL NOT affect any other session

### Requirement: Shared configuration across chat surfaces
Every chat surface for the same session SHALL read the same persisted effective configuration and SHALL react to committed configuration changes.

#### Scenario: Open mini chat after configuring the main window
- **WHEN** the main window commits a configuration change and mini chat opens for the same session
- **THEN** mini chat SHALL use the committed effective configuration without presenting duplicate advanced controls

#### Scenario: Observe a configuration event
- **WHEN** a session configuration is committed while another VaneHub window displays that session
- **THEN** the other window SHALL invalidate its stale configuration and reload the persisted value

#### Scenario: Keep configurations isolated by session
- **WHEN** a user switches between sessions with different persisted preferences
- **THEN** each chat surface SHALL load the preferences belonging only to its active session

### Requirement: Configuration service parity
The Tauri and Web/mock agent-service adapters SHALL implement the same session chat-configuration contract.

#### Scenario: Use the Tauri adapter
- **WHEN** a desktop surface gets or saves session chat configuration
- **THEN** the Tauri adapter SHALL call the Rust service boundary and SQLite SHALL remain inaccessible to React components

#### Scenario: Use the Web/mock adapter
- **WHEN** a browser surface gets or saves session chat configuration
- **THEN** the Web/mock adapter SHALL provide deterministic per-session persistence compatible with the same TypeScript interface
