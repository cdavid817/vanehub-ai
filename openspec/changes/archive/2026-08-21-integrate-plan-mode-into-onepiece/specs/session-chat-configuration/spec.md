## MODIFIED Requirements

### Requirement: Session-level chat configuration persistence
The system SHALL persist validated chat preferences per session, including `executionMode`, and SHALL compose them with the session's authoritative stable agent id and interaction mode to produce the effective `ChatConfig`. An eligible OnePiece session SHALL expose that persisted execution mode in its conversation bar as the sole Plan-mode selection surface.

#### Scenario: Save configuration from the main chat
- **WHEN** a user changes provider, model, execution mode, reasoning, streaming, thinking, or long-context preferences for the active session
- **THEN** the frontend SHALL save the validated preferences through `AgentService` and the active session SHALL retain them across window and application restarts

#### Scenario: Select Plan mode in OnePiece
- **WHEN** a session with stable agent id `onepiece` exposes execution-mode selection in its conversation bar and the user selects Plan
- **THEN** the frontend SHALL persist `executionMode: "plan"` through `AgentService`
- **AND** both desktop and Web/mock runtimes SHALL return the same selected mode and effective read-only behavior

#### Scenario: Keep session identity authoritative
- **WHEN** the system composes an effective configuration
- **THEN** `agentId` and `interactionMode` SHALL come from the referenced session's stable persisted fields rather than an independently writable configuration snapshot

#### Scenario: Reject invalid configuration
- **WHEN** a configuration contains an unsupported provider/model combination, execution mode, reasoning depth, or value type
- **THEN** the service SHALL reject it before it can reach runtime execution or CLI launch argument construction

