# interaction-modes Specification

## Purpose
TBD - created by archiving change unify-ai-agent-tool-management. Update Purpose after archive.
## Requirements
### Requirement: Browser interaction mode
The system SHALL support a browser interaction mode for agents that operate through web-based sessions or browser-controlled workflows.

#### Scenario: Start browser mode workflow
- **WHEN** a user starts a workflow with an agent that supports browser interaction mode
- **THEN** the system routes the workflow through the browser mode launch path

#### Scenario: Browser mode requires authentication
- **WHEN** browser interaction mode requires an authenticated web session
- **THEN** the system reports authentication readiness before launching the workflow

### Requirement: Native desktop interaction mode
The system SHALL support a native desktop window interaction mode for agents that operate through local desktop applications or OS-managed windows.

#### Scenario: Start native desktop workflow
- **WHEN** a user starts a workflow with an agent that supports native desktop interaction mode
- **THEN** the system routes the workflow through the native desktop mode launch path

#### Scenario: Native desktop mode is unsupported on platform
- **WHEN** native desktop interaction mode is unavailable on the current operating system
- **THEN** the system marks that mode unavailable and prevents workflows from launching through it

### Requirement: Interaction lifecycle state
The system SHALL expose common lifecycle states for interaction sessions without requiring every agent to share the same internal implementation.

#### Scenario: Session lifecycle changes
- **WHEN** an interaction session starts, runs, fails, or stops
- **THEN** the system reports the corresponding lifecycle state as starting, running, failed, or stopped

#### Scenario: Agent-specific session details
- **WHEN** an agent provides additional session details
- **THEN** the system keeps those details scoped to that agent's adapter while preserving the common lifecycle state

### Requirement: Interaction task lifecycle alignment
Interaction mode lifecycle reporting SHALL align with the common observable operation model while preserving mode-specific session details behind runtime adapters.

#### Scenario: Browser operation lifecycle
- **WHEN** a browser interaction workflow starts, runs, fails, or stops
- **THEN** the system SHALL report lifecycle updates using the common operation status values while keeping browser-specific readiness details scoped to the browser adapter

#### Scenario: Native desktop operation lifecycle
- **WHEN** a native desktop interaction workflow starts, runs, fails, or stops
- **THEN** the system SHALL report lifecycle updates using the common operation status values while keeping OS/window-specific details scoped to the native adapter
