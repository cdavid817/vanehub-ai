## ADDED Requirements

### Requirement: Lazy Loop Center loading
The workspace shell SHALL dynamically load the Loop Center task-board module on first activation while preserving mounted session workspace state.

#### Scenario: Use sessions without opening Loops
- **WHEN** the user operates the session workspace without selecting the Loops activity
- **THEN** the Loop Center module SHALL remain unloaded

#### Scenario: Open Loops for the first time
- **WHEN** the user selects the Loops activity before its module has loaded
- **THEN** the main content region SHALL show a localized loading state until Loop Center is available
- **AND** the session workspace SHALL retain its selected session and mounted tab state

#### Scenario: Return to a loaded Loop Center
- **WHEN** the user returns to Loops after its module has loaded
- **THEN** the Loop Center SHALL render without resetting its task-board state

#### Scenario: Fail to load Loop Center
- **WHEN** the Loop Center module load fails
- **THEN** the main content region SHALL provide a localized retry action
- **AND** the user SHALL still be able to return to the session workspace
