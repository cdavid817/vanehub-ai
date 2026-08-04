## ADDED Requirements

### Requirement: Stable built-in OnePiece identity
The system SHALL maintain OnePiece as a built-in Agent with stable id `onepiece`, display name `OnePiece`, `launch_kind = api`, `agent_origin = builtin`, API interaction support, and first-party capability metadata in both desktop and Web/mock registries.

#### Scenario: Initialize a clean registry
- **WHEN** VaneHub initializes a registry that has no `onepiece` row
- **THEN** the system SHALL seed the built-in OnePiece identity
- **AND** it SHALL keep the identity visible even though no provider configuration or credential exists yet

#### Scenario: Reopen an existing registry
- **WHEN** VaneHub starts against an existing database that does not contain OnePiece
- **THEN** idempotent registry seeding SHALL add OnePiece without modifying existing Agent, session, Skill, memory, usage, or Loop data

#### Scenario: Adopt an existing API Agent with the reserved id
- **WHEN** migration finds an existing `onepiece` row whose `launch_kind` is `api`
- **THEN** the system SHALL adopt that row as the built-in OnePiece identity in place
- **AND** it SHALL preserve its provider configuration, credential key, trust setting, sessions, messages, Skills, memories, usage records, and Loop references

#### Scenario: Reject an incompatible reserved-id collision
- **WHEN** migration finds an existing `onepiece` row whose launch kind is not `api`
- **THEN** initialization SHALL fail safely with a diagnostic identifying the incompatible reserved-id collision
- **AND** it SHALL NOT overwrite or delete the conflicting row

### Requirement: OnePiece configuration lifecycle
The system SHALL keep the OnePiece identity separate from multiple named catalog-backed upstream-provider Profiles, SHALL secure credentials independently per Profile, SHALL use at most one explicitly active Profile for runtime generation, and SHALL expose provider-directory, list, save, activate, delete, and remove-all operations through the shared frontend service boundary. New Profile creation SHALL select a reviewed endpoint type owned by the chosen provider and SHALL NOT accept an arbitrary provider identity, interface format, or Base URL from the user.

#### Scenario: Browse supported providers
- **WHEN** a user starts adding an OnePiece configuration
- **THEN** the settings surface SHALL show the shared searchable versioned 25-provider directory used by Agent configuration
- **AND** each provider SHALL supply its identity, reviewed endpoint records, default and fallback models, provider icon key, and safe documentation links
- **AND** each endpoint record SHALL supply its protocol type, interface format, immutable Base URL, authentication strategy, and model-discovery metadata
- **AND** the surface SHALL NOT offer a custom-provider action or editable Base URL field

#### Scenario: Choose among provider endpoints
- **WHEN** a selected provider exposes more than one endpoint type supported by the OnePiece runtime
- **THEN** the settings surface SHALL require or default an explicit endpoint selection and show the endpoint protocol and immutable Base URL
- **AND** saving SHALL persist the selected endpoint type with the Profile

#### Scenario: Reject an unavailable provider protocol
- **WHEN** a selected provider does not publish the requested Anthropic or OpenAI endpoint, or the OnePiece runtime does not support that endpoint type
- **THEN** the settings and application boundaries SHALL reject the selection without synthesizing a URL, storing a credential, or changing the active Profile

#### Scenario: Show unconfigured OnePiece
- **WHEN** OnePiece has no complete provider configuration or stored credential
- **THEN** registry and settings surfaces SHALL still show OnePiece with an actionable non-selectable readiness state

#### Scenario: Add the first provider Profile
- **WHEN** a user selects a supported provider endpoint and supplies a Profile name, model id, and API credential while no OnePiece Profile exists
- **THEN** the system SHALL persist the non-secret Profile and automatically make it active
- **AND** provider identity, endpoint type, interface format, and Base URL SHALL be resolved from the selected directory endpoint rather than user input
- **AND** it SHALL store the credential through the platform credential service without returning or persisting the raw secret in SQLite
- **AND** the OnePiece Agent row SHALL project the active Profile for existing runtime consumers

#### Scenario: Add another provider Profile
- **WHEN** one or more OnePiece Profiles already exist and the user activates “Add API provider”
- **THEN** the settings surface SHALL open an application-owned provider-catalog dialog instead of displaying a permanently expanded editor or custom endpoint form
- **AND** the new valid Profile SHALL be saved without replacing or activating over the current Profile

#### Scenario: Review multiple provider Profiles
- **WHEN** OnePiece has saved provider Profiles
- **THEN** the settings surface SHALL show one summary card per Profile with Profile name, provider, endpoint protocol, interface format, model, endpoint URL, credential-presence, readiness, and active-state metadata
- **AND** the active Profile SHALL have persistent visual emphasis and an explicit active label

#### Scenario: Edit a provider Profile
- **WHEN** a user edits a saved OnePiece Profile
- **THEN** the system SHALL preserve the Profile id and any stored credential unless a replacement credential is submitted
- **AND** it SHALL preserve the catalog provider identity, endpoint type, interface format, and resolved Base URL while allowing Profile name and model changes
- **AND** the dialog SHALL NOT repopulate the stored credential
- **AND** editing an active Profile SHALL update the runtime projection without changing stable Agent id `onepiece`

#### Scenario: Reject an unknown or forged provider selection
- **WHEN** a caller saves a new Profile with an unknown provider id or endpoint type, or attempts to provide provider/interface/Base URL values outside the catalog contract
- **THEN** the application boundary SHALL reject the request without storing credentials or changing the active Profile
- **AND** it SHALL NOT contact the submitted endpoint

#### Scenario: Activate a provider Profile
- **WHEN** a user confirms activation of a ready inactive Profile
- **THEN** the system SHALL make it the only active Profile and use its configuration and credential for subsequent OnePiece generations
- **AND** it SHALL NOT change the selected Agent, current Session, or stable Agent id
- **AND** an in-flight generation SHALL retain the configuration snapshot captured when that generation started

#### Scenario: Reject activation without a credential
- **WHEN** a user attempts to activate a Profile that has no stored credential or incomplete structural configuration
- **THEN** the system SHALL reject activation without changing the current active Profile
- **AND** it SHALL NOT contact the provider

#### Scenario: Delete an inactive provider Profile
- **WHEN** a user confirms deletion of an inactive Profile
- **THEN** the system SHALL remove that Profile and its scoped credential
- **AND** it SHALL leave the active runtime configuration unchanged

#### Scenario: Delete the active provider Profile
- **WHEN** a user confirms deletion of the active Profile
- **THEN** the system SHALL remove that Profile and its scoped credential, clear the runtime provider projection and active runtime credential, and leave any remaining Profiles inactive
- **AND** OnePiece SHALL return to a visible non-selectable readiness state until another Profile is explicitly activated

#### Scenario: Use OnePiece provider setup on a narrow viewport
- **WHEN** the configuration surface renders at a narrow supported viewport
- **THEN** the toolbar, status, provider cards, and dialog actions SHALL remain usable without horizontal page overflow

#### Scenario: Remove all OnePiece configuration
- **WHEN** a caller confirms the remove-all compatibility operation
- **THEN** the system SHALL delete every provider Profile and scoped credential, clear the active runtime provider fields and credential, and disable automatic tool approval
- **AND** it SHALL preserve the OnePiece identity, sessions, Skill bindings, memories, usage, and Loop references
- **AND** OnePiece SHALL return to a visible non-selectable readiness state

#### Scenario: Migrate a legacy single provider binding
- **WHEN** migration finds a complete pre-Profile OnePiece provider binding
- **THEN** it SHALL create one deterministic active Profile that preserves its provider fields
- **AND** it SHALL associate provider and endpoint ids only when the provider/interface/endpoint exactly matches a supported endpoint record, otherwise retaining a non-creatable legacy source
- **AND** the Profile-aware service SHALL preserve the existing runtime credential into the Profile-scoped credential account before the first provider switch

### Requirement: OnePiece provider model discovery
The system SHALL discover selectable OnePiece chat models through a shared service contract using only the selected catalog endpoint's model-list URL and authentication strategy, SHALL provide reviewed catalog models when live discovery is unavailable, and SHALL NOT require or permit free-text model identifiers in the OnePiece configuration UI.

#### Scenario: Discover models for a new Profile
- **WHEN** a user selects a catalog provider endpoint, enters a new API credential, and requests its models
- **THEN** the desktop adapter SHALL submit the provider id, endpoint type, and transient credential through the service boundary
- **AND** the native runtime SHALL contact only that endpoint record's compiled HTTPS model-list URL with its declared authentication strategy
- **AND** the raw credential, request headers, and provider response body SHALL NOT be persisted, returned, or written to logs

#### Scenario: Refresh models for an existing Profile
- **WHEN** a user requests models for an existing Profile without entering a replacement credential
- **THEN** the native runtime SHALL use that Profile's scoped stored credential
- **AND** it SHALL NOT reveal the stored credential to the frontend

#### Scenario: Normalize discovered models
- **WHEN** a supported provider returns a model list
- **THEN** the system SHALL trim and deduplicate identifiers, exclude known non-chat model types, merge reviewed catalog fallback models, and return deterministic searchable model options
- **AND** it SHALL NOT infer unsupported OnePiece tool or reasoning capabilities solely from an unknown model identifier

#### Scenario: Degrade when discovery fails
- **WHEN** a provider model-list request times out, is rejected, or returns an invalid response
- **THEN** the service SHALL return the preset's reviewed fallback models with a safe warning when fallback models exist
- **AND** unified logs SHALL contain only safe provider id, duration/count, and error-category metadata

#### Scenario: Preserve a historical selection
- **WHEN** an existing Profile's model is absent from the current live and fallback lists
- **THEN** the model selector SHALL retain and label the historical value instead of silently replacing it

#### Scenario: Discover models in Web/mock mode
- **WHEN** the Web/mock adapter receives a model-discovery request with valid mock input
- **THEN** it SHALL return deterministic models from the shared catalog contract without contacting a provider or retaining the submitted credential

### Requirement: OnePiece API-key verification
The system SHALL expose the shared API-credential verification capability for every catalog-backed OnePiece Profile using only the selected provider directory endpoint, selected model, and either a transient replacement credential or the Profile-scoped stored credential.

#### Scenario: Verify a new OnePiece credential
- **WHEN** a user selects a supported OnePiece provider endpoint and model, enters an API key in the add dialog, and requests verification
- **THEN** the frontend SHALL submit the provider id, endpoint type, model id, and transient credential through `AgentService`
- **AND** the native application SHALL resolve the immutable endpoint from the compiled directory and SHALL NOT persist the transient credential

#### Scenario: Verify an edited OnePiece Profile
- **WHEN** a user requests verification while editing an existing OnePiece Profile
- **THEN** a non-empty transient replacement credential SHALL take precedence for that check
- **AND** when no replacement is entered the native application SHALL use the Profile-scoped stored credential without revealing it to the frontend

#### Scenario: Verify from a OnePiece Profile card
- **WHEN** a saved OnePiece Profile with a configured credential is displayed in Agent configuration
- **THEN** its card SHALL provide a verification action that uses the saved provider endpoint, model, and scoped credential
- **AND** the result SHALL be displayed as ephemeral Profile feedback without activating the Profile or changing runtime configuration

#### Scenario: Reject a forged OnePiece verification target
- **WHEN** a caller requests verification with an unknown provider id, an endpoint type not owned by that provider, a mismatched Profile id, or a model that fails the existing model-id rules
- **THEN** the application SHALL reject the request before provider contact
- **AND** it SHALL NOT load or store a credential for the forged target

### Requirement: Versioned OnePiece core instructions
The system SHALL apply non-removable, versioned OnePiece core instructions to every OnePiece generation before optional Skill and memory sections, and the shipped core content MUST NOT exceed 8,000 Unicode characters.

#### Scenario: Generate without Skills or memories
- **WHEN** OnePiece starts a generation with no bound Skills and no scoped memories
- **THEN** the provider request SHALL still contain the complete OnePiece core-instruction section

#### Scenario: Trace the core version
- **WHEN** an OnePiece generation resolves its core instructions
- **THEN** safe generation diagnostics SHALL record the stable core-instruction version
- **AND** diagnostics SHALL NOT record the full core content

#### Scenario: Core asset exceeds its budget
- **WHEN** verification inspects a shipped OnePiece core-instruction asset longer than 8,000 Unicode characters
- **THEN** verification SHALL fail rather than truncating or shipping a partial identity prompt

#### Scenario: User manages Skills
- **WHEN** a user disables, unbinds, or deletes a Skill associated with OnePiece
- **THEN** the operation SHALL affect only the Skill section
- **AND** it SHALL NOT remove, edit, or disable OnePiece core instructions

### Requirement: Safe OnePiece tool defaults
The system SHALL initialize and reset OnePiece with automatic shell and file-write approval disabled and SHALL continue applying the existing MCP approval and plan-mode restrictions.

#### Scenario: First configuration retains approval prompts
- **WHEN** OnePiece is configured for the first time
- **THEN** shell and file-write calls SHALL require approval until the user explicitly enables the existing trust setting

#### Scenario: Trust does not bypass existing hard gates
- **WHEN** a trusted OnePiece requests an MCP tool or runs in plan mode
- **THEN** the existing MCP approval and plan-mode restrictions SHALL remain in force

### Requirement: OnePiece runtime and presentation parity
Desktop and Web/mock runtimes SHALL expose OnePiece through the same frontend service contracts, stable visual identity, configuration states, and session-selection behavior while differing only in native persistence, credential, and provider execution.

#### Scenario: Place OnePiece after CLI configuration tabs
- **WHEN** the Agent configuration page renders the Claude Code, OpenCode, Codex CLI, and OnePiece tabs
- **THEN** it SHALL place the three CLI tabs before OnePiece
- **AND** OnePiece SHALL be the final tab while remaining directly navigable

#### Scenario: Render OnePiece identity
- **WHEN** registry, settings, create-session, session-list, or session-detail UI renders stable id `onepiece`
- **THEN** it SHALL render the OnePiece display identity without persisting redundant icon metadata on sessions

#### Scenario: Configure OnePiece in Web/mock mode
- **WHEN** a user configures OnePiece in Web/mock mode with valid mock input
- **THEN** the Web adapter SHALL transition its in-memory readiness through the shared service contract
- **AND** it SHALL NOT access SQLite, the OS credential store, or a real provider

#### Scenario: Generate through the desktop runtime
- **WHEN** a ready OnePiece receives a message in the desktop runtime
- **THEN** the existing API process gateway SHALL execute the generation without a OnePiece-specific provider execution branch
