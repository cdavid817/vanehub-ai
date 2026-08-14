# agent-context-optimization Specification

## Purpose
Defines how OnePiece safely reduces a classified provider context after compaction has triggered, while preserving protocol invariants, actionable evidence, and continuity of the active task.
## Requirements
### Requirement: Immutable optimization plan
The system SHALL derive a bounded, versioned optimization plan from the complete context snapshot before changing provider request content, and every plan action SHALL reference whole components or complete API rounds from that snapshot.

#### Scenario: Plan an eligible context
- **WHEN** active compaction has triggered and the request has a valid context snapshot
- **THEN** the system SHALL produce an ordered optimization plan before constructing a replacement request
- **AND** the original prepared request SHALL remain available unchanged until the optimized candidate passes verification

#### Scenario: Protect incomplete protocol
- **WHEN** a snapshot contains a protocol-incomplete API round
- **THEN** the plan SHALL keep that round unchanged
- **AND** it SHALL NOT select any component of that round for removal, microcompaction, or summarization

### Requirement: Least-destructive ordered optimization
The optimizer SHALL apply eligible reductions from lowest to highest semantic cost: explicitly discardable transient content, reinjectable state, microcompactable tool output, and summarizable completed API rounds. It SHALL stop as soon as the configured target budget is satisfied.

#### Scenario: Low-cost reduction reaches the target
- **WHEN** discardable, reinjectable, or microcompactable content can satisfy the target budget
- **THEN** the optimizer SHALL produce a candidate without invoking the summary model

#### Scenario: Summary remains necessary
- **WHEN** eligible low-cost reductions do not satisfy the target budget
- **THEN** the optimizer SHALL summarize the oldest eligible completed API rounds
- **AND** protected and verbatim content SHALL remain byte-for-byte unchanged

#### Scenario: No safe reduction exists
- **WHEN** all removable content is protected or required verbatim
- **THEN** the optimizer SHALL return a bounded `insufficient-reclaimable-context` outcome
- **AND** it SHALL NOT construct a partially modified provider request

### Requirement: Protocol-safe tool-result microcompaction
The optimizer SHALL reduce an eligible tool result only through a deterministic bounded replacement that retains its tool-call relationship, completion state, failure state, source fingerprint, and a safe indication that content was compacted.

#### Scenario: Compact an old large tool result
- **WHEN** a completed older API round contains a large tool result selected as microcompactable
- **THEN** the candidate SHALL retain a syntactically valid result for the same tool request
- **AND** the replacement SHALL not include the removed raw output

#### Scenario: Preserve recent or unresolved tool output
- **WHEN** a tool result is recent, protected, or belongs to an incomplete API round
- **THEN** the optimizer SHALL preserve the result unchanged

### Requirement: Structured continuation summary
When summarization is required, the system SHALL request a bounded continuation summary without tools and SHALL require sections for primary user intent, technical constraints, decisions, files and code areas, errors and fixes, completed work, pending work, and the immediate next action.

#### Scenario: Produce a continuation summary
- **WHEN** one or more completed API rounds are selected for summarization
- **THEN** the summarization request SHALL contain only the selected rounds and bounded summary instructions
- **AND** it SHALL NOT declare or execute tools
- **AND** the accepted summary SHALL contain every required continuation section

#### Scenario: Reject unusable summary output
- **WHEN** the summary is empty, malformed, exceeds its output budget, omits required sections, or reports a provider failure
- **THEN** the optimizer SHALL reject that summary candidate
- **AND** it SHALL preserve the original context for compatibility fallback

### Requirement: Authoritative state reinjection
The optimizer SHALL reintroduce state classified as reinjectable from its current authoritative source rather than retaining stale historical copies, subject to bounded per-kind and aggregate budgets.

#### Scenario: Replace stale reinjectable history
- **WHEN** an optimization plan removes historical state that has a current authoritative source
- **THEN** the candidate SHALL include one bounded current representation of that state
- **AND** it SHALL identify the source revision through content-free metadata

#### Scenario: Reinjection source is unavailable
- **WHEN** authoritative state cannot be loaded or validated
- **THEN** the optimizer SHALL preserve the historical state or reject the candidate
- **AND** it SHALL NOT silently omit the state

### Requirement: Candidate verification and compatibility fallback
Before an optimized request replaces the original context, the system SHALL verify protocol completeness, protected and verbatim fingerprints, ordering constraints, required reinjections, aggregate coverage, and estimated occupancy. A failed or non-reducing candidate SHALL NOT be sent.

#### Scenario: Accept a safe reducing candidate
- **WHEN** the candidate preserves all required invariants and its estimated occupancy is lower than the original
- **THEN** the system SHALL use the candidate for the already-triggered provider request
- **AND** it SHALL retain before-and-after measurements for bounded diagnostics

#### Scenario: Candidate fails verification
- **WHEN** any candidate invariant fails or the candidate does not reduce estimated occupancy
- **THEN** the system SHALL discard the candidate
- **AND** it SHALL invoke the existing summary-only compatibility path using the untouched original turns

#### Scenario: Unknown Token estimate
- **WHEN** either side has only character measurement
- **THEN** verification SHALL compare complete recursive character coverage
- **AND** it SHALL label the reduction evidence as character-only

### Requirement: Content-free optimization evidence
The system SHALL record optimizer diagnostics through unified logging using bounded action counts, class totals, policy versions, safe fingerprints, measurement qualities, before-and-after occupancy, invariant outcomes, and fallback reason codes only.

#### Scenario: Record a successful optimization
- **WHEN** an optimized candidate is accepted
- **THEN** diagnostics SHALL identify the applied action counts, saved occupancy, measurement quality, and verifier version
- **AND** they SHALL correlate with the owning session, operation, generation, and provider invocation sequence

#### Scenario: Record optimizer fallback
- **WHEN** the optimizer falls back to summary-only compaction
- **THEN** diagnostics SHALL contain one bounded failure stage and reason code
- **AND** they SHALL NOT contain prompt text, summary text, message content, tool arguments, tool results, credentials, headers, or raw provider payloads

