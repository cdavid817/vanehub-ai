## ADDED Requirements

### Requirement: Cross-platform desktop smoke matrix
Pull-request and main-branch validation SHALL execute desktop smoke on native Windows, macOS, and Linux runners, with each runner building, launching, and testing its own compatible native desktop artifact.

#### Scenario: Validate desktop behavior across supported platforms
- **WHEN** CI validates a pull request or a push to the main branch
- **THEN** Windows, macOS, and Linux desktop smoke jobs each execute on their corresponding native runner
- **AND** a successful job reports only the platform it actually tested

#### Scenario: One platform fails
- **WHEN** desktop smoke fails on one matrix platform
- **THEN** that platform job fails without cancelling evidence collection or misreporting the other platform results

### Requirement: Desktop smoke CI diagnostics
CI SHALL upload run-scoped desktop failure evidence for each failed or blocked platform execution and SHALL avoid retaining test data when the desktop smoke succeeds.

#### Scenario: Desktop smoke fails in CI
- **WHEN** a platform desktop smoke job fails or is blocked after a run directory is created
- **THEN** CI uploads its summary, screenshots, driver diagnostics, process state, and redacted native logs as a platform-labelled artifact

#### Scenario: Desktop smoke succeeds in CI
- **WHEN** a platform desktop smoke job succeeds
- **THEN** CI does not upload the run's temporary application data as a diagnostic artifact
