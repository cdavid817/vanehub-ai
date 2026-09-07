## MODIFIED Requirements

### Requirement: Supported CLI Agent configuration profiles
The system SHALL manage user-level global configuration profiles for the stable Agent ids `claude-code`, `opencode`, `codex-cli`, `antigravity-cli`, `gemini-cli`, `qwen-code`, and `iflow-cli`, and SHALL reject profile operations for unsupported Agent ids without reading or writing a CLI configuration file. A CLI whose program offers no user-configurable third-party endpoint (`kimi-cli`, `qoder-cli`, `codebuddy-code`, `copilot-cli`, `cursor-agent-cli`) SHALL remain unsupported rather than receive a profile kind that has nothing to project.

#### Scenario: List profiles for a supported Agent
- **WHEN** a user opens global configuration management for Claude Code, OpenCode, Codex CLI, Antigravity CLI, Gemini CLI, Qwen Code, or iFlow
- **THEN** the system SHALL return only profiles belonging to that stable Agent id
- **AND** each profile SHALL include a stable profile id, display name, validation state, credential-presence state, and applied-state metadata

#### Scenario: Unsupported Agent requested
- **WHEN** a profile operation targets an API Agent, an unknown Agent id, or one of the CLIs without a third-party endpoint surface
- **THEN** the system SHALL reject the operation before accessing any global CLI configuration file

## ADDED Requirements

### Requirement: Qwen Code global configuration profiles
The Agent Configuration system SHALL support stable Agent id `qwen-code` with profiles that manage its OpenAI-compatible endpoint, model, authentication strategy, and advanced environment values in the user-level `~/.qwen/.env`, and SHALL select the matching authentication type in `~/.qwen/settings.json` when the profile relies on an API key, while preserving unrelated user configuration in both files.

#### Scenario: Apply a Qwen Code API-key profile
- **WHEN** the user applies a Qwen Code profile that uses an API key
- **THEN** the native projection SHALL write `OPENAI_API_KEY`, `OPENAI_BASE_URL`, and `OPENAI_MODEL` to `~/.qwen/.env` from the stored credential and profile values
- **AND** SHALL set `security.auth.selectedType` to `openai` in `~/.qwen/settings.json` so an earlier OAuth selection cannot override the endpoint
- **AND** every other `.env` key and settings value SHALL be preserved, and a failure on the second file SHALL restore the first

#### Scenario: Preserve official Qwen authentication
- **WHEN** the user applies a Qwen Code profile using the preserve-official authentication strategy
- **THEN** VaneHub SHALL write only the managed endpoint, model, and advanced values to `.env`, SHALL remove any managed API key it previously wrote, and SHALL NOT touch `settings.json`

#### Scenario: Discover the current Qwen Code configuration
- **WHEN** the user requests discovery for Qwen Code
- **THEN** the system SHALL read `~/.qwen/.env` without launching Qwen Code, SHALL offer its endpoint and model for import, and SHALL report the presence of a key without exposing it

#### Scenario: Validate a Qwen Code credential
- **WHEN** the user validates an API-key profile's credential
- **THEN** the system SHALL probe the profile endpoint through the OpenAI chat-completions protocol with bearer authentication
- **AND** a preserve-official profile SHALL be reported as having no key to verify

### Requirement: iFlow custom API configuration profiles
The Agent Configuration system SHALL support stable Agent id `iflow-cli` with profiles that manage its OpenAI-compatible custom API endpoint, model, and advanced settings in `~/.iflow/settings.json`. Because iFlow reads its authentication type and key only from that file, applying a profile SHALL materialize the stored credential into it, and every profile SHALL require a credential.

#### Scenario: Apply an iFlow profile
- **WHEN** the user applies a valid iFlow profile with a stored credential
- **THEN** the native projection SHALL set `selectedAuthType` to `openai-compatible` and write `apiKey`, `baseUrl`, and `modelName` at the document root
- **AND** SHALL preserve every unmanaged key, including identifiers iFlow wrote for itself

#### Scenario: Apply an iFlow profile without a stored credential
- **WHEN** an iFlow profile's credential-store entry is missing
- **THEN** the system SHALL reject application before changing the file and SHALL mark the profile as requiring credential repair

#### Scenario: Discover and import the current iFlow configuration
- **WHEN** the user requests discovery for iFlow
- **THEN** the system SHALL read `~/.iflow/settings.json` without launching iFlow, SHALL offer its endpoint and model for import, and SHALL copy a present `apiKey` into the credential store rather than into the profile payload

#### Scenario: Use Qwen Code and iFlow profiles in Web mode
- **WHEN** the same profile workflow runs in Web/mock mode for either Agent
- **THEN** list, create, edit, duplicate, validate, apply, import, delete, and status behavior SHALL remain available without native filesystem access and without claiming to write a CLI file
