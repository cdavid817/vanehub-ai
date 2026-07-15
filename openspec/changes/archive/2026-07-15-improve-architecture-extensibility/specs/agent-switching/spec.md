## ADDED Requirements

### Requirement: Observable agent launch operations
Agent launch flows that may start external processes, open browser workflows, or initialize native desktop sessions SHALL expose observable operation state when launch cannot complete as a short immediate command.

#### Scenario: Launch starts observable operation
- **WHEN** a user launches an active workflow and the launch path requires a long-running or externally visible operation
- **THEN** the system SHALL expose an operation id, lifecycle state, and user-displayable status through the agent service boundary

#### Scenario: Launch readiness remains separate
- **WHEN** the system checks Agent availability or browser readiness before launch
- **THEN** it SHALL perform those checks separately from starting the observable launch operation
