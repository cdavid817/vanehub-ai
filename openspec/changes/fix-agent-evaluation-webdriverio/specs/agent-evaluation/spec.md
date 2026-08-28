## ADDED Requirements

### Requirement: Provider-specific desktop evaluation qualification is truthful
The desktop evaluation harness SHALL support focused qualification of the stable `opencode` and `onepiece` Agent ids, SHALL distinguish deterministic fixture execution from live-provider execution, and SHALL NOT report a fixture result as proof that a real provider completed the benchmark.

#### Scenario: OpenCode fixture evaluation
- **WHEN** the required desktop gate evaluates `opencode` through the repository fixture
- **THEN** it completes the arena lifecycle without network credentials and records fixture provenance in the bounded test evidence

#### Scenario: Live OpenCode is unavailable
- **WHEN** live OpenCode qualification is requested but the executable or provider authentication is unavailable
- **THEN** the qualification reports `BLOCKED` with a safe prerequisite reason and does not substitute the fixture result

#### Scenario: Live OnePiece evaluation
- **WHEN** OnePiece qualification is requested with a process-scoped provider credential
- **THEN** the harness evaluates stable Agent id `onepiece`, waits for a terminal arena result, verifies persisted and rendered evidence, and removes the credential before evidence is written

#### Scenario: OnePiece credential is absent
- **WHEN** OnePiece qualification is requested without an accessible provider credential
- **THEN** the qualification reports `BLOCKED` and no arena is presented as a live-provider pass

### Requirement: Focused evaluation evidence is actionable and secret-safe
Each provider-specific evaluation qualification SHALL record its task identity, Agent id, fixture-or-live provenance, terminal outcome, arena and attempt correlation, and result status while omitting credentials, raw prompts, environment values, private absolute paths, and unbounded provider output.

#### Scenario: Evaluation dispatch fails
- **WHEN** OpenCode or OnePiece fails before deterministic verification
- **THEN** the evidence includes the bounded dispatch diagnostic exposed by the evaluation result and the qualification reports `FAILED` rather than timing out without explanation

#### Scenario: Evidence is audited
- **WHEN** the desktop layer finishes or aborts
- **THEN** its evidence can be checked for forbidden credential values and unsafe provider payloads without reading the provider credential itself
