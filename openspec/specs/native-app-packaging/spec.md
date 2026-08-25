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

### Requirement: Desktop bundles SHALL include only the local-media worker bridge, not inference environments or models

The packaged desktop application SHALL include the versioned local-media Python bridge and protocol metadata required to launch user-configured local engines. It SHALL NOT bundle a Python interpreter, PaddleOCR/Paddle/PaddleX, faster-whisper/CTranslate2, sherpa-onnx, CUDA runtimes, or OCR/STT/TTS models and voices as part of this change.

#### Scenario: A packaged application resolves the worker

* WHEN the native host starts a local-media worker from an installed/package build
* THEN it SHALL resolve the bridge through the Tauri resource mechanism
* AND it SHALL not rely on the repository's development working directory

#### Scenario: Bundle contents are inspected

* WHEN release artifacts are reviewed
* THEN the bridge source/resources SHALL be present
* AND unrequested Python environments, inference packages, CUDA libraries, and model bundles SHALL be absent

### Requirement: Worker launch SHALL preserve local paths safely across supported platforms

The native host SHALL start the configured Python executable directly with argument APIs, without shell interpolation, and SHALL support spaces and non-ASCII characters in executable, bridge, model, and temporary paths.

#### Scenario: A Windows Python path contains spaces

* WHEN the configured executable or model path contains spaces or non-ASCII characters
* THEN the worker SHALL be launched using structured process arguments
* AND `cmd.exe`, PowerShell, or string-built shell commands SHALL not be used

#### Scenario: The worker executable is not runnable

* WHEN the configured executable does not exist or cannot be executed
* THEN the operation/probe SHALL return `PYTHON_NOT_FOUND` or `PYTHON_EXECUTION_DENIED`
* AND it SHALL not attempt a system-wide Python fallback implicitly

### Requirement: macOS packaging SHALL declare microphone usage before release

The macOS application metadata SHALL contain a localized or product-approved `NSMicrophoneUsageDescription` that explains local speech input. Merely opening the composer or Local media settings SHALL not trigger a permission prompt.

#### Scenario: The user starts recording for the first time on macOS

* WHEN the operating system requires microphone consent
* THEN the packaged application SHALL present the configured usage description
* AND denial SHALL map to `MIC_PERMISSION_DENIED`

#### Scenario: The user opens settings without recording

* WHEN Local media settings loads or an engine path is edited
* THEN the application SHALL not request microphone permission solely for page rendering

### Requirement: Platform audio runtime prerequisites SHALL be explicit and verifiable

Release documentation and tests SHALL identify required native audio runtime dependencies for supported Windows, macOS, and Linux targets. The application SHALL report missing devices/services with stable errors and SHALL not silently install system packages.

#### Scenario: Linux audio services are unavailable

* WHEN the target environment lacks a usable supported input/output backend or device
* THEN capture/playback SHALL return `MIC_DEVICE_UNAVAILABLE` or `PLAYBACK_DEVICE_UNAVAILABLE`
* AND the application SHALL not execute a package manager

#### Scenario: Release documentation is generated

* WHEN this feature is included in a release
* THEN documentation SHALL state that users must provision compatible local Python packages/models and platform audio runtime dependencies
* AND it SHALL state that VaneHub does not download them through this feature
