## ADDED Requirements

### Requirement: Complete localized settings pages
All settings center pages and settings-owned dialogs SHALL render user-visible text through synchronized zh-CN and en translation resources.

#### Scenario: Agents settings page localized
- **WHEN** the Agents settings page renders in Simplified Chinese or English
- **THEN** its title, description, refresh action, filter controls, mode labels, configuration details, launch action, session detail labels, notices, and empty or error states SHALL use the active locale

#### Scenario: SDK settings page localized
- **WHEN** the SDK Dependencies page renders in Simplified Chinese or English
- **THEN** its title, description, refresh and update actions, stat cards, section headings, SDK status labels, version labels, operation actions, confirmations, notices, errors, empty states, and operation log labels SHALL use the active locale

#### Scenario: MCP settings page localized
- **WHEN** the MCP Servers page and its forms or import/export dialogs render in Simplified Chinese or English
- **THEN** titles, descriptions, actions, stat cards, scope labels, group labels, form labels, placeholders, validation messages, confirmations, notices, empty states, and modal controls SHALL use the active locale

#### Scenario: Existing settings translations corrected
- **WHEN** settings center locale resources contain equivalent zh-CN and en keys
- **THEN** each pair SHALL describe the same product concept and action semantics
- **AND** terminology for Agent, Skill, CLI, SDK, MCP, workspace, session, install, update, rollback, upgrade, and downgrade SHALL remain consistent across settings pages

### Requirement: Settings i18n regression coverage
The system SHALL include regression coverage that prevents settings pages from introducing untranslated visible text.

#### Scenario: Detect untranslated settings literals
- **WHEN** automated frontend tests run
- **THEN** they SHALL verify locale key parity
- **AND** they SHALL detect hard-coded user-visible strings in settings page components except for approved stable identifiers
