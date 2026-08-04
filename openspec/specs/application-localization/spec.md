# application-localization Specification

## Purpose

Define the supported application locale catalog, bundled frontend resource guarantees, locale-correct formatting, and native desktop localization behavior.

## Requirements

### Requirement: Supported application locale catalog
The application SHALL define one deterministic catalog of canonical BCP 47 locale ids for `zh-CN`, `en`, `zh-TW`, `ja`, and `ko`, and SHALL treat `zh-CN` as the default and fallback locale.

#### Scenario: Enumerate supported locales
- **WHEN** the frontend renders language choices or validates an application-language value
- **THEN** it SHALL use the registered locale ids `zh-CN`, `en`, `zh-TW`, `ja`, and `ko` in deterministic order
- **AND** each entry SHALL provide localized selector metadata and a bundled resource loader

#### Scenario: Preserve runtime locale parity
- **WHEN** either the Tauri desktop runtime or Web/mock runtime loads or saves application settings
- **THEN** it SHALL accept the same supported locale ids and reject values outside the catalog

#### Scenario: Resolve an unavailable locale
- **WHEN** a requested locale is unknown or its bundled resource cannot be loaded
- **THEN** the application SHALL use `zh-CN` without rendering raw translation keys or an empty formal surface
- **AND** SHALL retain localized user-displayable failure information when loading a persisted supported locale fails

### Requirement: Complete frontend translation resources
Every frontend-owned user-visible translation key SHALL have semantically equivalent values in each supported application locale.

#### Scenario: Render a supported locale
- **WHEN** a user selects `zh-CN`, `en`, `zh-TW`, `ja`, or `ko`
- **THEN** page titles, descriptions, navigation, controls, placeholders, statuses, notices, confirmations, dialogs, empty states, tooltips, accessibility labels, notifications, and frontend-owned errors SHALL render from that locale's resources

#### Scenario: Preserve literal data exceptions
- **WHEN** the UI displays product names, provider names, Agent or model identifiers, protocols, executables, packages, commands, file paths, URLs, log levels, stable ids, user content, or backend diagnostic data
- **THEN** those data values MAY remain literal while surrounding frontend-owned labels use the active locale

### Requirement: Local bundled resource loading
The frontend SHALL load supported translation resources from application-bundled assets and SHALL complete selected-locale activation before exposing the formal application surface.

#### Scenario: Restore an optional locale at startup
- **WHEN** settings hydration restores `en`, `zh-TW`, `ja`, or `ko`
- **THEN** the frontend SHALL load the corresponding local resource and activate it before rendering settings-dependent children
- **AND** SHALL NOT require a network translation service

#### Scenario: Switch language during a session
- **WHEN** the user selects another supported locale
- **THEN** the frontend SHALL finish loading that locale before committing the visible language change
- **AND** all mounted React surfaces SHALL observe the new active locale without a process restart

### Requirement: Translation resource integrity
Automated localization checks SHALL validate every registered locale resource against a canonical key and interpolation contract.

#### Scenario: Validate locale resource parity
- **WHEN** frontend tests inspect registered locale resources
- **THEN** every resource SHALL contain the same translation keys as the canonical English resource
- **AND** every matching key SHALL use the same interpolation variable names
- **AND** no raw resource file SHALL contain duplicate keys

#### Scenario: Validate registry and resources
- **WHEN** a supported locale is added, removed, or renamed
- **THEN** automated tests SHALL require its locale registry entry, resource file, TypeScript setting validation, and native supported-locale representation to remain aligned

### Requirement: Locale-correct plural and value formatting
Count-sensitive frontend messages and locale-sensitive dates, times, numbers, and percentages SHALL use the active locale's grammatical and formatting rules.

#### Scenario: Select a plural form
- **WHEN** a translation receives a count that changes grammatical number in the active locale
- **THEN** the caller SHALL provide a numeric `count`
- **AND** i18next SHALL select the locale-appropriate plural form without parenthetical pseudo-plurals

#### Scenario: Format locale-sensitive values
- **WHEN** the UI formats a date, time, number, or percentage
- **THEN** it SHALL use the active application locale or a documented locale derived from it

### Requirement: Native desktop localization
The Tauri desktop runtime SHALL resolve framework-owned native copy from the active supported application locale using the same deterministic fallback as the frontend.

#### Scenario: Initialize native localized copy
- **WHEN** the desktop runtime starts with a persisted supported locale
- **THEN** system-tray actions, the close-to-tray notice, and communications overload copy SHALL use that locale

#### Scenario: Refresh native copy after a language change
- **WHEN** the user saves a different supported application locale while the desktop runtime is running
- **THEN** persistent native controls SHALL update to the new locale without requiring an application restart
- **AND** later native notices and framework-owned communications messages SHALL use the new locale

#### Scenario: Preserve Web runtime separation
- **WHEN** the application runs through the Web/mock adapter
- **THEN** it SHALL apply the selected locale to the React application
- **AND** SHALL NOT claim to update system-tray, native-window, or background-connector copy
