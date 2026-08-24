## ADDED Requirements

### Requirement: Settings SHALL provide a dedicated Local media capability page

The settings center SHALL register a lazily loaded `local-media` page under the capabilities group. The page SHALL manage the context-owned LocalMediaProfile without adding engine-specific fields to the generic application settings aggregate.

#### Scenario: The settings center opens Local media

* WHEN the user selects the Local media navigation item
* THEN the page SHALL lazy-load through the existing settings-page loader mechanism
* AND the navigation selection SHALL be addressable through the existing settings section route/query convention

#### Scenario: The application runs in Web mode

* WHEN the Local media page opens without a Tauri native host
* THEN it SHALL explain that local media is native-only
* AND it SHALL not show a successful runtime probe or pretend that local devices are available

### Requirement: The Local media page SHALL expose independent typed engine configuration

The page SHALL provide a master enable and independent PaddleOCR, faster-whisper, and sherpa-onnx sections with engine-specific typed fields, status, and probe actions.

#### Scenario: PaddleOCR is configured

* WHEN the OCR section is enabled
* THEN the page SHALL require a local Python executable
* AND it SHALL require either a local PaddleX configuration or explicit local text-detection and text-recognition model directories
* AND it SHALL expose language, device, optional explicitly local orientation model, and bounded PDF-page settings

#### Scenario: faster-whisper is configured

* WHEN the STT section is enabled
* THEN the page SHALL require a local Python executable and local model directory
* AND it SHALL expose microphone, language, device, compute type, VAD filter, beam size, and bounded recording duration

#### Scenario: sherpa-onnx TTS is configured

* WHEN the TTS section is enabled
* THEN the page SHALL require a local Python executable, model kind, model path, and tokens path
* AND it SHALL conditionally require model-kind-specific lexicon/data/dictionary/rule-FST fields
* AND it SHALL expose speaker, speed, threads, and output device

#### Scenario: An engine is disabled

* WHEN an engine section is disabled
* THEN its required path validation SHALL not block saving the disabled profile
* AND its composer action SHALL remain disabled

### Requirement: Settings SHALL separate editing, saving, and readiness probing

Unsaved settings SHALL not alter active workers. Save SHALL validate and commit one new profile revision. Probe actions SHALL check the saved profile and report per-engine readiness independently.

#### Scenario: The user edits a field

* WHEN a local-media field changes
* THEN the page SHALL enter a dirty state
* AND active workers and in-flight operations SHALL continue using the previously saved profile revision

#### Scenario: The user saves valid settings

* WHEN native validation succeeds for the expected revision
* THEN the profile SHALL be committed with an incremented revision
* AND affected engine status SHALL become Needs check or Restart required until readiness is established

#### Scenario: The user probes with unsaved edits

* WHEN the page is dirty and the user activates an engine Check action
* THEN the UI SHALL make clear that the saved profile is being checked
* AND unsaved paths SHALL NOT be used by the runtime

#### Scenario: A probe is running

* WHEN an engine probe operation is non-terminal
* THEN that engine SHALL show Checking and stable operation progress
* AND unrelated engine sections SHALL remain editable/usable according to existing settings concurrency policy

#### Scenario: Revision conflict occurs

* WHEN save returns `PROFILE_REVISION_CONFLICT`
* THEN the page SHALL preserve the user's unsaved draft values
* AND offer localized reload/reconcile guidance
* AND it SHALL not overwrite the newer stored profile silently

### Requirement: Readiness UI SHALL expose safe diagnostics without sensitive paths or content

Each engine card SHALL show `Disabled`, `Not configured`, `Needs check`, `Checking`, `Ready`, `Unavailable`, or `Restart required` and SHALL provide safe error guidance and metadata.

#### Scenario: A probe succeeds

* WHEN a saved engine is ready
* THEN the page MAY display package/engine version, resolved device, safe model/voice identity, and last-checked time
* AND it SHALL not expose full model/executable paths outside their explicit editable fields

#### Scenario: A probe fails

* WHEN a worker returns a stable configuration/import/model/device error
* THEN the page SHALL localize the error by code/message key
* AND it SHALL not display raw Python tracebacks or protocol frames

### Requirement: Settings SHALL not install or download local-media dependencies

The Local media page SHALL not include package-manager execution, Python installation, model download, voice download, hosted provider fallback, or automatic remediation controls.

#### Scenario: A dependency is missing

* WHEN the probe reports `PYTHON_NOT_FOUND`, `ENGINE_IMPORT_FAILED`, or `MODEL_NOT_FOUND`
* THEN the page SHALL explain the missing local prerequisite
* AND it SHALL not run an installer or model downloader

#### Scenario: A user selects a path

* WHEN a path-picker action is available
* THEN it SHALL use the application's established native path-picker service/pattern
* AND it SHALL only update the settings draft
* AND it SHALL not execute the selected file

### Requirement: Local media settings SHALL use existing settings interaction conventions

The page SHALL follow existing semantic styling, compact control density, validation, dirty/discard/save behavior, loading/error surfaces, and lazy-page lifecycle.

#### Scenario: The user discards changes

* WHEN the page is dirty and the user selects Discard
* THEN fields SHALL reset to the latest persisted profile
* AND no worker SHALL restart

#### Scenario: The page unmounts with active probe status

* WHEN the user navigates to another settings page
* THEN the probe operation MAY continue under operation ownership
* AND the disposed page SHALL not update React state after unmount

### Requirement: The Local media page SHALL expose CPU acceleration and vendor-compatibility remediation

The OCR card SHALL offer the three CPU acceleration modes as an explicit control defaulting to
`library-default`. When a probe reports `PADDLE_ONEDNN_MODEL_INCOMPATIBLE`, the page SHALL state that
the configured model is incompatible with the acceleration backend, SHALL offer to disable it and
re-check, SHALL warn that performance may drop, and SHALL apply nothing until the user confirms.

#### Scenario: The incompatibility is offered a remedy

* WHEN an OCR probe reports `PADDLE_ONEDNN_MODEL_INCOMPATIBLE`
* THEN the card SHALL present the disable-and-recheck action alongside the performance caveat
* AND declining SHALL leave the saved profile and the readiness state untouched

#### Scenario: The control is a saved profile field

* WHEN the user changes the acceleration mode directly
* THEN it SHALL be saved through the same optimistic-concurrency path as every other profile field
* AND the probe SHALL run against the saved value rather than the draft

### Requirement: Path-encoding incompatibility SHALL be reported against the exact field

When an engine cannot open a configured path, the page SHALL identify which single field is affected
-- detection model, recognition model, text-line orientation model, TTS model, tokens file, data
directory, lexicon, voices, vocoder, or rule FST -- and SHALL state the remediation as relocating
those files to a path the engine can open and reselecting them. The page SHALL NOT display the path,
and SHALL NOT offer to move, copy, or download anything.

#### Scenario: One field of several is incompatible

* WHEN the TTS data directory cannot be opened but the model and tokens can
* THEN the page SHALL mark the data directory field and SHALL leave the other two unmarked
* AND the message SHALL name the field, not the path

#### Scenario: The path is supported

* WHEN a field's path contains non-ASCII characters and its engine opens it successfully
* THEN no warning SHALL be shown for that field
