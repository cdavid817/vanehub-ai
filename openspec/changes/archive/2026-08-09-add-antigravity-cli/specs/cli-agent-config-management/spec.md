## ADDED Requirements

### Requirement: Antigravity CLI settings profile kind
The system SHALL provide an `antigravity` configuration profile kind for the stable Agent id `antigravity-cli` that manages the settings Antigravity CLI itself honors — tool permission mode, terminal sandbox enablement, verbosity, and default model — plus pass-through preservation of settings keys the profile does not model. Applying such a profile SHALL write to the user-level Antigravity settings document at `~/.gemini/antigravity-cli/settings.json` and SHALL preserve unrelated keys in that document.

#### Scenario: Apply an Antigravity profile
- **WHEN** a user applies an `antigravity` profile
- **THEN** the system SHALL write the profile's modelled settings into `~/.gemini/antigravity-cli/settings.json`
- **AND** keys present in the document that the profile does not model SHALL be preserved unchanged

#### Scenario: Unmodelled keys survive a round trip
- **WHEN** a profile is imported from an existing settings document containing keys the profile kind does not model
- **THEN** those keys SHALL be retained on the profile and rewritten on the next apply

#### Scenario: Malformed settings document is reported, not overwritten
- **WHEN** `~/.gemini/antigravity-cli/settings.json` exists but is not parseable JSON
- **THEN** the system SHALL report the malformed drift state for `antigravity-cli`
- **AND** it SHALL NOT modify the existing file

### Requirement: Credential-free configuration profile kinds
The system SHALL allow a configuration profile kind to declare that it carries no credential and supports no provider-endpoint override, and SHALL derive credential capture, credential validation, and the credential-related validation state from that declaration rather than from the Agent id. For a credential-free kind, credential presence SHALL always report false and the `needs-credential` validation state SHALL be unreachable.

#### Scenario: Antigravity profiles expose no credential controls
- **WHEN** a user opens profile creation or editing for `antigravity-cli`
- **THEN** the system SHALL NOT present a credential field or a credential-validation action
- **AND** the resulting profile SHALL report credential presence as false

#### Scenario: Credential-free profiles never enter needs-credential
- **WHEN** validation runs for a profile whose kind is credential-free
- **THEN** the resulting validation state SHALL be `valid` or `invalid`
- **AND** it SHALL NOT be `needs-credential`

#### Scenario: Credential submission for a credential-free kind is rejected
- **WHEN** a save or validate request supplies a credential for a credential-free profile kind
- **THEN** the system SHALL reject the request without persisting a credential and without writing any CLI configuration file

### Requirement: Provider-endpoint switching is not offered for Antigravity CLI
The system SHALL NOT offer provider-endpoint, base-URL, or authentication-strategy configuration for `antigravity-cli`, because Antigravity CLI authenticates through the operating system keyring with Google Sign-In and speaks a Google-proprietary protocol that a third-party relay endpoint cannot serve.

#### Scenario: No preset offers an endpoint for Antigravity
- **WHEN** a user opens profile creation for `antigravity-cli`
- **THEN** the system SHALL NOT present relay or custom-provider presets that declare a base URL or authentication strategy for that Agent id

## MODIFIED Requirements

### Requirement: Supported CLI Agent configuration profiles
The system SHALL manage user-level global configuration profiles for the stable Agent ids `claude-code`, `opencode`, `codex-cli`, and `antigravity-cli`, and SHALL reject profile operations for unsupported Agent ids without reading or writing a CLI configuration file.

#### Scenario: List profiles for a supported Agent
- **WHEN** a user opens global configuration management for Claude Code, OpenCode, Codex CLI, or Antigravity CLI
- **THEN** the system SHALL return only profiles belonging to that stable Agent id
- **AND** each profile SHALL include a stable profile id, display name, validation state, credential-presence state, and applied-state metadata

#### Scenario: Unsupported Agent requested
- **WHEN** a profile operation targets `gemini-cli`, an API Agent, or an unknown Agent id
- **THEN** the system SHALL reject the operation before accessing any global CLI configuration file
