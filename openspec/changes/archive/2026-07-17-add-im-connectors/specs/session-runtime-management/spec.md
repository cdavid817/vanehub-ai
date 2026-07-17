## ADDED Requirements

### Requirement: Non-activating IM session creation
The native session runtime SHALL support creating an IM-owned session without changing the desktop workflow's active session.

#### Scenario: Create session for external chat
- **WHEN** the IM router creates a session for a new external-chat binding
- **THEN** the session SHALL be persisted with the configured Agent, CLI interaction mode, project path, IM source type, and source connector id
- **AND** `workflow_state.active_session_id` SHALL remain unchanged

### Requirement: IM session continuity
IM-owned sessions SHALL preserve the same provider runtime-session continuity as desktop-created sessions.

#### Scenario: Reuse IM-owned session
- **WHEN** a later external message targets an existing IM binding
- **THEN** the Agent invocation SHALL reuse the bound session and its persisted provider runtime-session id when supported

### Requirement: Shared message execution service
Desktop and IM message submission SHALL use one internal native execution service.

#### Scenario: Desktop submits message after refactor
- **WHEN** the frontend submits a desktop chat message through the existing Agent service
- **THEN** existing message persistence, streaming events, lifecycle state, CLI parsing, token accounting, and error behavior SHALL remain available

#### Scenario: Native IM router submits message
- **WHEN** the IM router submits a message directly to the internal service
- **THEN** it SHALL not require a frontend window, React event listener, or native-to-native Tauri command invocation

### Requirement: IM completion notification
The native chat runtime SHALL expose an internal terminal completion signal for IM-originated assistant messages.

#### Scenario: Assistant completes
- **WHEN** an IM-originated assistant message reaches completed, failed, or cancelled state
- **THEN** the waiting IM job SHALL receive exactly one terminal result associated with the session and assistant message

### Requirement: Deleted IM session recovery
Deleting an IM-owned session SHALL not leave a permanently unusable external-chat binding.

#### Scenario: User deletes IM session
- **WHEN** an IM-owned session is deleted from the existing session UI
- **THEN** its binding SHALL be removed or recognized as stale so the next external message can create a replacement session

