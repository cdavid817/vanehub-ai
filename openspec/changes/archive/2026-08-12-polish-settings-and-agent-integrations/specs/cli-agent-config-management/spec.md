## ADDED Requirements

### Requirement: Gemini CLI global configuration profiles
The Agent Configuration system SHALL support stable Agent id `gemini-cli` with profiles that manage Gemini API endpoint, model, authentication strategy, and advanced environment values while preserving unrelated user configuration.

#### Scenario: Configure Gemini with an API key
- **WHEN** the user saves and applies a Gemini CLI profile using API-key authentication
- **THEN** the credential SHALL be stored through the existing secret boundary
- **AND** the native projection SHALL materialize the selected endpoint and model using Gemini CLI-supported configuration values
- **AND** profile reads SHALL never return the credential value

#### Scenario: Preserve official Gemini authentication
- **WHEN** the user applies a Gemini CLI profile using the preserve-official authentication strategy
- **THEN** VaneHub SHALL update only its managed endpoint, model, and advanced values
- **AND** it SHALL preserve the user's existing Google sign-in authentication material

#### Scenario: Use Gemini profiles in Web mode
- **WHEN** the same profile workflow runs in Web/mock mode
- **THEN** list, create, edit, duplicate, validate, apply, import, delete, and status behavior SHALL remain available without native filesystem access

#### Scenario: Discover existing Gemini configuration
- **WHEN** the user requests discovery for Gemini CLI
- **THEN** the system SHALL inspect the supported global Gemini configuration locations without launching Gemini CLI
- **AND** it SHALL offer valid discovered values for import without exposing a discovered secret

