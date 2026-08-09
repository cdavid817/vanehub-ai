## ADDED Requirements

### Requirement: Plan execution trace correlation
The observability system SHALL correlate PlanRun, SubTaskRun, and SubTaskAttempt identities with their Agent sessions, provider generations, tool operations, validation operations, and state transitions while preserving the existing execution-run and trace topology.

#### Scenario: Inspect a SubTask attempt timeline
- **WHEN** a user opens the evidence for a SubTask attempt
- **THEN** the service boundary SHALL return a bounded timeline whose safe correlation fields connect the attempt to its session, generation, operations, and verification result

#### Scenario: Trace a PlanRun summary
- **WHEN** a PlanRun contains multiple serial attempts
- **THEN** the runtime SHALL expose their parent-child correlation and durations without embedding full session transcripts in the PlanRun summary

### Requirement: Redacted Plan telemetry
Plan execution diagnostics SHALL allow stable IDs, state names, durations, counts, safe filenames, exit classifications, and non-reversible fingerprints, and SHALL exclude user goals, generated task descriptions, prompts, credentials, raw tool arguments, raw tool results, and unredacted command output by default.

#### Scenario: Record an orchestration failure
- **WHEN** planning, dispatch, execution, verification, or recovery fails
- **THEN** the unified observability path SHALL persist a redacted classified event that remains useful for correlation without persisting prohibited content

#### Scenario: Preserve user-facing output separately
- **WHEN** a user inspects allowed Agent or validation output in the Plan UI
- **THEN** the frontend SHALL obtain it through the bounded session or operation presentation service rather than from diagnostic telemetry

