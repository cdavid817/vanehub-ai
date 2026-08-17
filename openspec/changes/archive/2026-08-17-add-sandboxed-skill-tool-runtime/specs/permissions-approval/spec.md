## ADDED Requirements

### Requirement: Skill tool approval provenance
An approval request caused by a Skill tool SHALL identify the parent agent, Skill, tool, effective revision, requested capability, delegated host operation, target resource, risk level, and bounded redacted input summary. Approval of one operation MUST NOT approve the Skill revision or future operations.

#### Scenario: Skill tool requires approval
- **WHEN** unified permission evaluation returns Ask for a delegated Skill tool operation
- **THEN** the approval surface shows both Skill provenance and the concrete operation awaiting a decision

#### Scenario: User approves one operation
- **WHEN** the user approves a pending Skill tool operation
- **THEN** only that immutable request proceeds and later requests receive independent evaluation

### Requirement: Approval invalidation and fail-closed resolution
Pending approval SHALL become invalid if the parent generation is cancelled, the effective Skill revision changes, the tool is disabled or quarantined, or the immutable request witness no longer matches. A late approval MUST NOT revive invalid work.

#### Scenario: Revision changes while approval is pending
- **WHEN** a Skill tool's effective revision changes before the user decides
- **THEN** the pending request is invalidated and cannot execute under either revision

#### Scenario: Desktop approval channel is unavailable
- **WHEN** a protected Skill tool operation requires approval but no supported approval channel is available
- **THEN** the system denies the operation rather than executing it silently

