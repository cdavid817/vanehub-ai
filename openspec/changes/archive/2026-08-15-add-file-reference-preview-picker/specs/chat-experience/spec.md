## ADDED Requirements

### Requirement: File reference preview and range picker
Selecting a file candidate from `@` completion SHALL open a preview of that file in which the user can pick the lines to reference, so that choosing a file and choosing a region are one action rather than a trip to another tab.

#### Scenario: Selecting a candidate opens the preview
- **WHEN** a user selects a file candidate from `@` completion
- **THEN** the composer SHALL request that file's content through the frontend service boundary and present it in a dialog
- **AND** no reference SHALL be attached until the user confirms one

#### Scenario: A typed range bypasses the preview
- **WHEN** a user completes a mention that already carries a range suffix
- **THEN** the reference SHALL be attached directly without opening the preview

#### Scenario: Content is shown with positions that match the prompt
- **WHEN** the preview displays file content
- **THEN** each line SHALL be labelled with its 1-based position in the file
- **AND** those positions SHALL be the same ones used when the reference is injected into the Agent prompt

#### Scenario: Pick a range by clicking two lines
- **WHEN** the user clicks one line and then another
- **THEN** the pending selection SHALL cover both lines and every line between them
- **AND** the result SHALL be the same regardless of which of the two was clicked first

#### Scenario: Pick a single line
- **WHEN** the user clicks one line and confirms without clicking a second
- **THEN** the attached reference SHALL carry that line as both its start and its end

#### Scenario: The pending selection is visible
- **WHEN** a selection is pending
- **THEN** the selected lines SHALL be visually distinguished from unselected ones
- **AND** the range about to be referenced SHALL be stated in the dialog

#### Scenario: Confirm a selection
- **WHEN** the user confirms the selected range
- **THEN** a reference carrying that range SHALL be attached and the dialog SHALL close

#### Scenario: Confirm the whole file
- **WHEN** the user chooses to reference the whole file
- **THEN** a reference carrying no range SHALL be attached and the dialog SHALL close
- **AND** this SHALL remain available whether or not a selection is pending

#### Scenario: Dismiss without referencing
- **WHEN** the user dismisses the dialog
- **THEN** no reference SHALL be attached
- **AND** the composer draft SHALL be left as it was before the candidate was selected

#### Scenario: File cannot be displayed
- **WHEN** the runtime reports the file as oversized or binary
- **THEN** the dialog SHALL state that condition in place of content
- **AND** it SHALL offer no way to attach a reference, because such a file contributes no content to the prompt and the existing "Reject unsafe reference" requirement already calls for it to be refused

#### Scenario: File is unavailable
- **WHEN** the runtime reports the file as missing or the request fails
- **THEN** the dialog SHALL report that with concise localized feedback and SHALL offer no way to attach a reference

#### Scenario: Large files stay responsive
- **WHEN** the previewed file is large enough that rendering every line at once would stall the interface
- **THEN** the preview SHALL remain interactive
- **AND** a line outside the initially rendered region SHALL still be reachable and selectable

#### Scenario: Keyboard access
- **WHEN** the dialog is open
- **THEN** focus SHALL be confined to it, dismissing SHALL be possible from the keyboard, and focus SHALL return to the composer when it closes

#### Scenario: Web runtime parity
- **WHEN** the preview is opened in Web/mock mode
- **THEN** it SHALL read through the same service contract and behave the same over the mock workspace
