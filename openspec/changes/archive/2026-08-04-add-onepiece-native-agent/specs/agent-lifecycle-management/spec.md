## ADDED Requirements

### Requirement: Shared multi-endpoint provider directory for Agent configuration
The system SHALL expose one versioned directory of exactly 25 stable fixed-host provider identities to the OnePiece, Claude Code, Codex CLI, and OpenCode configuration tabs. Each provider SHALL contain a partial map of reviewed protocol endpoint records rather than one vendor-wide Base URL, and the shared directory SHALL distinguish `anthropic-messages`, `openai-chat-completions`, and `openai-responses` endpoints when the provider publishes them.

#### Scenario: Render the shared directory in four Agent tabs
- **WHEN** a user adds a configuration from the OnePiece, Claude Code, Codex CLI, or OpenCode tab
- **THEN** the surface SHALL use the same provider identity, category, search/filter behavior, card presentation, help links, and provider mark for a matching provider id
- **AND** Agent-specific Profile fields and persistence SHALL remain owned by that Agent's existing configuration boundary

#### Scenario: Provider publishes multiple protocols
- **WHEN** the reviewed directory contains both Anthropic and OpenAI endpoint records for one provider
- **THEN** the catalog SHALL preserve their distinct endpoint types and immutable Base URLs
- **AND** an Agent adapter SHALL select or offer only the endpoint types its runtime can execute safely

#### Scenario: Provider publishes only one protocol
- **WHEN** the reviewed sources expose no Anthropic or no OpenAI endpoint for a provider
- **THEN** the directory SHALL represent that protocol as absent or unsupported
- **AND** the system SHALL NOT invent an endpoint by appending, removing, or replacing URL path segments

#### Scenario: Adapt endpoints for each Agent
- **WHEN** the shared directory is projected into Agent-specific configuration presets
- **THEN** Claude Code SHALL use a published Anthropic endpoint, Codex CLI SHALL prefer Responses and then Chat Completions, OpenCode SHALL map either supported protocol to its matching SDK provider, and OnePiece SHALL allow its supported Anthropic or OpenAI Chat endpoint
- **AND** an endpoint requiring an unavailable protocol-conversion layer SHALL be non-selectable with a safe reason

#### Scenario: Render copied provider marks safely
- **WHEN** a provider mark copied from Cherry Studio is rendered
- **THEN** the shared icon component SHALL use the recorded upstream asset and an appropriate light/dark variant when available
- **AND** the repository SHALL retain MIT attribution, exact provenance, and a trademark non-affiliation notice
- **AND** an unavailable mark SHALL fall back to deterministic initials

### Requirement: Shared API-credential verification for Agent configuration
The system SHALL let users explicitly verify API-key credentials from the Claude Code, Codex CLI, OpenCode, and OnePiece configuration surfaces through shared status semantics and presentation while each Agent context retains ownership of Profile validation, effective endpoint resolution, and secure credential access. Verification SHALL be ephemeral and SHALL NOT save a transient credential, mutate or apply a Profile, activate a provider, change Agent readiness, or persist a validity flag.

#### Scenario: Verify a transient credential while editing
- **WHEN** a user requests verification from a new or edited Agent Profile whose current draft supplies an API-key credential, effective endpoint, protocol, and model
- **THEN** the owning service boundary SHALL validate the draft using the same structural rules as Profile saving
- **AND** the native adapter SHALL issue one bounded minimal request through that effective provider configuration without first saving or applying the Profile

#### Scenario: Verify a saved Profile credential
- **WHEN** a user requests verification from a Claude Code, Codex CLI, OpenCode, or OnePiece Profile card
- **THEN** the owning application context SHALL load that Profile's scoped credential without returning it to React
- **AND** it SHALL resolve the effective protocol, endpoint, and model from the saved Profile before invoking the shared probe

#### Scenario: Classify the provider response
- **WHEN** the minimal provider request completes
- **THEN** a successful provider response SHALL produce `valid`, HTTP 401 or 403 SHALL produce `invalid-credential`, HTTP 400, 404, or 422 SHALL produce `configuration-rejected`, HTTP 429 SHALL produce `rate-limited`, and timeout, network, TLS, or HTTP 5xx failure SHALL produce `provider-unavailable`
- **AND** any unrecognized response SHALL remain safely inconclusive rather than being guessed as a valid or invalid key

#### Scenario: Validate each supported CLI wire protocol
- **WHEN** Claude Code, Codex CLI, or OpenCode resolves an API-key Profile for verification
- **THEN** Claude Code SHALL probe Anthropic Messages, Codex CLI SHALL probe its configured OpenAI Responses or Chat Completions API, and OpenCode SHALL probe the endpoint type selected by its reviewed preset or supported custom SDK/provider shape
- **AND** all three SHALL reuse the same bounded provider-probe implementation and shared UI status component

#### Scenario: Profile has no API-key authentication
- **WHEN** a Profile uses an authentication mode such as Claude Code existing authentication or Codex official preserved authentication that supplies no API key to VaneHub
- **THEN** API-key verification SHALL report `unsupported` or be disabled with an explanation
- **AND** it SHALL NOT claim to validate the external OAuth or CLI login state

#### Scenario: Keep verification secret-safe and bounded
- **WHEN** any credential verification succeeds or fails
- **THEN** the request SHALL use no conversation history, system prompt, tools, files, or application context, SHALL cap output at one token, SHALL use a 15-second timeout without retries or redirects, and SHALL bound any error-body read
- **AND** credentials, authorization headers, prompts, response bodies, and URL query strings SHALL NOT be returned, persisted, or written to logs
- **AND** unified logs MAY contain only safe Agent/Profile/provider identity, protocol, classification, and latency metadata

#### Scenario: Ignore a stale verification result
- **WHEN** provider, endpoint, model, credential, or Profile selection changes while verification is in flight
- **THEN** the UI SHALL cancel or ignore the superseded result
- **AND** the stale result SHALL NOT replace the current draft's verification state

#### Scenario: Verify credentials in Web/mock mode
- **WHEN** a Web/mock configuration surface requests credential verification with structurally valid mock input
- **THEN** the Web adapter SHALL return the same discriminated result contract deterministically without network access
- **AND** it SHALL NOT retain the submitted credential

## MODIFIED Requirements

### Requirement: Editing a registered API agent
The system SHALL let a user update a user-created API agent's display name, model id, Base URL, and stored API key without changing its id, provider, or interface format. For built-in OnePiece, the system SHALL use dedicated catalog-backed provider-Profile operations that preserve id `onepiece` while allowing multiple independently secured provider/endpoint/model configurations and one explicit active Profile; OnePiece provider, endpoint type, interface format, and Base URL SHALL be resolved from the selected built-in directory entry rather than edited directly.

#### Scenario: Display name, model, or base URL edited
- **WHEN** a user submits new values for an existing user-created API agent's display name, model id, and/or Base URL
- **THEN** the system SHALL persist the new values against the same agent id
- **AND** the agent's provider and interface format SHALL remain unchanged

#### Scenario: API key rotated
- **WHEN** a user submits a new API key for an existing API agent
- **THEN** the system SHALL replace the stored credential with the new value
- **AND** subsequent generations for that agent SHALL use the new key

#### Scenario: Edit re-validates like registration
- **WHEN** a user submits an edit that omits a required Base URL for an agent whose interface format is `openai-compatible`
- **THEN** the system SHALL reject the edit with a validation error
- **AND** it SHALL NOT persist any part of the edit

#### Scenario: Provider and interface format are immutable
- **WHEN** a user attempts to change a user-created API agent's provider or interface format through the ordinary edit operation
- **THEN** the system SHALL NOT apply that change

#### Scenario: Switch OnePiece provider and interface format
- **WHEN** a user activates a complete OnePiece provider Profile through the dedicated operation
- **THEN** the system SHALL use that Profile's catalog-resolved provider, endpoint type, interface format, and Base URL for subsequent generations
- **AND** the system SHALL preserve stable id `onepiece`, all existing references, and every inactive Profile

### Requirement: Deleting a registered API agent
The system SHALL let a user delete a user-created API agent and its stored credential, and SHALL reject the deletion without making any changes if that agent is still referenced by other stored data. The system SHALL reject deletion of any built-in API Agent, including OnePiece, regardless of references and SHALL direct the user to reset its configuration instead.

#### Scenario: Delete an unreferenced agent
- **WHEN** a user deletes a user-created API agent that has no sessions, messages, memories, usage records, or Loop worker/verifier assignments referencing it
- **THEN** the system SHALL remove the agent and its stored credential
- **AND** it SHALL remove any Skill-to-agent bindings for that agent

#### Scenario: Delete rejected when the agent is still referenced
- **WHEN** a user deletes a user-created API agent that has at least one session, memory, usage record, or Loop worker/verifier assignment referencing it
- **THEN** the system SHALL reject the deletion
- **AND** it SHALL report which kinds of data still reference the agent
- **AND** it SHALL NOT remove the agent, its credential, or any of the referencing data

#### Scenario: Delete OnePiece is rejected
- **WHEN** any caller requests deletion of stable id `onepiece`
- **THEN** the application and persistence boundaries SHALL reject the request
- **AND** the OnePiece identity, configuration, credential, and references SHALL remain unchanged
- **AND** user-facing surfaces SHALL offer configuration reset instead of identity deletion

### Requirement: Web runtime lifecycle-management parity
The Web/mock runtime SHALL simulate editing and deleting user-created API agents and listing, saving, activating, deleting, or removing all OnePiece provider Profiles through the same service contracts the desktop runtime uses, including referenced-agent and built-in-agent delete rejection.

#### Scenario: Mock edit and delete
- **WHEN** a user edits or deletes a user-created API agent in Web/mock mode
- **THEN** the Web adapter SHALL apply the same change to its in-memory agent registry
- **AND** it SHALL enforce the same referenced-agent delete rejection the desktop runtime enforces

#### Scenario: Mock OnePiece configuration lifecycle
- **WHEN** a user adds, edits, activates, deletes, or removes all OnePiece provider Profiles in Web/mock mode
- **THEN** the Web adapter SHALL apply equivalent non-secret Profile and readiness transitions in memory
- **AND** it SHALL reject deletion of the OnePiece identity
