## ADDED Requirements

### Requirement: Shared modal dialog behavior
Modal surfaces SHALL obtain dismissal, focus containment, focus return, and accessible labelling from the shared application dialog primitive rather than re-implementing them per page.

#### Scenario: Escape dismisses a modal
- **WHEN** a modal surface is open and its close action is not disabled
- **THEN** pressing Escape SHALL close it

#### Scenario: Focus stays inside an open modal
- **WHEN** a modal surface is open and the user cycles focus with Tab or Shift+Tab
- **THEN** focus SHALL remain within the modal and SHALL wrap at its first and last focusable controls

#### Scenario: Focus returns to the invoking control
- **WHEN** a modal surface closes
- **THEN** focus SHALL return to the control that opened it or to an explicitly designated element

#### Scenario: Modal exposes an accessible name
- **WHEN** a modal surface renders
- **THEN** it SHALL expose `aria-modal`, a programmatic label referencing its title, and a description reference when it renders descriptive text

#### Scenario: Blocking work suppresses dismissal
- **WHEN** a modal surface reports that closing is disabled because an operation is in flight
- **THEN** Escape and backdrop dismissal SHALL NOT close it

### Requirement: In-application text entry
Desktop surfaces SHALL collect user text input through in-application dialogs and SHALL NOT use browser-native `prompt`, `alert`, or `confirm`.

#### Scenario: Naming a new item
- **WHEN** a flow needs a name from the user before it can proceed
- **THEN** it SHALL present an in-application dialog whose field, validation message, and actions follow the active theme and the active application language

#### Scenario: Rejecting invalid input
- **WHEN** the submitted value is empty or the operation fails validation
- **THEN** the dialog SHALL remain open and SHALL present the reason next to the field rather than discarding the entry
