## ADDED Requirements

### Requirement: Synchronized release version
The release workflow MUST reject a tag when its semantic version does not match the versions declared in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.

#### Scenario: Release versions disagree
- **WHEN** a release tag or one of the three version declarations differs
- **THEN** the release workflow SHALL fail before building distributable artifacts

### Requirement: Cross-platform release artifacts
The release workflow SHALL build the declared Windows, macOS, and Linux desktop targets and SHALL publish only artifacts produced successfully for the release tag.

#### Scenario: All target builds succeed
- **WHEN** every declared target produces its expected bundle
- **THEN** one GitHub Release SHALL contain the collected platform artifacts and generated release notes

#### Scenario: A target build fails
- **WHEN** any required platform target cannot produce its bundle
- **THEN** the release workflow SHALL not publish a misleading complete release

### Requirement: Protected release credentials
Signing and notarization credentials MUST be read only from GitHub encrypted secrets scoped to a protected release environment and MUST NOT be committed to the repository or exposed to pull-request workflows.

#### Scenario: Release credentials are absent
- **WHEN** the repository has not been provisioned with real signing credentials
- **THEN** configuration SHALL identify the missing deployment prerequisite without fabricating or persisting a secret

#### Scenario: Release job accesses credentials
- **WHEN** an authorized release deployment runs
- **THEN** only the release job SHALL receive the environment-scoped credentials required by its target platform

### Requirement: GitHub Release publication
Successful tagged builds SHALL create a GitHub Release with generated notes, downloadable binaries, checksums, SBOM metadata, and integrity attestations.

#### Scenario: Valid version tag is pushed
- **WHEN** a `vX.Y.Z` tag matches all project version declarations and all builds succeed
- **THEN** the workflow SHALL publish exactly one GitHub Release for that tag
