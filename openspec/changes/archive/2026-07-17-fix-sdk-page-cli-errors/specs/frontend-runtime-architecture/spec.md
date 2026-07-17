## ADDED Requirements

### Requirement: Service-backed CLI refresh state
The frontend SHALL represent CLI refresh loading, running, success, and failure states through service-backed data and operation status.

#### Scenario: Refresh state uses service boundary
- **WHEN** the CLI management settings page starts or observes a CLI refresh operation
- **THEN** React components SHALL use the Agent service and operation service interfaces to derive refresh state
- **AND** React components SHALL NOT import or call Tauri APIs directly

#### Scenario: Web runtime simulates refresh state
- **WHEN** the CLI management settings page runs in the Web/mock runtime and a refresh is requested
- **THEN** the Web adapter SHALL return a mock operation that allows the page to show refresh-in-progress behavior without requiring native commands or writing local log files

### Requirement: Frontend critical CLI failure reporting
The frontend SHALL report critical CLI refresh and package-operation failures through the service boundary when those failures require durable diagnostics beyond the operation log.

#### Scenario: Report refresh start failure
- **WHEN** starting a CLI refresh request fails before the backend returns an operation id
- **THEN** the frontend SHALL surface a user-displayable error
- **AND** in the Tauri runtime it SHALL report the failure through the logging service boundary for native persistence

#### Scenario: Report package start failure
- **WHEN** starting a CLI package operation fails before the backend returns an operation id
- **THEN** the frontend SHALL surface a user-displayable error
- **AND** in the Tauri runtime it SHALL report the failure through the logging service boundary for native persistence
