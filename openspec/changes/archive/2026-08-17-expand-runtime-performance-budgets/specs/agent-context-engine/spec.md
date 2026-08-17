## ADDED Requirements

### Requirement: Context Engine performance evidence is phase-aware
The Context Engine benchmark SHALL report bounded measurements for candidate collection, ranking, deduplication, budgeting, evidence projection, and index-backed queries together with candidate, selected-item, byte, Token, duplicate-saving, and overflow counts.

#### Scenario: Versioned context dataset is measured
- **WHEN** the small, medium, or large synthetic repository dataset is processed
- **THEN** every applicable phase SHALL emit a measurement correlated with the dataset and policy version
- **AND** selected evidence SHALL remain within byte and Token occupancy budgets

#### Scenario: Optional source is unavailable
- **WHEN** LSP or another optional candidate source is unavailable during measurement
- **THEN** the evidence SHALL identify bounded degradation without failing fallback collection

### Requirement: Context structural gates are deterministic
Context hard gates SHALL use deterministic candidate, operation, occupancy, projection, and overflow bounds. Ranking and query latency MAY be recorded as dedicated P50/P95 evidence but SHALL NOT be a fixed shared-CI millisecond gate.

#### Scenario: Candidate work grows beyond its declared bound
- **WHEN** a regression performs more collection, ranking, or projection work than the versioned dataset budget permits
- **THEN** the deterministic performance suite SHALL fail with the phase, baseline, measured work, and budget

