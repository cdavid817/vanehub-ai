## ADDED Requirements

### Requirement: Provider and Runner remain orthogonal
Provider adapters SHALL continue to own provider invocation, prompt/input translation, output parsing, usage, provider sessions, cancellation semantics, and provider error mapping, while Runner adapters SHALL own execution location, transport, process/channel I/O, inspection, cleanup, and runner errors. Generic orchestration MUST NOT select Runner behavior by provider id or provider behavior by Runner kind.

#### Scenario: Run one provider locally and remotely
- **WHEN** a provider declares the capabilities required by both an eligible Local and SSH Runner
- **THEN** the same provider adapter prepares both invocations and the selected Runner executes the resulting bounded specification

#### Scenario: Runner transport fails
- **WHEN** transport fails before a provider terminal protocol event
- **THEN** provider parsing does not fabricate a provider error and orchestration preserves the Runner classification

