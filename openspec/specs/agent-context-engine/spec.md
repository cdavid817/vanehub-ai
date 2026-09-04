# agent-context-engine Specification

## Purpose
Assemble the evidence portion of an OnePiece generation's context deterministically: budget-bounded selection over retrieval, memory, and workspace sources, with per-generation evidence manifests that record what was offered and why, so context composition stays explainable and auditable.
## Requirements
### Requirement: Context planning is turn and budget aware
Before a OnePiece provider request, the system SHALL derive a bounded retrieval plan from the user task, session project/worktree, selected model capacity, explicit references, current authoritative plan or task, and a versioned context-budget policy.

#### Scenario: Plan a project turn
- **WHEN** a OnePiece turn has a project, known model capacity, and an active task
- **THEN** the planner SHALL identify eligible evidence sources and their bounded collection requests
- **AND** it SHALL reserve system, task, recent-turn, evidence, and emergency budget before collection

#### Scenario: Capacity is unknown
- **WHEN** verified model capacity is unavailable
- **THEN** the planner SHALL use a conservative versioned evidence ceiling
- **AND** it SHALL mark capacity provenance unknown without inventing utilization

### Requirement: Multi-source collection reuses authoritative capabilities
The engine SHALL normalize bounded candidates from explicit file references, retrieval and Tree-sitter symbols, LSP definitions/references or call relations when supported, relevant tests, recent workspace changes, cross-session memory, and authoritative plan/task state through existing owning-context contracts.

#### Scenario: Function bug gathers related evidence
- **WHEN** a user asks about a function and available sources identify its definition, callers, and relevant tests
- **THEN** those results SHALL enter one normalized candidate pipeline with source provenance

#### Scenario: LSP is unavailable
- **WHEN** an LSP source is warming, unavailable, timed out, or failed
- **THEN** retrieval and Tree-sitter candidates SHALL remain eligible
- **AND** the LSP outcome SHALL NOT fail the generation

### Requirement: Candidates use one bounded internal model
Each candidate SHALL have a stable id, source kind and reference, lazy or bounded content reference, token estimate with quality, relevance inputs, freshness, authority, redundancy group, required/protected flags, safe fingerprint, and allowlisted metadata; sources MUST NOT append free-form provider text directly.

#### Scenario: Source returns unsafe provenance
- **WHEN** a source result cannot be confined to its session/workspace or normalized within limits
- **THEN** the result SHALL be rejected with a bounded reason code
- **AND** its content SHALL NOT reach the provider or manifest

### Requirement: Ranking is deterministic and explainable
The first production policy SHALL rank with versioned deterministic weights for explicitness, semantic relevance, symbol relation, path proximity, recency, authority, duplication, and estimated cost, and SHALL use stable tie-breaking. Every selected candidate and retained top rejection SHALL have bounded reason codes and score buckets.

#### Scenario: Repeat identical selection
- **WHEN** identical normalized candidates and policy are evaluated repeatedly
- **THEN** ordering, selection, rejection, and reason codes SHALL be identical

#### Scenario: Explicit reference competes with a high semantic score
- **WHEN** a valid explicit user reference competes with optional high-scoring candidates
- **THEN** the explicit reference SHALL remain protected and SHALL NOT be displaced by them

### Requirement: Duplicate and overlapping evidence occupies one primary range
Before budgeting, the engine SHALL collapse exact safe fingerprints and merge overlapping canonical file ranges at complete semantic or line boundaries while retaining combined provenance and duplicate-savings measurements.

#### Scenario: Three sources find one snippet
- **WHEN** text retrieval, Tree-sitter, and LSP identify the same code range
- **THEN** the evidence set SHALL charge one primary content range against the budget
- **AND** the manifest SHALL retain all contributing source kinds without content

### Requirement: Budgeting preserves protected and semantic boundaries
The engine SHALL apply a versioned budget policy with source-class limits, protected explicit references and authoritative state, and an emergency reserve. It SHALL remove lower-value optional evidence before higher-value evidence and MUST clip code only at symbol, complete line-range, or complete tool-result boundaries.

#### Scenario: Optional evidence exceeds budget
- **WHEN** normalized optional candidates exceed the available evidence budget
- **THEN** the engine SHALL reject the lowest-value candidates according to deterministic policy
- **AND** final projected occupancy SHALL remain within the effective provider budget

#### Scenario: Protected content alone exceeds the ceiling
- **WHEN** protected references cannot fit within the hard request ceiling
- **THEN** the engine SHALL return a typed protected-overflow outcome
- **AND** it SHALL NOT silently truncate protected content

### Requirement: Evidence projection is compact and verifiable
The provider request SHALL receive only selected evidence content with compact source type, workspace-relative path, line range, symbol, and reason labels. The engine SHALL verify range validity, protected fingerprints, deduplication, and final occupancy before installing the projection.

#### Scenario: Projection verification fails
- **WHEN** any protected fingerprint, range, or budget invariant fails
- **THEN** the engine SHALL discard the candidate projection
- **AND** generation SHALL continue through the existing safe request path without partial evidence injection

### Requirement: Evidence manifests support inspection and evaluation
Each completed selection SHALL produce a bounded content-free manifest containing policy and correlation ids, budget allocation, selected evidence metadata, top rejected summaries, source outcomes, compaction correlation, score/reason data, token estimates, duplicate savings, and latency buckets. Desktop SHALL retain bounded manifest metadata and Web/mock SHALL provide contract-compatible in-memory manifests.

#### Scenario: User opens Context Inspector
- **WHEN** an advanced Session/OnePiece user inspects a completed turn
- **THEN** the UI SHALL show budget, selected source/range/estimate/reasons, top rejection reasons, source degradation, and compaction state
- **AND** ordinary chat SHALL remain usable without opening debug detail

#### Scenario: Manifest persistence fails
- **WHEN** desktop metadata persistence is unavailable
- **THEN** provider generation SHALL retain its selection result
- **AND** the failure SHALL be recorded only through redacted unified logging

### Requirement: Context telemetry is content-free
Diagnostics and persisted manifests SHALL contain only allowlisted source kinds, bounded counts, safe fingerprints, score or latency buckets, estimates, reason codes, policy versions, and correlations. They MUST NOT contain source code, prompt or message text, memory bodies, tool payloads, credentials, headers, environment values, or raw provider data.

#### Scenario: Sensitive evidence is selected
- **WHEN** selected evidence contains secrets or private source content
- **THEN** diagnostics and persisted manifest metadata SHALL contain none of that content
- **AND** negative tests SHALL verify representative sensitive markers are absent

### Requirement: Context quality benchmark is reproducible
The repository SHALL include a deterministic synthetic dataset covering definition retrieval, cross-file references, test discovery, explicit preservation, duplicate elimination, LSP fallback, budget pressure, and memory relevance, and SHALL calculate Recall@budget, Precision@budget, useful-token ratio, candidate collection latency, ranking latency, duplicate savings, and overflow rate.

#### Scenario: Benchmark corpus is repeated
- **WHEN** the same corpus, engine policy, and source fixtures are evaluated repeatedly
- **THEN** selection metrics and structural budget outcomes SHALL be identical
- **AND** performance evidence SHALL distinguish measured latency from deterministic CI budgets

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

### Requirement: Context planning uses the routed Profile budget
Before collecting or projecting evidence, the Context Engine SHALL consume the immutable Profile selected for that generation and use its effective context window, reserved output, provenance, and confidence. It MUST NOT reuse the globally active Profile when Hybrid Routing selected another Profile.

#### Scenario: Local Profile is selected by a rule
- **WHEN** a Hybrid rule selects a local Profile with a configured conservative context window
- **THEN** context planning SHALL budget against that window and record configured-estimate provenance

#### Scenario: Routed Profile capacity is unknown
- **WHEN** the selected Profile has no verified or configured conservative capacity
- **THEN** the planner SHALL use its existing versioned conservative unknown-capacity ceiling
- **AND** it SHALL not invent utilization or retry indefinitely

#### Scenario: Fallback Profile is selected
- **WHEN** routing chooses a policy-compatible fallback Profile before request construction
- **THEN** context collection and final projection SHALL be recomputed for the fallback Profile budget rather than reusing an oversized projection
