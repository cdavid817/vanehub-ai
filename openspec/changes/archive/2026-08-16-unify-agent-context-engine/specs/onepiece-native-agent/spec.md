## ADDED Requirements

### Requirement: OnePiece assembles proactive evidence through the Context Engine
For eligible project turns, OnePiece SHALL invoke the Context Engine before final provider request construction and SHALL accept only a verified bounded projection; optional candidate-source failure or manifest persistence failure MUST NOT fail the generation.

#### Scenario: Evidence selection succeeds
- **WHEN** the Context Engine returns a verified evidence set
- **THEN** OnePiece SHALL include its compact projection in the provider request
- **AND** it SHALL preserve existing provider, tool, cancellation, accounting, and compaction behavior

#### Scenario: Engine cannot produce a safe projection
- **WHEN** planning, collection, normalization, ranking, budgeting, or verification cannot safely complete
- **THEN** OnePiece SHALL continue through the existing request path without partial injected evidence
- **AND** it SHALL emit only a bounded redacted outcome
