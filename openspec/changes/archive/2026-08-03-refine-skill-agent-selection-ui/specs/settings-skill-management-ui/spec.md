## ADDED Requirements

### Requirement: Explicit Agent Skill selection board
The Skills settings page SHALL present the selected stable Agent's Skill relationships as an assignment-focused, responsive selection board without representing immediate binding mutations as checkboxes.

#### Scenario: Compare Assigned and Available Skills on a wide layout
- **WHEN** a user selects a compatible CLI or API Agent on a wide settings layout
- **THEN** the page SHALL present separately labeled Assigned and Available panels in parallel columns
- **AND** each panel SHALL show its own deterministic count and empty state

#### Scenario: Preserve selection order on a narrow layout
- **WHEN** the selected-Agent view is rendered below the wide-layout breakpoint
- **THEN** the Assigned and Available panels SHALL stack in a single document order with Assigned first
- **AND** every row action SHALL remain visible without horizontal page scrolling

#### Scenario: Assign an Available Skill
- **WHEN** a user activates Assign for a Skill in the Available panel
- **THEN** the page SHALL invoke the existing granular bind operation with the selected stable Agent id
- **AND** SHALL keep the Skill in its original panel until the refreshed overview confirms success
- **AND** SHALL disable duplicate actions only for the affected Skill while the operation is pending

#### Scenario: Remove an Assigned Skill
- **WHEN** a user activates Remove for a Skill in the Assigned panel
- **THEN** the page SHALL invoke the existing granular unbind operation with the selected stable Agent id
- **AND** SHALL keep global enablement and every other Agent assignment unchanged

#### Scenario: Keep a failed relationship mutation attached to its row
- **WHEN** an Agent assignment or removal fails
- **THEN** the Skill SHALL remain in its original panel
- **AND** the actionable error SHALL remain associated with that Skill row
- **AND** unrelated rows and filters SHALL remain operable

#### Scenario: Focus selected-Agent rows on relationship management
- **WHEN** the page renders a selected-Agent Skill row
- **THEN** the row SHALL show global enabled or paused state, Agent binding state, preview, and one explicit Assign or Remove action
- **AND** SHALL NOT render mutable global enablement, edit, or delete controls
- **AND** the action accessible name SHALL identify the selected Agent without using its display name as the service identity

#### Scenario: Distinguish CLI and API relationships
- **WHEN** the selected Agent is CLI-kind or API-kind
- **THEN** the page SHALL continue to describe CLI relationships as configured, mounted, or paused and API relationships as prompt injection or paused
- **AND** SHALL use the same selection-board interaction without hard-coded provider branches
