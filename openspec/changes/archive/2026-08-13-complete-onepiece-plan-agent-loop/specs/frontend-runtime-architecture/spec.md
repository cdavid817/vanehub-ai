## ADDED Requirements

### Requirement: Plan-to-Agent loop frontend contract
The frontend SHALL access OnePiece planning discovery state, effective Plan or Agent mode, approval transition details, background driver status, originating-session association, editable criterion evidence policy, validation commands, repair history, final verification evidence, and user recovery actions through shared typed services implemented by both Tauri and Web/mock adapters.

#### Scenario: Desktop observes background progress
- **WHEN** a native PlanRun advances without an open Plan view
- **THEN** the Tauri adapter SHALL return bounded current projections through declared commands or subscriptions and React SHALL NOT drive native scheduling itself

#### Scenario: Web simulates the loop honestly
- **WHEN** the Plan workflow runs through the Web/mock adapter
- **THEN** it SHALL provide deterministic mode, approval, execution, repair, and verification transitions marked as simulated without claiming native Git, provider, command, or SQLite work

#### Scenario: Present mode without color dependence
- **WHEN** the OnePiece composer or Plan view displays Plan, Agent, running, verifying, repairing, paused, action-required, final-verifying, or awaiting-acceptance state
- **THEN** it SHALL use accessible text and icon semantics in addition to theme tokens and SHALL expose keyboard-operable controls with visible focus

#### Scenario: Remove UI scheduling authority
- **WHEN** an approved PlanRun is active
- **THEN** the Plan UI SHALL offer observation and durable controls rather than requiring an “execute next” action to make progress

#### Scenario: Resolve a session association consistently
- **WHEN** React asks for the PlanRun associated with a OnePiece session id
- **THEN** both adapters SHALL return the same bounded nullable association shape and SHALL never select a run by global recency
