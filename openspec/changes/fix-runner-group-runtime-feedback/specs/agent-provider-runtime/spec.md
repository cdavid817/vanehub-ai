## MODIFIED Requirements

### Requirement: Side-effect-free detection and health diagnostics
Executable availability, version detection, readiness, Runner discovery, and health assessment SHALL use bounded non-interactive probe specifications and SHALL NOT start an interactive Agent session or generation. Independent discovery results SHALL be projected fail-soft so a failed optional or remote lookup does not hide an independently usable local Runner. Failures SHALL be classified and persisted only through unified redacted diagnostics.

#### Scenario: Check provider availability
- **WHEN** runtime availability is refreshed for a registered provider
- **THEN** the provider SHALL return or execute only its bounded detection specifications
- **AND** SHALL NOT deliver a prompt or create a provider session

#### Scenario: Version detection fails
- **WHEN** an executable is missing, times out, exits unsuccessfully, or returns an invalid version
- **THEN** the runtime SHALL return a classified safe diagnostic
- **AND** unified logs SHALL omit credentials, prompt content, sensitive arguments, environment values, and unbounded raw output

#### Scenario: Optional Runner discovery fails
- **WHEN** an optional or remote Runner lookup fails while the Local Runner remains usable
- **THEN** Runner discovery SHALL still return the Local Runner as available
- **AND** the unresolvable optional Runner SHALL be omitted rather than converted into a global discovery failure
- **AND** no interactive process or provider session SHALL be started

### Requirement: Provider and Runner remain orthogonal
Provider adapters SHALL continue to own provider invocation, prompt/input translation, output parsing, usage, provider sessions, cancellation semantics, and provider error mapping, while Runner adapters SHALL own execution location, transport, process/channel I/O, inspection, cleanup, discovery, and runner errors. Generic orchestration MUST NOT select Runner behavior by provider id or provider behavior by Runner kind, and one Runner's discovery failure MUST NOT change another independently discovered Runner's readiness.

#### Scenario: Run one provider locally and remotely
- **WHEN** a provider declares the capabilities required by both an eligible Local and SSH Runner
- **THEN** the same provider adapter prepares both invocations and the selected Runner executes the resulting bounded specification

#### Scenario: Runner transport fails
- **WHEN** transport fails before a provider terminal protocol event
- **THEN** provider parsing does not fabricate a provider error and orchestration preserves the Runner classification

#### Scenario: Local Runner survives remote discovery failure
- **WHEN** discovery cannot read or validate an SSH target
- **THEN** orchestration SHALL preserve the Local Runner's independently established availability
