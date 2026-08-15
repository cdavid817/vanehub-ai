## ADDED Requirements

### Requirement: Routed workspace destinations
Activating a workspace destination SHALL change the route, and destination state SHALL survive that navigation.

#### Scenario: Destination activation navigates
- **WHEN** the user activates the Sessions, Plans, Loops, or Todo Board activity entry
- **THEN** the workspace SHALL navigate to that destination's route
- **AND** the activity bar SHALL mark the entry active from the route

#### Scenario: Mounted destination state survives navigation
- **WHEN** the user leaves a visited destination and later returns to it
- **THEN** that destination SHALL retain the state it had when it was left
- **AND** it SHALL NOT be remounted or refetched solely because the route changed

#### Scenario: Return from a cross-destination jump
- **WHEN** the user opens a session from the Loop Center or Plan Center and then triggers Back
- **THEN** the workspace SHALL return to the destination the jump started from

#### Scenario: Unknown destination segment
- **WHEN** a workspace route names a destination that does not exist
- **THEN** the workspace SHALL fall back to the Sessions destination rather than rendering an empty region

### Requirement: Addressable session creation
Opening the create-session dialog SHALL be expressible as a route.

#### Scenario: External trigger opens creation
- **WHEN** the floating assistant or another external surface requests a new session
- **THEN** the workspace SHALL navigate to the session-creation route and open the dialog

#### Scenario: Dismissing creation leaves the route
- **WHEN** the user closes the create-session dialog without creating a session
- **THEN** the workspace SHALL return to the destination route it came from
