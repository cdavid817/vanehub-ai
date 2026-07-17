# settings-extension-management-ui Specification

## Purpose
TBD - created by archiving change split-settings-center-ui-spec. Update Purpose after archive.
## Requirements
### Requirement: Extension Capabilities settings page
The settings center SHALL provide a service-backed Extension Capabilities page after SDK Dependencies for managing OCR, ASR, and TTS frameworks.

#### Scenario: Navigate to Extension Capabilities
- **WHEN** the settings sidebar renders
- **THEN** it SHALL include a localized Extension Capabilities entry after SDK Dependencies and before MCP Servers

#### Scenario: Display capability overview
- **WHEN** the Extension Capabilities page loads
- **THEN** it SHALL show localized summary counts and grouped OCR, ASR, and TTS framework cards from the extension service rather than hard-coded page data

#### Scenario: Search extensions
- **WHEN** the user enters a settings search term on the Extension Capabilities page
- **THEN** the visible framework cards SHALL be filtered by localized capability, framework, description, requirement, and status text

### Requirement: Extension lifecycle controls and feedback
The Extension Capabilities page SHALL provide compatibility-aware install, enable, start, stop, self-test, and uninstall controls with card-local progress and logs.

#### Scenario: Native operation is running
- **WHEN** an extension operation task is queued or running
- **THEN** the affected card SHALL display its current status and logs while unrelated cards and settings navigation remain interactive

#### Scenario: Web runtime limitation
- **WHEN** the page runs through the Web/mock adapter
- **THEN** it SHALL display a localized desktop-only notice and SHALL not imply that mock frameworks are installed on the host

### Requirement: Extension visual-style consistency
The Extension Capabilities page SHALL use shared settings layout components and semantic design tokens without branching on theme names.

#### Scenario: Render both registered styles
- **WHEN** either `futuristic` or `minimal` is active
- **THEN** extension cards, status badges, dialogs, logs, buttons, focus states, and empty/error states SHALL remain readable and visually consistent with the rest of the settings center

### Requirement: Localized extension text
All Extension Capabilities user-visible text SHALL use synchronized Simplified Chinese and English translation resources.

#### Scenario: Switch application language
- **WHEN** the active application language changes between `zh-CN` and `en`
- **THEN** navigation, headings, descriptions, requirements, statuses, actions, confirmations, notices, and errors on the extension page SHALL render in the active locale

#### Scenario: Maintain translation parity
- **WHEN** extension translation keys are added or changed
- **THEN** the existing i18n resource parity check SHALL require matching keys in both locale files
