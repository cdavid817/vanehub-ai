# continuous-integration Specification

## Purpose
TBD - created by archiving change configure-github-repository. Update Purpose after archive.
## Requirements
### Requirement: GitHub change validation
The repository SHALL run automated frontend, specification, native Rust, platform, and browser validation for every pull request and every push to the `main` branch.

#### Scenario: Pull request validation
- **WHEN** a pull request targets the repository
- **THEN** GitHub Actions SHALL run ESLint, the TypeScript/Vite build, full Vitest, contract conformance, strict OpenSpec validation, Rust formatting, Cargo check, Clippy, Rust tests, native platform checks, and Playwright E2E tests

#### Scenario: Main branch validation
- **WHEN** a commit is pushed to `main`
- **THEN** GitHub Actions SHALL run the same required validation contract

### Requirement: Reproducible least-privilege CI
CI workflows MUST install JavaScript dependencies from the committed npm lockfile, MUST provision declared Rust and native dependencies, and MUST use only the token permissions required by each job.

#### Scenario: Validation job starts
- **WHEN** a validation job starts on a GitHub-hosted runner
- **THEN** it SHALL provision the declared toolchain and dependencies before invoking repository checks without receiving write permissions

### Requirement: Current-run validation
CI SHALL cancel superseded runs for the same pull request or branch so required status reflects the newest commit.

#### Scenario: Pull request receives a newer commit
- **WHEN** a newer commit is pushed while an earlier CI run is active
- **THEN** GitHub Actions SHALL cancel the superseded run and validate the newer commit

### Requirement: Playwright failure diagnostics
CI SHALL retain the Playwright HTML report as a GitHub Actions artifact when Playwright E2E execution fails.

#### Scenario: E2E test failure
- **WHEN** Playwright E2E execution fails and produces a report
- **THEN** the workflow SHALL upload that report even though the test step failed

#### Scenario: Successful E2E run
- **WHEN** Playwright E2E execution succeeds
- **THEN** the workflow SHALL not upload a failure-only report artifact

### Requirement: Native linker prerequisite validation
Continuous integration SHALL provision and exercise the declared linker prerequisites for every supported target that has a repository target-scoped linker policy.

#### Scenario: Validate Linux x86_64 native code
- **WHEN** CI validates `x86_64-unknown-linux-gnu`
- **THEN** the runner SHALL install or verify Clang and mold before invoking native compilation
- **AND** at least one validation step SHALL link a native artifact using the declared linker

#### Scenario: Validate Windows x86_64 MSVC native code
- **WHEN** CI validates `x86_64-pc-windows-msvc`
- **THEN** the runner SHALL verify that the selected Rust toolchain provides the declared LLD linker
- **AND** at least one validation step SHALL link a native artifact using that linker

#### Scenario: Linker prerequisite is unavailable
- **WHEN** a required linker or linker driver cannot be provisioned on a declared CI target
- **THEN** native validation SHALL fail before reporting the target as successfully validated

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

