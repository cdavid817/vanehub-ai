## ADDED Requirements

### Requirement: CLI terminal theme setting
The shared application settings model SHALL include a `cliTerminalTheme` display preference with the values `light` and `dark`, defaulting to `dark`, persisted through the existing settings storage in both runtimes.

#### Scenario: Default on a fresh install or an older store
- **WHEN** settings are loaded and no `cliTerminalTheme` row or key exists
- **THEN** the effective value SHALL be `dark`
- **AND** no other setting SHALL change as a result

#### Scenario: Persist and restore a chosen value
- **WHEN** the user saves `light` or `dark`
- **THEN** the desktop runtime SHALL store it through the native settings service in the existing `settings` key/value table
- **AND** the Web/mock runtime SHALL store it through its existing browser persistence
- **AND** the same value SHALL be returned after a reload or relaunch

#### Scenario: Recover from an invalid stored value
- **WHEN** a stored `cliTerminalTheme` value is outside `light` and `dark`
- **THEN** both runtimes SHALL fall back to `dark` on read without failing the settings load

#### Scenario: Refuse an invalid write
- **WHEN** a caller attempts to save a `cliTerminalTheme` value outside `light` and `dark`
- **THEN** the save SHALL be rejected with a validation error
- **AND** the stored value SHALL remain unchanged rather than being silently coerced

#### Scenario: Stay independent of the application theme
- **WHEN** the application theme changes
- **THEN** `cliTerminalTheme` SHALL keep its stored value
- **AND** saving `cliTerminalTheme` SHALL NOT change the application theme

#### Scenario: Return to default on global reset
- **WHEN** the user resets application settings to defaults
- **THEN** `cliTerminalTheme` SHALL return to `dark` alongside the other resettable keys
