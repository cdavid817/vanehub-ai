## MODIFIED Requirements

### Requirement: Turn ownership visibility
The system SHALL show who currently holds the turn throughout a multi-Agent session and SHALL distinguish an active member that has not emitted reply text yet from an idle or stalled participant.

#### Scenario: A seat holds the turn
- **WHEN** a seat is producing a reply
- **THEN** the session SHALL display that seat as the turn holder and the chain position against its configured limit
- **AND** SHALL show an active execution indication before the first visible token arrives

#### Scenario: The human holds a paused turn
- **WHEN** a blocking handoff has transferred the turn to the human
- **THEN** the session SHALL display that the turn is waiting on the user and how long it has been waiting

#### Scenario: A member produces activity before text
- **WHEN** the current seat starts, thinks, or invokes a tool before emitting response text
- **THEN** the interface SHALL update that member's bounded activity state so the user can distinguish execution from inactivity

### Requirement: Active roster presence
The system SHALL expose the active multi-Agent roster and each participant's current state without turning the roster into a dispatch control. The current state SHALL be derived from stable seat-attributed turn and message events and SHALL distinguish idle, starting or active, producing output, waiting, completed, and failed states where those events are known.

#### Scenario: Show the active roster
- **WHEN** a multi-Agent session is displayed
- **THEN** the interface SHALL show each active participant's role, Agent, and current state
- **AND** it SHALL visually distinguish the current turn holder from idle participants

#### Scenario: Show member output progress
- **WHEN** the current participant receives thinking, tool, token, or terminal events
- **THEN** that participant's roster state SHALL update without waiting for the whole multi-Agent round to finish

#### Scenario: Keep routing conversational
- **WHEN** the active roster is displayed
- **THEN** the roster SHALL NOT provide a control that selects the next speaker
- **AND** routing SHALL remain driven by line-leading mentions and Agent handoff

