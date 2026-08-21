## MODIFIED Requirements

### Requirement: Routed workspace destinations
Activating a workspace destination SHALL change the route, and destination state SHALL survive that navigation. The workspace SHALL NOT expose a standalone Plans destination; planning SHALL remain within an eligible OnePiece session.

#### Scenario: Destination activation navigates
- **WHEN** the user activates the Sessions, Loops, or Todo Board activity entry
- **THEN** the workspace SHALL navigate to that destination's route
- **AND** the activity bar SHALL mark the entry active from the route

#### Scenario: Mounted destination state survives navigation
- **WHEN** the user leaves a visited destination and later returns to it
- **THEN** that destination SHALL retain the state it had when it was left
- **AND** it SHALL NOT be remounted or refetched solely because the route changed

#### Scenario: Return from a cross-destination jump
- **WHEN** the user opens a session from the Loop Center and then triggers Back
- **THEN** the workspace SHALL return to the destination the jump started from

#### Scenario: Unknown destination segment
- **WHEN** a workspace route names a destination that does not exist or names the retired Plans destination
- **THEN** the workspace SHALL fall back to the Sessions destination rather than rendering an empty region

## ADDED Requirements

### Requirement: OnePiece owns the planning surface
The workspace SHALL expose Plan mode only within the conversation bar of an eligible OnePiece session and SHALL NOT expose Plan draft, PlanRun, or Plan execution controls in the left activity bar or another global workspace destination.

#### Scenario: Open planning controls
- **WHEN** the active session uses the stable agent id `onepiece`
- **THEN** its conversation bar SHALL expose the available session execution modes including Plan mode
- **AND** the user SHALL remain on the session route while selecting or using Plan mode

#### Scenario: Render the activity bar
- **WHEN** the workspace activity bar renders in the desktop or Web runtime
- **THEN** it SHALL NOT render a Plans or Plan execution entry
