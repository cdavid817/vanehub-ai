## ADDED Requirements

### Requirement: Unified architecture fitness entry point
The repository SHALL provide one documented architecture fitness command that executes the registered frontend, native, and repository architecture rules without duplicating their underlying implementations.

#### Scenario: Developer runs the architecture gate
- **WHEN** a developer runs the repository architecture fitness command
- **THEN** all registered architecture rule groups SHALL execute and the command SHALL fail if any group reports a violation

#### Scenario: Architecture rule fails
- **WHEN** a registered rule detects a violation
- **THEN** its diagnostic SHALL include a stable rule id, affected file and line or module, and a concise repair direction

### Requirement: Prohibited production dependencies
Production frontend source SHALL use React built-in state and context and MUST NOT import Redux, Zustand, or MobX packages.

#### Scenario: Prohibited state library is imported
- **WHEN** production frontend source imports Redux, Zustand, or MobX directly or through their standard React bindings
- **THEN** architecture fitness SHALL fail with the dependency rule id and source location

#### Scenario: Historical package entry is unused
- **WHEN** a prohibited package remains declared but has no production use
- **THEN** the change SHALL either remove it safely or record a bounded removal task rather than add a permanent exemption

### Requirement: Architecture detector fixture coverage
Every architecture detector introduced by the repository SHALL have deterministic accepting and rejecting fixtures, including diagnostics assertions for rejected input.

#### Scenario: Detector fixtures run
- **WHEN** architecture detector unit tests execute
- **THEN** compliant fixtures SHALL pass and one fixture for every prohibited construct SHALL fail with the expected rule id and location

### Requirement: Existing source constraints remain enforced
The architecture gate SHALL preserve the repository's existing TypeScript, React, Rust, and file-size constraints and MUST NOT introduce a new blanket or permanent exemption.

#### Scenario: Production source violates an existing constraint
- **WHEN** production TypeScript uses explicit `any` or `@ts-ignore`, a new production TypeScript file exceeds 300 physical lines, or production Rust uses a prohibited panic shortcut
- **THEN** the configured repository checks SHALL reject the source

