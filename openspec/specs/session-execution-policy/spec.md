# session-execution-policy Specification

## Purpose
Defines one fail-closed contract for combining durable Agent policy templates with per-session execution intent across native and managed CLI execution paths.

## Requirements

### Requirement: Agent policy is the execution safety ceiling
The system SHALL resolve every session execution against the stable agent principal's assigned `readonly`, `standard`, `trusted`, or `yolo` policy template, and a session execution mode SHALL NOT produce behavior more permissive than that template.

#### Scenario: Readonly Agent selects execute
- **WHEN** a session for an Agent assigned `readonly` uses execution mode `execute`
- **THEN** the effective execution policy SHALL remain read-only
- **AND** shell execution and file modification SHALL NOT be enabled by the session selection

#### Scenario: Trusted Agent selects plan
- **WHEN** a session for an Agent assigned `trusted` or `yolo` uses execution mode `plan`
- **THEN** the effective execution policy SHALL be read-only

### Requirement: Session execution modes have fixed composition semantics
The system SHALL support exactly `inherit`, `plan`, and `execute`: `inherit` and `execute` SHALL retain the Agent template's effective `Deny`, `Ask`, or `Allow` posture for shell and file-write actions, while `plan` SHALL narrow those actions to `Deny`.

#### Scenario: Standard Agent inherits or executes
- **WHEN** an Agent assigned `standard` uses execution mode `inherit` or `execute`
- **THEN** the effective execution policy SHALL require approval for shell and file-write actions

#### Scenario: Any Agent plans
- **WHEN** a session uses execution mode `plan`
- **THEN** the effective execution policy SHALL deny shell and file-write actions regardless of the Agent template

### Requirement: Effective execution behavior is service-visible
The desktop and Web/mock service contracts SHALL expose the Agent policy, selected execution mode, and resolved effective behavior for the active session without requiring React components to reproduce native policy resolution.

#### Scenario: UI reads effective behavior
- **WHEN** a chat surface loads or changes a session execution mode
- **THEN** `AgentService` SHALL return the selected mode and effective `readonly`, `ask`, or `allow` behavior
- **AND** the UI SHALL describe that behavior to the user

### Requirement: Resolution failures fail closed
The system SHALL reject execution when it cannot resolve the Agent policy or cannot map the effective policy safely for the selected runtime.

#### Scenario: Policy lookup fails before launch
- **WHEN** Agent policy lookup fails for a new chat generation or Agent Terminal launch
- **THEN** the launch SHALL fail with an actionable error
- **AND** the system SHALL NOT substitute a permissive default

#### Scenario: Runtime lacks a safe mapping
- **WHEN** a runtime cannot express the resolved effective policy through its supported enforcement mechanism
- **THEN** execution SHALL fail before a process or write-capable tool starts

### Requirement: Policy changes affect future execution only
Changing an Agent policy SHALL affect future chat generations and future Agent Terminal processes but SHALL NOT mutate a process or generation that is already running.

#### Scenario: Policy changes during execution
- **WHEN** an Agent policy changes while a CLI process or native generation is active
- **THEN** that active execution SHALL retain its original resolved policy
- **AND** the next execution SHALL resolve against the new Agent policy
