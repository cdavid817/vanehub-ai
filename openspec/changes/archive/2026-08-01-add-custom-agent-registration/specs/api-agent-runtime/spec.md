## ADDED Requirements

### Requirement: API-based agent registration
The system SHALL allow a user to register an agent whose `launch_kind` is `api`, capturing a display name, a provider identifier, an API key, a model id, and an interface format.

#### Scenario: Register a new API-based agent
- **WHEN** a user submits a display name, provider, API key, model id, and interface format for a new API-based agent
- **THEN** the system SHALL create a registered agent entry with `launch_kind = api` and a stable kebab-case agent id
- **AND** the agent SHALL appear in the agent registry alongside CLI-managed agents

#### Scenario: Reject incomplete registration
- **WHEN** a registration submission is missing a required field (display name, provider, API key, model id, or interface format)
- **THEN** the system SHALL reject the registration without creating a partial agent entry

### Requirement: Interface format selection
The system SHALL support exactly two `interface_format` values for API-based agents: `anthropic` (Anthropic Messages API, fixed endpoint) and `openai-compatible` (OpenAI Chat Completions-compatible, user-supplied endpoint). An `openai-compatible` agent SHALL also capture a `base_url`; an `anthropic` agent SHALL NOT require one.

#### Scenario: Register an OpenAI-compatible agent
- **WHEN** a user registers an API-based agent with `interface_format = openai-compatible` and a `base_url`
- **THEN** the system SHALL persist the `base_url` alongside the agent's other configuration
- **AND** chat generation for that agent SHALL call `{base_url}/chat/completions` rather than the Anthropic endpoint

#### Scenario: Reject OpenAI-compatible registration without a base URL
- **WHEN** a user submits `interface_format = openai-compatible` without a `base_url`
- **THEN** the system SHALL reject the registration without creating a partial agent entry

#### Scenario: Anthropic agent ignores base URL
- **WHEN** a user registers an API-based agent with `interface_format = anthropic`
- **THEN** the system SHALL use the fixed Anthropic Messages API endpoint regardless of any `base_url` value submitted

### Requirement: API key credential storage
The system SHALL store an API-based agent's API key through the platform credential store and SHALL NOT persist the raw key value in a plaintext database column.

#### Scenario: Store credential on registration
- **WHEN** a user registers an API-based agent with an API key
- **THEN** the system SHALL write the key to the platform credential store
- **AND** the agent's persisted record SHALL reference the stored credential rather than embedding the key value

#### Scenario: Credential omitted from reads
- **WHEN** the agent registry or settings UI reads an API-based agent's configuration
- **THEN** the response SHALL NOT include the raw API key value

### Requirement: API-based agent availability
The system SHALL report an API-based agent as available when it has a display name, provider, model id, a stored credential reference, and — for `interface_format = openai-compatible` — a non-empty `base_url`, without making a network call to the provider.

#### Scenario: Available when configured
- **WHEN** an API-based agent has a complete registration and a stored credential reference
- **THEN** the system SHALL mark it as selectable

#### Scenario: Unavailable when misconfigured
- **WHEN** an API-based agent is missing its credential reference or model id
- **THEN** the system SHALL mark it as unavailable with a reason suitable for user display

#### Scenario: Unavailable when an OpenAI-compatible agent is missing its base URL
- **WHEN** an API-based agent has `interface_format = openai-compatible` and no `base_url`
- **THEN** the system SHALL mark it as unavailable with a reason suitable for user display

### Requirement: API-based chat generation
The system SHALL generate assistant responses for an API-based agent's session by calling the configured provider's API directly instead of spawning a CLI process, using the request shape, authentication header, and endpoint appropriate to the agent's `interface_format`.

#### Scenario: Send message to API-based agent
- **WHEN** a user sends a message in a session whose agent has `launch_kind = api`
- **THEN** the system SHALL call the configured provider API with the conversation history and the agent's configured model
- **AND** it SHALL emit the same `started`, `token`, `thinking`, `completed`, or `failed` chat events used for CLI sessions

#### Scenario: Send message to an OpenAI-compatible agent
- **WHEN** a user sends a message in a session whose agent has `interface_format = openai-compatible`
- **THEN** the system SHALL call `{base_url}/chat/completions` with an `Authorization: Bearer` header and an OpenAI Chat Completions-shaped streaming request
- **AND** it SHALL translate the response's `choices[].delta.content` into `token` events and a `reasoning_content` delta (when present) into `thinking` events

#### Scenario: Missing or invalid credential
- **WHEN** chat generation starts for an API-based agent whose stored credential is missing or rejected by the provider
- **THEN** the user message SHALL remain persisted
- **AND** the assistant message SHALL be marked `failed` with a concise user-facing error
- **AND** detailed diagnostics SHALL be written through unified logging with the API key redacted

#### Scenario: Network or provider failure
- **WHEN** the provider API call fails or the connection is interrupted mid-stream
- **THEN** already-streamed content SHALL remain persisted
- **AND** the assistant message SHALL be marked `failed`

### Requirement: API-based generation excludes CLI-only surfaces
API-based agent sessions SHALL NOT use Agent Terminal, CLI launch-parameter profiles, or Prompt Hook chat assembly.

#### Scenario: No Agent Terminal for API-based sessions
- **WHEN** a user opens a session for an API-based agent
- **THEN** the system SHALL NOT offer or start an Agent Terminal process for that session

#### Scenario: No CLI parameter profile
- **WHEN** chat generation runs for an API-based agent
- **THEN** the system SHALL NOT apply a CLI Parameter Management profile or Prompt Hook assembly to that generation

### Requirement: Web runtime parity for API-based agents
The Web/mock runtime SHALL expose the same registration and generation service contract as the desktop runtime without making real network calls to the provider.

#### Scenario: Web mock registration
- **WHEN** a user registers an API-based agent in Web mode
- **THEN** the Web adapter SHALL persist deterministic mock agent data through the frontend service contract without accessing SQLite or the OS credential store

#### Scenario: Web mock generation
- **WHEN** a user sends a message to an API-based agent's session in Web mode
- **THEN** the Web adapter SHALL emit deterministic simulated chat events without making a real provider API call
