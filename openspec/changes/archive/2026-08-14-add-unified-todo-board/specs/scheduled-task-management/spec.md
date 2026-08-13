## ADDED Requirements

### Requirement: Durable scheduled-task run history
The system SHALL persist a bounded history record for each Scheduled Task execution attempt with stable identity, task identity, optional Session identity, status, timestamps, and concise redacted error information.

#### Scenario: Record scheduled execution
- **WHEN** a scheduled execution succeeds, fails, is skipped, or is backfilled
- **THEN** the system SHALL append a run-history record while preserving the Scheduled Task's existing latest-run projection

#### Scenario: List scheduled execution history
- **WHEN** a caller inspects a Scheduled Task source from the board
- **THEN** it SHALL receive recent run records ordered newest first without reading feature-local log files

### Requirement: Scheduled Task board reconciliation
Scheduled Tasks SHALL participate in unified board reconciliation without changing their recurrence or enabled semantics.

#### Scenario: Reconcile Scheduled Task
- **WHEN** an enabled or disabled Scheduled Task has no existing work-item link
- **THEN** board reconciliation SHALL create one Planned work item linked to the stable Scheduled Task id
