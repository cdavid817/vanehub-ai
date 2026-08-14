# agent-context-quality-evaluation Specification

## Purpose
Defines privacy-safe assessment, persistence, aggregation, and deterministic regression evaluation for OnePiece context-compaction policy outcomes.
## Requirements
### Requirement: Every evaluated compaction decision has a bounded outcome assessment
The system SHALL produce at most one content-free quality assessment whenever automatic context compaction reaches an eligibility decision, covering compacted, bypassed, fallback, and failed outcomes with stable bounded fields.

#### Scenario: Optimizer compaction succeeds
- **WHEN** an eligible generation is compacted through the optimizer path
- **THEN** the assessment SHALL identify the compacted outcome, optimizer path, trigger source, measurement quality, policy versions, non-negative before and after measurements, savings, and structural-retention result

#### Scenario: Automatic compaction is bypassed
- **WHEN** an otherwise evaluated compaction is bypassed by user preference, request suppression, cooldown, or circuit state
- **THEN** the assessment SHALL identify the bypassed outcome and bounded reason without storing context content

#### Scenario: Optimizer falls back or compaction fails
- **WHEN** optimization falls back to compatibility compaction or no safe compaction result can be produced
- **THEN** the assessment SHALL identify the final path and outcome together with bounded fallback or failure reason codes

### Requirement: Quality history is private and bounded
The desktop runtime SHALL persist allowlisted assessment metadata in SQLite and SHALL enforce both the configured retention window and a hard record-count ceiling without making generation success depend on history persistence.

#### Scenario: Assessment contains sensitive source context
- **WHEN** the evaluated generation includes prompts, summaries, tool arguments, tool results, credentials, headers, environment values, or private paths
- **THEN** persisted assessment rows and unified diagnostics SHALL omit those values and retain only approved counters, enums, versions, timestamps, correlations, and safe fingerprints

#### Scenario: Persistence is unavailable
- **WHEN** the assessment repository cannot write or prune history
- **THEN** generation SHALL continue according to the compaction result and the failure SHALL be reported through bounded unified logging

#### Scenario: Retention limit is exceeded
- **WHEN** records are older than the configured retention window or exceed the hard ceiling
- **THEN** the runtime SHALL prune the oldest excess records without deleting chat messages or accounting observations

### Requirement: Policy evaluation is deterministic and non-authoritative
The system SHALL evaluate active and candidate context policies against a versioned content-safe regression corpus using deterministic structural, retention, savings, fallback, and failure criteria, and SHALL NOT change live policy selection from evaluation results.

#### Scenario: Candidate policy is evaluated repeatedly
- **WHEN** the same policy versions and corpus version are evaluated more than once
- **THEN** the produced case outcomes and aggregate comparison SHALL be identical

#### Scenario: Candidate loses required context
- **WHEN** a candidate result violates protocol structure or required retention semantics in a corpus case
- **THEN** the case SHALL fail regardless of its measured savings

#### Scenario: Evaluation completes
- **WHEN** active and candidate policies finish evaluation
- **THEN** the result SHALL report bounded pass counts, regression counts, savings distribution, fallback distribution, and all participating versions without activating the candidate

### Requirement: Context policy health is queryable with explicit quality
The runtime SHALL expose paginated assessment history and bounded aggregate summaries over a requested supported time range, preserving measurement-quality and outcome provenance.

#### Scenario: No assessments exist
- **WHEN** history and summary are requested for a range with no records
- **THEN** the runtime SHALL return an empty page and zero-valued summary rather than synthesize successful compactions

#### Scenario: Mixed measurement qualities exist
- **WHEN** a range contains reported, estimated, and characters-only assessments
- **THEN** the summary SHALL keep their coverage separate and SHALL NOT present character savings as token or billing savings

