## ADDED Requirements

### Requirement: Chat messages support durable Rich Blocks
The system SHALL support structured Rich Blocks as durable attachments on chat messages while preserving existing text, thinking, and tool-use message fields.

#### Scenario: List message with persisted Rich Blocks
- **WHEN** a session message contains persisted Rich Blocks
- **THEN** listing messages for that session SHALL return the message with its `richBlocks` in stable order
- **AND** the existing `content`, `thinkingContent`, and `toolUse` fields SHALL remain available

#### Scenario: Existing message without Rich Blocks
- **WHEN** a message was created before Rich Block support or has no Rich Blocks
- **THEN** the chat service SHALL return it without requiring a Rich Block payload
- **AND** the message SHALL render using the existing text, thinking, and tool-use behavior

### Requirement: Rich Block stream events append structured blocks
The chat stream SHALL support a `rich_block` event that appends a structured block to the target assistant message through the frontend agent service boundary.

#### Scenario: Receive Rich Block event
- **WHEN** the agent service emits a `rich_block` event for the active session and assistant message
- **THEN** the message list SHALL show the new Rich Block on that assistant message without duplicating the assistant message
- **AND** the Rich Block SHALL remain visible after message data is reloaded

#### Scenario: Receive duplicate Rich Block id
- **WHEN** a `rich_block` event carries a block id that already exists on the target message
- **THEN** the client SHALL update or replace the existing block with that id rather than rendering duplicate blocks

### Requirement: Desktop runtime persists Rich Blocks
The desktop runtime SHALL persist Rich Blocks for chat messages through the Rust/Tauri SQLite layer.

#### Scenario: Persist streamed Rich Block
- **WHEN** the desktop runtime normalizes provider output into a `rich_block` event
- **THEN** the runtime SHALL append the block to the assistant message's persisted Rich Blocks
- **AND** the same block SHALL be returned by `list_messages` after refresh

#### Scenario: Preserve failed or cancelled Rich Blocks
- **WHEN** generation fails or is cancelled after one or more Rich Blocks have been received
- **THEN** already persisted Rich Blocks SHALL remain attached to the assistant message
- **AND** the assistant message SHALL still show its terminal failed or cancelled status

### Requirement: Web runtime preserves Rich Block contract parity
The Web/mock runtime SHALL implement the same Rich Block message and event contract as the desktop runtime.

#### Scenario: Web mock streams Rich Blocks
- **WHEN** the app runs through the Web/mock adapter and emits a mock Rich Block event
- **THEN** the message list SHALL render the block using the same React components as desktop mode
- **AND** re-listing Web/mock messages SHALL retain the mock Rich Block data

### Requirement: Rich Block renderers support first-version block kinds
The chat UI SHALL render supported first-version Rich Block kinds with localized labels and visual styling consistent with both configured visual styles.

#### Scenario: Render supported block kinds
- **WHEN** a message contains `card`, `diff`, `checklist`, `media_gallery`, `file`, `audio`, `html_widget`, or `interactive` Rich Blocks
- **THEN** the chat UI SHALL render each block with a stable layout that does not overlap message text, status labels, or adjacent blocks
- **AND** frontend-owned labels and fallback text SHALL use the active locale

#### Scenario: Render unknown or invalid block
- **WHEN** a message contains an unsupported, unknown, or invalid Rich Block
- **THEN** the chat UI SHALL render a localized fallback that identifies the unsupported block kind when available
- **AND** the rest of the message SHALL remain readable

### Requirement: HTML widget Rich Blocks are sandboxed
The chat UI SHALL render `html_widget` Rich Blocks inside a constrained sandbox rather than injecting provider HTML into the React document.

#### Scenario: Render HTML widget safely
- **WHEN** a message contains an `html_widget` block
- **THEN** the UI SHALL render the block in an iframe or equivalent sandboxed boundary
- **AND** the block height SHALL be bounded to prevent it from breaking the chat layout

### Requirement: Interactive Rich Blocks are read-only in first version
The first Rich Block implementation SHALL treat `interactive` blocks as read-only previews until an explicit interaction contract is added.

#### Scenario: Render interactive preview
- **WHEN** a message contains an `interactive` Rich Block
- **THEN** the UI SHALL show the title, description, and options without sending chat messages or invoking native commands from option clicks
- **AND** the UI SHALL show localized text indicating that interactive actions are not enabled yet
