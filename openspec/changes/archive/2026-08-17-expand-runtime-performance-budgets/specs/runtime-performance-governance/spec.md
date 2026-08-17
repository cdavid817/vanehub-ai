## ADDED Requirements

### Requirement: Versioned runtime performance harness
The repository SHALL provide a repeatable performance command driven by versioned deterministic dataset manifests. Each result SHALL identify the source commit, operating system and architecture, build profile, dataset id and version, metric id, metric class, measured value, unit, baseline, budget, and outcome.

#### Scenario: Run the deterministic performance suite
- **WHEN** the repository performance command runs twice from the same source and dataset version
- **THEN** it SHALL select the same fixtures and deterministic structural budgets
- **AND** it SHALL emit parseable result records with the required provenance

#### Scenario: Reject malformed or unsafe fixture metadata
- **WHEN** a dataset or result omits required provenance, uses an unknown metric class or unit, duplicates an id, exceeds declared fixture bounds, or references a path outside its fixture root
- **THEN** the harness SHALL fail before executing the affected workload and identify the safe validation reason

### Requirement: Performance metrics use stable gate classes
Every runtime performance metric SHALL be classified as `deterministic-gate`, `dedicated-benchmark`, or `informational-telemetry`. Shared CI SHALL enforce deterministic structural budgets and SHALL NOT enforce dedicated or informational wall-clock, throughput, CPU, or memory measurements as fixed absolute timing gates.

#### Scenario: Shared CI evaluates mixed metrics
- **WHEN** deterministic, dedicated, and informational records are compared
- **THEN** only a deterministic over-budget result SHALL fail the shared-CI command
- **AND** all classes SHALL remain present in the evidence report

### Requirement: Budgets derive from recorded baselines
A hard or relative budget SHALL cite a measured baseline and justified headroom. The comparator SHALL report metric id, baseline, measured value, budget, delta, dataset, platform, and profile for every regression.

#### Scenario: Metric exceeds its budget
- **WHEN** a deterministic measurement is greater than its declared upper bound or lower than its declared lower bound
- **THEN** comparison SHALL fail with the complete actionable metric context

#### Scenario: Negative regression fixture is evaluated
- **WHEN** the repository's known-over-budget fixture is compared
- **THEN** the comparator SHALL reject it deterministically without changing the accepted baseline

### Requirement: Runtime surfaces retain dedicated and informational evidence
The harness SHALL support dedicated evidence for latency, throughput, and memory and informational evidence for cold start, time to interactive, idle memory, idle CPU, and main-thread long tasks without exposing raw prompts, responses, credentials, terminal content, file content, or unrestricted paths.

#### Scenario: Device evidence is recorded
- **WHEN** a supported desktop measurement is captured
- **THEN** the record SHALL contain bounded numeric metrics and environment provenance only
- **AND** another operating system SHALL remain `NOT RUN` unless it was actually measured

