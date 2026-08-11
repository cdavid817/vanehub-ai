## MODIFIED Requirements

### Requirement: Message list displays conversation history
The system SHALL display chat messages for the active session in chronological order and attribute every Agent message through an immutable speaker identity.

#### Scenario: Empty session shows welcome screen
- **WHEN** the active session has no messages
- **THEN** the main chat area SHALL show the welcome screen
- **AND** no message item SHALL be shown

#### Scenario: Existing messages are listed
- **WHEN** the active session has existing messages
- **THEN** the message list SHALL display them in chronological order
- **AND** each message SHALL use role-appropriate rendering

#### Scenario: Multi-seat messages are attributed
- **WHEN** the active session has held more than one participant
- **THEN** each Agent message SHALL render the speaking participant's captured role avatar, role colour, and a label naming both the role and the Agent
- **AND** leaving or reordering the active roster SHALL NOT change historical attribution
- **AND** a participant recommended as a cross-family reviewer SHALL be marked as such

#### Scenario: Single-seat messages keep their existing presentation
- **WHEN** the active session has never held more than one participant
- **THEN** message presentation SHALL remain unchanged from the single-Agent experience

#### Scenario: Load earlier messages
- **WHEN** the active session has more messages than the initial page size and the user requests earlier messages
- **THEN** older messages SHALL be loaded before the current first message
- **AND** the current scroll position SHALL remain stable

#### Scenario: Preserve the visible conversation while the workspace resizes
- **WHEN** focus mode or a workspace visibility control changes the message viewport width
- **THEN** chronological message order SHALL remain unchanged
- **AND** a reader near the latest message SHALL remain pinned to the bottom
- **AND** a reader reviewing history SHALL retain the preceding bottom offset instead of jumping to another part of the thread

## ADDED Requirements

### Requirement: Non-duplicative conversation header
The chat header SHALL present session identity, runtime state, and conversation actions without duplicating member identity that belongs in the information panel.

#### Scenario: Keep multi-Agent identity out of the conversation header
- **WHEN** the active session holds more than one participant
- **THEN** the chat header SHALL show the session title and bounded multi-Agent summary
- **AND** it SHALL NOT render participant role chips, Agent names, or CLI ids
- **AND** member details SHALL remain available in the information panel

#### Scenario: Keep single-Agent identity out of the conversation header
- **WHEN** the active session holds one participant and has no departed participants
- **THEN** the chat header SHALL show the session title and interaction mode
- **AND** it SHALL NOT render a participant or CLI identity row

#### Scenario: Present a desktop messaging hierarchy
- **WHEN** the chat tab is displayed on a desktop viewport
- **THEN** the session identity and runtime state SHALL remain in a stable top header
- **AND** the message canvas SHALL use the available conversation width with bounded adaptive edge gutters
- **AND** individual message bubbles SHALL retain a readable maximum width
- **AND** the composer SHALL attach to the conversation bottom with a quiet top divider instead of floating as a separate card
- **AND** member details SHALL remain available from the information panel without reducing the message area unnecessarily

#### Scenario: Use released panel width without oversized blank margins
- **WHEN** focus mode or an overflow action collapses an adjacent workspace panel
- **THEN** the message canvas SHALL expand with the conversation surface
- **AND** it SHALL NOT retain a fixed centered width that creates oversized blank margins on both sides
- **AND** assistant and user bubbles SHALL remain aligned to their respective conversation edges

#### Scenario: Preserve header alignment in focus mode
- **WHEN** focus mode collapses the surrounding panels
- **THEN** the session title, runtime state, and overflow actions SHALL retain their relative order and alignment
- **AND** the workspace SHALL NOT animate layout-affecting grid tracks that can transiently reorder header content

### Requirement: Unified composer completion
The composer SHALL provide distinguishable completion results for participant routing and file references without allowing one kind to be mistaken for the other.

#### Scenario: Present one integrated composer surface
- **WHEN** the chat composer is available
- **THEN** it SHALL provide a spacious borderless editor within one quiet bordered container
- **AND** runtime selectors and message actions SHALL remain in a bottom toolbar inside that container
- **AND** selected references, completion, keyboard submission, disabled state, and visible keyboard focus SHALL remain available

#### Scenario: Complete a participant mention
- **WHEN** the user types `@` at the start of a line in a multi-Agent session
- **THEN** completion SHALL list active participants with role, Agent, and model-family identity
- **AND** selecting a participant SHALL insert its unique routing handle

#### Scenario: Complete a file reference
- **WHEN** the user requests file completion
- **THEN** completion SHALL identify results as files and SHALL preserve file attachment behavior
- **AND** a file result SHALL NOT be interpreted as a participant route

#### Scenario: Exclude departed participants
- **WHEN** a participant has left the session
- **THEN** participant completion SHALL NOT offer that participant as a routing target

### Requirement: Responsive message submission feedback
The composer SHALL acknowledge a valid submission immediately while preserving recoverability when the service rejects it.

#### Scenario: Optimistically display a submitted user message
- **WHEN** the user submits a valid message
- **THEN** the draft and selected references SHALL clear immediately
- **AND** the shared thread SHALL immediately display a temporary user message without waiting for native prompt assembly or CLI launch
- **AND** the send action SHALL remain protected from duplicate submission while the command is pending

#### Scenario: Roll back a rejected optimistic message
- **WHEN** the message service rejects the submission
- **THEN** the temporary user message SHALL be removed
- **AND** the submitted draft and file references SHALL be restored
- **AND** the existing localized error feedback SHALL remain available
