## ADDED Requirements

### Requirement: Automated dependency maintenance
The repository SHALL monitor npm, Cargo, and GitHub Actions dependencies and SHALL create grouped, scheduled update pull requests from committed configuration.

#### Scenario: Scheduled dependency scan
- **WHEN** the configured weekly schedule occurs
- **THEN** Dependabot SHALL inspect the npm root, the `src-tauri` Cargo project, and GitHub Actions references

### Requirement: Vulnerable dependency prevention
The repository SHALL enable dependency alerts and security updates and MUST review pull-request dependency changes for newly introduced vulnerabilities.

#### Scenario: Pull request introduces a vulnerable dependency
- **WHEN** a manifest or lockfile change introduces a dependency at or above the configured severity threshold
- **THEN** dependency review SHALL fail and prevent merge while it is required by the default-branch ruleset

### Requirement: Static security analysis
The repository SHALL analyze JavaScript/TypeScript and Rust with CodeQL on pull requests, default-branch updates, and a recurring schedule.

#### Scenario: CodeQL detects a security issue
- **WHEN** CodeQL finds a reportable issue in supported source code
- **THEN** GitHub SHALL publish the result as a code-scanning alert and annotate the associated change when supported

### Requirement: Hardened workflow dependencies
GitHub Actions workflows MUST declare least-privilege token permissions and MUST reference external Actions by immutable full commit SHA.

#### Scenario: Workflow configuration is reviewed
- **WHEN** a workflow invokes a GitHub-owned or third-party Action
- **THEN** the `uses` reference SHALL resolve to a full commit SHA with a human-readable version comment

### Requirement: Release integrity metadata
Every published desktop release SHALL include checksums, an SPDX JSON SBOM, and GitHub artifact attestations that cover the released artifacts and SBOM.

#### Scenario: User verifies a release
- **WHEN** a user downloads a published installer
- **THEN** the release SHALL provide a checksum and GitHub-hosted provenance evidence for integrity verification
