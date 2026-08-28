# multi-agent-group-chat Specification

## Purpose
TBD - created by archiving change add-multi-agent-group-chat-session. Update Purpose after archive.
## Requirements
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

### Requirement: Seats change while a session runs

The system SHALL allow seats to be added to and removed from a running session while preserving a stable identity and history for every participant that has joined the session.

#### Scenario: Add a seat mid-session

- **WHEN** a user adds a seat to a running session
- **THEN** the system SHALL assign the seat a stable identity that is not derived from its current array position
- **AND** the seat SHALL become routable from the next turn onward
- **AND** its Agent SHALL receive the preceding turns of the thread within its context budget

#### Scenario: Remove a seat mid-session

- **WHEN** a user removes a seat from a running session
- **THEN** the seat SHALL stop being routable and SHALL stop appearing in the active roster published to other seats
- **AND** the participant identity SHALL remain available for rendering its historical messages
- **AND** another seat SHALL NOT reuse that identity

#### Scenario: A session always keeps at least one seat

- **WHEN** a user attempts to remove the only remaining active seat
- **THEN** the system SHALL reject the removal

#### Scenario: Removing the first seat updates the mirrored agent id

- **WHEN** the first active seat leaves a session holding more than one active seat
- **THEN** the session's agent id SHALL be updated to the next active seat's agent id

#### Scenario: Growing a single-Agent session

- **WHEN** a user adds a second seat to a session that holds exactly one active seat
- **THEN** the session SHALL become a multi-seat session without being recreated
- **AND** its existing messages SHALL retain their original attribution

### Requirement: Shared thread with speaker identity
Every message in a multi-Agent session SHALL identify its speaker with a stable participant identity, and the system SHALL render the identity captured for that participant without resolving it through a mutable seat position.

#### Scenario: Agent message is attributed
- **WHEN** a seat's Agent produces a message
- **THEN** the message SHALL record the stable identity of the seat that produced it
- **AND** the rendered message SHALL show the captured role avatar, role colour, and a label naming both the role and the Agent

#### Scenario: Removed participant remains attributable
- **WHEN** a participant leaves after producing messages
- **THEN** every earlier message SHALL continue to render that participant's original role and Agent identity
- **AND** no earlier message SHALL be relabelled as another participant

#### Scenario: Role asset changes after participation
- **WHEN** an expert role is renamed, recoloured, or deleted after a participant joins a session
- **THEN** messages already attributed to that participant SHALL retain the session-captured role presentation

#### Scenario: Human message is attributed
- **WHEN** the user sends a message
- **THEN** the message SHALL be attributed to the user rather than to any seat

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

### Requirement: Role briefing launch compatibility
The system SHALL deliver multi-Agent role briefing to supported CLI runtimes without routing structured arguments through an unsafe or incompatible shell representation.

#### Scenario: Launch Codex from a Windows npm installation
- **WHEN** Codex is detected through a recognized Windows npm batch shim and its role briefing contains multiline text
- **THEN** structured generation SHALL invoke the packaged native Codex executable with separate arguments
- **AND** the launch SHALL NOT fail with a batch-file argument validation error
- **AND** unknown or incomplete installations SHALL retain the existing diagnosable fallback behavior

### Requirement: Delegated group execution uses child Runs
Multi-Agent delegated execution SHALL use parent/child canonical Run links while Seat assignment, turn ownership, speaker identity, and human routing remain owned by group chat.

#### Scenario: Delegated turn is cancelled by parent
- **WHEN** the parent generation is cancelled during a delegated turn
- **THEN** the child Run is cancelled without changing persisted Seat or speaker semantics

### Requirement: IM-originated human routing
An IM-originated user message in a multi-Agent session SHALL use the same stable seat identities and conversational routing semantics as a desktop-originated user message.

#### Scenario: IM user mentions one active seat
- **WHEN** an accepted IM message begins with exactly one valid active-seat mention
- **THEN** the system SHALL route the turn to that seat and preserve the IM origin for final-response delivery

#### Scenario: IM user does not mention a seat
- **WHEN** an accepted IM message has no valid line-leading seat mention
- **THEN** the system SHALL route it to the current conversational owner or the first active seat when no owner exists

#### Scenario: Agent handoff follows an IM turn
- **WHEN** the receiving Agent completes with a valid line-leading mention of another active seat
- **THEN** the existing bounded serial Agent-to-Agent handoff SHALL continue
- **AND** only the terminal response for the accepted external turn SHALL be delivered to its originating IM chat

#### Scenario: IM surface offers no dispatch override
- **WHEN** a multi-Agent session is enabled for IM
- **THEN** neither the information panel nor the external chat guidance SHALL offer a separate next-speaker selector
