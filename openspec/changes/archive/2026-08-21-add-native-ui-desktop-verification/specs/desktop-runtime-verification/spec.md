## ADDED Requirements

### Requirement: Native UI interaction coverage
Desktop verification SHALL exercise the client's primary interactive surfaces in the real desktop runtime rather than only asserting that they mount. Coverage SHALL include the session workspace tab set and the main-path dialogs, and SHALL assert rendered content and focus behavior produced by the desktop webview.

#### Scenario: Session workspace tabs carry their own content
- **WHEN** the layer opens a real session and selects each workspace tab in turn
- **THEN** every tab in the workspace tablist SHALL become the selected tab when activated
- **AND** the visible panel SHALL correspond to the selected tab and render that tab's own content
- **AND** no fatal frontend error SHALL be recorded during the traversal

#### Scenario: A main-path dialog honors its contract
- **WHEN** the layer opens a main-path dialog in the desktop client
- **THEN** the dialog SHALL be exposed as a dialog to assistive technology
- **AND** focus SHALL move into the dialog
- **AND** Escape SHALL close it and return focus to the surface that opened it

#### Scenario: Interaction coverage cannot substitute a mock runtime
- **WHEN** a requirement in this section is verified
- **THEN** it SHALL be verified against the native desktop artifact and its real service boundary
- **AND** a Web/mock adapter result SHALL NOT be accepted as evidence for it

### Requirement: Settings persistence across a real relaunch
Desktop verification SHALL prove that a setting changed through the desktop UI reaches native storage and survives an application restart, observed through the settings service rather than through browser storage.

#### Scenario: A changed setting survives relaunch
- **WHEN** the layer changes a setting through the rendered settings UI
- **THEN** the settings service SHALL report the new value
- **AND** after the application is relaunched against the same application-data directory, the settings service SHALL still report it
- **AND** the rendered settings UI SHALL present the restored value

#### Scenario: Persistence evidence is native
- **WHEN** the layer asserts that a setting persisted
- **THEN** it SHALL read the value through the native settings boundary
- **AND** it SHALL NOT accept browser storage as evidence of persistence
