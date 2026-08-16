## ADDED Requirements

### Requirement: Plan execution projects a Run hierarchy
Each PlanRun SHALL correlate to a parent canonical Run and each executing SubTask/Attempt SHALL correlate to a child Run. Pause, retry, verification, cancellation, timeout, and recovery SHALL project canonical transitions while Plan topology and status remain authoritative.

#### Scenario: Plan is cancelled
- **WHEN** a PlanRun is cancelled
- **THEN** its parent Run cancels all non-terminal child Runs and existing Plan cancellation evidence remains compatible
