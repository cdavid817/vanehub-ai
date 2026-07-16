# app-settings Specification

## Purpose
TBD - created by archiving change complete-general-settings-i18n-font-fix. Update Purpose after archive.
## Requirements
### Requirement: Common settings model
The system SHALL manage common application settings for application language, font size, visual theme, and default folder path through a shared settings model.

#### Scenario: Load default settings
- **WHEN** no persisted common settings exist
- **THEN** the system SHALL provide valid defaults for language, font size, visual theme, and default folder path

#### Scenario: Reject invalid setting value
- **WHEN** a setting value is outside the supported values for its setting key
- **THEN** the system SHALL reject the value before applying it to the application UI

### Requirement: Settings side effects
The system SHALL apply common settings through centralized side effects owned by the settings provider.

#### Scenario: Apply language setting
- **WHEN** the application language setting changes between Chinese and English
- **THEN** the system SHALL synchronize the active i18next language with the selected value

#### Scenario: Apply font size setting
- **WHEN** the font size setting changes to 12px, 14px, 16px, or 18px
- **THEN** the system SHALL set the root `html` font size so rem-based Tailwind sizing scales with the selected value

#### Scenario: Apply visual theme setting
- **WHEN** the visual theme setting changes between futuristic and minimal styles
- **THEN** the system SHALL update the document theme attribute used by CSS variable groups

### Requirement: Settings persistence
The system SHALL persist common settings through the active runtime adapter.

#### Scenario: Persist desktop setting
- **WHEN** the application runs in the Tauri desktop runtime and a user saves a common setting
- **THEN** the system SHALL persist the setting through a Tauri command backed by SQLite storage

#### Scenario: Persist Web setting
- **WHEN** the application runs in the browser Web runtime and a user saves a common setting
- **THEN** the system SHALL persist the setting through the Web adapter without requiring a Tauri command

#### Scenario: Restore saved settings
- **WHEN** the application starts after common settings have been saved
- **THEN** the system SHALL restore and apply the saved setting values for the active runtime

### Requirement: Node.js environment display
The system SHALL expose read-only Node.js environment information for the Basic Configuration page.

#### Scenario: Node.js is available
- **WHEN** the runtime can resolve a Node.js executable and version
- **THEN** the settings page SHALL display the resolved path and version as read-only information

#### Scenario: Node.js is unavailable
- **WHEN** the runtime cannot resolve a Node.js executable or version
- **THEN** the settings page SHALL display an unavailable read-only state without blocking other settings controls

