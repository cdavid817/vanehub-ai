## ADDED Requirements

### Requirement: Mission Control performance fixtures cover 100 and 1,000 Runs
Mission Control performance coverage SHALL use deterministic 100-Run and 1,000-Run histories to verify indexed selection, bounded pages, lazy detail loading, a query count independent of result count, safe summaries, and coalesced frontend updates.

#### Scenario: One thousand Runs are listed
- **WHEN** the maximum versioned history is queried and rendered through the existing service boundary
- **THEN** overview query count SHALL remain constant, returned rows SHALL remain page-bounded, detail SHALL remain lazy, and the frontend SHALL NOT create one update per token event

#### Scenario: N plus one regression is introduced
- **WHEN** the performance negative fixture reports query count growing with Run count
- **THEN** the deterministic gate SHALL fail with the query-count baseline, measured value, and budget

