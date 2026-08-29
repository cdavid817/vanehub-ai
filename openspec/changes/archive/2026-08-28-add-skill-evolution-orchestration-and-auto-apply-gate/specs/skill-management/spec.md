## ADDED Requirements

### Requirement: Evolution orchestration service boundary
The Skill management service SHALL expose scheduler status, runs, stages, checkpoints, policy, consent, allowlist, eligibility, application, probation, breaker, manual-run, cancellation, and breaker-acknowledgement operations through matching Tauri and Web adapters. React components MUST NOT invoke native commands directly.

#### Scenario: Desktop manual run
- **WHEN** the desktop UI requests a manual run through the Skill service
- **THEN** the Tauri adapter schedules it through the native orchestrator and returns typed status

#### Scenario: Web orchestration query
- **WHEN** Web UI requests orchestration state
- **THEN** the Web adapter returns behaviorally equivalent page-active mock state with explicit mock provenance

### Requirement: Conflict-safe orchestration policy
Policy, consent, allowlist, cancellation, and breaker operations SHALL require current version witnesses and SHALL return stable conflict or health reasons without weakening safety state.

#### Scenario: Stale policy update
- **WHEN** two settings surfaces update orchestration policy concurrently
- **THEN** the stale operation fails and returns the current policy without overwriting it

