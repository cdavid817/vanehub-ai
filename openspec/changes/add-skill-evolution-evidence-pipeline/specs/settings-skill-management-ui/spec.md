## ADDED Requirements

### Requirement: Per-Skill evidence Evolution area
Skill details SHALL provide an evidence-only Evolution area showing collection state, the runtime-event-to-signal-to-seed funnel, extractor counts, attribution distribution, source-Agent distribution, category and polarity distribution, retention, quota, and dropped counts. It SHALL clearly state that target selection and Skill modification are not active in this change.

#### Scenario: Evidence funnel displayed
- **WHEN** a user opens Evolution for a Skill with retained evidence
- **THEN** the page SHALL show bounded event, signal, grouped, and seed counts with their time range and collection status

#### Scenario: Correlated CLI evidence displayed
- **WHEN** evidence includes correlated, weak, or unattributed CLI signals
- **THEN** the UI SHALL distinguish each attribution class and explain which classes cannot drive automatic targeting

#### Scenario: Collection degraded
- **WHEN** queue drops, storage failure, retention failure, or quota pressure degraded evidence collection
- **THEN** the area SHALL show a safe status and affected counts without implying the originating Agent tasks failed

#### Scenario: No evidence
- **WHEN** a Skill has no retained evidence
- **THEN** the area SHALL show an explanatory empty state, active source coverage, and retention policy rather than fabricated metrics

### Requirement: Evidence signal and seed inspection
The Evolution area SHALL provide bounded filters and read-only detail for sanitized signals and candidate seeds, including source kind, stable Agent, workspace, extractor and sanitizer version, category, polarity, severity, attribution rationale, Skill revision, lineage, and occurrence time.

#### Scenario: Inspect signal
- **WHEN** a user opens one signal
- **THEN** the detail SHALL show sanitized bounded evidence and safe source references without raw prompts, transcripts, commands, tool results, files, credentials, or full paths

#### Scenario: Inspect seed lineage
- **WHEN** a user opens one candidate seed
- **THEN** the detail SHALL show grouping reason, readiness, attribution limits, source distribution, and contributing sanitized signals

#### Scenario: Filter evidence
- **WHEN** a user combines source Agent, extractor, attribution, category, polarity, severity, readiness, and time filters
- **THEN** the page SHALL preserve the canonical Skill and workspace scope and update bounded counts and results through the service boundary

### Requirement: Evidence privacy and retention presentation
The Evolution area SHALL display the active sanitizer version, metadata-only or redacted-summary mode, twelve redaction classes, 90-day retention, quota status, and dropped or expired counts using localized explanations.

#### Scenario: Privacy details opened
- **WHEN** a user opens evidence privacy details
- **THEN** the page SHALL explain what is retained and explicitly identify prohibited raw content that is not copied into evidence storage

#### Scenario: Quota pressure displayed
- **WHEN** evidence was discarded because of quota pressure
- **THEN** the page SHALL show bounded discard counts and retention priority without exposing discarded content

### Requirement: Scoped evidence purge UI
The Evolution area SHALL provide a localized confirmation flow for purging evidence by current Skill and workspace, plus navigation to broader purge scope when available. It SHALL explain that source conversations, traces, logs, usage, Skills, and Overlays remain unchanged.

#### Scenario: Confirm Skill purge
- **WHEN** a user confirms purge for the current Skill scope
- **THEN** the page SHALL submit the operation through the frontend service boundary, prevent duplicate submission, and refresh evidence only after success

#### Scenario: Purge fails
- **WHEN** purge fails
- **THEN** the dialog SHALL remain open with a safe actionable error and existing evidence SHALL remain visible

#### Scenario: Purge accessibility
- **WHEN** the purge dialog opens or closes
- **THEN** it SHALL expose a localized accessible name, contain keyboard focus, support safe dismissal, and restore focus to its trigger

### Requirement: Evidence UI adapter parity
Desktop and Web/mock Skills settings SHALL consume the same frontend evidence contracts and render equivalent healthy, empty, correlated-CLI, degraded, quota-pressure, lineage, and purge states.

#### Scenario: Web evidence simulation
- **WHEN** the Web/mock adapter emits representative evidence summaries and seed lineage
- **THEN** the UI SHALL render the same scope, privacy, attribution, filtering, and purge semantics as equivalent desktop data

