# goal-management Specification Delta

## ADDED Requirements

### Requirement: Searchable goal execution-target picker
Goal relationship creation SHALL use searchable typed pickers for supported Session, Run, Loop, Work Item, and other canonical execution targets instead of requiring ordinary users to enter raw target ids.

#### Scenario: Search a target
- **WHEN** the user adds a relationship
- **THEN** the picker SHALL query bounded safe summaries from the owning service and identify target type, title, status, project, and stable identity

#### Scenario: Choose a target
- **WHEN** the user selects a valid result
- **THEN** the UI SHALL submit the stable target type and id through the existing goal service contract
- **AND** display labels SHALL not be used as identity

#### Scenario: Paste an advanced id
- **WHEN** an explicitly enabled diagnostic path accepts a raw id
- **THEN** the id SHALL be validated and resolved before submission
- **AND** unresolved ids SHALL not create a relationship

### Requirement: Nonblocking goal mutations
Goal create, edit, lifecycle, relationship, acceptance, and archive mutations SHALL preserve the loaded goal list and detail and disable only conflicting target actions.

#### Scenario: Update a goal
- **WHEN** a goal mutation is pending
- **THEN** the selected goal SHALL show local pending state while unrelated goals and navigation remain operable

#### Scenario: Mutation fails
- **WHEN** the service rejects the change
- **THEN** the prior canonical detail and the user's recoverable input SHALL remain visible with a local error

#### Scenario: Mutation changes selection
- **WHEN** the selected goal is archived or deleted according to existing semantics
- **THEN** the route SHALL choose a deterministic remaining goal or a clear empty state
- **AND** the list SHALL not flash blank

### Requirement: Goal master-detail presentation
The Goal Center SHALL use the shared master-detail layout with a scannable goal list, a bounded identity and progress summary, related execution sections, and grouped state-aware actions.

#### Scenario: Select a goal
- **WHEN** a goal row is activated
- **THEN** the route and detail SHALL identify the stable selected goal
- **AND** list filters and scroll position SHALL remain preserved

#### Scenario: Render goal progress
- **WHEN** status, progress, milestones, or acceptance state exists
- **THEN** the detail SHALL distinguish derived status from user actions and SHALL not rely on color alone

#### Scenario: Create or edit
- **WHEN** the user starts goal creation or editing
- **THEN** a shared editor sheet SHALL open instead of expanding a form in the page header

### Requirement: State-aware goal lifecycle actions
The Goal Center SHALL present one state-appropriate primary action and place permitted secondary or destructive lifecycle actions in a grouped menu with explicit consequences.

#### Scenario: Goal needs manual acceptance
- **WHEN** the domain reports that acceptance is required
- **THEN** the primary action MAY open the authoritative acceptance flow
- **AND** the UI SHALL not mark the goal complete from inferred child state

#### Scenario: Action is incompatible
- **WHEN** a lifecycle transition is not permitted in the current version or state
- **THEN** the action SHALL be absent or disabled with an accessible reason

#### Scenario: Confirm destructive action
- **WHEN** archive, abandon, delete, or another destructive operation is chosen
- **THEN** the confirmation SHALL state the effect on linked objects and SHALL not claim linked Runs or Sessions will be deleted unless true

### Requirement: Goal relationship overview
Goal detail SHALL summarize related milestones, Work Items, Sessions, Runs, Loops, and acceptance evidence as bounded grouped links or a readable relationship view.

#### Scenario: Inspect related execution
- **WHEN** linked targets exist
- **THEN** the detail SHALL show target type, safe title, current state, and EvidenceLink to the owning surface

#### Scenario: Target is missing or restricted
- **WHEN** a stored relationship cannot be resolved or viewed
- **THEN** the relationship SHALL remain identifiable as unavailable or restricted
- **AND** it SHALL not be silently removed by rendering

#### Scenario: Use compact width
- **WHEN** the relationship view cannot fit a graph or multi-column layout
- **THEN** it SHALL fall back to a grouped list with equivalent information and navigation

### Requirement: Responsive Goal Center navigation
Goal list, detail, filters, creation, and lifecycle actions SHALL remain complete in wide master-detail and compact list-then-detail compositions.

#### Scenario: Open detail on compact width
- **WHEN** a compact user selects a goal
- **THEN** the detail SHALL replace or overlay the list with a clear Back action
- **AND** returning SHALL restore the prior list query and anchor

#### Scenario: Use keyboard
- **WHEN** a user navigates goal rows, relationship links, and actions
- **THEN** focus order, selected state, and action availability SHALL remain clear and stable
