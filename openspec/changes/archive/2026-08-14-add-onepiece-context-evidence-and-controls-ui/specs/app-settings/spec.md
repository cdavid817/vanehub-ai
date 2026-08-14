## ADDED Requirements

### Requirement: Automatic context compaction application setting
The shared application settings model SHALL include a boolean automatic-context-compaction preference, default it to enabled, and persist it through the active settings adapter without a dedicated storage table.

#### Scenario: Existing installation has no saved preference
- **WHEN** settings are loaded without a saved automatic-context-compaction value
- **THEN** desktop and Web/mock runtimes SHALL return the preference as enabled

#### Scenario: Save desktop preference
- **WHEN** a user changes the preference in the desktop runtime
- **THEN** the settings service SHALL validate and persist the boolean value through the native settings layer

#### Scenario: Preserve Web mock parity
- **WHEN** the preference is loaded or saved through the Web/mock settings adapter
- **THEN** the adapter SHALL preserve the same boolean key and behavior without claiming SQLite access

