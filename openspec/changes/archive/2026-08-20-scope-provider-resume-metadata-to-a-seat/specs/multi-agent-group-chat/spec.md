## MODIFIED Requirements

### Requirement: Agent-to-Agent handoff
After a seat's Agent completes a reply, the system SHALL route the turn to any seat named by a line-leading mention in that reply. A routed seat SHALL take its turn on its own Agent's provider thread, so a handoff between seats on different Agents produces a reply rather than a failed turn.

#### Scenario: Reply hands off to another seat
- **WHEN** a completed Agent reply contains a mention of another seat at the start of a line
- **THEN** the system SHALL route the turn to that seat and record the handoff
- **AND** the receiving Agent SHALL be given the preceding turns of the thread within its context budget

#### Scenario: Handoff crosses to a different Agent
- **WHEN** the routed seat's Agent differs from the Agent that has already spoken in the session
- **THEN** the routed seat SHALL take its turn without resuming a provider thread belonging to another seat's Agent
- **AND** the turn SHALL produce a reply attributed to the routed seat

#### Scenario: Mention is not at the start of a line
- **WHEN** a mention appears anywhere other than the start of a line
- **THEN** the system SHALL treat it as ordinary text and SHALL NOT route the turn

#### Scenario: Mention appears inside a fenced code block
- **WHEN** a mention appears inside a fenced code block
- **THEN** the system SHALL ignore it for routing

#### Scenario: Handoff is bounded
- **WHEN** an Agent-to-Agent chain reaches its configured maximum depth, or a single reply mentions more than the configured maximum number of seats, or a reply mentions its own seat
- **THEN** the system SHALL stop extending the chain, SHALL NOT route to the excess or self mentions, and SHALL surface why the chain ended

#### Scenario: Handoffs resolve serially
- **WHEN** more than one seat is routed within a round
- **THEN** the seats SHALL respond one at a time, each seeing the preceding replies
