## ADDED Requirements

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
