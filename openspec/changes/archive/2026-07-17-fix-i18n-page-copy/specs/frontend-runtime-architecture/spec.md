## ADDED Requirements

### Requirement: Application-wide frontend localization
The frontend SHALL render user-visible application text through synchronized Simplified Chinese and English i18n resources.

#### Scenario: Render active language across frontend surfaces
- **WHEN** the application language is set to Simplified Chinese or English
- **THEN** page titles, descriptions, actions, placeholders, status labels, notices, confirmations, modal labels, empty states, and user-facing frontend errors SHALL render from the active locale resource
- **AND** React page components SHALL NOT hard-code those user-visible strings outside the i18n resources

#### Scenario: Preserve stable identifiers
- **WHEN** the UI displays provider names, Agent display names, package names, executable names, file paths, command strings, protocol names, model names, or stable ids
- **THEN** the frontend MAY render those identifiers literally without translating them

#### Scenario: Keep locale resources aligned
- **WHEN** a frontend translation key is added, removed, or renamed
- **THEN** the zh-CN and en locale resources SHALL contain the same key set
- **AND** automated tests SHALL fail when the key sets diverge

#### Scenario: Format dates with active language
- **WHEN** frontend code formats user-visible dates or times
- **THEN** it SHALL use the active i18n language or an explicit locale derived from it rather than a hard-coded locale unrelated to the active language

### Requirement: Project i18n development contract
The project standards SHALL require all future frontend page changes to support Simplified Chinese and English localization.

#### Scenario: Add or change user-visible page text
- **WHEN** a developer adds or changes user-visible text in a React page, shared UI module, dialog, or frontend-owned service message
- **THEN** the change SHALL add or update both zh-CN and en translation values
- **AND** the implementation SHALL keep translation parity and hard-coded text guardrail tests passing

#### Scenario: Document i18n standard
- **WHEN** project standards are updated for frontend development rules
- **THEN** they SHALL state that new page/UI changes must use i18n resources for both zh-CN and en rather than hard-coded user-visible copy
