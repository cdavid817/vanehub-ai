# agent-context-measurement Specification

## Purpose
Defines provider-neutral measurement and classification of the complete OnePiece request context so later optimization decisions can rely on protocol-safe groups, explicit retention semantics, known model capacity, and auditable measurement quality.
## Requirements
### Requirement: Complete request context snapshot
Before each OnePiece provider request, the system SHALL produce a bounded context snapshot covering the system instructions, declared tool schemas, conversation messages, tool-loop additions, and other request content that contributes to the model context.

#### Scenario: Analyze initial generation request
- **WHEN** OnePiece prepares the first provider request for a generation
- **THEN** the system SHALL analyze the complete request context before sending it
- **AND** the snapshot SHALL distinguish its major context components

#### Scenario: Analyze tool continuation request
- **WHEN** OnePiece appends an assistant tool request and its result before a continuation request
- **THEN** the system SHALL produce a new snapshot that includes the added content
- **AND** it SHALL correlate the snapshot with that specific provider invocation sequence

### Requirement: Measurement quality provenance
The system SHALL report context occupancy using the best available normalized provider usage and deterministic local estimation, and SHALL label each snapshot as `reported`, `reported-plus-estimated-delta`, `estimated`, or `characters-only` without presenting estimates as provider-reported Tokens.

#### Scenario: Reuse a matching provider measurement
- **WHEN** a valid provider-reported input measurement corresponds to the unchanged request context being analyzed
- **THEN** the snapshot SHALL use that measurement
- **AND** it SHALL label the occupancy quality as `reported`

#### Scenario: Extend a reported measurement with local changes
- **WHEN** a valid provider-reported input measurement exists for a prior request in the same generation and the runtime can identify content added or changed since that request
- **THEN** the snapshot SHALL combine the reported baseline with a deterministic estimated delta
- **AND** it SHALL label the occupancy quality as `reported-plus-estimated-delta`

#### Scenario: Estimate a request without matching usage
- **WHEN** no valid matching provider usage observation is available
- **THEN** the system SHALL estimate the request context locally
- **AND** it SHALL label Token estimates as `estimated` or a character-only fallback as `characters-only`

#### Scenario: Reject malformed reported usage
- **WHEN** provider usage is negative, all-zero without meaningful-zero semantics, malformed, or cannot be correlated with the analyzed request
- **THEN** the system SHALL NOT classify it as a reported context measurement
- **AND** it SHALL fall back to an explicitly lower-quality measurement

### Requirement: Model capacity representation
The system SHALL resolve context-window and reserve metadata for the active OnePiece provider model when verified metadata is available and SHALL represent unknown capacity without inventing a utilization percentage.

#### Scenario: Calculate capacity for a known model
- **WHEN** verified context-window metadata exists for the active model
- **THEN** the snapshot SHALL expose total capacity, reserved capacity, occupied capacity, remaining capacity, and utilization
- **AND** all derived values SHALL identify the versioned policy used to calculate them

#### Scenario: Preserve unknown model capacity
- **WHEN** the active custom model has no verified context-window metadata
- **THEN** the snapshot SHALL mark capacity as unknown
- **AND** it SHALL NOT emit a fabricated remaining-token value or utilization percentage

### Requirement: Protocol-safe API-round grouping
The system SHALL group conversational content at complete provider API-round boundaries and SHALL keep a tool request, its tool result, and content belonging to the same assistant response in one indivisible group.

#### Scenario: Group a completed tool round
- **WHEN** an assistant response requests one or more tools and the corresponding results are present
- **THEN** all requests and results from that response SHALL belong to the same API-round group

#### Scenario: Detect an incomplete tool round
- **WHEN** a tool request has no matching result or a result has no matching request
- **THEN** the group SHALL be marked protocol-incomplete
- **AND** the analysis SHALL NOT classify that group as independently removable or summarizable

#### Scenario: Separate consecutive assistant responses
- **WHEN** a new assistant response begins after the previous API round is complete
- **THEN** the system SHALL create a new API-round group

### Requirement: Semantic and retention classification
The system SHALL classify context components using stable semantic categories and SHALL assign every component or API-round group a retention class of `protected`, `verbatim`, `summarizable`, `microcompactable`, `reinjectable`, or `discardable`.

#### Scenario: Protect control context
- **WHEN** a component contains system instructions, active role constraints, safety constraints, or unresolved protocol state
- **THEN** the system SHALL classify it as `protected`

#### Scenario: Preserve current conversational intent
- **WHEN** a group contains the current user request, a user correction, or recent context required to continue the active task
- **THEN** the system SHALL classify it as `verbatim`

#### Scenario: Identify older completed conversation
- **WHEN** a completed older API-round group contains conversation history that is not protected or required verbatim
- **THEN** the system SHALL classify it as `summarizable`

#### Scenario: Identify reclaimable tool output
- **WHEN** a completed older API-round group contains a large or duplicate tool result whose durable facts can be preserved separately
- **THEN** the system SHALL classify that result as `microcompactable`
- **AND** the containing API-round boundary SHALL remain intact

#### Scenario: Handle unknown content safely
- **WHEN** the classifier encounters an unknown message or content-block type
- **THEN** it SHALL apply a conservative non-discardable retention class
- **AND** it SHALL record only a bounded unknown-type reason code

### Requirement: Safe shadow diagnostics
The system SHALL emit shadow-analysis diagnostics through unified logging using bounded counters, quality values, policy versions, stable correlations, hashes, and reason codes only.

#### Scenario: Record a decision disagreement
- **WHEN** the shadow decision differs from the active character-count decision
- **THEN** the diagnostic SHALL identify both decisions, their measurement quality, and a bounded disagreement reason
- **AND** it SHALL correlate the event with the session, operation, generation, and invocation sequence where available

#### Scenario: Exclude context content from diagnostics
- **WHEN** any context snapshot or shadow diagnostic is persisted
- **THEN** it SHALL NOT contain prompts, message text, tool arguments, tool results, credentials, request headers, raw provider frames, or raw provider payloads

### Requirement: Token-aware production decision
The system SHALL evaluate a versioned Token-aware compaction decision from each complete request snapshot. When verified model capacity and a Token measurement are available, that decision SHALL be eligible to control automatic compaction; the system SHALL retain the prior character-count outcome as comparison evidence and as the fallback when Token-aware evidence is insufficient.

#### Scenario: Use sufficient Token-aware evidence
- **WHEN** a request snapshot has known verified capacity and a complete local or correlated provider Token measurement
- **THEN** the system SHALL compare occupied Tokens with the versioned reserve-and-buffer threshold
- **AND** that result SHALL control whether automatic compaction is eligible

#### Scenario: Model capacity is unknown
- **WHEN** model capacity is unknown
- **THEN** the Token-aware decision SHALL report `insufficient-capacity-metadata`
- **AND** automatic compaction eligibility SHALL fall back to the fixed character-count decision

#### Scenario: Token measurement is unavailable
- **WHEN** the snapshot has only character measurement
- **THEN** the Token-aware decision SHALL report `characters-only-measurement`
- **AND** automatic compaction eligibility SHALL fall back to the fixed character-count decision

#### Scenario: Analysis fails
- **WHEN** snapshot construction, estimation, grouping, or classification fails
- **THEN** automatic compaction eligibility SHALL fall back to the fixed character-count decision
- **AND** the failure SHALL NOT alter or discard request content

#### Scenario: Compare active and legacy decisions
- **WHEN** the Token-aware decision controls automatic compaction
- **THEN** diagnostics SHALL retain both the Token-aware result and the legacy character-count result
- **AND** they SHALL identify which decision source was authoritative

#### Scenario: Compare shadow and active decisions
- **WHEN** sufficient Token-aware evidence is evaluated after the shadow introduction phase
- **THEN** the system SHALL retain the legacy character-count outcome as comparison evidence
- **AND** it SHALL identify the Token-aware production decision as authoritative

#### Scenario: Shadow capacity is unknown
- **WHEN** the active model has no verified capacity metadata
- **THEN** the production decision SHALL preserve the bounded `insufficient-capacity-metadata` outcome introduced by shadow mode
- **AND** it SHALL select character fallback as the authority

#### Scenario: Shadow analysis fails
- **WHEN** the analysis that originated in shadow mode fails after production promotion
- **THEN** the provider request SHALL continue under character fallback
- **AND** the failure SHALL NOT alter or discard request content

### Requirement: Injected evidence has distinct occupancy provenance
Complete request snapshots SHALL measure Context Engine evidence separately from system instructions, declared tools, conversation, and tool-loop additions while retaining compatible aggregate request occupancy and measurement-quality semantics.

#### Scenario: Evidence is projected before provider invocation
- **WHEN** the Context Engine installs a verified evidence set
- **THEN** the next request snapshot SHALL include its evidence occupancy and policy version
- **AND** existing compaction decisions SHALL operate on the complete assembled request

### Requirement: Context performance measurements remain content-free
Context performance records SHALL contain only allowlisted phase names, policy and dataset versions, correlations, duration or count buckets, byte and Token estimates, measurement quality, occupancy values, and bounded outcomes. They MUST NOT persist prompt text, message text, tool arguments or results, credentials, request headers, raw provider frames, evidence content, or unrestricted paths.

#### Scenario: Context timing is persisted or exported
- **WHEN** a request snapshot or Context Engine phase emits performance evidence
- **THEN** sensitive context content SHALL be excluded before unified logging or benchmark output
- **AND** occupancy quality SHALL continue to distinguish reported and estimated values

