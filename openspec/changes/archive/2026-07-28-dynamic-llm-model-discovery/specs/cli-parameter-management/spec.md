## MODIFIED Requirements

### Requirement: Typed and documented parameter catalog
Every exposed CLI parameter SHALL be defined by a backend-authoritative catalog entry with a stable parameter id, literal provider flag, control kind, localized name key, localized detailed-description key, default value, launch scope, risk classification, and allowed values when applicable. The `model` parameter for all four managed CLIs SHALL use a composite control that presents known catalog values in a dropdown alongside a free-text field for arbitrary model identifiers.

#### Scenario: Render composite model parameter
- **WHEN** a catalog entry has control kind `custom-text` with known enum values
- **THEN** the page SHALL render a dropdown containing the known catalog values plus a "Custom…" option
- **AND** when "Custom…" is selected, a free-text input SHALL appear for entering any model identifier

#### Scenario: Select known catalog model value
- **WHEN** a user selects a known catalog model value from the dropdown (e.g., "sonnet" for Claude Code)
- **THEN** the system SHALL behave identically to the current enum control

#### Scenario: Enter custom model value
- **WHEN** a user selects "Custom…" and enters `deepseek-chat` in the free-text field
- **THEN** the system SHALL save the value `deepseek-chat` as the model parameter selection

#### Scenario: Render enum parameter
- **WHEN** a catalog entry has control kind `enum`
- **THEN** the page SHALL render a single-select dropdown using only the catalog's allowed values
- **AND** it SHALL show the localized description for the selected value

#### Scenario: Render boolean parameter
- **WHEN** a catalog entry has control kind `boolean`
- **THEN** the page SHALL render an accessible switch that controls whether the mapped provider flag is effective

#### Scenario: Render repeatable enum parameter
- **WHEN** a catalog entry has control kind `multi-enum`
- **THEN** the page SHALL render a multi-select control using only the catalog's allowed values
- **AND** the service SHALL preserve the catalog-defined value order when producing arguments

## ADDED Requirements

### Requirement: Custom-text parameter control kind
The parameter catalog SHALL support a `custom-text` control kind that combines a dropdown of known values with an optional free-text input. This control kind SHALL be used for parameters where the provider accepts both known values and arbitrary identifiers.

#### Scenario: Validation accepts known enum values
- **WHEN** a `custom-text` parameter receives a value matching one of the known catalog entries
- **THEN** validation SHALL accept the value

#### Scenario: Validation accepts arbitrary non-empty values
- **WHEN** a `custom-text` parameter receives a value not in the known catalog entries
- **THEN** validation SHALL accept the value provided it is non-empty and contains no control characters

#### Scenario: Validation rejects control characters
- **WHEN** a `custom-text` parameter receives a value containing control characters (e.g., newlines, null bytes)
- **THEN** validation SHALL reject the value

#### Scenario: Validation rejects empty values
- **WHEN** a `custom-text` parameter receives an empty or whitespace-only string
- **THEN** validation SHALL reject the value or normalize it to the catalog default

### Requirement: All four managed CLIs use custom-text for model parameter
The `model` parameter for `claude-code`, `codex-cli`, `gemini-cli`, and `opencode` SHALL change from `Enum` control to `custom-text` control, preserving all existing known values as the dropdown options.

#### Scenario: Claude Code model parameter
- **WHEN** the `claude-code` model parameter catalog is loaded
- **THEN** SHALL have control kind `custom-text`
- **AND** SHALL present known values `default`, `sonnet`, `opus`, `haiku` in the dropdown
- **AND** SHALL accept arbitrary model identifiers via free-text input

#### Scenario: Codex CLI model parameter
- **WHEN** the `codex-cli` model parameter catalog is loaded
- **THEN** SHALL have control kind `custom-text`
- **AND** SHALL present known values `default`, `gpt-5.5`, `gpt-5.4`, `gpt-5.2-codex`, `gpt-5.1-codex-max` in the dropdown
- **AND** SHALL accept arbitrary model identifiers via free-text input

#### Scenario: Gemini CLI model parameter
- **WHEN** the `gemini-cli` model parameter catalog is loaded
- **THEN** SHALL have control kind `custom-text`
- **AND** SHALL present known values `default`, `gemini-2.5-pro`, `gemini-2.5-flash` in the dropdown
- **AND** SHALL accept arbitrary model identifiers via free-text input

#### Scenario: OpenCode model parameter (no model parameter currently)
- **WHEN** the `opencode` parameter catalog is loaded
- **THEN** the catalog SHALL remain as-is with agent/thinking/autoApprove parameters only
- **AND** OpenCode model discovery SHALL rely solely on native config file reading

### Requirement: Argument preview renders custom model values directly
When a `custom-text` parameter has a value that is not in the known enum list, the argument preview SHALL render it as-is in the CLI flag sequence.

#### Scenario: Custom model value in argument preview
- **WHEN** Claude Code has model value `deepseek-chat` and the preview is generated for the `chat` scope
- **THEN** the argument preview SHALL include `--model deepseek-chat`

#### Scenario: Known model value in argument preview (unchanged)
- **WHEN** Claude Code has model value `sonnet` and the preview is generated
- **THEN** the argument preview SHALL include `--model sonnet` as before

#### Scenario: Default model value omitted from preview (unchanged)
- **WHEN** any CLI has model value `default` and the preview is generated
- **THEN** the argument preview SHALL omit the `--model` flag entirely
