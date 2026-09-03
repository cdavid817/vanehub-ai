## ADDED Requirements

### Requirement: Evolution orchestration dashboard
The Skill Evolution area SHALL show scheduler mode, idle-gate state, pending trigger counts, active and recent runs, stage progress, checkpoints, budgets, partial results, and sanitized failures.

#### Scenario: Run waits for idle
- **WHEN** an automatic run is blocked by active work
- **THEN** the UI shows the safe blocking category and does not suggest bypassing it

### Requirement: Automatic-application policy controls
The UI SHALL present `off`, `observe`, and `enabled` modes, versioned consent disclosure, per-Skill allowlist, fixed exclusions, rate limits, cooldowns, and Web capability differences before enabling automatic application.

#### Scenario: User enables automatic application
- **WHEN** the user confirms disclosure and selects allowed Skills
- **THEN** policy is updated through the Skill service and the UI shows the effective version and limits

#### Scenario: User selects Web/mock runtime
- **WHEN** the application lacks native background and filesystem capabilities
- **THEN** the UI labels orchestration and application results as simulated and does not claim real auto application

### Requirement: Eligibility and observe-mode inspection
The UI SHALL show every auto-apply condition, pass/fail reason, draft provenance, final-preflight state, and observed-would-apply result without exposing unsafe correction content.

#### Scenario: Candidate is routed to Curator
- **WHEN** one eligibility condition fails
- **THEN** the UI identifies the stable condition and links to Curator where applicable

### Requirement: Probation and circuit-breaker controls
The UI SHALL show automatic applications under probation, verified outcome summaries, Skill/workspace suspensions, breaker cause, health status, and explicit acknowledgement controls. It MUST NOT offer automatic rollback.

#### Scenario: Breaker is open
- **WHEN** auto application is suspended
- **THEN** the UI keeps pipeline monitoring available, disables automatic mutation, and links regression review to Curator

### Requirement: Manual run and cancellation controls
The UI SHALL allow a user to request one manual run or cooperative cancellation and SHALL explain that manual action cannot bypass writer locks, mutation gates, limits, or breakers.

#### Scenario: User cancels an active run
- **WHEN** cancellation is accepted
- **THEN** the UI shows cancel-requested until the next safe checkpoint and preserves completed results

