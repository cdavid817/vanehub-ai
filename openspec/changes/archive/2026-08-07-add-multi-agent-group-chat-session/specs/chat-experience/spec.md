## MODIFIED Requirements

### Requirement: Message list displays conversation history
The system SHALL display chat messages for the active session in chronological order, attributing each message to its speaker.

#### Scenario: Empty session shows welcome screen
- **WHEN** the active session has no messages
- **THEN** the main chat area SHALL show the welcome screen
- **AND** no message item SHALL be shown

#### Scenario: Existing messages are listed
- **WHEN** the active session has existing messages
- **THEN** the message list SHALL display them in chronological order
- **AND** each message SHALL use role-appropriate rendering

#### Scenario: Multi-seat messages are attributed
- **WHEN** the active session holds more than one seat
- **THEN** each Agent message SHALL render the speaking seat's role avatar, role colour, and a label naming both the role and the Agent
- **AND** a seat recommended as a cross-family reviewer SHALL be marked as such

#### Scenario: Single-seat messages keep their existing presentation
- **WHEN** the active session holds exactly one seat
- **THEN** message presentation SHALL remain unchanged from the single-Agent experience

#### Scenario: Load earlier messages
- **WHEN** the active session has more messages than the initial page size and the user requests earlier messages
- **THEN** older messages SHALL be loaded before the current first message
- **AND** the current scroll position SHALL remain stable

## ADDED Requirements

### Requirement: Composer seat mention completion
In a multi-seat session the composer SHALL offer completion for seat mentions and SHALL make the line-leading routing rule discoverable.

#### Scenario: Completion lists seats
- **WHEN** the user types a mention trigger in the composer
- **THEN** the composer SHALL list the session's seats with their role name, Agent, and model family
- **AND** selecting one SHALL insert its mention

#### Scenario: Routing rule is discoverable
- **WHEN** the completion list is shown
- **THEN** the composer SHALL indicate that only a mention at the start of a line routes the message

#### Scenario: Single-seat session offers no completion
- **WHEN** the active session holds exactly one seat
- **THEN** the composer SHALL NOT offer seat mention completion
