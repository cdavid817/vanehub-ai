## ADDED Requirements

### Requirement: Desktop chat uses session runtime execution
Desktop chat generation SHALL be produced through a session-scoped real Agent CLI runtime execution path rather than a hard-coded preview or mock response.

#### Scenario: Send message to available runtime
- **WHEN** a user sends a message in the desktop runtime for a session whose selected Agent CLI is supported and installed
- **THEN** the desktop runtime SHALL run the message through the session-scoped real CLI runtime path
- **AND** stream events SHALL update the assistant message for that same session

#### Scenario: Runtime unavailable
- **WHEN** a user sends a message in the desktop runtime and the selected Agent CLI is unavailable, not installed, or unsupported
- **THEN** the user message SHALL remain persisted
- **AND** the assistant message SHALL be marked `failed`
- **AND** the failure SHALL identify the unavailable runtime without returning a fake or preview successful answer
- **AND** the chat UI SHALL show a concise user-facing error while detailed diagnostics are written to unified logs

### Requirement: Message status and session status stay synchronized
The chat service SHALL keep persisted message status and owning session lifecycle synchronized during generation.

#### Scenario: Streaming begins
- **WHEN** an assistant message starts streaming
- **THEN** the assistant message SHALL have `streaming` status
- **AND** the owning session SHALL have an active lifecycle state

#### Scenario: Streaming completes
- **WHEN** an assistant message completes
- **THEN** the assistant message SHALL have `completed` status
- **AND** the owning session SHALL no longer be marked running

#### Scenario: Streaming fails or is cancelled
- **WHEN** an assistant message fails or is cancelled
- **THEN** the assistant message SHALL retain already captured content and terminal status
- **AND** the owning session lifecycle SHALL reflect the failure or stopped state
