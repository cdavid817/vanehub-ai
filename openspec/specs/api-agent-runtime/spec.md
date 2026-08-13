# api-agent-runtime Specification

## Purpose
TBD - created by archiving change add-custom-agent-registration. Update Purpose after archive.
## Requirements
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
The system SHALL report an API-based agent as available only when it has a display name, provider, model id, an actually present stored credential, and — for `interface_format = openai-compatible` — a non-empty `base_url`, without making a network call to the provider. A structurally incomplete API Agent SHALL be unavailable; a structurally complete Agent with no credential SHALL require authentication; a credential-store access failure SHALL produce a safe non-selectable state without preventing unrelated Agents from being listed.

#### Scenario: Available when configured
- **WHEN** an API-based agent has a complete provider configuration and its credential is present in the platform credential store
- **THEN** the system SHALL mark it as selectable
- **AND** the availability check SHALL NOT contact the configured provider

#### Scenario: Unavailable when misconfigured
- **WHEN** an API-based agent is missing its model id or another required non-secret configuration field
- **THEN** the system SHALL mark it unavailable with a reason suitable for user display

#### Scenario: Authentication required when credential is missing
- **WHEN** an API-based agent has complete non-secret configuration but its credential is absent from the platform credential store
- **THEN** the system SHALL mark it `needs-auth` with a reason suitable for user display

#### Scenario: Unavailable when an OpenAI-compatible agent is missing its base URL
- **WHEN** an API-based agent has `interface_format = openai-compatible` and no non-empty `base_url`
- **THEN** the system SHALL mark it unavailable with a reason suitable for user display

#### Scenario: Credential store cannot be inspected
- **WHEN** credential presence cannot be determined because the credential store returns an error
- **THEN** that API Agent SHALL be non-selectable with a safe reason
- **AND** the system SHALL write a redacted warning through unified logging
- **AND** other Agent registry entries SHALL remain available to the caller

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

### Requirement: Pre-registered API Agent configuration
The system SHALL allow a built-in API Agent that exists before credentials are configured to own multiple catalog-backed provider Profiles, use at most one active Profile for generation, and remove all provider configuration without changing its stable Agent identity.

#### Scenario: Configure a pre-registered API Agent
- **WHEN** a user submits a supported provider id, endpoint type, Profile name, model, and credential for the built-in OnePiece API Agent
- **THEN** the system SHALL persist the non-secret Profile against stable id `onepiece`
- **AND** it SHALL derive provider, interface format, Base URL, and model-discovery behavior from the selected catalog endpoint record
- **AND** it SHALL store the submitted API key through a Profile-scoped credential port
- **AND** the first Profile SHALL become active while later Profiles require explicit activation

#### Scenario: Reject an incomplete built-in configuration
- **WHEN** a built-in API Agent Profile omits its name, supported provider id, endpoint type, model, or credential, names an unknown directory entry, or selects an endpoint type that the provider or runtime does not support
- **THEN** the system SHALL reject the operation without changing the active Profile

#### Scenario: Resolve the active Profile for generation
- **WHEN** OnePiece starts a generation with an active ready provider Profile
- **THEN** the API runtime SHALL use that Profile's provider configuration and credential snapshot
- **AND** inactive Profiles SHALL NOT influence the request

#### Scenario: Remove all configuration from a pre-registered API Agent
- **WHEN** a user removes all OnePiece provider Profiles
- **THEN** the API runtime SHALL retain the registered Agent entry but SHALL NOT attempt provider generation until a Profile is configured and active again

### Requirement: API provider invocation usage accounting
Every API-based Agent model request SHALL emit a normalized accounting observation when valid provider usage is available, including user-visible, tool-continuation, compaction, memory-extraction, failed, cancelled, and retry attempts.

#### Scenario: Capture Anthropic streaming usage
- **WHEN** an Anthropic Messages stream reports input or cache usage at message start and output usage during message progress or completion
- **THEN** the runtime SHALL combine those events into one invocation observation
- **AND** it SHALL finalize that observation with the provider's authoritative values and semantic mapping

#### Scenario: Capture supported OpenAI-compatible streaming usage
- **WHEN** a catalog endpoint declares a supported streaming usage strategy and its final usage chunk arrives
- **THEN** the runtime SHALL normalize that chunk into one invocation observation
- **AND** endpoint-specific cache and reasoning dimensions SHALL follow the declared strategy

#### Scenario: Avoid speculative paid retry
- **WHEN** an endpoint does not declare support for an optional usage request parameter
- **THEN** the runtime SHALL NOT retry a potentially accepted model request merely to add or remove that parameter
- **AND** absent usage SHALL degrade to the configured estimation behavior

#### Scenario: Account for tool round trips
- **WHEN** an API Agent executes tools and sends their results through additional provider requests
- **THEN** every request SHALL receive a distinct invocation sequence and `tool-continuation` purpose
- **AND** all unique invocations SHALL contribute to the owning generation projection

#### Scenario: Account for internal model calls
- **WHEN** API-Agent context compaction or automatic memory extraction invokes the provider
- **THEN** the call SHALL be recorded with its internal purpose separately from final-response consumption

