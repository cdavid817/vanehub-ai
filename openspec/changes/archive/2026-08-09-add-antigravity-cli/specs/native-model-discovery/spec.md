## ADDED Requirements

### Requirement: Discover model from Antigravity CLI native config
The system SHALL read the active model from Antigravity CLI's settings document at `~/.gemini/antigravity-cli/settings.json` when building session chat configuration defaults for the `antigravity-cli` agent.

#### Scenario: Model found in settings document
- **WHEN** `~/.gemini/antigravity-cli/settings.json` exists and records a model value
- **THEN** that model SHALL be used as the session's initial model

#### Scenario: Settings document has no model key
- **WHEN** `~/.gemini/antigravity-cli/settings.json` exists but records no model value
- **THEN** the system SHALL fall back to the VaneHub CLI profile default model for `antigravity-cli`

#### Scenario: Settings document absent
- **WHEN** `~/.gemini/antigravity-cli/settings.json` does not exist
- **THEN** the system SHALL fall back to the VaneHub CLI profile default model for `antigravity-cli`

#### Scenario: Settings document is malformed
- **WHEN** `~/.gemini/antigravity-cli/settings.json` exists but contains invalid JSON
- **THEN** the system SHALL log a diagnostic warning and fall back to the VaneHub CLI profile default model for `antigravity-cli`
- **AND** it SHALL NOT modify the file
