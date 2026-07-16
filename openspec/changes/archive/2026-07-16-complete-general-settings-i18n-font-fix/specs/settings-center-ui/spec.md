## ADDED Requirements

### Requirement: Service-backed basic configuration
The Basic Configuration page SHALL render common application settings through the shared settings provider and frontend service boundary.

#### Scenario: Display common settings controls
- **WHEN** a user opens the Basic Configuration page
- **THEN** the page SHALL display controls for application language, font size, visual theme, default folder path, and read-only Node.js environment information

#### Scenario: Update common setting
- **WHEN** a user changes language, font size, visual theme, or default folder path from the Basic Configuration page
- **THEN** the page SHALL save the setting through the shared settings provider without directly calling a Tauri command

#### Scenario: Preserve settings page layout
- **WHEN** Basic Configuration renders common settings controls
- **THEN** the page SHALL use the shared settings center layout, semantic design tokens, controls, and internal scrolling behavior

### Requirement: Localized settings center text
The settings center SHALL render user-visible text through synchronized zh-CN and en translation resources.

#### Scenario: Render Chinese language
- **WHEN** the active application language is Chinese
- **THEN** settings center pages SHALL render extracted zh-CN translation values instead of hard-coded Chinese literals

#### Scenario: Render English language
- **WHEN** the active application language is English
- **THEN** settings center pages SHALL render corresponding en translation values for the same translation keys

#### Scenario: Translation resources stay aligned
- **WHEN** a translation key is added for settings center or related application surfaces
- **THEN** the zh-CN and en translation resources SHALL contain matching keys
