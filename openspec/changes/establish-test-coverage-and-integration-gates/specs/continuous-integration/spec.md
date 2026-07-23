## ADDED Requirements

### Requirement: Coverage-gated change validation
GitHub change validation SHALL collect frontend and native Rust coverage from production source, SHALL enforce committed non-regression baselines for the wider codebase, and MUST enforce at least 80% line coverage for the designated critical Rust paths covering Agent startup and terminal control, MCP routing, and SQLite transaction behavior.

#### Scenario: Pull request meets coverage policy
- **WHEN** a pull request runs the frontend and native coverage jobs and every committed baseline and critical-path threshold is satisfied
- **THEN** the coverage checks SHALL succeed and publish concise totals for review

#### Scenario: Critical Rust coverage falls below threshold
- **WHEN** coverage for any designated critical Rust path falls below 80% line coverage
- **THEN** the required coverage check SHALL fail and identify the affected path group and measured value

#### Scenario: Wider coverage regresses
- **WHEN** frontend or native total coverage falls below its committed non-regression baseline
- **THEN** the required coverage check SHALL fail even if the critical Rust path thresholds still pass

#### Scenario: Unimported production source exists
- **WHEN** production frontend source is not imported by any test
- **THEN** the coverage report SHALL include that source as uncovered rather than omit it from the denominator

### Requirement: Reviewable coverage diagnostics
CI SHALL retain bounded frontend and native coverage reports as workflow artifacts and SHALL make a concise coverage summary available without requiring a third-party coverage service.

#### Scenario: Coverage job completes
- **WHEN** a frontend or native coverage job produces a report
- **THEN** CI SHALL upload the configured machine-readable and human-readable coverage outputs with a finite retention period

#### Scenario: Coverage threshold fails
- **WHEN** coverage collection succeeds but a configured threshold fails
- **THEN** CI SHALL preserve the generated report and the original threshold failure SHALL remain the job result

### Requirement: Zero-warning static quality gates
Required frontend and Rust validation SHALL treat configured ESLint warnings and Clippy warnings across supported targets as merge-blocking failures.

#### Scenario: Frontend lint warning is introduced
- **WHEN** ESLint reports a warning or error under the committed lint configuration
- **THEN** the required frontend quality check SHALL fail

#### Scenario: Rust target emits a Clippy warning
- **WHEN** Clippy checks all configured Rust targets and emits a warning
- **THEN** the required Rust quality check SHALL fail
