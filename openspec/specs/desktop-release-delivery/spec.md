# desktop-release-delivery Specification

## Purpose
Define how VaneHub validates synchronized versions, builds and publishes desktop releases, protects release credentials, and exposes integrity metadata and installation guidance for downloadable artifacts.
## Requirements
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

### Requirement: Cross-platform release artifacts
The release workflow SHALL build the declared Windows, macOS, and Linux desktop targets and SHALL publish only artifacts produced successfully for the release tag.

#### Scenario: All target builds succeed
- **WHEN** every declared target produces its expected bundle
- **THEN** one GitHub Release SHALL contain the collected platform artifacts and generated release notes

#### Scenario: A target build fails
- **WHEN** any required platform target cannot produce its bundle
- **THEN** the release workflow SHALL not publish a misleading complete release

### Requirement: Protected release credentials
Signing and notarization credentials MUST be read only from GitHub encrypted secrets scoped to a protected release environment and MUST NOT be committed to the repository or exposed to pull-request workflows. A credential that has not been provisioned MUST NOT reach the build as an empty value.

#### Scenario: Release credentials are absent
- **WHEN** the repository has not been provisioned with real signing credentials
- **THEN** configuration SHALL identify the missing deployment prerequisite without fabricating or persisting a secret

#### Scenario: Release job accesses credentials
- **WHEN** an authorized release deployment runs
- **THEN** only the release job SHALL receive the environment-scoped credentials required by its target platform

#### Scenario: Build runs without provisioned credentials
- **WHEN** a platform build runs and no signing credential is provisioned for it
- **THEN** the build SHALL receive no credential variable at all rather than a blank one
- **AND** it SHALL produce an unsigned distributable rather than failing

### Requirement: GitHub Release publication
Successful tagged builds SHALL create a GitHub Release with generated notes, downloadable binaries, checksums, SBOM metadata, and integrity attestations. The tag MAY carry a semantic-versioning pre-release identifier.

#### Scenario: Valid version tag is pushed
- **WHEN** a version tag matches all project version declarations and all builds succeed
- **THEN** the workflow SHALL publish exactly one GitHub Release for that tag

### Requirement: Published release assets
A published release SHALL attach only distributable package files. Intermediate build output produced while assembling a package MUST NOT be attached.

#### Scenario: Packaging leaves intermediate output beside a distributable
- **WHEN** a bundler writes staging directories or unpacked trees into the bundle output directory
- **THEN** those files SHALL NOT be collected as release assets

#### Scenario: Release assets are collected
- **WHEN** platform artifacts are gathered for publication
- **THEN** each collected file SHALL be an installable package in one of the declared distributable formats

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

### Requirement: Platform signing verification and stable gate
The existing release workflow SHALL sign Windows artifacts with an authorized publisher and timestamp, and SHALL sign macOS x64 and arm64 artifacts with Developer ID, notarize them, staple the ticket, and verify each result before publication. A stable tagged release MUST fail before publication when required credentials or any verification result is absent. A prerelease or manual rehearsal MAY produce unsigned artifacts only when its status is explicit.

#### Scenario: Signed Windows artifact is accepted
- **WHEN** the Windows release build has protected signing credentials
- **THEN** the workflow SHALL verify the artifact signature, expected publisher identity, and trusted timestamp before collection

#### Scenario: macOS artifact is accepted
- **WHEN** a macOS release build has protected signing and notarization credentials
- **THEN** the workflow SHALL run build, codesign verification, notarization, stapling, stapled-ticket verification, and collection in that order

#### Scenario: Stable signing prerequisite is absent
- **WHEN** a stable tag build lacks required Windows or macOS credentials or verification evidence
- **THEN** the workflow SHALL fail without publishing a release

#### Scenario: Unsigned rehearsal runs
- **WHEN** a manual non-publishing rehearsal has no production credentials
- **THEN** it SHALL build explicitly labeled unsigned artifacts and exercise the unsigned branch without fabricating signing success

### Requirement: Signed updater publication
The release workflow SHALL produce platform updater artifacts and signed channel metadata using the protected updater private key, include only the public key in client configuration, and publish metadata only after every required build and platform verification succeeds. Stable and preview metadata SHALL remain separate and SHALL name only compatible artifacts.

#### Scenario: Signed updater release succeeds
- **WHEN** every platform build and verification succeeds and the updater key is available
- **THEN** the workflow SHALL publish signed updater artifacts and the metadata for the tag's channel

#### Scenario: Updater signing is incomplete
- **WHEN** an updater artifact or metadata signature is missing or invalid
- **THEN** the workflow SHALL fail before publishing that release or replacing channel metadata

#### Scenario: Preview tag is published
- **WHEN** a semantic-version prerelease tag is eligible for publication
- **THEN** its updater metadata SHALL be published only to the preview channel
- **AND** stable channel metadata SHALL remain unchanged

### Requirement: Release credential isolation
Production signing, notarization, and updater private credentials MUST be available only to tag-triggered jobs using the protected `release` environment. Pull-request and manual rehearsal jobs MUST NOT receive those secrets, and logs, caches, artifacts, repository files, and release notes MUST NOT expose their values.

#### Scenario: Pull request workflow executes
- **WHEN** a pull-request workflow builds or tests release configuration
- **THEN** no production signing or notarization secret SHALL be requested or available

#### Scenario: Protected release job executes
- **WHEN** an authorized tag release enters the protected environment
- **THEN** only the target-specific signing step SHALL receive its required credentials
- **AND** verification output SHALL contain identity/status evidence but no credential value

### Requirement: Linux integrity disclosure
Linux release artifacts SHALL retain SHA-256 checksums, SPDX SBOM metadata, and provenance/SBOM attestations. Release documentation and machine-readable status MUST distinguish those integrity controls from operating-system code signing.

#### Scenario: Linux artifact is published
- **WHEN** a Linux distributable is included in a release
- **THEN** its served name and digest SHALL be present in the checksum manifest
- **AND** its SBOM and attestation evidence SHALL be published

#### Scenario: Release status describes Linux evidence
- **WHEN** release notes summarize platform verification
- **THEN** they SHALL describe Linux evidence as integrity/provenance and SHALL NOT claim code signing

### Requirement: Non-publishing release rehearsal
The manual release rehearsal SHALL validate workflow configuration, artifact collection, updater manifest schema, channel selection, signed test-fixture behavior, and explicit unsigned behavior without production private credentials, and MUST NOT create or mutate a production release or channel manifest.

#### Scenario: Maintainer runs rehearsal
- **WHEN** the package workflow is dispatched manually from a branch
- **THEN** all declared platform builds and release-policy tests SHALL run without production signing credentials
- **AND** no GitHub Release or production updater metadata SHALL be published

#### Scenario: Tampered rehearsal fixture is checked
- **WHEN** the rehearsal changes a signed test payload or signature
- **THEN** updater verification SHALL fail and the negative test SHALL pass only when publication remains disabled

### Requirement: Release verification reporting
Every platform matrix run SHALL report signing/notarization status as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN`, and release notes SHALL distinguish signed, unsigned, notarized, and integrity-only artifacts based on actual evidence.

#### Scenario: Platform did not execute
- **WHEN** native verification did not run for a platform
- **THEN** its status SHALL be `NOT RUN` rather than inferred from another platform

#### Scenario: Credential-dependent verification cannot run
- **WHEN** the target runner executes but protected credentials or an external signing service are unavailable
- **THEN** the platform status SHALL be `BLOCKED` for rehearsal or SHALL fail a stable release
