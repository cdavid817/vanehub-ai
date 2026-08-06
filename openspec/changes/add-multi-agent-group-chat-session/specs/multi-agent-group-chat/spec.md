## ADDED Requirements

### Requirement: Seat assignment
A multi-Agent session SHALL be composed of seats, each pairing one expert role with one Agent, so a role is reusable across sessions and an Agent may play different roles in different sessions.

#### Scenario: Create a multi-Agent session
- **WHEN** a user selects the multi Agent mode and assigns at least two seats
- **THEN** the system SHALL create a session carrying those seats in the assigned order
- **AND** each seat SHALL expose its role identity, its Agent, and its normalized model family

#### Scenario: Reject a seat with an unavailable Agent
- **WHEN** a user assigns a seat to an Agent that is unavailable
- **THEN** the system SHALL reject the assignment with a localized reason and SHALL NOT create the session

#### Scenario: Recommend a cross-family reviewer
- **WHEN** a user assigns a seat whose role requires a different model family for review
- **THEN** the system SHALL recommend Agents whose normalized family differs from the Agent under review
- **AND** when no cross-family Agent is available the system SHALL still offer same-family Agents together with an explicit notice that the cross-family preference could not be satisfied

### Requirement: Shared thread with speaker identity
Every message in a multi-Agent session SHALL carry the identity of its speaker, and the system SHALL render that identity.

#### Scenario: Agent message is attributed
- **WHEN** a seat's Agent produces a message
- **THEN** the message SHALL record the seat that produced it
- **AND** the rendered message SHALL show the role avatar, the role colour, and a label naming both the role and the Agent

#### Scenario: Human message is attributed
- **WHEN** the user sends a message
- **THEN** the message SHALL be attributed to the user rather than to any seat

### Requirement: Agent-to-Agent handoff
After a seat's Agent completes a reply, the system SHALL route the turn to any seat named by a line-leading mention in that reply.

#### Scenario: Reply hands off to another seat
- **WHEN** a completed Agent reply contains a mention of another seat at the start of a line
- **THEN** the system SHALL route the turn to that seat and record the handoff
- **AND** the receiving Agent SHALL be given the preceding turns of the thread within its context budget

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
- **AND** the system SHALL NOT run seats concurrently within a round

### Requirement: Handoff to the human
An Agent SHALL be able to hand the turn to the human with an explicit intent, and the intent SHALL determine whether work pauses.

#### Scenario: Informational handoff does not interrupt
- **WHEN** an Agent hands to the human with an informational intent
- **THEN** the turn SHALL remain with the Agents and work SHALL continue
- **AND** the system SHALL NOT block the composer or raise a blocking prompt

#### Scenario: Blocking handoff pauses the round
- **WHEN** an Agent hands to the human with a blocking intent
- **THEN** the turn SHALL transfer to the human, no further seat SHALL be invoked, and a waiting duration SHALL begin accumulating

#### Scenario: Completion handoff ends the round
- **WHEN** an Agent hands to the human with a completion intent
- **THEN** the round SHALL be recorded as complete and no further seat SHALL be invoked

### Requirement: Turn ownership visibility
The system SHALL show who currently holds the turn throughout a multi-Agent session.

#### Scenario: A seat holds the turn
- **WHEN** a seat is producing a reply
- **THEN** the session SHALL display that seat as the turn holder and the chain position against its configured limit

#### Scenario: The human holds a paused turn
- **WHEN** a blocking handoff has transferred the turn to the human
- **THEN** the session SHALL display that the turn is waiting on the user and how long it has been waiting

### Requirement: Human routing by mention
The user SHALL direct a message to a specific seat using a line-leading mention, and the system SHALL NOT provide any control for assigning work to a seat by other means.

#### Scenario: User mentions a seat
- **WHEN** the user sends a message beginning with a mention of a seat
- **THEN** the system SHALL route that message to the mentioned seat

#### Scenario: User mentions nobody
- **WHEN** the user sends a message containing no line-leading mention
- **THEN** the system SHALL route it to the seat that most recently held the turn
- **AND** when no seat has held the turn yet, the system SHALL route it to the first seat

#### Scenario: No dispatch control is offered
- **WHEN** a multi-Agent session is displayed
- **THEN** the interface SHALL NOT offer a control that selects which seat speaks next, because routing belongs to the Agents and to mentions
