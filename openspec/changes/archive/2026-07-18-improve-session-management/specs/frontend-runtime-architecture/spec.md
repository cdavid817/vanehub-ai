## ADDED Requirements

### Requirement: Session management service boundary extensions
Session search, category management, session export, and chat file references SHALL remain behind frontend service interfaces and runtime adapters.

#### Scenario: React uses service methods
- **WHEN** React UI searches sessions, manages categories, exports a session, or resolves file references
- **THEN** it SHALL call frontend service methods and SHALL NOT call Tauri `invoke()` or SQLite directly

#### Scenario: Adapter parity
- **WHEN** the Tauri adapter gains a session-management method
- **THEN** the Web/mock adapter SHALL expose the same method signature with deterministic behavior

### Requirement: Render-only Mermaid integration
Mermaid rendering SHALL be implemented as chat UI rendering behavior without changing native message persistence semantics.

#### Scenario: Persist source content
- **WHEN** a message contains a Mermaid code block
- **THEN** the persisted message content SHALL remain the original Markdown source and rendering SHALL be derived in the frontend
