## MODIFIED Requirements

### Requirement: Synchronized release version
The release workflow MUST reject a tag when its semantic version does not match the versions declared in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. A semantic-versioning pre-release identifier MUST be treated as part of the version and MUST be present identically in all three declarations and in the tag.

#### Scenario: Release versions disagree
- **WHEN** a release tag or one of the three version declarations differs
- **THEN** the release workflow SHALL fail before building distributable artifacts

#### Scenario: Pre-release version is synchronized
- **WHEN** a tag carrying a pre-release identifier matches all three version declarations exactly
- **THEN** the version check SHALL pass without stripping or normalizing the pre-release identifier

#### Scenario: Pre-release identifier differs from the tag
- **WHEN** the three declarations agree on a base version but their pre-release identifier differs from the tag's
- **THEN** the release workflow SHALL fail before building distributable artifacts

### Requirement: GitHub Release publication
Successful tagged builds SHALL create a GitHub Release with generated notes, downloadable binaries, checksums, SBOM metadata, and integrity attestations. The tag MAY carry a semantic-versioning pre-release identifier.

#### Scenario: Valid version tag is pushed
- **WHEN** a version tag matches all project version declarations and all builds succeed
- **THEN** the workflow SHALL publish exactly one GitHub Release for that tag

## ADDED Requirements

### Requirement: Pre-release publication marking
The release workflow SHALL determine a release's pre-release status from the published tag alone and SHALL mark a release carrying a pre-release identifier as a GitHub pre-release that is not promoted to the repository's latest release.

#### Scenario: Pre-release tag is published
- **WHEN** the published tag carries a semantic-versioning pre-release identifier
- **THEN** the resulting GitHub Release SHALL be marked as a pre-release
- **AND** it SHALL NOT be marked as the repository's latest release

#### Scenario: Stable tag is published
- **WHEN** the published tag carries no pre-release identifier
- **THEN** the resulting GitHub Release SHALL NOT be marked as a pre-release

#### Scenario: Maintainer omits an explicit pre-release setting
- **WHEN** a maintainer pushes a pre-release tag without configuring any additional workflow input
- **THEN** the release SHALL still be marked as a pre-release

### Requirement: Untagged release rehearsal
The release workflow SHALL support a manual run that validates project versions and exercises the full build matrix without requiring a tag, and version validation MUST NOT interpret a branch reference as a release tag.

#### Scenario: Manual run on a branch
- **WHEN** a maintainer starts the release workflow manually against a branch
- **THEN** version validation SHALL verify that the three version declarations agree
- **AND** it SHALL NOT compare the branch name against a version

#### Scenario: Manual run builds every declared target
- **WHEN** a manual run passes version validation
- **THEN** the workflow SHALL run every declared platform build and upload its artifacts

#### Scenario: Manual run does not publish
- **WHEN** a manual run completes successfully without a tag
- **THEN** the workflow SHALL NOT create a GitHub Release

### Requirement: Unsigned distribution disclosure
When release artifacts are produced without code-signing or notarization credentials, the release notes SHALL state that the packages are unsigned and SHALL provide the per-platform steps a downloader needs to install and launch the application despite operating-system protections.

#### Scenario: Unsigned release is published
- **WHEN** a release is published without signing credentials
- **THEN** its notes SHALL state that the packages are unsigned and un-notarized
- **AND** they SHALL explain that checksums, SBOM metadata, and attestations establish integrity but do not replace operating-system code signing

#### Scenario: Downloader installs an unsigned package
- **WHEN** a downloader follows the published notes on macOS, Windows, or Linux
- **THEN** the notes SHALL identify the protection prompt that platform presents
- **AND** they SHALL give the concrete step required to proceed

#### Scenario: Release notes are reviewed before publication
- **WHEN** the installation guidance changes
- **THEN** it SHALL be reviewed as a tracked repository file rather than authored inside the workflow run
