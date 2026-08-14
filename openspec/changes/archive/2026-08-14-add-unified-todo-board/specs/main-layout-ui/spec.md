## ADDED Requirements

### Requirement: Todo Board workspace destination
The workspace SHALL expose Todo Board as a first-class full-screen activity destination alongside Sessions, Plans, and Loops.

#### Scenario: Open Todo Board
- **WHEN** the user activates the Todo Board activity entry
- **THEN** the workspace SHALL mark that entry active and render the board in the primary workspace region
- **AND** it SHALL preserve the existing Session, Plan, and Loop destination state for later return
