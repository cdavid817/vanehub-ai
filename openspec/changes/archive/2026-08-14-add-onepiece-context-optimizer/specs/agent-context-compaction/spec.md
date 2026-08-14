## MODIFIED Requirements

### Requirement: Summarization compaction
When compaction triggers, the system SHALL first attempt provider-neutral context optimization using the classified snapshot, ordered low-cost reductions, structured summarization only when needed, and post-optimization verification. The existing summary-only behavior SHALL remain available as a compatibility fallback. The most recent required context SHALL remain unchanged, and this optimization phase SHALL NOT change the active character-count trigger.

#### Scenario: Optimizer handles triggered compaction
- **WHEN** the existing character-count rule triggers compaction
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

#### Scenario: Active trigger remains character based
- **WHEN** context measurement or optimizer evidence recommends a different trigger outcome
- **THEN** the system SHALL continue to follow the existing fixed character-count trigger
- **AND** it SHALL NOT automatically trigger or suppress compaction from Token-aware evidence in this phase
