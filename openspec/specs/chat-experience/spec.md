# chat-experience Specification

## Purpose
Defines the main-window chat experience, including prompt submission, selector-driven chat configuration, conversation history rendering, streamed assistant output, cancellation, persistence, and service boundary rules.

## Requirements
### Requirement: Chat input submits user messages
The system SHALL allow the user to submit a non-empty text message from the main chat input for the active session through the frontend agent service.

#### Scenario: Submit non-empty message
- **WHEN** an active session is selected and the chat input contains non-whitespace text
- **THEN** submitting the input SHALL send the message through the frontend agent service
- **AND** the submitted user message SHALL appear in the active session message list
- **AND** the input SHALL be cleared

#### Scenario: Do not submit empty message
- **WHEN** the chat input is empty or contains only whitespace
- **THEN** the send action SHALL be disabled or ignored
- **AND** no message SHALL be sent

#### Scenario: Preserve IME composition
- **WHEN** the user presses Enter while native IME composition is active
- **THEN** the system SHALL NOT submit the message
- **AND** the input composition SHALL continue normally

### Requirement: Chat configuration remains valid
The system SHALL keep chat configuration values valid when provider, agent, model, mode, or reasoning selections change.

#### Scenario: Provider change resets dependent selections
- **WHEN** the user changes the active provider
- **THEN** the system SHALL select a valid default model for that provider
- **AND** the system SHALL select a valid agent for that provider when one is available
- **AND** the reasoning depth SHALL be adjusted to a value supported by the selected model

#### Scenario: Unsupported reasoning is hidden
- **WHEN** the selected model does not support reasoning
- **THEN** the reasoning selector SHALL NOT be shown

#### Scenario: Unsupported permission mode is unavailable
- **WHEN** the active provider does not support a permission mode
- **THEN** that permission mode SHALL NOT be selectable

### Requirement: Message list displays conversation history
The system SHALL display chat messages for the active session in chronological order.

#### Scenario: Empty session shows welcome screen
- **WHEN** the active session has no messages
- **THEN** the main chat area SHALL show the welcome screen
- **AND** no message item SHALL be shown

#### Scenario: Existing messages are listed
- **WHEN** the active session has existing messages
- **THEN** the message list SHALL display them in chronological order
- **AND** each message SHALL use role-appropriate rendering

#### Scenario: Load earlier messages
- **WHEN** the active session has more messages than the initial page size and the user requests earlier messages
- **THEN** older messages SHALL be loaded before the current first message
- **AND** the current scroll position SHALL remain stable

### Requirement: Assistant responses stream into the message list
The system SHALL display assistant responses incrementally as stream events arrive through the frontend agent service.

#### Scenario: Assistant response starts
- **WHEN** the agent service emits a `started` event for the active session
- **THEN** an assistant message with `streaming` status SHALL appear
- **AND** a waiting indicator SHALL be visible until response content arrives

#### Scenario: Token event appends content
- **WHEN** the agent service emits a `token` event for a streaming assistant message
- **THEN** the token content SHALL be appended to that assistant message
- **AND** the message SHALL NOT be duplicated

#### Scenario: Thinking event appends thinking content
- **WHEN** the agent service emits a `thinking` event for a streaming assistant message
- **THEN** the thinking content SHALL be appended to a collapsible thinking block for that message

#### Scenario: Tool event appends tool use
- **WHEN** the agent service emits a `tool_use` event for a streaming assistant message
- **THEN** the tool-use data SHALL be shown in a collapsible tool-use block for that message

#### Scenario: Completion marks message complete
- **WHEN** the agent service emits a `completed` event for a streaming assistant message
- **THEN** the assistant message status SHALL become `completed`
- **AND** the waiting indicator SHALL be hidden

#### Scenario: Failure marks message failed
- **WHEN** the agent service emits a `failed` event for a streaming assistant message
- **THEN** the assistant message status SHALL become `failed`
- **AND** the error SHALL be visible to the user

### Requirement: User can stop active generation
The system SHALL allow the user to stop the active assistant generation for the current session.

#### Scenario: Stop while streaming
- **WHEN** an assistant response is streaming and the user activates the stop action
- **THEN** the system SHALL request cancellation through the frontend agent service
- **AND** the active assistant message SHALL be marked `cancelled`
- **AND** already generated content SHALL remain visible

#### Scenario: Stop has no effect when idle
- **WHEN** no assistant response is active and stop is requested
- **THEN** no backend cancellation SHALL be performed
- **AND** the chat input SHALL remain idle

### Requirement: Messages persist in desktop runtime
The desktop runtime SHALL persist chat messages for each session through the Rust/Tauri SQLite layer.

#### Scenario: Persist completed conversation
- **WHEN** the user sends a message and the assistant response completes in the desktop runtime
- **THEN** the user message SHALL be stored with the active session id
- **AND** the assistant message SHALL be stored with the active session id and `completed` status
- **AND** both messages SHALL be returned when the session messages are listed

#### Scenario: Persist failed message state
- **WHEN** an assistant response fails during generation in the desktop runtime
- **THEN** the assistant message SHALL be stored with `failed` status
- **AND** diagnostic metadata SHALL be retained when available

#### Scenario: Preserve Web runtime behavior
- **WHEN** the app runs in Web mode
- **THEN** the Web adapter SHALL provide the same message service contract without requiring SQLite or local Agent CLI access

### Requirement: Components use the chat service boundary
The system SHALL keep chat message operations behind the frontend agent service boundary.

#### Scenario: React sends message
- **WHEN** React UI code sends, lists, stops, or subscribes to chat messages
- **THEN** it SHALL call the frontend agent service interface
- **AND** it SHALL NOT call Tauri `invoke()` directly

#### Scenario: Tauri adapter handles native calls
- **WHEN** the desktop frontend performs a chat message operation
- **THEN** Tauri `invoke()` and event listening SHALL remain inside the Tauri-specific frontend adapter
