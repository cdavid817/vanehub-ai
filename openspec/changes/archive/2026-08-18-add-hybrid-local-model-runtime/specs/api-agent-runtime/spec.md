## MODIFIED Requirements

### Requirement: API-based agent registration
The system SHALL allow a user to register an agent whose `launch_kind` is `api`, capturing a display name, provider identifier, model id, interface format, endpoint metadata, and an optional API key only when the endpoint Profile explicitly permits unauthenticated local/private access.

#### Scenario: Register a new API-based agent
- **WHEN** a user submits a display name, provider, API key, model id, interface format, and valid endpoint metadata for a new API-based agent
- **THEN** the system SHALL create a registered agent entry with `launch_kind = api` and a stable kebab-case agent id
- **AND** the agent SHALL appear in the agent registry alongside CLI-managed agents

#### Scenario: Register an unauthenticated local API-based agent
- **WHEN** a user submits complete OpenAI-compatible loopback or explicitly confirmed private endpoint metadata whose authentication mode is `none`
- **THEN** the system SHALL create the API Agent without fabricating or storing a credential

#### Scenario: Reject incomplete registration
- **WHEN** a registration submission is missing a required display name, provider, model id, interface format, Base URL, endpoint privacy classification, timeout, or required credential
- **THEN** the system SHALL reject the registration without creating a partial agent entry

### Requirement: API key credential storage
The system SHALL store any supplied API-based Agent key through the platform credential store, SHALL NOT persist the raw value in a plaintext database column, and SHALL preserve an explicit `none` authentication mode without creating a placeholder secret.

#### Scenario: Store credential on registration
- **WHEN** a user registers an API-based agent with an API key
- **THEN** the system SHALL write the key to the platform credential store
- **AND** the agent's persisted record SHALL reference the stored credential rather than embedding the key value

#### Scenario: Save an unauthenticated Profile
- **WHEN** a valid local/private Profile uses authentication mode `none`
- **THEN** the system SHALL persist that mode without writing an empty or synthetic credential

#### Scenario: Credential omitted from reads
- **WHEN** the agent registry or settings UI reads an API-based agent's configuration
- **THEN** the response SHALL NOT include the raw API key value

### Requirement: API-based agent availability
The system SHALL report an API-based Agent as structurally selectable only when its required non-secret endpoint fields and any credential required by its authentication mode are present, without making a network call. Network readiness SHALL be reported separately by explicit verification; credential-store failure SHALL not prevent unrelated Agents from being listed.

#### Scenario: Available when configured
- **WHEN** an API-based agent has complete provider configuration and its required credential is present
- **THEN** the system SHALL mark it structurally selectable without contacting the provider

#### Scenario: Available when authentication is none
- **WHEN** a complete local/private API Agent explicitly uses authentication mode `none`
- **THEN** absence of a stored credential SHALL NOT produce `needs-auth`

#### Scenario: Unavailable when misconfigured
- **WHEN** an API-based agent is missing its model id or another required non-secret Profile field
- **THEN** the system SHALL mark it unavailable with a reason suitable for user display

#### Scenario: Authentication required when credential is missing
- **WHEN** an API-based Agent's authentication mode requires a credential and the credential is absent
- **THEN** the system SHALL mark it `needs-auth` with a reason suitable for user display

#### Scenario: Unavailable when an OpenAI-compatible agent is missing its base URL
- **WHEN** an API-based agent has `interface_format = openai-compatible` and no non-empty `base_url`
- **THEN** the system SHALL mark it unavailable with a reason suitable for user display

#### Scenario: Credential store cannot be inspected
- **WHEN** required credential presence cannot be determined because the credential store returns an error
- **THEN** that API Agent SHALL be non-selectable with a safe reason
- **AND** the system SHALL write a redacted warning through unified logging
- **AND** other Agent registry entries SHALL remain available to the caller

## ADDED Requirements

### Requirement: API generation uses an immutable endpoint Profile snapshot
Each API generation SHALL capture the selected endpoint Profile, capability metadata, privacy classification, timeout, context budget, and credential reference before routing and request construction.

#### Scenario: Profile changes during generation
- **WHEN** a user edits or activates another Profile after generation starts
- **THEN** the in-flight generation SHALL retain its starting snapshot
- **AND** subsequent generations SHALL use the new selection
