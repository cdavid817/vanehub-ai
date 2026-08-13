## ADDED Requirements

### Requirement: Autonomous Plan loop trace correlation
The observability system SHALL correlate Plan driver activation, scheduling cycles, discovery sessions, original and repair Attempts, SubTask verification, final verification, pause and cancellation boundaries, and user recovery actions while preserving metadata-only diagnostic defaults.

#### Scenario: Trace an automatic repair chain
- **WHEN** one SubTask has multiple original or repair Attempts
- **THEN** the execution topology SHALL retain their sequence, parent PlanRun and SubTask identities, safe failure classes, durations, and terminal states without storing prompts or raw validation output in diagnostics

#### Scenario: Trace background continuation
- **WHEN** the native driver advances a PlanRun while no Plan view is open
- **THEN** unified logging SHALL record bounded lifecycle and correlation events sufficient to distinguish activation, claim, execution, verification, repair, and stop boundaries

#### Scenario: Correlate originating session navigation safely
- **WHEN** a PlanRun is associated with its originating OnePiece session
- **THEN** diagnostics MAY correlate non-secret session and PlanRun ids while excluding session titles, prompts, goals, and message content

#### Scenario: Inspect final verification evidence
- **WHEN** a user requests final verification details through the Plan service
- **THEN** the user-facing bounded evidence path MAY return allowed command summaries while persistent diagnostics SHALL continue excluding unredacted command output
