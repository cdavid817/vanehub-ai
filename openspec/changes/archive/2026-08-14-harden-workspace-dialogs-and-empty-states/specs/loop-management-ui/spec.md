## ADDED Requirements

### Requirement: Loop Center first-run state
The Loop Center SHALL present an explanatory empty state with a primary creation action when no Loop definition exists.

#### Scenario: No Loop definitions exist
- **WHEN** the Loop Center opens and the definition list is empty
- **THEN** it SHALL present an icon, a title, and an explanation of what a Loop definition is
- **AND** it SHALL present a primary action that starts Loop creation

#### Scenario: Creation remains reachable once definitions exist
- **WHEN** at least one Loop definition exists
- **THEN** the definition list SHALL continue to expose its creation control
- **AND** the empty state SHALL NOT be rendered

#### Scenario: Inspector reflects the empty state
- **WHEN** the Loop Center has no definitions and therefore no selectable run
- **THEN** the inspector SHALL state that no run is available rather than presenting an empty panel with no explanation

#### Scenario: Localized empty state
- **WHEN** the Loop Center empty state renders
- **THEN** its title, explanation, and primary action label SHALL use the active application language
