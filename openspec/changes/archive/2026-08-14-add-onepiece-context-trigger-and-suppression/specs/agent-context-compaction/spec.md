## RENAMED Requirements

- FROM: `### Requirement: Character-count compaction trigger`
- TO: `### Requirement: Token-aware compaction trigger with character fallback`

## MODIFIED Requirements

### Requirement: Token-aware compaction trigger with character fallback
The system SHALL use the versioned Token-aware production decision as the authoritative automatic compaction trigger when verified model capacity and a Token measurement are available. When that evidence is unavailable or analysis fails, it SHALL use the existing fixed character-count trigger without inventing capacity or Token values.

#### Scenario: Token threshold is reached
- **WHEN** sufficient Token-aware evidence reports occupancy at or above the versioned threshold
- **THEN** the system SHALL make the context eligible for automatic compaction even if the fixed character threshold has not been crossed

#### Scenario: Token threshold is not reached
- **WHEN** sufficient Token-aware evidence reports occupancy below the versioned threshold
- **THEN** the system SHALL NOT automatically compact even if the fixed character threshold has been crossed

#### Scenario: Unknown capacity uses character fallback
- **WHEN** the active model has no verified context-window metadata
- **THEN** the system SHALL use the fixed character-count trigger

#### Scenario: Character-only evidence uses character fallback
- **WHEN** the request snapshot cannot produce a Token measurement
- **THEN** the system SHALL use the fixed character-count trigger

#### Scenario: Triggered context has no safe reduction boundary
- **WHEN** the authoritative trigger is true but the prepared context has no old complete API round eligible for compaction
- **THEN** the system SHALL send the prepared context unchanged
- **AND** it SHALL record a bounded insufficient-context reason

#### Scenario: Below threshold, no compaction
- **WHEN** the authoritative Token-aware decision or character fallback is below its threshold
- **THEN** the system SHALL send the request unmodified

#### Scenario: Threshold crossed by session history
- **WHEN** the complete initial request context, including session history, crosses the authoritative threshold
- **THEN** the system SHALL make it eligible for compaction before the first provider request of that generation

#### Scenario: Threshold crossed during a tool-use loop
- **WHEN** context accumulated during a generation's tool-use loop crosses the authoritative threshold
- **THEN** the system SHALL make it eligible for compaction before the loop's next provider request

#### Scenario: Shadow decision does not control compaction
- **WHEN** diagnostics compare the former shadow result with the legacy character result after production promotion
- **THEN** the system SHALL treat the versioned production selector, rather than a separately computed shadow outcome, as authoritative
- **AND** it SHALL preserve both outcomes as bounded comparison evidence

### Requirement: Summarization compaction
When the authoritative automatic compaction trigger is eligible and no compaction control suppresses it, the system SHALL first attempt provider-neutral context optimization using the classified snapshot, ordered low-cost reductions, structured summarization only when needed, and post-optimization verification. The existing summary-only behavior SHALL remain available as a compatibility fallback, and the most recent required context SHALL remain unchanged.

#### Scenario: Optimizer handles triggered compaction
- **WHEN** the authoritative trigger makes automatic compaction eligible and compaction controls allow it
- **THEN** the system SHALL attempt to build and verify an optimized candidate from the complete context snapshot
- **AND** it SHALL use that candidate only if all safety and reduction postconditions pass

#### Scenario: Optimizer requires summarization
- **WHEN** low-cost optimizer actions cannot meet the target budget
- **THEN** the system SHALL call the configured provider at most once for the initial structured summary attempt
- **AND** the summarization request SHALL NOT declare tools
- **AND** recent protected and verbatim context SHALL remain unchanged

#### Scenario: Older turns replaced by a summary
- **WHEN** the verified optimizer plan selects older complete API rounds for structured summarization
- **THEN** the system SHALL call the configured provider once for that optimizer summary attempt
- **AND** it SHALL replace only those selected rounds with one structured continuation summary boundary
- **AND** protected, verbatim, and recent retained context SHALL remain unchanged

#### Scenario: Summarization call does not declare tools
- **WHEN** the optimizer or compatibility path makes a summarization call
- **THEN** the request SHALL NOT declare any tools

#### Scenario: Optimizer falls back safely
- **WHEN** optimizer planning, reduction, reinjection, summarization, or verification fails
- **THEN** the system SHALL run the existing summary-only compaction against the untouched original turns
- **AND** the optimizer failure SHALL NOT fail the owning generation

#### Scenario: Compaction control suppresses an eligible trigger
- **WHEN** the authoritative trigger is eligible but request suppression, cooldown, or the failure circuit prevents automatic compaction
- **THEN** the system SHALL NOT call either optimizer summarization or compatibility summarization
- **AND** it SHALL send the prepared context unchanged

#### Scenario: Active trigger remains character based
- **WHEN** Token-aware evidence is insufficient to produce a production decision
- **THEN** the active trigger SHALL remain the existing fixed character-count decision for that request
- **AND** it SHALL record character fallback as the authoritative source
