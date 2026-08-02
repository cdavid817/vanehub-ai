## MODIFIED Requirements

### Requirement: Service-backed basic configuration
The Basic Configuration page SHALL render common application settings through the shared settings provider and frontend service boundary.

#### Scenario: Display common settings controls
- **WHEN** a user opens the Basic Configuration page
- **THEN** the page SHALL display controls for application language, font size, visual theme, default folder path, and read-only Node.js environment information

#### Scenario: Display every supported application locale
- **WHEN** the application-language control renders
- **THEN** it SHALL present `zh-CN`, `en`, `zh-TW`, `ja`, and `ko` from the supported-locale registry in deterministic order
- **AND** each option SHALL have a recognizable localized label rather than a binary Chinese-or-English fallback label

#### Scenario: Update common setting
- **WHEN** a user changes language, font size, visual theme, or default folder path from the Basic Configuration page
- **THEN** the page SHALL save the setting through the shared settings provider without directly calling a Tauri command

#### Scenario: Preserve settings page layout
- **WHEN** Basic Configuration renders common settings controls in any supported locale
- **THEN** the page SHALL use the shared settings center layout, semantic design tokens, controls, and internal scrolling behavior
- **AND** the language selector SHALL remain readable and operable at desktop and narrow viewport widths
