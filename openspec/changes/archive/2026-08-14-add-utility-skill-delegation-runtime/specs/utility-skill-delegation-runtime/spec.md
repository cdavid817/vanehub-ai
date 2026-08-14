## Purpose

Defines a bounded, auditable native runtime for delegating specialist work to an exact effective Utility Skill revision without conflating delegation with Role Skill loading or ordinary tool execution.

## ADDED Requirements

### Requirement: Exact Utility revision resolution
Before starting an attempt, the runtime SHALL resolve an enabled, available, effective Utility Skill for the active canonical workspace and SHALL bind the attempt to its canonical id and immutable revision. It SHALL reject Role Skills, shadowed definitions, ambiguous aliases, and revisions that change before start.

#### Scenario: Effective Utility starts
- **WHEN** a supported native API Agent delegates to an enabled effective Utility Skill
- **THEN** the runtime SHALL start one attempt bound to the resolved canonical id, revision, workspace, parent run, and delegation id

#### Scenario: Role Skill rejected
- **WHEN** delegation targets a Role Skill
- **THEN** the runtime SHALL refuse the request without executing or reclassifying the Skill

#### Scenario: Revision changes before start
- **WHEN** the selected effective Utility revision changes between resolution and attempt admission
- **THEN** the runtime SHALL reject the stale admission and require fresh resolution

### Requirement: Fixed-schema native delegation tool
Supported native API Agents SHALL receive a fixed-schema `delegate_utility_skill` tool accepting a canonical Skill id, bounded task description, and bounded execution limits. The tool SHALL NOT accept host paths, arbitrary environment variables, provider credentials, or nested runtime configuration.

#### Scenario: Supported native Agent delegates
- **WHEN** a supported native API Agent calls the tool with valid bounded input
- **THEN** the runtime SHALL execute the selected Utility attempt and return a bounded structured terminal result

#### Scenario: CLI output is not delegation
- **WHEN** a CLI provider emits a tool event or text that resembles Utility delegation
- **THEN** the system SHALL NOT create an authoritative Utility attempt unless a validated native delegation boundary admitted it

### Requirement: Bounded execution and nesting
Each Utility attempt SHALL enforce configured ceilings for task characters, instruction characters, duration, tool calls, approvals, result-summary characters, and delegation depth. Initial delivery SHALL prohibit nested Utility delegation.

#### Scenario: Runtime limit reached
- **WHEN** an admitted attempt reaches any configured execution ceiling
- **THEN** the runtime SHALL terminate it with a specific limit classification and preserve the measured safe counts

#### Scenario: Nested delegation rejected
- **WHEN** a Utility attempt requests another Utility delegation
- **THEN** the runtime SHALL reject the nested request without starting a child attempt

### Requirement: Authoritative lifecycle and cancellation
The runtime SHALL assign one delegation id and attempt id, publish exactly one started fact, and converge on exactly one terminal status: succeeded, failed, cancelled, timed-out, limited, or refused. Parent cancellation SHALL propagate to an active Utility attempt.

#### Scenario: Successful terminal state
- **WHEN** the delegated execution completes successfully
- **THEN** the runtime SHALL publish exactly one succeeded terminal fact with duration and safe counts

#### Scenario: Parent cancelled
- **WHEN** the parent generation is cancelled while its Utility attempt is active
- **THEN** the runtime SHALL request cancellation and publish exactly one cancelled terminal fact after execution stops

#### Scenario: Duplicate completion callback
- **WHEN** an adapter delivers the same terminal callback more than once
- **THEN** the runtime SHALL preserve the first valid terminal fact and ignore later duplicates

### Requirement: Safe observability and evolution evidence
Lifecycle projections SHALL include only correlation ids, canonical Utility id and revision, workspace scope, terminal classification, duration, bounded counts, and fidelity. Raw task text, Skill instructions, tool arguments, tool outputs, provider responses, secrets, and unrestricted paths SHALL NOT enter evolution evidence or unified logs.

#### Scenario: Terminal evidence projection
- **WHEN** an admitted Utility attempt reaches a terminal state
- **THEN** the evidence pipeline SHALL receive an authoritative Utility terminal envelope containing its exact revision and safe terminal facts

#### Scenario: Evidence sink unavailable
- **WHEN** evidence projection cannot be enqueued
- **THEN** the Utility terminal result SHALL remain valid and the failure SHALL be reported through safe rate-limited diagnostics

### Requirement: Browser capability honesty
The Web/mock adapter SHALL expose the same delegation capability shape but SHALL report native Utility execution as unavailable and SHALL NOT simulate a successful authoritative Utility attempt.

#### Scenario: Web delegation requested
- **WHEN** browser mode requests Utility execution
- **THEN** the adapter SHALL return a deterministic native-runtime-unavailable refusal without emitting success evidence

