## ADDED Requirements

### Requirement: Global Plan summary discovery
The system SHALL expose paginated Plan summaries suitable for automatic board reconciliation, including stable Plan identity, latest goal and project, Plan status, latest run identity and status when present, and timestamps.

#### Scenario: List draft and executed Plans
- **WHEN** a caller requests global Plan summaries
- **THEN** the result SHALL include Plans with only drafts and Plans with one or more runs without requiring the caller to know Plan ids

#### Scenario: Aggregate Plan runs
- **WHEN** one Plan has multiple versions or runs
- **THEN** global discovery SHALL return one Plan summary with its latest relevant version and run rather than one top-level record per run
