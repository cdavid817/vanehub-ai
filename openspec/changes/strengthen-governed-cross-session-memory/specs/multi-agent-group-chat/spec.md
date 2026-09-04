## ADDED Requirements

### Requirement: Memory episodes aggregate multi-Agent extraction
The system SHALL maintain a memory episode as the aggregation boundary for multi-Agent automatic memory extraction. An episode SHALL group one or more execution rounds of one session, reusing the existing `seat_round_id` for rounds rather than defining a parallel round identity. A handoff to the human SHALL pause the episode, and the new execution round created by the user's reply SHALL be linked to the same episode. While an episode is open, seat turns SHALL accumulate extraction evidence only and per-seat automatic extraction SHALL be suppressed; one bounded aggregate extraction SHALL run when the episode reaches a terminal state, deduplicating against explicit `remember` candidates produced during the episode.

#### Scenario: Seat turns do not extract individually during an episode
- **WHEN** several seats complete successful turns inside one open memory episode
- **THEN** the system SHALL record evidence for those turns without running per-seat automatic extraction
- **AND** one aggregate extraction SHALL run at the episode's terminal state

#### Scenario: A handoff to the human does not split the episode
- **WHEN** a round ends with a handoff to the human and the user replies
- **THEN** the reply's new execution round SHALL be linked to the originating episode
- **AND** the aggregate extraction at terminal state SHALL cover evidence from both rounds

#### Scenario: Explicit remember proposals are deduplicated
- **WHEN** an Agent produced an explicit `remember` candidate during the episode and the aggregate extraction proposes the same fact
- **THEN** the aggregate SHALL deduplicate against the explicit candidate rather than queueing a second proposal
