## ADDED Requirements

### Requirement: Goal Center presentation
The Goal Center SHALL present goals so that identity, derived status, and progress are scannable from the list, and SHALL separate a selected goal's information from the actions that change it.

#### Scenario: Scan the goal list
- **WHEN** the Goal Center renders its goal list
- **THEN** each row SHALL show the goal title as its most prominent element together with its derived status and its progress
- **AND** derived status SHALL be identifiable through text in addition to color
- **AND** the selected row SHALL be distinguishable from unselected rows by more than a border color alone

#### Scenario: Read a selected goal
- **WHEN** a goal is selected
- **THEN** the detail pane SHALL present the goal's identity and description separately from its linked execution targets
- **AND** acceptance, reopening, activation, abandonment, editing, and deletion controls SHALL be grouped together rather than interleaved with goal information

#### Scenario: Empty and busy states
- **WHEN** the Goal Center has no goals, or a goal has no linked execution targets
- **THEN** it SHALL show a localized explanation of the empty state rather than an empty region
- **AND** while a mutation is in flight the affected controls SHALL be disabled without replacing already loaded content

#### Scenario: Compact viewport
- **WHEN** the available width cannot show the goal list and the detail pane side by side
- **THEN** the Goal Center SHALL preserve access to the list, the detail content, and every goal action without clipping required controls
