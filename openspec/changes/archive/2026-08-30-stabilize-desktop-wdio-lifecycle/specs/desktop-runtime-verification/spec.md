## ADDED Requirements

### Requirement: Stable desktop automation worker lifecycle
The desktop verification harness SHALL ensure its automation driver is accepting sessions before each isolated worker starts, including after a preceding worker has cleanly closed the native application.

#### Scenario: Start a worker after native shutdown
- **WHEN** a desktop verification worker starts after the preceding worker has completed owned-process shutdown
- **THEN** the harness verifies that the automation driver accepts a new session before executing the worker's spec
- **AND** it recovers the test-owned driver when the previous shutdown invalidated it

#### Scenario: Driver cannot be restored
- **WHEN** the automation driver cannot accept a session within the configured readiness deadline
- **THEN** the affected worker reports `FAILED`
- **AND** its run-scoped evidence identifies the driver readiness failure rather than attributing it to application behavior

### Requirement: Diagnosable frontend failure evidence
The desktop verification harness SHALL preserve redacted details for the browser error or unhandled rejection that triggers a fatal frontend marker, so test results can distinguish an application failure from test instrumentation.

#### Scenario: Browser error triggers the fatal marker
- **WHEN** a browser error or unhandled rejection occurs during native desktop verification
- **THEN** the run-scoped evidence records the event type and a redacted diagnostic message or reason
- **AND** the failing assertion identifies that captured diagnostic detail

#### Scenario: No frontend error details are available
- **WHEN** the browser supplies no serializable message or rejection reason
- **THEN** the evidence records that the detail was unavailable
- **AND** it does not claim a specific application root cause
