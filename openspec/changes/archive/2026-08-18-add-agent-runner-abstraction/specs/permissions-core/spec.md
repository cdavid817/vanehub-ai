## ADDED Requirements

### Requirement: Runner-targeted execution authority
Permission evaluation for Agent execution SHALL include the stable principal, action, Runner kind, bounded target identity, and current authority revision. An Allow for one Runner or target MUST NOT authorize another, and incomplete or stale Runner context MUST fail closed before process/channel creation.

#### Scenario: Local grant is reused for SSH
- **WHEN** a remembered Local execution grant is presented for an SSH target
- **THEN** permission evaluation returns Ask or Deny according to policy and does not start remote work

#### Scenario: Runner authority changes during preparation
- **WHEN** permission, SSH profile, host trust, or credential revision changes before spawn
- **THEN** the stale preparation is rejected and must be evaluated again

### Requirement: Secret injection is explicitly runner scoped
Secret material SHALL be resolved only inside the native Runner preparation boundary after permission admission and SHALL be injected only into the approved process/channel. Secret values MUST NOT enter Run metadata, frontend contracts, SQLite, command-safe errors, telemetry attributes, or unified logs.

#### Scenario: Unapproved secret is requested
- **WHEN** a provider invocation requests a credential not admitted for the selected Runner and target
- **THEN** launch fails closed before secret resolution or transport write

