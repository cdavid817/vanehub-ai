# agent-context-evidence-projection Specification

## Purpose
Defines a content-free evidence projection for successful OnePiece context compaction so users can inspect measurable savings and policy provenance without exposing conversation content.
## Requirements
### Requirement: Successful compaction produces structured evidence
Every successful automatic OnePiece context compaction SHALL produce one structured evidence projection containing before and after character counts, before and after token counts when available, the quality of each measurement, non-negative savings, trigger source, compaction path, and policy version.

#### Scenario: Token measurements are available
- **WHEN** a successful compaction has authoritative token measurements before and after optimization
- **THEN** the evidence SHALL contain both token counts and the computed token savings
- **AND** SHALL identify the token measurement quality

#### Scenario: Only character measurements are available
- **WHEN** a successful compaction cannot obtain authoritative token measurements
- **THEN** the evidence SHALL still contain before and after character counts and character savings
- **AND** SHALL mark token values unavailable rather than fabricating estimates

#### Scenario: Compatibility compaction succeeds
- **WHEN** the compatibility compaction path succeeds after the optimizer path is unavailable or declines the context
- **THEN** the evidence SHALL identify the compatibility path while retaining the same metric shape

### Requirement: Compaction evidence is content-free
Compaction evidence SHALL contain only bounded numeric measurements, stable enum-like labels, and version identifiers; it SHALL NOT contain prompts, summaries, tool inputs or outputs, provider response text, credentials, filesystem content, or other conversation payloads.

#### Scenario: Sensitive context is compacted
- **WHEN** the source context contains secrets, prompts, tool payloads, or filesystem content
- **THEN** none of those values SHALL appear in the projected evidence or its diagnostic metadata

### Requirement: Desktop and Web evidence contracts remain compatible
The desktop and Web/mock runtimes SHALL project successful compaction evidence through the same rich-card field and metadata contract, while allowing Web/mock to report character-only measurement quality.

#### Scenario: Web mock simulates compaction
- **WHEN** Web/mock runtime deterministically simulates a successful automatic compaction
- **THEN** it SHALL emit a contract-compatible evidence card with character metrics and unavailable token metrics

### Requirement: Evidence cards correlate with quality assessments
Every successful compaction evidence card SHALL expose the same content-free attempt correlation used by its quality assessment so restored chat evidence can be reconciled with bounded policy-health history.

#### Scenario: Assessment persistence succeeds
- **WHEN** successful compaction produces both a rich evidence card and a persisted assessment
- **THEN** both projections SHALL carry the same stable attempt correlation and policy version

#### Scenario: Assessment persistence fails
- **WHEN** successful compaction evidence is produced but assessment persistence is unavailable
- **THEN** the evidence card SHALL remain available with its attempt correlation and generation SHALL remain successful

### Requirement: Context selection manifests are inspectable evidence
The system SHALL project a content-free Context Engine manifest for each completed OnePiece evidence selection through the same desktop and Web/mock service contract, independently of existing compaction evidence cards.

#### Scenario: Selection and compaction both occur
- **WHEN** a turn selects proactive evidence and later triggers compaction
- **THEN** the inspector SHALL distinguish the selection manifest from compaction evidence
- **AND** it SHALL correlate both by stable content-free turn and generation identifiers

