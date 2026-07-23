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
