## ADDED Requirements

### Requirement: Declared distributable format set
The Tauri bundle configuration SHALL declare an explicit set of distributable formats rather than requesting every format the bundler supports. A format that cannot be produced from the project's declared version scheme SHALL be excluded from that set instead of being attempted and allowed to fail.

#### Scenario: Package build runs with a declared format set
- **WHEN** a local or workflow package command runs on a supported host platform
- **THEN** the bundler SHALL produce only the declared formats applicable to that platform

#### Scenario: Format is incompatible with the declared version
- **WHEN** a distributable format cannot represent the project's declared version, including a semantic-versioning pre-release identifier
- **THEN** that format SHALL be excluded from the declared set
- **AND** the exclusion SHALL be recorded as a known limitation rather than left to fail during a release build

#### Scenario: Every supported platform retains an installable format
- **WHEN** formats are excluded from the declared set
- **THEN** Windows, macOS, and Linux SHALL each retain at least one installable distributable format

## MODIFIED Requirements

### Requirement: Packaging documentation
The system SHALL document local prerequisites, local packaging commands, GitHub Actions behavior, artifact locations, produced distributable formats, excluded formats and the reason for each exclusion, and known platform or architecture limitations.

#### Scenario: Maintainer reads packaging documentation
- **WHEN** a maintainer follows the packaging documentation
- **THEN** they can identify required local tooling, the command to run, and where to find generated artifacts

#### Scenario: Maintainer reviews CI documentation
- **WHEN** a maintainer reviews the CI packaging documentation
- **THEN** they can identify workflow triggers, artifact naming, and unsupported or credential-dependent release steps

#### Scenario: Maintainer looks for an excluded format
- **WHEN** a maintainer or downloader looks for a distributable format the project does not produce
- **THEN** the documentation SHALL identify that the format is not produced and why
- **AND** it SHALL identify the format that serves the same platform instead
