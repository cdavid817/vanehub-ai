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

#### Scenario: Observe frontend startup handoff
- **WHEN** the native application has created its main window but visible React application content is not yet rendered
- **THEN** the application reports frontend readiness as `starting` and keeps the branded startup surface visible
- **AND** readiness transitions to `ready` only after the application root contains rendered surface content

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
Desktop verification SHALL exercise the client's primary interactive surfaces in the real desktop runtime rather than only asserting that they mount. Coverage SHALL include the session workspace tab set, main-path dialogs, and scheduled-task management, and SHALL assert rendered content, focus behavior, and native persistence produced through the desktop webview.

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

#### Scenario: Startup activity differs from the tested surface
- **WHEN** a native UI test starts while an unrelated activity is selected
- **THEN** the test SHALL navigate explicitly through a stable accessible control before interacting with the target surface
- **AND** it SHALL NOT assume that a content-specific control exists on every startup activity

#### Scenario: Scheduled task native lifecycle
- **WHEN** the layer opens Scheduled Tasks and submits a valid task for a stable CLI Agent id
- **THEN** the rendered list and native scheduled-task service SHALL expose the created record and recurrence
- **AND** disabling and enabling the task through the UI SHALL persist the corresponding native state
- **AND** confirming deletion through the UI SHALL remove the native record

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

### Requirement: Two desktop verification gates with distinct prerequisites

Desktop verification SHALL be split into a Required Hermetic Desktop Gate and an External Provider Desktop Suite, and every desktop spec SHALL belong to exactly one of them.

The split exists because a single suite cannot be both. A gate every pull request must pass cannot depend on a real CLI Agent, a real credential, or a real vendor download, and a suite that verifies the real thing cannot be hermetic. Merging them means either the gate silently requires a developer's machine — which is what made `desktop-smoke` fail on all three hosted runners for want of `codex` on PATH — or the real-integration cases quietly stop running.

#### Scenario: Required gate runs on an ordinary pull request

- **WHEN** the Required Hermetic Desktop Gate runs on Windows, macOS, or Linux
- **THEN** it SHALL run against a temporary HOME, PATH, user-data directory, and SQLite database
- **AND** it SHALL resolve every CLI Agent to a fixture executable rather than a host installation
- **AND** it SHALL NOT contact a real provider, read a credential store, download from a vendor, or read the user's application state
- **AND** any failing required spec SHALL fail the gate

#### Scenario: Required gate cannot silently degrade

- **WHEN** a required spec cannot run because a CLI Agent, package manager, or other fixture-resolvable prerequisite is missing
- **THEN** the gate SHALL report `FAILED` rather than skipping the spec
- **AND** the missing prerequisite SHALL be treated as a defect in the fixture, not as an environment block

#### Scenario: Required spec reports a genuinely external prerequisite

- **WHEN** part of a required spec depends on something no fixture can stand in for, such as a live vendor release endpoint
- **THEN** that part MAY record a `BLOCKED` reason and continue
- **AND** the reason SHALL name the prerequisite
- **AND** the gate SHALL still report `PASSED` only if no required assertion failed

#### Scenario: External provider suite runs outside the gate

- **WHEN** the External Provider Desktop Suite is dispatched
- **THEN** it SHALL be triggered manually, on a schedule, or by a protected label rather than by an ordinary pull request
- **AND** it SHALL NOT be a required check for merging

#### Scenario: External provider suite lacks its prerequisites

- **WHEN** a real CLI Agent, credential, or provider endpoint the suite needs is absent
- **THEN** it SHALL record `BLOCKED` with the specific missing prerequisite
- **AND** it SHALL NOT record `PASSED`
- **AND** the `BLOCKED` result SHALL NOT count toward the Required Hermetic Desktop Gate

### Requirement: Every desktop spec is classified and the classification is enforced

Each desktop spec SHALL carry exactly one classification of `required-fixture`, `external-provider`, or `duplicate-replaced`, recorded in a manifest that automated tests check.

A classification kept only in prose drifts the first time a spec is added or renamed. Enforcing it mechanically is what keeps "every spec is classified" true rather than aspirational.

#### Scenario: A spec is added without a classification

- **WHEN** a desktop spec file exists that the manifest does not classify
- **THEN** the desktop verification tests SHALL fail and name the unclassified spec

#### Scenario: The manifest names a spec that no longer exists

- **WHEN** a manifest entry has no corresponding spec file
- **THEN** the desktop verification tests SHALL fail and name the stale entry

#### Scenario: A required spec declares an external prerequisite

- **WHEN** a spec classified `required-fixture` declares a real credential, a real provider, or vendor network access
- **THEN** the desktop verification tests SHALL fail

#### Scenario: An external spec reaches the required command

- **WHEN** a spec classified `external-provider` is included in the Required Hermetic Desktop Gate's spec set
- **THEN** the desktop verification tests SHALL fail

#### Scenario: A replaced spec names no replacement

- **WHEN** a spec is classified `duplicate-replaced`
- **THEN** the manifest SHALL name the spec or layer that covers the same behaviour
- **AND** the desktop verification tests SHALL fail if that replacement does not exist

### Requirement: Fixture-resolvable behaviour belongs to the required gate

A desktop spec that verifies CLI process lifecycle, standard output or error handling, session creation, tab, drawer or dialog behaviour, operations, cancellation, error reporting, persistence, PATH resolution, or the Agent Runtime call boundary SHALL be classified `required-fixture` and driven by a fixture CLI.

These behaviours are properties of this application, not of any vendor's binary. Verifying them against a real CLI Agent buys nothing and costs the ability to run the gate anywhere.

#### Scenario: A spec needs an installed Agent to exercise application behaviour

- **WHEN** a required spec needs a CLI Agent to be present
- **THEN** the gate SHALL place fixture executables for the managed Agent names ahead of the inherited PATH
- **AND** the spec SHALL exercise the same production resolution, launch, and persistence paths against them

#### Scenario: Only vendor-specific truth is external

- **WHEN** a spec verifies a real provider login, real account permissions, a real server response, real model output, or a real vendor CLI's current-version compatibility
- **THEN** it SHALL be classified `external-provider`
- **AND** it SHALL declare its prerequisites and the reason it is blocked without them

### Requirement: Feishu IM desktop verification layer
Desktop verification SHALL provide a WebdriverIO layer that launches the native Tauri client with isolated state and deterministically exercises the session IM switch and Feishu delivery boundaries without requiring live credentials for its default run.

#### Scenario: Verify default-off opt-in
- **WHEN** the Feishu IM desktop layer opens a new single-Agent or multi-Agent session
- **THEN** it SHALL observe the information-panel IM switch as off through the real desktop WebView
- **AND** it SHALL verify through the native service boundary that inbound delivery is ineligible

#### Scenario: Verify single-Agent delivery
- **WHEN** the layer enables IM, establishes a fixture Feishu binding, and injects a unique direct-message event
- **THEN** it SHALL observe exactly one Agent turn and one ordered final-response delivery through deterministic fixtures

#### Scenario: Verify multi-Agent routing
- **WHEN** the layer injects messages with a valid seat mention, no seat mention, and an invalid seat mention into an enabled multi-Agent session
- **THEN** it SHALL verify the required stable-seat routing, default routing, and safe rejection behaviors

#### Scenario: Verify resilience boundaries
- **WHEN** the layer exercises duplicate events, disabled sessions, connector interruption, oversized output, malformed events, and application restart
- **THEN** it SHALL verify idempotency, no execution while disabled, safe recovery, ordered chunking, redacted failure evidence, and persisted switch state

### Requirement: Live Feishu qualification is reported separately
Verification results SHALL distinguish deterministic connector fixtures from tests executed against a real Feishu tenant and SHALL never report fixture success as live-platform success.

#### Scenario: Live credentials are unavailable
- **WHEN** no explicitly supplied Feishu test tenant and credentials are available
- **THEN** deterministic desktop scenarios MAY pass
- **AND** live Feishu authentication, event reception, acknowledgement, and reply delivery SHALL be reported as `NOT RUN` or `BLOCKED` with the missing prerequisite

#### Scenario: Live qualification is authorized
- **WHEN** an operator explicitly supplies a Feishu test tenant, application credentials, and a permitted test chat
- **THEN** the qualification SHALL exercise authentication, connection lifecycle, direct-message receipt, duplicate delivery, single-Agent reply, multi-Agent routing, and outbound reply
- **AND** retained evidence SHALL exclude credentials, external identifiers, and message contents

### Requirement: Multi-connector session authorization verification
Desktop verification SHALL exercise connector-scoped session authorization through the rendered Tauri client and native persistence boundary without requiring live external credentials.

#### Scenario: Verify non-Feishu default denial
- **WHEN** the deterministic desktop layer injects a Telegram, DingTalk, WeCom, or personal WeChat direct message for a session without matching enabled access
- **THEN** it SHALL observe no Agent execution and a safe disabled outcome

#### Scenario: Verify selected connector isolation
- **WHEN** the layer enables one connector for a session while another connector remains disabled
- **THEN** pairing and inbound delivery SHALL succeed only for the enabled connector

#### Scenario: Verify persisted connector choice
- **WHEN** the layer selects and enables a non-Feishu connector and relaunches the desktop client
- **THEN** the information panel SHALL restore that connector's native persisted access state
- **AND** the layer SHALL NOT use browser storage as persistence evidence
