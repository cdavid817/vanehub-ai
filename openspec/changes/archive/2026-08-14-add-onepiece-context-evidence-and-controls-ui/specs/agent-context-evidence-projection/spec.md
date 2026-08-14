## Purpose

Defines a content-free evidence projection for successful OnePiece context compaction so users can inspect measurable savings and policy provenance without exposing conversation content.

## ADDED Requirements

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

