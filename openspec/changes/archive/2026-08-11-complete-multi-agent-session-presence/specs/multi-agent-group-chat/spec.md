## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Active roster presence
The system SHALL expose the active multi-Agent roster and each participant's current state without turning the roster into a dispatch control.

#### Scenario: Show the active roster
- **WHEN** a multi-Agent session is displayed
- **THEN** the interface SHALL show each active participant's role, Agent, and current state
- **AND** it SHALL visually distinguish the current turn holder from idle participants

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
