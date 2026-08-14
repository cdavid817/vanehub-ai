## RENAMED Requirements

- FROM: `### Requirement: Non-mutating shadow decision`
- TO: `### Requirement: Token-aware production decision`

## MODIFIED Requirements

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
