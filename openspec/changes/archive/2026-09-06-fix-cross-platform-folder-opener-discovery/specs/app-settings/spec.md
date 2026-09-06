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

#### Scenario: Reject a default folder path that does not exist
- **WHEN** a non-empty default folder path is saved in the desktop runtime and it is not an existing directory
- **THEN** the native settings layer SHALL refuse the save with a concise error
- **AND** an empty value SHALL remain accepted as "no default"

#### Scenario: Offer a native directory picker
- **WHEN** the settings service runs in the desktop runtime
- **THEN** it SHALL expose a directory-picker operation that resolves to the chosen directory or to nothing when the user cancels
- **AND** the Web/mock adapter SHALL expose the same operation and reject it with a localized desktop-only message

### Requirement: Launch-on-startup application setting
The system SHALL include launch-on-startup in common application settings and apply it through centralized settings side effects.

#### Scenario: Load startup setting
- **WHEN** application settings are loaded
- **THEN** the settings service SHALL return a boolean launch-on-startup value with a safe default of disabled

#### Scenario: Save startup setting
- **WHEN** the launch-on-startup setting is saved
- **THEN** the settings service SHALL validate and persist the boolean value
- **AND** desktop runtime side effects SHALL remain owned by the settings/native layer

#### Scenario: Preserve Web mock parity
- **WHEN** app settings are loaded or saved in the Web/mock runtime
- **THEN** the Web adapter SHALL preserve the launch-on-startup key shape without claiming native startup registration is active

#### Scenario: Report startup capability explicitly
- **WHEN** application settings are loaded
- **THEN** the response SHALL carry a read-only `launchOnStartupAvailable` flag that is true from the native runtime and false from the Web/mock adapter
- **AND** the UI SHALL gate the startup control on that flag rather than on an unrelated capability such as whether a directory can be opened
