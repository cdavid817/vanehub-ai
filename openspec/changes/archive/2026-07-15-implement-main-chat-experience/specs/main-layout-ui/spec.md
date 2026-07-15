## MODIFIED Requirements

### Requirement: Flexible main content area
The main content panel SHALL render a chat-first workspace area that resizes with the available workspace area while keeping the bottom composer usable and connected to the active session message list.

#### Scenario: Chat transcript flexes with panel height
- **WHEN** the workspace panel height changes
- **THEN** the chat transcript area SHALL flex to fill the remaining main content space without a fixed minimum height forcing overflow

#### Scenario: Chat transcript scrolls internally
- **WHEN** chat message content exceeds the available transcript height
- **THEN** the transcript SHALL scroll inside the main content panel without scrolling the whole workspace shell

#### Scenario: Composer remains fixed
- **WHEN** the main content panel becomes shorter
- **THEN** the bottom composer SHALL retain its usable size and SHALL remain within the main content panel bounds

#### Scenario: Main content expands after panel collapse
- **WHEN** the information panel is collapsed
- **THEN** the main content panel SHALL smoothly expand to occupy the space released by the information panel

#### Scenario: Message list renders for active session
- **WHEN** an active session is selected
- **THEN** the main content panel SHALL render the message list for that active session above the composer

#### Scenario: Composer sends to active session
- **WHEN** the user submits the bottom composer
- **THEN** the submitted chat message SHALL target the active session
