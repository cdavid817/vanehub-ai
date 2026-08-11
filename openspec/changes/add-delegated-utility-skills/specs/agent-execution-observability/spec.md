## ADDED Requirements

### Requirement: Native Utility delegation topology
Every Utility delegation attempt SHALL be represented as a child execution span or linked child run under the parent generation, with stable delegation, attempt, parent Agent, child principal, canonical Utility, effective revision, workspace, status, and fidelity metadata.

#### Scenario: Successful Utility delegation traced
- **WHEN** a Utility child starts and completes
- **THEN** the parent trace SHALL contain correlated start, provider, tool, approval, result, and terminal events available at native fidelity

#### Scenario: Delegation retry traced
- **WHEN** the parent invokes the same Utility again after a terminal attempt
- **THEN** observability SHALL create a new attempt identity linked to the same parent and canonical Utility without reusing the prior child span identity

#### Scenario: Delegation refused before child creation
- **WHEN** validation, eligibility, permission, or limits refuse delegation before a child starts
- **THEN** the parent tool span SHALL record the safe refusal reason and SHALL NOT invent a child model span

#### Scenario: Child cancelled
- **WHEN** the parent or user cancels an active Utility child
- **THEN** the child topology SHALL end with a cancelled status and identify the cancellation source

### Requirement: Privacy-bounded Utility telemetry
Utility delegation telemetry SHALL default to metadata-only capture. It SHALL omit raw task and context bodies, Utility instructions, hidden reasoning, credentials, file contents, unrestricted paths, and unbounded model or tool output.

#### Scenario: Delegation metadata recorded
- **WHEN** a delegation executes under default capture
- **THEN** telemetry SHALL record safe ids, hashes, counts, limits, durations, statuses, capability ids, and approval outcomes without content bodies

#### Scenario: Redacted capture enabled
- **WHEN** an existing redacted-content capture mode is enabled
- **THEN** Utility telemetry MAY include bounded redacted summaries while still excluding credentials and hidden reasoning

### Requirement: Utility execution metrics
The system SHALL expose bounded low-cardinality metrics for Utility attempt counts, terminal status, duration, limit exhaustion, approval outcomes, tool counts, and canonical Utility id according to existing observability privacy and retention policy.

#### Scenario: Utility attempt completes
- **WHEN** a Utility attempt reaches a terminal state
- **THEN** the system SHALL update bounded delegation metrics without using task text, context, paths, or attempt ids as metric dimensions

