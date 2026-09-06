## ADDED Requirements

### Requirement: One immutable generation snapshot per turn
The system SHALL assemble one immutable generation snapshot at the start of every OnePiece generation or seat turn — provider profile id and revision, endpoint, interface format, model id, resolved model capacity, the frozen tool catalog, the effective personalization/skill/memory views, the observed permission policy revision, and the generation options — and every stage of that turn SHALL execute against that snapshot. A configuration edit made while the turn runs SHALL take effect from the next turn only.

#### Scenario: Mid-turn configuration edits do not drift the running turn
- **WHEN** a generation starts under one active profile and the user activates a different profile before the turn completes
- **THEN** the running turn SHALL keep using the snapshot resolved at its start
- **AND** the next generation SHALL resolve a snapshot reflecting the new profile

#### Scenario: The tool catalog is frozen for the turn
- **WHEN** a Skill, MCP server, or extension is enabled or disabled while a tool loop is in flight
- **THEN** the in-flight turn SHALL keep the catalog its snapshot froze
- **AND** the next turn SHALL resolve the updated catalog
