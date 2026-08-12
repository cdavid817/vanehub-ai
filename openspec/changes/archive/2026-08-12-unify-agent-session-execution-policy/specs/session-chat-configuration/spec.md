## MODIFIED Requirements

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
