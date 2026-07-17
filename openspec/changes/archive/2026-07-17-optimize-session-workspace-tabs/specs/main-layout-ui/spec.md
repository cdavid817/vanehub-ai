## MODIFIED Requirements

### Requirement: Flexible main content area
The main content panel SHALL render a session-tab workspace that keeps Chat as the default workflow, resizes with the available workspace area, and keeps the bottom composer usable and connected to the active session message list only while Chat is active.

#### Scenario: Chat transcript flexes with panel height
- **WHEN** the workspace panel height changes while Chat is active
- **THEN** the chat transcript area SHALL flex to fill the remaining main content space without a fixed minimum height forcing overflow

#### Scenario: Chat transcript scrolls internally
- **WHEN** chat message content exceeds the available transcript height
- **THEN** the transcript SHALL scroll inside the main content panel without scrolling the whole workspace shell

#### Scenario: Composer remains fixed
- **WHEN** the main content panel becomes shorter while Chat is active
- **THEN** the bottom composer SHALL retain its usable size and SHALL remain within the main content panel bounds

#### Scenario: Hide composer outside Chat
- **WHEN** the active session workspace tab is not Chat
- **THEN** the bottom composer SHALL NOT be visible and the active tab content SHALL use the released space

#### Scenario: Main content expands after panel collapse
- **WHEN** the information panel is collapsed
- **THEN** the main content panel SHALL smoothly expand to occupy the space released by the information panel

#### Scenario: Message list renders for active session
- **WHEN** an active session is selected and Chat is active
- **THEN** the main content panel SHALL render the message list for that active session above the composer

#### Scenario: Composer sends to active session
- **WHEN** the user submits the bottom composer
- **THEN** the submitted chat message SHALL target the active session

#### Scenario: Preserve compact right-panel context
- **WHEN** the main session workspace renders full Files or Changes content
- **THEN** the right information panel SHALL retain its compact Agent Info, Files, and Changes overview tabs

