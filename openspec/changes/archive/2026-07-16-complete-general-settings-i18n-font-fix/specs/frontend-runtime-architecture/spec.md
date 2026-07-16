## ADDED Requirements

### Requirement: Common settings adapter boundary
The frontend SHALL expose common settings operations through the service interface and runtime adapters rather than direct runtime calls from React components.

#### Scenario: Desktop common settings adapter
- **WHEN** the frontend runs inside the Tauri desktop runtime and common settings are loaded, saved, or inspected for Node.js information
- **THEN** the Tauri adapter SHALL call declared Tauri commands through the service boundary

#### Scenario: Web common settings adapter
- **WHEN** the frontend runs outside the Tauri desktop runtime and common settings are loaded, saved, or inspected for Node.js information
- **THEN** the Web adapter SHALL provide Web-compatible behavior without importing or invoking Tauri APIs

#### Scenario: Components use settings provider
- **WHEN** React components render or mutate common settings
- **THEN** they SHALL use the settings provider or frontend service interface instead of calling runtime-specific APIs directly

### Requirement: Global preference application
The frontend SHALL apply language, font-size, and theme settings at application scope in a runtime-independent way.

#### Scenario: Apply language globally
- **WHEN** the settings provider receives a valid language setting
- **THEN** it SHALL update i18next so all localized components use the selected language

#### Scenario: Apply root font size globally
- **WHEN** the settings provider receives a valid font size setting
- **THEN** it SHALL update the root document font size and SHALL NOT use CSS `zoom` for global scaling

#### Scenario: Apply theme globally
- **WHEN** the settings provider receives a valid visual theme setting
- **THEN** it SHALL update the document theme attribute used by shared CSS variables
