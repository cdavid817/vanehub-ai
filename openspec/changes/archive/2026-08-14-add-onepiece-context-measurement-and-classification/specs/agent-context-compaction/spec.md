## MODIFIED Requirements

### Requirement: Character-count compaction trigger
The system SHALL measure a generation's accumulated turns by summed character count and SHALL trigger compaction when that count exceeds a fixed threshold. During the context-measurement introduction phase, this character-count decision SHALL remain authoritative, and any token-aware shadow decision SHALL NOT trigger, suppress, or otherwise change compaction. The system SHALL NOT depend on real provider-reported token counts to make the active determination in this phase.

#### Scenario: Below threshold, no compaction
- **WHEN** a generation's accumulated turns are below the character-count threshold
- **THEN** the system SHALL send the request unmodified even if a shadow decision recommends compaction

#### Scenario: Threshold crossed by session history
- **WHEN** a session's conversation history alone exceeds the character-count threshold
- **THEN** the system SHALL compact before sending the first request of that generation

#### Scenario: Threshold crossed during a tool-use loop
- **WHEN** turns accumulated during a generation's tool-use loop cause the running character total to exceed the threshold
- **THEN** the system SHALL compact before sending the loop's next request

#### Scenario: Shadow decision does not control compaction
- **WHEN** the token-aware shadow decision and character-count decision disagree
- **THEN** the system SHALL follow the character-count decision
- **AND** it SHALL leave the request and resulting compaction behavior unchanged by the shadow result

