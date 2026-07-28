## MODIFIED Requirements

### Requirement: Backward-compatible configuration defaults
The system SHALL keep existing sessions usable when they do not yet contain a persisted chat-configuration snapshot. When the native model discovery module provides a discovered model from the CLI's native config file, that discovered model SHALL serve as the initial model for sessions without an explicit persisted override.

#### Scenario: Load an existing session without a snapshot
- **WHEN** an existing session is opened after the additive migration
- **THEN** the service SHALL derive a valid effective configuration from the session's agent, interaction mode, supported model catalog, existing CLI profile, native-discovered model (if available), and defined defaults

#### Scenario: Native-discovered model takes precedence over hardcoded default
- **WHEN** a session without a persisted configuration override is opened and the CLI's native config contains a model value
- **THEN** the native-discovered model SHALL be the effective model for that session
- **AND** the hardcoded agent default SHALL serve as a fallback only when no native model is discovered

#### Scenario: Persist the first explicit update
- **WHEN** a user explicitly changes a derived preference for a session without a snapshot
- **THEN** the service SHALL persist the normalized snapshot without changing the session id, agent id, interaction mode, or history

#### Scenario: Delete a configured session
- **WHEN** a session is deleted
- **THEN** its persisted chat configuration SHALL be removed with the session and SHALL NOT affect any other session

## ADDED Requirements

### Requirement: Reject invalid configuration
The system SHALL validate chat configuration before it can reach CLI launch argument construction. Provider mismatch, unsupported permission modes, and unsupported reasoning depths SHALL be rejected. Unknown model IDs SHALL be accepted as valid custom models.

#### Scenario: Reject provider mismatch
- **WHEN** a configuration pairs a `gemini-cli` session with `providerId: "openai"`
- **THEN** the service SHALL reject the configuration with a provider mismatch error

#### Scenario: Reject unsupported permission mode
- **WHEN** a configuration supplies a permission mode not in the recognized set (`default`, `plan`, `agent`, `auto`)
- **THEN** the service SHALL reject the configuration

#### Scenario: Reject unsupported reasoning depth
- **WHEN** a configuration supplies a reasoning depth not in the recognized set (`low`, `medium`, `high`, `max`)
- **THEN** the service SHALL reject the configuration

#### Scenario: Accept valid custom model ID
- **WHEN** a configuration supplies a model ID not in the hardcoded catalog (e.g., `deepseek-chat` for `claude-code`) but the provider matches the agent's expected provider
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
