## ADDED Requirements

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

