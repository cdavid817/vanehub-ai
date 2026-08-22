
## Why

VaneHub AI's primary chat composer currently supports text, file references, model/runtime selectors, prompt enhancement, send, and stop actions, but it has no first-class local media input or playback controls. Users who work from screenshots, scanned documents, or spoken notes must leave the composer, use an external application, and paste the result back manually.

The repository already specifies a OnePiece OCR tool backed by a managed local PaddleOCR runtime. Implementing a second PaddleOCR integration directly inside the composer would duplicate model configuration, process supervision, resource use, privacy controls, and error handling. The application instead needs one reusable local-media bounded context that owns OCR, whole-utterance speech-to-text, text-to-speech, native recording/playback, and ephemeral media lifecycle.

The requested interaction is deliberately local-first:

- The user presses and holds a microphone control.
- VaneHub records the utterance on the device.
- Releasing the control finalizes the complete recording and sends only a local temporary WAV path to a locally installed `faster-whisper` worker.
- The final transcript is appended to the current draft without automatically sending it.
- Audio bytes, OCR inputs, transcripts, and synthesis text are never uploaded by this feature, and there is no hosted fallback or implicit model download.

## What Changes

### Product behavior

- Add three compact actions to the primary chat composer:
  - **OCR**: choose a local image or PDF, run local PaddleOCR, review/edit the result, then append it to the draft.
  - **Hold to talk**: start native microphone capture on press, show recording duration and state, then transcribe the entire utterance with local `faster-whisper` on release and append the transcript to the latest draft.
  - **Speak / stop**: synthesize the current text selection, or the entire draft when there is no selection, with local `sherpa-onnx`; play it locally and allow immediate stop.
- Keep send behavior explicit. None of the three features sends a chat message automatically.
- Preserve existing IME composition, slash-command completion, file-reference completion, send, and stop behavior.
- Add a **Local media** settings page under the capabilities group for engine enablement, Python executables, local model paths, devices, languages, voices/speakers, and readiness probes.
- Represent unavailable local capabilities truthfully in Web/mock mode without simulating a real microphone, OCR engine, or speech engine.

### Architecture

- Add a peer native bounded context named `local_media`.
- Add a frontend `LocalMediaService` boundary with Tauri and Web/mock adapters. React components do not call Tauri `invoke` directly.
- Keep microphone capture, WAV creation, playback, temporary files, worker supervision, cancellation, and task lifecycle in Rust.
- Run PaddleOCR, faster-whisper, and sherpa-onnx in three independent, lazily started, long-lived Python workers using a versioned JSON Lines protocol over stdin/stdout.
- Require explicit local Python/model configuration. Workers run in offline-only mode and must not trigger package or model downloads.
- Publish a narrow `local_media::api` so the OnePiece OCR tool and composer OCR share exactly one PaddleOCR runtime and admission pipeline.
- Reuse the existing operation/task infrastructure for long-running probes and inference. Return a stable operation ID immediately and expose typed local-media results separately from generic operation status.
- Add native audio dependencies for capture, PCM WAV encoding, and playback.
- Add packaging metadata and platform verification for microphone permission and local worker resources.

### Privacy and safety

- Audio samples never cross the JavaScript/Tauri IPC boundary.
- A selected OCR file is validated and copied into an operation-owned application temporary directory before a Python worker receives it.
- The local feature has no network provider, hosted fallback, telemetry payload containing user media, or automatic package/model installation.
- Logs exclude audio bytes, OCR text, transcripts, synthesis source text, full local paths, Python protocol frames, and raw tracebacks.
- Temporary recordings, admitted OCR sources, and generated speech are deleted on success, failure, or cancellation; stale files are swept on startup.
- Cancellation is bounded. A non-cooperative Python inference call causes only its engine worker to be terminated and lazily restarted.

## Capabilities

### New capability

- `local-media-runtime`
  - Local engine profile and readiness model.
  - Native microphone capture and local playback.
  - PaddleOCR, faster-whisper, and sherpa-onnx worker supervision.
  - Input admission, asynchronous operations, typed results, cancellation, cleanup, privacy, and Web/mock parity.

### Modified capabilities

- `chat-experience`
  - Adds OCR, hold-to-talk, and speech playback controls and their draft-insertion behavior.
- `app-settings`
  - Adds the Local media settings page and readiness workflow.
- `onepiece-ocr-tool`
  - Reuses the shared PaddleOCR runtime while retaining artifact-only tool inputs and structured provenance.
- `local-extension-management`
  - Drops the managed PaddleOCR inference consumer, whose implementation this change deletes, and narrows the capability to dependency installation, version inventory, and a health-only management sidecar that owns no inference runtime, readiness, model lifecycle, or worker.
- `application-localization`
  - Adds all local-media strings to every registered locale.
- `native-runtime-architecture`
  - Registers `local_media` as a peer bounded context with a published API and operation integration.
- `native-app-packaging`
  - Packages the worker bridge and platform microphone metadata without bundling Python, inference packages, or models.
- `desktop-runtime-verification`
  - Adds deterministic desktop verification for capture, transcription, OCR, synthesis, playback, permissions, cancellation, and offline enforcement.

## Impact

### Expected code areas

- `src/components/chat/ChatInputBox.tsx`
- `src/components/chat/ButtonArea.tsx`
- `src/session-workspace/api-session-composer.tsx`
- New composer media components and controller/hooks under the existing chat/session-workspace boundaries
- New `src/services/local-media-service.ts`
- Tauri and Web/mock service adapters
- `src/settings/settings-pages.ts`
- `src/settings/settings-page-loaders.ts`
- New Local media settings page/components
- `src-tauri/src/contexts/local_media/**`
- Thin Tauri command mappings under `src-tauri/src/commands/**`
- Existing operations/task integration
- Existing OnePiece OCR adapter/controller integration
- `src-tauri/resources/local-media-worker/**`
- `src-tauri/Cargo.toml`, workspace dependency declarations, Tauri resource and platform permission configuration
- Locale catalogs and architecture/context map documentation
- Unit, integration, Web, and native desktop tests

### Dependencies

Rust dependencies are expected to include:

- `cpal` for native audio capture and device enumeration.
- `hound` for bounded PCM WAV writing.
- `rodio` for local audio playback.

The application does not install Python packages or models. The user configures local environments containing:

- PaddleOCR and its compatible Paddle/PaddleX runtime.
- faster-whisper and CTranslate2.
- sherpa-onnx with a compatible local TTS model bundle.

### Compatibility

- Existing chat input, API sessions, send/stop actions, and OnePiece artifact contracts remain compatible.
- The new service is unavailable in Web mode by design; controls remain visible but disabled with an explanation.
- Existing OnePiece OCR behavior must not regress. If an implementation already exists, it is migrated behind `local_media::api`; if it is still specification-only, only the shared implementation is created.
- No migration of existing general application settings is required. The new bounded context owns a dedicated versioned local-media profile record with disabled defaults.

## Non-goals

- Streaming partial transcription while the microphone is held.
- Voice activity auto-start, wake words, continuous dictation, or background listening.
- Cloud OCR, cloud STT, cloud TTS, or provider fallback.
- Downloading or installing Python, packages, models, CUDA runtimes, or voices from the UI.
- Screen-region capture, clipboard OCR, camera capture, or live video OCR.
- Automatic reading of assistant responses or global auto-play.
- Voice cloning, speaker enrollment, or biometric identification.
- Sending generated audio as a chat attachment.
- Exposing the engine workers as general-purpose arbitrary Python execution.

## Success Criteria

1. Holding the microphone records locally; release transcribes the complete utterance with the configured local faster-whisper model and appends the result to the active draft without sending it.
2. Audio bytes do not enter frontend state or Tauri command payloads, and no feature code path uploads media or silently falls back to a hosted service.
3. OCR accepts a bounded, user-selected image/PDF, runs the shared local PaddleOCR runtime, presents an editable review, and appends only after explicit confirmation.
4. TTS reads the current selection or draft using the configured local sherpa-onnx model, plays locally, and stops immediately when requested.
5. The three engines have explicit configuration, readiness, error, cancellation, and cleanup states in settings and composer UI.
6. OnePiece OCR and composer OCR use one worker supervisor and one PaddleOCR model configuration.
7. All new strings exist in `zh-CN`, `en`, `zh-TW`, `ja`, and `ko`; controls are keyboard-operable and screen-reader-labelled.
8. Existing architecture, lint, unit, build, Rust, OpenSpec, Web E2E, and desktop verification gates pass or have explicit platform evidence marked `BLOCKED`/`NOT RUN` with a reason.
