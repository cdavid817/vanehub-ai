## Purpose

Defines how VaneHub AI produces native desktop package artifacts locally and through GitHub Actions for supported operating systems and architectures.
## Requirements
### Requirement: Local one-command packaging
The system SHALL provide a documented local command that builds the frontend and produces Tauri desktop package artifacts for the current host platform.

#### Scenario: Maintainer runs the local package command
- **WHEN** a maintainer runs the documented local package command on a supported host platform
- **THEN** the system builds the web frontend and invokes the Tauri desktop bundler for that host platform

#### Scenario: Local package command completes successfully
- **WHEN** the local package command finishes without errors
- **THEN** package artifacts are available under the Tauri bundle output directory

### Requirement: Platform package coverage
The system SHALL support native package generation for Windows, macOS, and Linux through platform-appropriate build environments.

#### Scenario: Windows package build runs
- **WHEN** the package workflow runs in a Windows build environment
- **THEN** the system produces Windows desktop package artifacts using the Tauri bundler

#### Scenario: macOS package build runs
- **WHEN** the package workflow runs in a macOS build environment
- **THEN** the system produces macOS desktop package artifacts using the Tauri bundler

#### Scenario: Linux package build runs
- **WHEN** the package workflow runs in a Linux build environment with required native dependencies installed
- **THEN** the system produces Linux desktop package artifacts using the Tauri bundler

### Requirement: Architecture target coverage
The system SHALL define x86_64 and ARM64 packaging targets for supported platform builds.

#### Scenario: x86_64 target build runs
- **WHEN** a packaging workflow entry is configured for an x86_64 target
- **THEN** the build uses the matching Rust target and emits artifacts labeled for x86_64

#### Scenario: ARM64 target build runs
- **WHEN** a packaging workflow entry is configured for an ARM64 target
- **THEN** the build uses the matching Rust target and emits artifacts labeled for ARM64

#### Scenario: Unsupported target combination is documented
- **WHEN** a requested platform and architecture combination cannot be built reliably by the configured environment
- **THEN** the limitation is documented instead of being silently treated as supported

### Requirement: GitHub Actions native build workflow
The system SHALL provide a GitHub Actions workflow that builds native desktop packages on operating-system runners matching the target platform.

#### Scenario: Workflow is triggered manually
- **WHEN** a maintainer starts the package workflow from GitHub Actions
- **THEN** the workflow runs the configured platform and architecture build matrix

#### Scenario: Workflow uses native runners
- **WHEN** the workflow builds a Windows, macOS, or Linux package
- **THEN** the job runs on a GitHub Actions runner for that same operating-system family

#### Scenario: Workflow installs platform prerequisites
- **WHEN** a workflow job requires system packages or Rust targets for its platform
- **THEN** the job installs those prerequisites before invoking the Tauri build

### Requirement: CI artifact publication
The system SHALL upload build artifacts from GitHub Actions with names that identify the application, platform, architecture, and source version context.

#### Scenario: CI build succeeds
- **WHEN** a GitHub Actions packaging job completes successfully
- **THEN** the job uploads the generated desktop package artifacts

#### Scenario: Artifact is named
- **WHEN** an artifact is uploaded from CI
- **THEN** its artifact name includes VaneHub AI, the target platform, the target architecture, and version or git reference context

### Requirement: Packaging documentation
The system SHALL document local prerequisites, local packaging commands, GitHub Actions behavior, artifact locations, and known platform or architecture limitations.

#### Scenario: Maintainer reads packaging documentation
- **WHEN** a maintainer follows the packaging documentation
- **THEN** they can identify required local tooling, the command to run, and where to find generated artifacts

#### Scenario: Maintainer reviews CI documentation
- **WHEN** a maintainer reviews the CI packaging documentation
- **THEN** they can identify workflow triggers, artifact naming, and unsupported or credential-dependent release steps

### Requirement: Optimized distributable native artifacts
Native package commands and release workflows SHALL build distributable binaries with the declared optimized Cargo release profile.

#### Scenario: Run local native packaging
- **WHEN** a maintainer runs a supported local Tauri package command
- **THEN** the packaged native binary SHALL use the declared release optimization and debuginfo-stripping policy

#### Scenario: Run tagged release packaging
- **WHEN** the release workflow builds a supported native target
- **THEN** the packaged native binary SHALL use the same declared release profile as local release packaging
- **AND** the workflow SHALL fail rather than publish an artifact built without required linker prerequisites

### Requirement: Packaging optimization documentation
Packaging documentation SHALL identify the release profile, platform linker prerequisites, expected release-build tradeoffs, and retained production diagnostic behavior.

#### Scenario: Review release build guidance
- **WHEN** a maintainer prepares a local or CI package build
- **THEN** the documentation SHALL identify required linker tools for the target platform
- **AND** it SHALL explain that ThinLTO and a single codegen unit can increase release build time and can affect optimization or distributable size without guaranteeing a size reduction
- **AND** it SHALL distinguish measured artifact sizes from a demonstrated before/after size improvement
- **AND** it SHALL state that operational debug-level unified logs remain available in release packages
