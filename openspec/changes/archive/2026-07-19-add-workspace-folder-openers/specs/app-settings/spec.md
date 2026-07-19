## ADDED Requirements

### Requirement: Folder-opener application preferences
The shared application settings model SHALL expose atomically persisted folder-opener preferences containing one configured default stable id and a validated enabled stable-id list, while keeping runtime discovery data outside persisted user settings.

#### Scenario: Load first-use defaults
- **WHEN** no folder-opener preferences have been persisted
- **THEN** the desktop runtime SHALL provide a valid configured default and enabled list with File Explorer enabled
- **AND** SHALL prefer VS Code for the initial configured default when it is discovered according to the defined initialization policy

#### Scenario: Restore saved preferences
- **WHEN** the application starts after folder-opener preferences were saved
- **THEN** the active runtime adapter SHALL restore the configured default and enabled ids
- **AND** SHALL recompute availability and the effective default from the current environment

#### Scenario: Save preferences atomically
- **WHEN** a user submits valid default and enabled folder-opener preferences
- **THEN** the desktop runtime SHALL persist the aggregate in one transaction
- **AND** subscribers SHALL observe one coherent settings change

#### Scenario: Preserve Web settings parity
- **WHEN** preferences are saved through the Web/mock adapter
- **THEN** it SHALL preserve the same validated preference shape without claiming that native discovery or launch is active

