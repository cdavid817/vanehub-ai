## ADDED Requirements

### Requirement: Seat-scoped and session-scoped tabs
Each session workspace tab SHALL declare whether its content belongs to a single seat or to the whole session, and seat-scoped tabs SHALL let the user choose which seat's content is shown.

#### Scenario: Seat-scoped tabs expose a seat switcher
- **WHEN** a multi-seat session displays the terminal transcript, Shell, or logs tab
- **THEN** the tab SHALL present a seat switcher listing every seat by role and Agent
- **AND** the tab SHALL show only the selected seat's content

#### Scenario: Session-scoped tabs are unaffected by seats
- **WHEN** a multi-seat session displays the workspace, changes, documents, files, or report tab
- **THEN** the tab SHALL show session-wide content without a seat switcher, because those views describe the project rather than an Agent

#### Scenario: Execution trace distinguishes seats without splitting
- **WHEN** a multi-seat session displays the execution trace tab
- **THEN** the trace SHALL remain session-scoped and SHALL distinguish entries by seat

#### Scenario: Single-seat session hides seat switchers
- **WHEN** a session holds exactly one seat
- **THEN** seat-scoped tabs SHALL NOT display a seat switcher and SHALL behave as they do today

#### Scenario: Tab count does not grow with seats
- **WHEN** seats are added to a session
- **THEN** the number of workspace tabs SHALL NOT change, because seat selection happens inside a tab rather than by adding tabs
