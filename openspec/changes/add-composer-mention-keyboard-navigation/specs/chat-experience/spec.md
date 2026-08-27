## ADDED Requirements

### Requirement: Keyboard-navigable composer completion
The composer SHALL let keyboard users navigate and activate the visible unified `@` completion results while focus remains in the editor.

#### Scenario: First navigation key selects the first result
- **WHEN** `@` completion is visible and no result is selected
- **THEN** the first `ArrowDown` or `ArrowUp` press SHALL select the first visible result
- **AND** the draft SHALL remain unchanged

#### Scenario: Move through unified results
- **WHEN** a completion result is selected
- **THEN** `ArrowDown` SHALL move selection to the next visible participant or file result
- **AND** `ArrowUp` SHALL move selection to the previous visible result
- **AND** navigation SHALL remain bounded within the visible result list

#### Scenario: Activate or dismiss keyboard selection
- **WHEN** a completion result is selected and the user presses `Enter`
- **THEN** the composer SHALL perform the same participant insertion or file-selection behavior as activating that result with a pointer
- **WHEN** the user presses `Escape` while completion selection is active
- **THEN** the selection SHALL clear without changing the draft or submitting a message

#### Scenario: Preserve composition and submission behavior
- **WHEN** an IME composition is active or no completion result is selected
- **THEN** completion navigation SHALL NOT intercept composition input
- **AND** existing Enter and Shift+Enter message submission behavior SHALL remain unchanged

#### Scenario: Expose the active result accessibly
- **WHEN** keyboard navigation selects a completion result
- **THEN** the completion SHALL expose which result is active through semantic option state
- **AND** the active result SHALL remain visibly distinguishable without relying only on color
