## ADDED Requirements

### Requirement: GitHub change validation
The repository SHALL run automated frontend, contract, native Rust, and browser validation for every pull request and every push to the `main` branch.

#### Scenario: Pull request validation
- **WHEN** a pull request targets any branch
- **THEN** GitHub Actions SHALL run ESLint, the TypeScript/Vite build, Vitest contract conformance, Rust formatting, Cargo check, Clippy, Rust tests, and Playwright E2E tests

#### Scenario: Main branch validation
- **WHEN** a commit is pushed to `main`
- **THEN** GitHub Actions SHALL run the same frontend, contract, native Rust, and browser checks

### Requirement: Reproducible CI environments
The CI workflow MUST install JavaScript dependencies from the committed npm lockfile and MUST provision the Rust and Linux dependencies required to validate the Tauri crate.

#### Scenario: Install CI dependencies
- **WHEN** a validation job starts on a GitHub-hosted runner
- **THEN** it SHALL provision the declared Node.js or Rust toolchain and install dependencies before invoking repository checks

### Requirement: Playwright failure diagnostics
The CI workflow SHALL retain the Playwright HTML report as a GitHub Actions artifact when the Playwright E2E check fails.

#### Scenario: E2E test failure
- **WHEN** Playwright E2E execution fails and produces a report
- **THEN** the workflow SHALL upload that report even though the test step failed

#### Scenario: Successful E2E run
- **WHEN** Playwright E2E execution succeeds
- **THEN** the workflow SHALL not upload a failure report artifact
