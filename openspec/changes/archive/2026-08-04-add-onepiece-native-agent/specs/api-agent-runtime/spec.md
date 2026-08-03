## ADDED Requirements

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

## MODIFIED Requirements

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
