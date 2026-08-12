# session-chat-configuration Specification

## Purpose
Defines validated session-level chat configuration persistence, backward-compatible defaults, shared configuration across chat surfaces, and parity between desktop and Web/mock service adapters.
## Requirements
### Requirement: Session-level chat configuration persistence
The system SHALL persist validated chat preferences per session, including `executionMode`, and SHALL compose them with the session's authoritative stable agent id and interaction mode to produce the effective `ChatConfig`.

#### Scenario: Save configuration from the main chat
- **WHEN** a user changes provider, model, execution mode, reasoning, streaming, thinking, or long-context preferences for the active session
- **THEN** the frontend SHALL save the validated preferences through `AgentService` and the active session SHALL retain them across window and application restarts

#### Scenario: Keep session identity authoritative
- **WHEN** the system composes an effective configuration
- **THEN** `agentId` and `interactionMode` SHALL come from the referenced session's stable persisted fields rather than an independently writable configuration snapshot

#### Scenario: Reject invalid configuration
- **WHEN** a configuration contains an unsupported provider/model combination, execution mode, reasoning depth, or value type
- **THEN** the service SHALL reject it before it can reach runtime execution or CLI launch argument construction

### Requirement: Configuration defaults reset the removed permission model
The system SHALL initialize sessions without a valid new-format chat-configuration snapshot with `executionMode: "inherit"`. It SHALL NOT translate legacy `permissionMode` values into execution modes. When native model discovery provides a discovered model, that model SHALL remain the initial model for sessions without an explicit new-format model override.

#### Scenario: Load a session after the breaking migration
- **WHEN** a session has no new-format configuration because its legacy snapshot was removed
- **THEN** the service SHALL derive a valid configuration with `executionMode: "inherit"`
- **AND** it SHALL preserve the session identity and history

#### Scenario: Reject the removed field
- **WHEN** a client submits `permissionMode` or `permission_mode`
- **THEN** the service SHALL reject the request rather than translating the value

#### Scenario: Native-discovered model takes precedence over hardcoded default
- **WHEN** a session without an explicit new-format model override is opened and the CLI's native config contains a model value
- **THEN** the native-discovered model SHALL be the effective model for that session
- **AND** the hardcoded agent default SHALL serve as a fallback only when no native model is discovered

#### Scenario: Persist the first explicit update
- **WHEN** a user explicitly changes a derived preference for a session without a new-format snapshot
- **THEN** the service SHALL persist the validated snapshot without changing the session id, agent id, interaction mode, or history

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

### Requirement: Reject invalid configuration
The system SHALL validate chat configuration before it can reach runtime execution or CLI launch argument construction. Provider mismatch, execution modes outside `inherit`, `plan`, and `execute`, and unsupported reasoning depths SHALL be rejected. Unknown model IDs SHALL be accepted as valid custom models.

#### Scenario: Reject provider mismatch
- **WHEN** a configuration pairs a `gemini-cli` session with `providerId: "openai"`
- **THEN** the service SHALL reject the configuration with a provider mismatch error

#### Scenario: Reject unsupported execution mode
- **WHEN** a configuration supplies an execution mode outside `inherit`, `plan`, and `execute`
- **THEN** the service SHALL reject the configuration

#### Scenario: Reject unsupported permission mode
- **WHEN** a configuration supplies any legacy permission mode such as `default`, `agent`, or `auto`
- **THEN** the service SHALL reject the configuration

#### Scenario: Reject unsupported reasoning depth
- **WHEN** a configuration supplies a reasoning depth not in the recognized set (`low`, `medium`, `high`, `max`)
- **THEN** the service SHALL reject the configuration

#### Scenario: Accept valid custom model ID
- **WHEN** a configuration supplies a model ID not in the hardcoded catalog but the provider matches the agent's expected provider
- **THEN** the service SHALL accept the configuration with the unknown model ID preserved

#### Scenario: Accept known catalog model ID
- **WHEN** a configuration supplies a model ID matching a hardcoded catalog entry
- **THEN** the service SHALL accept the configuration with behavior identical to the current implementation

#### Scenario: Reject empty or whitespace-only model ID
- **WHEN** a configuration supplies an empty string or whitespace-only model ID
- **THEN** the service SHALL reject or normalize it to the agent's default model

### Requirement: Custom model ID passthrough in model resolution
The `model_id_from_cli` function SHALL accept and passthrough any non-empty model string that does not match a known CLI alias. Unknown values SHALL be returned as-is rather than returning `None`.

#### Scenario: Passthrough of unknown model value
- **WHEN** `model_id_from_cli("claude-code", "deepseek-chat")` is called
- **THEN** the function SHALL return `Some("deepseek-chat")`

#### Scenario: Known alias still resolves correctly
- **WHEN** `model_id_from_cli("claude-code", "sonnet")` is called
- **THEN** the function SHALL return `Some("claude-sonnet-5")` as before

#### Scenario: Default string still returns None
- **WHEN** `model_id_from_cli("claude-code", "default")` is called
- **THEN** the function SHALL return `None` so the caller falls back to the default model

#### Scenario: Empty string returns None
- **WHEN** `model_id_from_cli("claude-code", "")` is called
- **THEN** the function SHALL return `None`

### Requirement: Conservative capability defaults for custom models
When a model ID is not in the hardcoded catalog, the system SHALL apply conservative capability defaults: reasoning depth clamped to `low` and long-context disabled.

#### Scenario: Custom model reasoning depth is clamped to low
- **WHEN** `clamp_reasoning_for_model("deepseek-chat", Some("high"))` is called
- **THEN** the function SHALL return `Some("low")`

#### Scenario: Custom model does not support long context
- **WHEN** a session uses an unknown model ID
- **THEN** `long_context` SHALL default to `false`

#### Scenario: Known model capabilities are unchanged
- **WHEN** `clamp_reasoning_for_model("claude-opus-4-8", Some("high"))` is called
- **THEN** the function SHALL return `Some("high")` as before
