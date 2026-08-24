## ADDED Requirements

### Requirement: The application SHALL own local OCR, speech recognition, and speech synthesis in a dedicated native context

The native host SHALL provide a peer bounded context named `local_media` that owns local-media profiles, engine readiness, worker supervision, OCR input admission, native microphone capture, whole-utterance transcription, speech synthesis, local playback, typed results, and ephemeral media lifecycle. React components and other native contexts SHALL access this capability only through published service/API boundaries.

#### Scenario: React requests a local-media action

* WHEN a composer or settings component needs local-media behavior
* THEN it SHALL call the frontend `LocalMediaService`
* AND the Tauri adapter SHALL invoke a thin native command
* AND the component SHALL NOT call Tauri `invoke` directly

#### Scenario: Another native context needs OCR

* WHEN OnePiece requests OCR for an admitted artifact
* THEN it SHALL call `local_media::api`
* AND it SHALL NOT import worker, process, profile repository, or temporary-file infrastructure directly

### Requirement: Local-media profiles SHALL be explicit, versioned, and locally provisioned

The system SHALL persist a context-owned `LocalMediaProfile` containing independent PaddleOCR, faster-whisper, and sherpa-onnx configuration. The profile SHALL be disabled and unconfigured by default, SHALL use optimistic revision control, and SHALL require explicit local Python executable/model paths for enabled engines.

#### Scenario: First use has no saved profile

* WHEN the profile repository has no `default` row
* THEN the system SHALL return and persist disabled defaults
* AND it SHALL NOT launch Python or request microphone permission

#### Scenario: A stale settings page saves changes

* WHEN `expectedRevision` does not equal the stored profile revision
* THEN the save SHALL fail with `PROFILE_REVISION_CONFLICT`
* AND the stored profile SHALL remain unchanged

#### Scenario: An enabled engine omits required local files

* WHEN an enabled engine profile lacks its required executable/model fields
* THEN validation SHALL fail with `ENGINE_UNCONFIGURED` or `MODEL_NOT_CONFIGURED`
* AND no worker SHALL start

### Requirement: Engine readiness SHALL be explicit and independent

The system SHALL expose independent readiness for OCR, STT, and TTS using the states `Disabled`, `Unconfigured`, `Checking`, `Ready`, `Unavailable`, and `RestartRequired`. Readiness SHALL identify the saved profile revision and SHALL not expose full local paths.

#### Scenario: One engine probe fails

* WHEN the PaddleOCR probe fails but faster-whisper and sherpa-onnx are ready
* THEN only OCR SHALL be unavailable
* AND STT and TTS SHALL remain usable

#### Scenario: A ready engine profile changes

* WHEN a saved engine profile revision changes
* THEN an idle worker using the previous revision SHALL be stopped before its next request
* AND status SHALL indicate that a restart/check is required until readiness is re-established

### Requirement: Local-media inference SHALL use isolated versioned workers

The application SHALL run PaddleOCR, faster-whisper, and sherpa-onnx in three independently supervised local Python processes. Each worker SHALL use a bounded versioned JSON Lines protocol over stdin/stdout, one active inference at a time, bounded queueing, and isolated restart/backoff.

#### Scenario: A worker starts successfully

* WHEN the supervisor launches an engine worker
* THEN the worker SHALL emit a protocol-v1 hello frame within the handshake deadline
* AND the engine identifier and supported methods SHALL match the requested worker slot

#### Scenario: A worker contaminates stdout

* WHEN stdout contains malformed JSON, an oversized frame, an unknown protocol version, a mismatched response ID, or non-protocol text
* THEN the supervisor SHALL terminate that worker
* AND the active operation SHALL fail with `WORKER_PROTOCOL_ERROR`
* AND unrelated engine workers SHALL remain running

#### Scenario: An engine queue is full

* WHEN one engine already has an active request and its bounded waiting queue is full
* THEN a new request for that engine SHALL fail with `ENGINE_BUSY`
* AND the request SHALL NOT be stored in an unbounded queue

### Requirement: Workers SHALL be offline-only and SHALL NOT acquire models or packages

The feature SHALL have no cloud OCR/STT/TTS adapter, no hosted fallback, and no automatic package/model installation. Workers SHALL receive explicit local model paths, offline environment configuration, and local-only model loading options where supported.

#### Scenario: A configured model path is absent

* WHEN a worker cannot resolve the explicit local model path
* THEN it SHALL return `MODEL_NOT_FOUND` or `MODEL_NOT_CONFIGURED`
* AND it SHALL NOT attempt to download a replacement

#### Scenario: A library attempts network access during a test

* WHEN socket creation is denied and a dependency attempts remote model resolution
* THEN the operation SHALL fail with `MODEL_DOWNLOAD_BLOCKED` or a stable local configuration error
* AND no hosted fallback SHALL run

#### Scenario: Proxy variables exist in the parent application

* WHEN a worker is launched
* THEN inherited proxy variables SHALL be removed by default
* AND relevant offline flags SHALL be set

### Requirement: Long-running local-media work SHALL use stable asynchronous operations

Engine probes, OCR, STT, and TTS SHALL return stable operation IDs immediately and SHALL integrate with the existing operation status/cancellation runtime. Media-specific results SHALL be available through a typed local-media result query.

#### Scenario: OCR is accepted

* WHEN a valid staged OCR source is submitted
* THEN the command SHALL return an operation ID without waiting for inference completion
* AND generic operation status SHALL expose its phase and terminal state
* AND typed result lookup SHALL expose the bounded OCR result after success

#### Scenario: A result is read twice

* WHEN a caller requests a retained terminal result more than once
* THEN each read SHALL return the same typed result
* AND reading SHALL NOT re-run inference

#### Scenario: Result retention expired

* WHEN a terminal typed result has expired under operation retention policy
* THEN result lookup SHALL return `OPERATION_RESULT_EXPIRED`

### Requirement: Every operation SHALL use an immutable profile snapshot

An accepted operation SHALL capture the saved profile revision, engine settings, validated local path identities, limits, device parameters, and creation metadata. Later settings changes SHALL affect only later operations.

#### Scenario: Settings change during transcription

* WHEN the user saves a different faster-whisper model while an STT operation is running
* THEN the running operation SHALL continue with its captured profile snapshot unless cancelled
* AND the next operation SHALL use the new revision

### Requirement: OCR inputs SHALL be admitted into operation-owned local storage

Composer OCR SHALL accept only a user-selected supported image or PDF. OnePiece OCR SHALL accept only managed artifacts. Before inference, the system SHALL validate content and limits, copy/materialize input under the local-media temporary root with an opaque ID, and pass only that canonical admitted path to the worker.

#### Scenario: A composer selects a valid image

* WHEN the selected regular file passes content sniffing, byte, and pixel limits
* THEN it SHALL be copied to an opaque staging directory
* AND the frontend SHALL receive only staged metadata and an opaque staged-input ID
* AND the worker SHALL receive the staged path rather than the original caller path

#### Scenario: A selected file uses a misleading extension

* WHEN content sniffing does not match an allowed image/PDF type
* THEN staging SHALL fail with `UNSUPPORTED_MEDIA_TYPE`
* AND no OCR worker request SHALL be sent

#### Scenario: A staged input is reused

* WHEN an already claimed or expired staged-input ID is submitted again
* THEN the request SHALL fail with `INPUT_NOT_FOUND`
* AND it SHALL NOT access another operation's file

#### Scenario: A OnePiece caller supplies a host path

* WHEN an arbitrary host path is supplied outside the artifact contract
* THEN the OnePiece OCR request SHALL be rejected

### Requirement: OCR processing SHALL be local, bounded, structured, and shared

The system SHALL run OCR with the configured local PaddleOCR environment and explicit local model configuration. It SHALL preserve page/reading order, return structured page results and provenance, derive deterministic plain text, and share one runtime owner between composer OCR and OnePiece OCR.

#### Scenario: PaddleOCR is configured with explicit model directories

* WHEN OCR starts
* THEN the worker SHALL use the configured text-detection and text-recognition model directories
* AND optional orientation/preprocessing models SHALL remain disabled unless their local paths are explicitly configured

#### Scenario: A multi-page PDF succeeds

* WHEN OCR completes within page/output limits
* THEN page results SHALL remain in source order
* AND lines SHALL remain in engine reading order
* AND derived plain text SHALL join lines with newlines and pages with a blank line
* AND provenance SHALL identify PaddleOCR, profile revision, language, and safe model/version metadata

#### Scenario: No text is recognized

* WHEN the engine returns no recognized text without an infrastructure error
* THEN the result SHALL report `NO_TEXT_DETECTED`
* AND the condition SHALL NOT be mapped to `WORKER_CRASHED`

#### Scenario: Composer and OnePiece run OCR

* WHEN both product entry points submit OCR operations
* THEN both SHALL use the same `local_media` PaddleOCR worker slot, profile, admission policy, and error mapping
* AND no second PaddleOCR process owner SHALL exist

### Requirement: Microphone capture SHALL remain native and bounded

The system SHALL capture microphone samples in Rust, SHALL keep audio bytes out of frontend/Tauri JSON payloads, SHALL permit only one active recording application-wide, and SHALL write a committed 16-bit PCM mono WAV through a bounded non-real-time writer path.

#### Scenario: Recording starts

* WHEN a configured/default microphone opens successfully
* THEN the native host SHALL return an opaque recording ID
* AND raw samples SHALL remain in native memory/file handling
* AND no audio bytes SHALL be placed in frontend state or command payloads

#### Scenario: A second recording is requested

* WHEN a recording is already active
* THEN the second start SHALL fail with `RECORDING_ALREADY_ACTIVE`

#### Scenario: The audio writer cannot keep up

* WHEN the bounded sample channel overruns
* THEN recording SHALL fail with `AUDIO_CAPTURE_OVERRUN`
* AND the system SHALL NOT silently drop an unbounded portion of the utterance

#### Scenario: Microphone permission is denied

* WHEN the operating system denies microphone access
* THEN start SHALL fail with `MIC_PERMISSION_DENIED`
* AND no recording file SHALL remain

### Requirement: Hold release SHALL transcribe the complete utterance with local faster-whisper

Releasing a valid active recording SHALL finalize the full WAV and start one local faster-whisper operation. V1 SHALL return only the final transcript and bounded language/duration metadata; it SHALL NOT stream partial transcripts.

#### Scenario: A valid hold is released

* WHEN recording duration is at least 300 ms and the user releases the hold control
* THEN the WAV SHALL be finalized before inference
* AND one STT operation SHALL run against the complete WAV
* AND the worker SHALL load the configured local model directory with local-only behavior

#### Scenario: The hold is too short

* WHEN finalized duration is below 300 ms
* THEN transcription SHALL not start
* AND the result SHALL be `RECORDING_TOO_SHORT`
* AND the temporary WAV SHALL be deleted

#### Scenario: Maximum duration is reached

* WHEN recording reaches the configured/hard 120-second maximum
* THEN capture SHALL stop automatically
* AND the complete recording SHALL proceed to transcription
* AND the result/status SHALL include a non-fatal `RECORDING_LIMIT_REACHED` warning

#### Scenario: No speech is detected

* WHEN faster-whisper produces an empty final transcript without infrastructure failure
* THEN the outcome SHALL be `NO_SPEECH_DETECTED`
* AND no draft mutation SHALL be requested

### Requirement: Local TTS SHALL synthesize and play through sherpa-onnx

The system SHALL synthesize bounded non-empty text with the configured local sherpa-onnx model and play the generated WAV through a native output device. Only one local-media playback SHALL be active, and the output file SHALL remain operation-owned and ephemeral.

#### Scenario: TTS succeeds

* WHEN valid text of at most 4,000 Unicode code points is submitted to a ready engine
* THEN the worker SHALL generate audio only at the pre-authorized operation output path
* AND the native host SHALL validate and play that WAV locally
* AND the operation SHALL remain in `playing` until completion or stop

#### Scenario: TTS text is too long

* WHEN input exceeds 4,000 Unicode code points
* THEN the request SHALL fail with `TTS_TEXT_TOO_LONG`
* AND the text SHALL NOT be silently truncated

#### Scenario: Playback is stopped

* WHEN the caller stops active generation or playback
* THEN generation/playback SHALL stop as soon as the worker/device permits
* AND the operation SHALL become cancelled/stopped according to operation semantics
* AND the generated WAV SHALL be deleted

#### Scenario: Worker returns an unauthorized path

* WHEN a TTS worker response references a path other than the pre-authorized operation output
* THEN the worker SHALL be treated as protocol-invalid
* AND the file SHALL NOT be played

### Requirement: Cancellation SHALL be bounded and isolated

Recording, OCR, STT, TTS, and playback SHALL support cancellation. If a Python native inference call does not stop cooperatively within the configured grace period, the supervisor SHALL terminate only that engine worker and SHALL allow a later lazy restart.

#### Scenario: STT cancellation is non-cooperative

* WHEN the faster-whisper worker does not acknowledge cancellation within the grace period
* THEN only the faster-whisper worker SHALL be terminated
* AND the STT operation SHALL become cancelled
* AND PaddleOCR and sherpa-onnx workers SHALL remain unaffected

#### Scenario: Cancellation races with success

* WHEN cancellation is accepted before a result is committed
* THEN no result SHALL be applied to a composer
* AND operation-owned temporary media SHALL be cleaned

### Requirement: Ephemeral media SHALL be cleaned on every terminal path

Staged inputs, recordings, admitted OCR files, and generated speech SHALL use opaque names under a canonical application-owned local-media root. They SHALL be deleted on success, failure, cancellation, and shutdown, with a bounded startup sweep for stale entries older than 24 hours.

#### Scenario: Inference fails

* WHEN OCR, STT, or TTS fails after creating operation media
* THEN all files owned by that operation SHALL be scheduled for immediate cleanup

#### Scenario: The application previously crashed

* WHEN the application starts and finds local-media entries older than 24 hours
* THEN a bounded stale sweep SHALL remove entries that remain inside the canonical root
* AND it SHALL reject symlink/reparse-point escape

### Requirement: Sensitive local-media content SHALL not enter logs or telemetry

The system SHALL exclude raw media, OCR text, transcripts, synthesis text, full local paths, complete protocol frames, and raw Python tracebacks from logs, operation labels, telemetry, crash reports, and exported diagnostics.

#### Scenario: A worker raises an exception containing a path and transcript

* WHEN stderr contains sensitive exception text
* THEN the host SHALL map it to a stable error code and redacted diagnostic correlation ID
* AND the raw content SHALL NOT be forwarded to the frontend or normal logs

#### Scenario: An operation is observed

* WHEN operation spans/events are emitted
* THEN they MAY include operation ID, engine, safe version/device, phase, duration, counts, queue depth, restart count, and stable error code
* AND they SHALL NOT include user content or full paths

### Requirement: Web/mock behavior SHALL remain truthful

Production Web mode SHALL report local-media inference as native-only/unavailable. It SHALL not claim to record, OCR, transcribe, synthesize, or play locally. Tests MAY inject a deterministic fake through the standard service boundary.

#### Scenario: Web composer renders

* WHEN the application runs without the Tauri native host
* THEN local-media controls SHALL retain compatible layout and accessible explanation
* AND native actions SHALL be disabled or return `LOCAL_MEDIA_NATIVE_ONLY`

#### Scenario: A UI test needs success states

* WHEN a test injects the documented fake `LocalMediaService`
* THEN the test MAY drive deterministic results
* AND production Web service behavior SHALL remain unchanged

### Requirement: PaddleOCR CPU acceleration SHALL be explicitly controllable

The OCR profile SHALL carry a `cpuAcceleration` field with exactly three values: `library-default`,
`enabled`, and `disabled`. The default SHALL be `library-default`, which passes no acceleration
argument and leaves the decision to PaddleOCR. The worker SHALL map `disabled` to
`enable_mkldnn=False` and `enabled` to `enable_mkldnn=True`, and SHALL apply the mapping to every
pipeline stage the request passes through, including both text detection and text recognition. The
system SHALL NOT set a process-wide acceleration flag, because PaddleX configures its runners
independently of the global flag and a process-wide setting would silently fail to take effect.

#### Scenario: The default is unchanged behaviour

* WHEN an OCR profile does not name a CPU acceleration mode
* THEN the mode SHALL be `library-default`
* AND the worker SHALL pass no acceleration argument to PaddleOCR

#### Scenario: Acceleration is disabled

* WHEN an OCR profile sets `cpuAcceleration` to `disabled`
* THEN every pipeline stage of the resulting inference SHALL receive `enable_mkldnn=False`
* AND a model that fails under the library default SHALL be recognized successfully

#### Scenario: The mode belongs to the operation snapshot

* WHEN an operation is accepted and the saved profile's acceleration mode changes before it finishes
* THEN the running operation SHALL continue with the mode captured in its snapshot
* AND the changed mode SHALL apply only to operations accepted afterwards

### Requirement: Engine readiness SHALL be established by real inference, not by loading

An engine probe SHALL execute a minimal real inference and SHALL NOT report `Ready` on the strength
of an import or a model load alone. A canary that loads successfully and then fails to infer SHALL
report `Unavailable` with the classifying error, because the failure this distinguishes -- a runtime
that accepts a model and cannot execute its graph -- is invisible to a load-only probe and surfaces
later as a failed user operation.

#### Scenario: A model loads but cannot execute

* WHEN an engine constructs successfully and its minimal canary inference raises
* THEN readiness SHALL be `Unavailable`
* AND the reported error SHALL be the classified stable code for that failure, not a generic one

#### Scenario: A canary succeeds

* WHEN the canary inference completes
* THEN readiness SHALL be `Ready`
* AND no canary input or output SHALL be retained after the probe returns

### Requirement: Model paths SHALL be classified per field and verified when they leave ASCII

For every model-related profile field the system SHALL record whether the configured path contains
spaces and whether it contains non-ASCII characters. A path that contains non-ASCII characters SHALL
be verified by a real canary inference before its engine is reported `Ready`, because a path outside
the active code page can be resolved and stated by the host and still be unopenable by the engine's
native code. The system SHALL NOT reject non-ASCII paths categorically: they are supported wherever
the underlying runtime supports them.

#### Scenario: A non-ASCII path an engine supports

* WHEN a faster-whisper model directory contains non-ASCII characters and the canary transcribes
* THEN readiness SHALL be `Ready`
* AND no warning SHALL be raised about the path

#### Scenario: A non-ASCII path an engine cannot open

* WHEN a PaddleOCR or sherpa-onnx model path contains non-ASCII characters and the canary fails to
  open it
* THEN readiness SHALL be `Unavailable`
* AND the error SHALL name the single profile field whose path could not be opened

### Requirement: Third-party incompatibilities SHALL surface as stable, actionable errors

The system SHALL classify known third-party incompatibilities into stable error codes rather than a
generic engine failure. The codes SHALL include `PADDLE_ONEDNN_MODEL_INCOMPATIBLE`,
`MODEL_PATH_ENCODING_UNSUPPORTED`, `TTS_DATA_PATH_ENCODING_UNSUPPORTED`, and
`TTS_PHONEMIZER_DATA_UNAVAILABLE`. Each error SHALL carry exactly `engine`, `field`,
`containsSpaces`, `containsNonAscii`, `packageVersion`, the stable code, and a remediation
identifier. An error SHALL NOT carry a full path, a raw exception message, or a traceback.

#### Scenario: An acceleration incompatibility is classified

* WHEN PaddleOCR fails with an unimplemented-operator error from its acceleration backend
* THEN the operation SHALL fail with `PADDLE_ONEDNN_MODEL_INCOMPATIBLE`
* AND the payload SHALL name the engine and the package version and SHALL NOT contain a path

#### Scenario: A path encoding failure is attributed to one field

* WHEN sherpa-onnx cannot open its phonemizer data directory
* THEN the operation SHALL fail with `TTS_DATA_PATH_ENCODING_UNSUPPORTED`
* AND `field` SHALL identify the data directory rather than the model or the tokens file

### Requirement: Acceleration remediation SHALL require explicit user confirmation

When an operation fails with `PADDLE_ONEDNN_MODEL_INCOMPATIBLE`, the system SHALL offer to disable
CPU acceleration and re-probe, and SHALL apply that change only after the user confirms it. The
system SHALL NOT retry automatically, SHALL NOT degrade automatically, and SHALL NOT modify the
saved profile without confirmation.

#### Scenario: The user declines

* WHEN the incompatibility is reported and the user does not confirm
* THEN the saved profile SHALL be unchanged
* AND the engine SHALL remain `Unavailable`

#### Scenario: The user confirms

* WHEN the user confirms the remediation
* THEN `cpuAcceleration` SHALL be saved as `disabled` through the ordinary optimistic-concurrency
  save path
* AND a new probe SHALL run against the saved profile

### Requirement: The system SHALL NOT fall back between execution providers silently

The system SHALL NOT retry a failed inference under a different execution provider, device, or
acceleration mode without an explicit saved profile change. A result SHALL always be attributable to
the mode recorded in its operation snapshot.

#### Scenario: An acceleration failure is not retried

* WHEN inference fails with an acceleration-backend incompatibility
* THEN the operation SHALL fail
* AND no second attempt SHALL be made under a different acceleration mode

### Requirement: The system SHALL NOT relocate, copy, or acquire models on the user's behalf

When a model path is incompatible with its engine, the system SHALL state which field is affected and
SHALL direct the user to move the files themselves. The system SHALL NOT copy, move, hard-link,
junction, short-path, rename, or download any model or data directory.

#### Scenario: An unopenable model path is reported

* WHEN a model path cannot be opened by its engine because of its encoding
* THEN the remediation SHALL be to relocate the files to a path the engine can open and reselect them
* AND no file SHALL be created, copied, or moved by the application

### Requirement: Real-engine qualification SHALL cover spaces and non-ASCII paths per engine

The opt-in real-engine qualification SHALL record, per engine, the outcome for an ASCII path, a path
containing spaces, and a path containing non-ASCII characters, together with the missing-model
offline case. The record SHALL state the outcome per engine rather than in aggregate, because the
three engines do not share a path-handling implementation and an aggregate result would hide which
of them fails.

#### Scenario: The matrix is recorded

* WHEN the real-engine qualification runs
* THEN each engine SHALL have a recorded outcome for ASCII, spaces, and non-ASCII paths
* AND each inference SHALL record the number of denied network attempts
* AND no recognized text, transcript, synthesis input, or full path SHALL appear in the record
