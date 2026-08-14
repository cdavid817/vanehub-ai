## Purpose

Defines a provider-neutral, invocation-grained Token accounting contract that preserves reported semantics, reconciles cumulative sources, and produces safe idempotent projections across native API and CLI runtimes.

## ADDED Requirements

### Requirement: Invocation-grained accounting identity
The system SHALL represent every observed model invocation independently and SHALL correlate it with the available generation, run, operation, session, message, Agent, provider, model, attempt, interaction kind, and purpose identities.

#### Scenario: Account for a multi-call generation
- **WHEN** one user generation performs an initial model request and one or more tool-continuation requests
- **THEN** the system SHALL retain a distinct invocation and usage observation for every provider request
- **AND** it SHALL project their combined usage to the owning generation and assistant message without losing the invocation breakdown

#### Scenario: Account for an internal invocation
- **WHEN** context compaction or automatic memory extraction calls a model outside the final assistant response
- **THEN** the system SHALL record the invocation under its internal purpose
- **AND** it SHALL allow the message id to be absent while preserving run and session correlation

#### Scenario: Account for a failed or cancelled invocation
- **WHEN** a failed, cancelled, or retried invocation yields valid provider-reported usage
- **THEN** the system SHALL retain that consumption with its terminal status
- **AND** it SHALL NOT require a successfully completed assistant message

### Requirement: Lossless normalized usage observations
The system SHALL preserve non-negative provider-reported input, output, cached-input, cache-write-input, reasoning-output, and authoritative-total values when present, together with explicit field-overlap semantics and a normalization version.

#### Scenario: Provider reports authoritative total
- **WHEN** a provider reports a valid total Token count
- **THEN** aggregate headline usage SHALL use that authoritative total
- **AND** category fields SHALL remain explanatory dimensions rather than being unconditionally added to it

#### Scenario: Provider omits authoritative total
- **WHEN** a provider reports categories without an authoritative total
- **THEN** its versioned adapter SHALL derive a total only from documented overlap semantics
- **AND** unknown subset or exclusivity relationships SHALL NOT be guessed

#### Scenario: Preserve cache and reasoning dimensions
- **WHEN** cached, cache-write, or reasoning Token counts are reported
- **THEN** the system SHALL retain them as separate normalized dimensions
- **AND** query and presentation contracts SHALL NOT irreversibly fold them into another field

### Requirement: Accounting quality separation
The system SHALL classify measurements as `reported`, `reported-derived`, or `estimated` and SHALL keep Token observations separate from character estimates.

#### Scenario: Use exact provider usage
- **WHEN** a provider emits an interval usage record for one invocation
- **THEN** the system SHALL classify it as `reported`

#### Scenario: Derive an interval from cumulative snapshots
- **WHEN** a provider exposes only cumulative session totals and two valid ordered snapshots exist
- **THEN** the system SHALL persist their non-negative difference as `reported-derived`
- **AND** it SHALL retain provenance linking the delta to both snapshot states

#### Scenario: Fall back without reported usage
- **WHEN** a successful visible response has no valid provider usage observation
- **THEN** the system SHALL retain input and output character counts as `estimated`
- **AND** it SHALL NOT label or add those characters as reported Tokens

### Requirement: Idempotent ingestion and cumulative reset handling
The system SHALL make provider event replay, polling, process restart, file reread, and Web/mock fixture replay idempotent by using stable source keys and persisted ingestion cursors.

#### Scenario: Replay one provider event
- **WHEN** the same provider usage event is ingested more than once
- **THEN** exactly one logical usage observation SHALL contribute to projections

#### Scenario: Advance cumulative snapshot
- **WHEN** a cumulative source increases within the same provider session epoch
- **THEN** only the positive delta since the persisted cursor SHALL be added
- **AND** prior dated observations SHALL remain unchanged

#### Scenario: Detect counter reset or source rotation
- **WHEN** a cumulative counter decreases, its provider session changes, or its source is replaced
- **THEN** the system SHALL open a new reconciliation epoch
- **AND** it SHALL NOT emit a negative delta or combine incompatible snapshots

### Requirement: Safe accounting persistence and diagnostics
Token accounting SHALL persist only bounded counters, safe identities, statuses, timestamps, semantic versions, provenance keys, and hashes; all diagnostics SHALL use unified logging after redaction.

#### Scenario: Persist provider usage
- **WHEN** provider usage is normalized
- **THEN** the accounting store SHALL NOT contain prompts, model response content, credentials, request headers, raw protocol frames, or raw provider payloads

#### Scenario: Diagnose malformed usage
- **WHEN** provider usage is missing, negative, malformed, semantically unsupported, or rejected as a duplicate
- **THEN** the runtime SHALL write a bounded redacted diagnostic with safe correlation and reason codes
- **AND** normal ingestion degradation SHALL NOT create a feature-local log file

### Requirement: First-version ledger cutover
The system SHALL use invocation observations as the only Token accounting source and SHALL derive message, generation, session, Agent, provider, model, purpose, quality, and local-calendar projections directly from them.

#### Scenario: Start with the ledger schema
- **WHEN** a database receives the fine-grained Token accounting schema
- **THEN** it SHALL create the invocation, observation, and cursor structures without importing pre-release `usage_records` rows
- **AND** development-only historical usage MAY be discarded

#### Scenario: Query accounting data
- **WHEN** session or global usage is requested
- **THEN** the result SHALL be derived only from active invocation observations
- **AND** no legacy row, compatibility projection, or frontend character reaggregation SHALL contribute
