## ADDED Requirements

### Requirement: Skill evolution evidence summaries
Skill management detail responses SHALL provide read-only bounded evolution evidence summaries containing collection status, signal and seed counts, extractor distribution, attribution distribution, source-Agent distribution, categories, polarities, severities, first and last occurrence, retention, quota, and dropped counts for the canonical Skill and workspace context.

#### Scenario: Skill has verified evidence
- **WHEN** a Skill has retained verified signals and candidate seeds
- **THEN** its detail response SHALL identify verified counts and seed readiness without claiming that any change has been generated or applied

#### Scenario: Skill has correlated CLI evidence
- **WHEN** a Skill has only correlated CLI evidence
- **THEN** the response SHALL label it human-review-only and SHALL not present it as automatically targetable

#### Scenario: Skill has no evidence
- **WHEN** no retained evidence is associated with a Skill
- **THEN** the response SHALL return a valid empty evidence state and collection status

### Requirement: Bounded Skill signal and seed queries
The Skill management service SHALL provide cursor-paginated queries for sanitized signals and candidate seeds filtered by canonical Skill id, workspace, stable source Agent id, extractor, attribution, category, polarity, severity, readiness, and time range.

#### Scenario: Query recent signals
- **WHEN** a client requests recent signals for one canonical Skill and workspace
- **THEN** the service SHALL return newest-first sanitized summaries, lineage fields, and a continuation cursor without prohibited raw content

#### Scenario: Inspect seed
- **WHEN** a client requests one accessible candidate seed
- **THEN** the service SHALL return its deterministic grouping metadata and bounded contributing signal summaries

#### Scenario: Cross-workspace query refused
- **WHEN** a request attempts to read Project evidence from another inaccessible workspace
- **THEN** the service SHALL reject or isolate the request without exposing that workspace's evidence

### Requirement: Scoped evidence purge service
The Skill management service SHALL provide confirmed evidence purge operations globally and by workspace, canonical Skill id, source Agent id, time range, and evidence kind. Purge responses SHALL report deleted signal and seed counts without deleting source runtime data or Skill content.

#### Scenario: Purge Skill evidence
- **WHEN** a confirmed purge for one Skill succeeds
- **THEN** the response SHALL report deleted signals and rebuilt or deleted seeds and the refreshed Skill evidence summary SHALL be consistent

#### Scenario: Web purge simulation
- **WHEN** Web/mock mode receives a scoped purge request
- **THEN** it SHALL update only its simulated evidence state through the same service contract

