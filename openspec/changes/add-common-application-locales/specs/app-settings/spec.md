## MODIFIED Requirements

### Requirement: Common settings model
The system SHALL manage common application settings for application language, font size, visual theme, and default folder path through a shared settings model.

#### Scenario: Load default settings
- **WHEN** no persisted common settings exist
- **THEN** the system SHALL provide valid defaults for language, font size, visual theme, and default folder path

#### Scenario: Accept a supported language setting
- **WHEN** the application-language value is `zh-CN`, `en`, `zh-TW`, `ja`, or `ko`
- **THEN** both desktop and Web/mock settings implementations SHALL accept and preserve the canonical locale id

#### Scenario: Reject invalid setting value
- **WHEN** a setting value is outside the supported values for its setting key
- **THEN** the system SHALL reject the value before applying it to the application UI

### Requirement: Settings side effects
The system SHALL apply common settings through centralized side effects owned by the settings provider and native settings layer.

#### Scenario: Apply language setting
- **WHEN** the application language changes to any supported locale
- **THEN** the settings provider SHALL load and synchronize the active i18next language with the selected value
- **AND** the desktop native settings layer SHALL refresh persistent framework-owned native copy when running under Tauri

#### Scenario: Apply font size setting
- **WHEN** the font size setting changes to 12px, 14px, 16px, or 18px
- **THEN** the system SHALL set the root `html` font size so rem-based Tailwind sizing scales with the selected value

#### Scenario: Apply visual theme setting
- **WHEN** the visual theme setting changes between futuristic and minimal styles
- **THEN** the system SHALL update the document theme attribute used by CSS variable groups

### Requirement: Settings persistence
The system SHALL persist common settings through the active runtime adapter and SHALL complete initial settings hydration before displaying the formal application surface.

#### Scenario: Persist desktop setting
- **WHEN** the application runs in the Tauri desktop runtime and a user saves a common setting
- **THEN** the system SHALL persist the setting through a Tauri command backed by SQLite storage

#### Scenario: Persist Web setting
- **WHEN** the application runs in the browser Web runtime and a user saves a common setting
- **THEN** the system SHALL persist the setting through the Web adapter without requiring a Tauri command

#### Scenario: Restore saved settings
- **WHEN** the application starts after common settings have been saved
- **THEN** the system SHALL restore and apply the saved setting values for the active runtime
- **AND** the formal application surface SHALL first become visible with the restored root font size, visual theme, and supported application language resource already applied

#### Scenario: Fall back when initial settings cannot be loaded
- **WHEN** the active runtime fails to load common settings or its persisted supported language resource during application startup
- **THEN** the system SHALL apply the shared default settings before displaying the formal application surface
- **AND** the settings provider SHALL retain a localized user-displayable error without preventing startup
