# desktop-runtime-verification Specification

## Purpose
Defines a safe, repeatable verification contract for building, launching, exercising, and diagnosing the real VaneHub AI desktop runtime on each supported operating system.
## Requirements
### Requirement: Current-platform native verification
The verification system SHALL detect the host operating system and architecture, build a compatible native VaneHub AI desktop test artifact, and report only platforms that were actually executed.

#### Scenario: Verify on a supported host
- **WHEN** desktop verification starts on Windows, macOS, or Linux
- **THEN** it builds and selects an artifact for that host operating system and architecture
- **AND** other operating systems are reported as `NOT RUN`, not as passed

#### Scenario: Encounter an unsupported host
- **WHEN** desktop verification starts on an unsupported operating system or architecture
- **THEN** it stops before launch with a `BLOCKED` result that identifies the unsupported target

### Requirement: Real desktop runtime boundary
Desktop smoke verification MUST launch a native Tauri application and exercise its real WebView, React surface, Tauri IPC boundary, Rust command handler, and isolated native persistence. Browser-only execution or replacement of the verified IPC operation with a mock MUST NOT satisfy this requirement.

#### Scenario: Prove desktop backend readiness
- **WHEN** the native application reaches frontend readiness
- **THEN** the smoke suite executes a real registered read-only Tauri command against the running application
- **AND** it validates the returned native result without replacing that command with a mock

#### Scenario: Run browser-only E2E
- **WHEN** the Playwright Web/mock suite completes successfully without launching a native Tauri artifact
- **THEN** its result is reported as browser E2E and does not satisfy desktop smoke verification

### Requirement: Test automation is excluded from production builds
Automation plugins, automation permissions, and automation-only global APIs MUST be enabled only in an explicitly selected desktop test build and MUST be absent from normal production and release builds.

#### Scenario: Build a desktop test artifact
- **WHEN** the dedicated desktop test build is requested
- **THEN** the resulting native artifact exposes only the automation capabilities required by the smoke suite

#### Scenario: Build a production artifact
- **WHEN** a normal production or release build is requested
- **THEN** the resulting artifact excludes desktop test plugins, permissions, and automation-only global APIs

### Requirement: Desktop smoke acceptance
The desktop smoke suite SHALL verify successful process launch, main-window and React readiness, real backend IPC readiness, one stable basic interaction, absence of fatal frontend or native failures, and bounded clean shutdown.

#### Scenario: Desktop smoke passes
- **WHEN** the application starts, becomes ready, completes the real IPC probe and basic interaction, and exits within configured deadlines
- **THEN** desktop smoke reports `PASSED`
- **AND** no owned application process remains after cleanup

#### Scenario: Application fails during smoke
- **WHEN** startup, readiness, interaction, IPC, or shutdown fails or exceeds its deadline
- **THEN** desktop smoke reports `FAILED`
- **AND** it preserves failure evidence before cleanup

### Requirement: Isolated desktop test state
Each desktop verification run MUST use a unique temporary absolute `VANEHUB_APP_DATA_DIR` and run identifier so the test database, configuration, workspace fixtures, and logs do not read or mutate the user's normal application state.

#### Scenario: Start an isolated run
- **WHEN** the orchestrator launches a desktop test artifact
- **THEN** it supplies a unique temporary absolute application-data directory and run identifier
- **AND** the native runtime creates and uses its SQLite and log state under that isolated location

#### Scenario: Isolation cannot be established
- **WHEN** a safe temporary data directory cannot be created or validated
- **THEN** verification stops before application launch with a `BLOCKED` result

### Requirement: Metadata-driven artifact resolution
The verification system SHALL resolve the executable from declared Tauri and Cargo metadata plus the selected platform, architecture, and build profile, and SHALL fail explicitly rather than silently choosing an ambiguous or stale artifact.

#### Scenario: Resolve one matching artifact
- **WHEN** the requested desktop build succeeds and exactly one matching executable is present
- **THEN** verification records its absolute path, platform, architecture, profile, and test-build status before launch

#### Scenario: Resolve an ambiguous artifact
- **WHEN** no artifact or multiple incompatible artifacts match the requested build metadata
- **THEN** verification reports `FAILED` with the inspected locations and does not launch an arbitrary executable

### Requirement: Owned process lifecycle
The orchestrator MUST track the root application process and test-owned child processes for the active run and MUST restrict forced cleanup to those owned processes.

#### Scenario: Clean up a timed-out run
- **WHEN** a test-owned application exceeds a startup, interaction, or shutdown deadline
- **THEN** the orchestrator captures evidence and terminates only processes attributed to that test run

#### Scenario: Another VaneHub AI instance is running
- **WHEN** a user-owned or separately launched VaneHub AI process exists during cleanup
- **THEN** the orchestrator leaves that process running

### Requirement: Reviewable failure evidence
Failed desktop verification SHALL retain a run-scoped summary, assertion details, screenshot when a window is available, frontend and driver diagnostics, process state, and the existing redacted unified native logs. Evidence collection MUST NOT create a parallel unredacted native log sink.

#### Scenario: Preserve failure artifacts
- **WHEN** desktop smoke fails after the test run has been created
- **THEN** available evidence is written under a run-scoped test-results directory before process cleanup
- **AND** the summary identifies unavailable evidence without hiding the original failure

#### Scenario: Evidence contains application diagnostics
- **WHEN** native application logs are collected
- **THEN** they come from the isolated unified log directory and retain its required redaction behavior

### Requirement: Stable verification entry points and results
The repository SHALL provide independent npm entry points for desktop artifact construction and desktop smoke, plus a composed desktop verification entry point. Every requested verification layer MUST report one of `PASSED`, `FAILED`, `BLOCKED`, `NOT RUN`, or `NOT REQUIRED`, and `NOT REQUIRED` MUST include an impact-based reason.

#### Scenario: Run composed desktop verification
- **WHEN** a developer or coding agent invokes the composed desktop verification command on a supported host
- **THEN** it builds the desktop test artifact and runs desktop smoke in the defined order
- **AND** its process exit code is non-zero for `FAILED` or `BLOCKED`

#### Scenario: Skip an inapplicable layer
- **WHEN** impact analysis determines a verification layer is not required
- **THEN** the final result reports `NOT REQUIRED` with the reason instead of silently omitting the layer

### Requirement: CLI Agent terminal round-trip verification
Desktop verification SHALL prove, against the real native runtime, that a managed CLI Agent terminal starts, streams its output to the frontend, accepts input, and stops cleanly. The Agent binary under this layer MUST be a deterministic fixture that performs no network I/O and reads no credential store, so the layer's result depends on the runtime under test rather than on an installed Agent, a model provider, or an account.

#### Scenario: CLI terminal round trip succeeds
- **WHEN** the layer opens an Agent terminal for a CLI session whose Agent resolves to the fixture executable
- **THEN** the terminal SHALL reach `running` state with native capability
- **AND** the fixture's ready banner SHALL arrive at the frontend as terminal output
- **AND** content written through the Agent terminal input boundary SHALL come back in that Agent's terminal output
- **AND** stopping the terminal SHALL leave no owned Agent process running

#### Scenario: Fixture Agent cannot be resolved
- **WHEN** the fixture executable is absent from the resolution path, is not executable, or the Agent terminal never reaches `running`
- **THEN** the layer SHALL report `FAILED` and preserve its evidence
- **AND** it SHALL NOT fall back to a real installed CLI Agent

#### Scenario: Layer isolation from other desktop layers
- **WHEN** the CLI terminal layer runs
- **THEN** its fixture executable resolution SHALL NOT alter the environment of any other desktop verification layer
- **AND** each desktop verification layer SHALL report its own `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN` result and its own evidence directory

### Requirement: Native UI interaction coverage
Desktop verification SHALL exercise the client's primary interactive surfaces in the real desktop runtime rather than only asserting that they mount. Coverage SHALL include the session workspace tab set and the main-path dialogs, and SHALL assert rendered content and focus behavior produced by the desktop webview.

#### Scenario: Session workspace tabs carry their own content
- **WHEN** the layer opens a real session and selects each workspace tab in turn
- **THEN** every tab in the workspace tablist SHALL become the selected tab when activated
- **AND** the visible panel SHALL correspond to the selected tab and render that tab's own content
- **AND** no fatal frontend error SHALL be recorded during the traversal

#### Scenario: A main-path dialog honors its contract
- **WHEN** the layer opens a main-path dialog in the desktop client
- **THEN** the dialog SHALL be exposed as a dialog to assistive technology
- **AND** focus SHALL move into the dialog
- **AND** Escape SHALL close it and return focus to the surface that opened it

#### Scenario: Interaction coverage cannot substitute a mock runtime
- **WHEN** a requirement in this section is verified
- **THEN** it SHALL be verified against the native desktop artifact and its real service boundary
- **AND** a Web/mock adapter result SHALL NOT be accepted as evidence for it

### Requirement: Settings persistence across a real relaunch
Desktop verification SHALL prove that a setting changed through the desktop UI reaches native storage and survives an application restart, observed through the settings service rather than through browser storage.

#### Scenario: A changed setting survives relaunch
- **WHEN** the layer changes a setting through the rendered settings UI
- **THEN** the settings service SHALL report the new value
- **AND** after the application is relaunched against the same application-data directory, the settings service SHALL still report it
- **AND** the rendered settings UI SHALL present the restored value

#### Scenario: Persistence evidence is native
- **WHEN** the layer asserts that a setting persisted
- **THEN** it SHALL read the value through the native settings boundary
- **AND** it SHALL NOT accept browser storage as evidence of persistence

### Requirement: Desktop verification SHALL exercise local-media behavior through deterministic native fixtures

The desktop test harness SHALL verify OCR, recording, whole-utterance transcription, synthesis, playback, cancellation, worker failure, and composer integration without requiring a physical microphone, large production model, or audible output in default CI. Test-only fixture ports SHALL use the same application/service boundaries as production.

#### Scenario: CI verifies hold-to-talk

* WHEN a desktop test injects deterministic audio samples and a fake faster-whisper worker
* THEN press SHALL start native recording state
* AND release SHALL finalize one complete WAV operation
* AND the final transcript SHALL append to the latest active draft
* AND no send action SHALL occur

#### Scenario: CI verifies OCR

* WHEN a bounded image/PDF fixture is staged and the fake PaddleOCR worker succeeds
* THEN the composer SHALL show editable review
* AND only explicit confirmation SHALL append the edited text

#### Scenario: CI verifies TTS

* WHEN a fake sherpa-onnx worker generates a valid fixture WAV
* THEN desktop state SHALL enter generating then playing
* AND stop SHALL cancel playback and clean the output

### Requirement: Desktop verification SHALL cover cancellation and race boundaries

Tests SHALL cover pointer/keyboard cancellation, window blur, session switch, application shutdown, non-cooperative workers, and result/cancel races.

#### Scenario: The session changes during STT

* WHEN the active session changes before a transcription result commits
* THEN no draft in the new session SHALL be modified
* AND the disposed composer SHALL not apply the result

#### Scenario: A worker ignores cancellation

* WHEN a fake worker hangs after cancellation
* THEN the supervisor SHALL terminate only that engine worker after the grace period
* AND a later operation SHALL be able to start a replacement worker

#### Scenario: The app shuts down during recording/playback

* WHEN desktop shutdown begins with active media
* THEN capture/playback SHALL stop
* AND operation-owned files SHALL be cleaned or left eligible for the stale sweep

### Requirement: Desktop verification SHALL cover permission, device, path, and offline failures

The test matrix SHALL include stable error mapping for microphone denial/unavailability, playback unavailability, Python/import/model configuration, paths containing spaces/non-ASCII characters, malformed protocol, and denied network access.

#### Scenario: Microphone permission is denied

* WHEN the native capture fixture returns a permission-denied condition
* THEN the UI SHALL render localized `MIC_PERMISSION_DENIED` guidance
* AND it SHALL return to a recoverable idle state

#### Scenario: A model loader attempts networking

* WHEN worker socket creation is denied during an engine test
* THEN no external request SHALL succeed
* AND the operation SHALL produce `MODEL_DOWNLOAD_BLOCKED` or a stable local configuration error

#### Scenario: A configured path contains spaces

* WHEN the fake/local executable and model paths contain spaces or non-ASCII characters
* THEN the worker launch/request SHALL preserve the exact argument values without shell parsing

### Requirement: Real-platform evidence SHALL be recorded honestly

In addition to deterministic CI, available Windows, macOS, and Linux environments SHALL verify real device/permission/package behavior. Each platform/check SHALL be reported as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN` with a concrete reason; unavailable hardware or packages SHALL not be treated as passed.

#### Scenario: A platform is unavailable

* WHEN no macOS runner/device is available for real microphone permission verification
* THEN the evidence SHALL be marked `NOT RUN` or `BLOCKED` with the missing prerequisite
* AND deterministic tests SHALL not be misrepresented as real permission evidence

#### Scenario: A real local engine smoke test runs

* WHEN a developer machine has an explicitly configured compatible engine and model
* THEN the smoke test SHALL run without downloading anything
* AND evidence MAY record package/model/device versions
* AND it SHALL not record input media, transcript/OCR/TTS text, or full local paths

### Requirement: Existing Web and desktop regression suites SHALL remain part of acceptance

The change SHALL pass the repository's existing lint, unit, build, architecture, Rust, panic, OpenSpec, Playwright, and native desktop gates in addition to local-media-specific tests.

#### Scenario: Final change validation runs

* WHEN implementation is complete
* THEN `openspec validate add-local-composer-media-tools --strict` SHALL pass
* AND `openspec validate --specs --strict` SHALL pass
* AND required repository gates SHALL be reported with their actual outcomes

### Requirement: Vendor-compatibility qualification SHALL be separated from fixture verification

The record SHALL distinguish four kinds of evidence and SHALL NOT let one stand for another:
deterministic fixture verification, real-engine qualification, real-hardware qualification performed
by a person, and platforms with no host. A real-engine result SHALL name the package version and the
path shape it was obtained under. A fixture result SHALL NOT be recorded as evidence about an engine,
a device, or an operating-system permission prompt.

#### Scenario: An engine passes under one path shape and fails under another

* WHEN an engine qualifies with an ASCII path and fails with a non-ASCII one
* THEN both outcomes SHALL be recorded against that engine
* AND the engine SHALL NOT be recorded as qualified

#### Scenario: A scenario needs a person

* WHEN a scenario requires speaking, listening, or changing an operating-system setting
* THEN it SHALL be recorded as NOT RUN or BLOCKED until a person performs it
* AND no fixture or automated substitute SHALL be recorded in its place
