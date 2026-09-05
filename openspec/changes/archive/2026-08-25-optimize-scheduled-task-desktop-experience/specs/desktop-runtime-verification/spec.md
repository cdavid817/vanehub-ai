## MODIFIED Requirements

### Requirement: Native UI interaction coverage
Desktop verification SHALL exercise the client's primary interactive surfaces in the real desktop runtime rather than only asserting that they mount. Coverage SHALL include the session workspace tab set, main-path dialogs, and scheduled-task management, and SHALL assert rendered content, focus behavior, and native persistence produced through the desktop webview.

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

#### Scenario: Startup activity differs from the tested surface
- **WHEN** a native UI test starts while an unrelated activity is selected
- **THEN** the test SHALL navigate explicitly through a stable accessible control before interacting with the target surface
- **AND** it SHALL NOT assume that a content-specific control exists on every startup activity

#### Scenario: Scheduled task native lifecycle
- **WHEN** the layer opens Scheduled Tasks and submits a valid task for a stable CLI Agent id
- **THEN** the rendered list and native scheduled-task service SHALL expose the created record and recurrence
- **AND** disabling and enabling the task through the UI SHALL persist the corresponding native state
- **AND** confirming deletion through the UI SHALL remove the native record

#### Scenario: Interaction coverage cannot substitute a mock runtime
- **WHEN** a requirement in this section is verified
- **THEN** it SHALL be verified against the native desktop artifact and its real service boundary
- **AND** a Web/mock adapter result SHALL NOT be accepted as evidence for it

