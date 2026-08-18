## MODIFIED Requirements

### Requirement: OnePiece configuration lifecycle
The system SHALL keep the OnePiece identity separate from multiple named endpoint Profiles, secure credentials independently per Profile, use at most one explicitly active Profile for direct generation, and expose provider-directory, custom endpoint, list, save, activate, delete, remove-all, privacy, capability, and routing operations through the shared frontend service boundary. Catalog Profiles SHALL resolve immutable reviewed endpoints; custom local/private Profiles SHALL accept only validated explicit OpenAI-compatible endpoint metadata and SHALL retain user-configured provenance.

#### Scenario: Browse supported providers
- **WHEN** a user starts adding an OnePiece configuration
- **THEN** the settings surface SHALL show the shared searchable versioned provider directory and a separate custom local/private endpoint action
- **AND** catalog endpoint records SHALL remain immutable and provenance-bearing

#### Scenario: Choose among provider endpoints
- **WHEN** a selected catalog provider exposes more than one endpoint type supported by the OnePiece runtime
- **THEN** the settings surface SHALL require or default an explicit endpoint selection and show the endpoint protocol and immutable Base URL
- **AND** saving SHALL persist the selected endpoint type with the Profile

#### Scenario: Reject an unavailable provider protocol
- **WHEN** a selected catalog provider does not publish the requested Anthropic or OpenAI endpoint, or the runtime does not support it
- **THEN** settings and application boundaries SHALL reject the selection without synthesizing a URL, storing a credential, or changing the active Profile

#### Scenario: Show unconfigured OnePiece
- **WHEN** OnePiece has no complete endpoint Profile
- **THEN** registry and settings surfaces SHALL still show OnePiece with an actionable non-selectable readiness state

#### Scenario: Add the first provider Profile
- **WHEN** a user selects a supported provider endpoint and supplies a Profile name, model id, and required credential while no OnePiece Profile exists
- **THEN** the system SHALL persist the non-secret Profile and automatically make it active
- **AND** it SHALL resolve catalog-owned fields from the directory and secure the credential through the existing credential boundary

#### Scenario: Add the first custom local Profile
- **WHEN** a user supplies a Profile name, validated Base URL, OpenAI-compatible interface, model id, authentication mode, timeout, privacy classification, capabilities, and context metadata while no Profile exists
- **THEN** the system SHALL persist the Profile with configured provenance and automatically make it active only when structurally ready
- **AND** optional absent authentication SHALL remain absent rather than becoming a placeholder key

#### Scenario: Add another provider Profile
- **WHEN** one or more OnePiece Profiles already exist and the user adds a catalog or custom endpoint
- **THEN** the new valid Profile SHALL be saved without replacing or activating over the current Profile

#### Scenario: Review multiple provider Profiles
- **WHEN** OnePiece has saved provider Profiles
- **THEN** the settings surface SHALL show Profile name, Local or Private label where justified, provider/runtime kind, interface, model, endpoint origin, authentication presence, timeout, privacy, capabilities, context provenance, readiness, and active state
- **AND** it SHALL NOT describe a Profile as secure merely because it is local

#### Scenario: Edit a provider Profile
- **WHEN** a user edits a saved OnePiece Profile
- **THEN** the system SHALL preserve its id and stored credential unless a replacement or authentication-mode change is submitted
- **AND** catalog-owned endpoint fields SHALL remain immutable while custom configured fields may be validated and changed
- **AND** editing an active Profile SHALL update future runtime snapshots without changing stable Agent id `onepiece`

#### Scenario: Reject an unknown or forged provider selection
- **WHEN** a caller provides provider, interface, or Base URL values outside a selected catalog endpoint contract
- **THEN** the application SHALL reject the request without storing credentials, contacting the submitted endpoint, or changing the active Profile

#### Scenario: Reject an unsafe custom endpoint
- **WHEN** a caller submits credentials in a URL, an unsupported scheme, an empty host, invalid timeout or context bounds, or inconsistent capabilities
- **THEN** the application SHALL reject the request before persistence or network contact

#### Scenario: Activate a provider Profile
- **WHEN** a user confirms activation of a structurally ready inactive Profile
- **THEN** the system SHALL make it the only active Profile for direct generation
- **AND** it SHALL preserve the selected Agent, current Session, and stable Agent id
- **AND** an in-flight generation SHALL retain its starting Profile snapshot

#### Scenario: Reject activation without a credential
- **WHEN** a Profile requires a credential that is absent or has incomplete structural configuration
- **THEN** activation SHALL fail without changing the current active Profile or contacting the provider

#### Scenario: Delete an inactive provider Profile
- **WHEN** a user confirms deletion of an inactive Profile
- **THEN** the system SHALL remove that Profile, its scoped credential, and routing references to it
- **AND** it SHALL leave the active runtime configuration unchanged

#### Scenario: Delete the active provider Profile
- **WHEN** a user confirms deletion of the active Profile
- **THEN** the system SHALL remove it and its scoped credential, clear direct active projection, disable rules that require it, and leave remaining Profiles inactive

#### Scenario: Use OnePiece provider setup on a narrow viewport
- **WHEN** the configuration and routing surfaces render at a narrow supported viewport
- **THEN** controls, Profile summaries, rule rows, status, and dialogs SHALL remain usable without horizontal page overflow

#### Scenario: Remove all OnePiece configuration
- **WHEN** a caller confirms the remove-all compatibility operation
- **THEN** the system SHALL delete all Profiles, credentials, and Hybrid rules, clear active runtime fields, and disable automatic tool approval
- **AND** it SHALL preserve OnePiece identity, sessions, Skill bindings, memories, usage, and Loop references

#### Scenario: Migrate a legacy single provider binding
- **WHEN** migration reads a catalog-backed or legacy OnePiece Profile created before Hybrid metadata existed
- **THEN** it SHALL preserve id, selection, provider fields, model, endpoint, credential mapping, and active state
- **AND** it SHALL assign conservative compatibility defaults without classifying an arbitrary endpoint as local or verified

## ADDED Requirements

### Requirement: OnePiece Profile operations preserve adapter parity
Desktop and Web/mock adapters SHALL expose contract-compatible Profile, discovery, verification, capability, privacy, and routing states; Web/mock SHALL simulate them without SQLite, credential-store, or network access.

#### Scenario: Configure Hybrid Routing in Web mode
- **WHEN** a user creates Profiles and rules through the Web/mock settings surface
- **THEN** deterministic in-memory state SHALL produce the same frontend contract and validation semantics as desktop

### Requirement: OnePiece uses routed Profiles through the shared API gateway
OnePiece SHALL execute a selected local, private, or cloud Profile through the existing API generation gateway and MUST NOT add endpoint-product-specific generation branches.

#### Scenario: Complete a local text turn
- **WHEN** routing selects a ready OpenAI-compatible localhost Profile that supports text generation
- **THEN** OnePiece SHALL stream the turn through the shared API request/event path
- **AND** endpoint-product identity SHALL not alter generic request orchestration
