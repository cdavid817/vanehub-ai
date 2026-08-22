
> Execute in order unless a task explicitly says it may run in parallel. Keep each commit/buildable slice small. Do not enable composer controls until the native service, cancellation, and fake-driven UI tests exist.

## 0. Baseline and change validation

- [x] 0.1 Run `git status --short` and preserve unrelated user changes.
- [x] 0.2 Read `AGENTS.md`, `openspec/project.md`, and every capability delta in this change before editing code.
- [x] 0.3 Run `openspec validate add-local-composer-media-tools --strict` and fix proposal/spec formatting before implementation.
- [x] 0.4 Capture baseline results for `npm run lint:ci`, `npm run test`, `npm run build`, `npm run architecture:check`, `cargo check --workspace`, and relevant existing desktop tests.
- [x] 0.5 Search the repository for all existing PaddleOCR/OnePiece OCR process, model, command, and configuration implementations; record whether they are implemented or specification-only.
- [x] 0.6 Identify the exact frontend service factory/provider, command registry, operation/task API, SQLite migration mechanism, locale catalogs, Tauri resource resolver, and desktop test harness. Reuse them; do not introduce parallel registries.

## 1. Register the bounded context and contracts

- [x] 1.1 Add `local_media` to the canonical bounded-context map in `openspec/project.md` with the ownership statement from `design.md`.
- [x] 1.2 Add `src-tauri/src/contexts/local_media/mod.rs` and the required `domain`, `application`, and `infrastructure` modules.
- [x] 1.3 Add/update architecture dependency tests so the filesystem context set exactly matches the documented map.
- [x] 1.4 Define domain IDs/enums for engines, operations, recordings, playback, staged inputs, readiness, worker state, and stable error codes.
- [x] 1.5 Define `LocalMediaProfile`, engine sub-profiles, validation, disabled defaults, profile revision, and immutable operation snapshot.
- [x] 1.6 Define typed OCR, transcription, TTS/playback, probe, and runtime-status results.
- [x] 1.7 Define the published `local_media::api` trait/facade; do not expose infrastructure/process/temp-path types.
- [x] 1.8 Add domain unit tests for profile validation, model-kind-specific TTS fields, path shape, numeric bounds, stable errors, and snapshot immutability.

## 2. Persist the local-media profile

- [x] 2.1 Add the next repository-compatible SQLite migration for `local_media_profiles`.
- [x] 2.2 Implement a context-owned profile repository with disabled first-read defaults.
- [x] 2.3 Implement atomic optimistic-concurrency save using `expectedRevision` and `PROFILE_REVISION_CONFLICT`.
- [x] 2.4 Ensure no secret classification is introduced and full paths are excluded from repository logs.
- [x] 2.5 Add migration/repository tests for creation, round-trip, stale revision, malformed stored JSON handling, and backward-compatible defaults.

## 3. Add the frontend service boundary

- [x] 3.1 Create `src/services/local-media-service.ts` with typed profile, status, device, recording, operation, result, and error DTOs.
- [x] 3.2 Register the service through the existing service provider/factory pattern.
- [x] 3.3 Implement a Tauri adapter with no component-level `invoke` leakage.
- [x] 3.4 Implement a production Web/mock adapter that reports native-only/unavailable state truthfully.
- [ ] 3.5 Add a deterministic injected fake service for unit/E2E tests without changing production Web semantics.
- [x] 3.6 Add adapter contract tests covering DTO serialization, discriminated results, cancellation, and stable error mapping.

## 4. Add thin Tauri commands and operation integration

- [x] 4.1 Register thin commands for profile get/save, status, device list, probe, OCR staging/start, recording start/finish/cancel, TTS start/stop, and typed result lookup.
- [x] 4.2 Keep commands limited to DTO mapping and application-service calls; add an architecture test or code review assertion preventing process/file/audio logic in command modules.
- [x] 4.3 Register operation kinds `local-media.probe`, `local-media.ocr`, `local-media.stt`, and `local-media.tts` in the existing operation/task mechanism.
- [x] 4.4 Implement immediate stable operation-ID acceptance and phases defined in `design.md`.
- [x] 4.5 Implement typed local-media result storage/read while retaining generic status/cancel through the existing operation service.
- [x] 4.6 Implement result expiry semantics and `OPERATION_RESULT_EXPIRED`.
- [x] 4.7 Add operation integration tests for accepted/running/succeeded/failed/cancelled states and idempotent result reads.

## 5. Implement temporary-media storage and OCR admission

- [x] 5.1 Add the dedicated application temp/cache root and restrictive file/directory creation.
- [x] 5.2 Implement opaque staged-input, recording, operation, and playback ownership IDs.
- [x] 5.3 Implement canonical-root checks and reject symlink/reparse-point escape, non-regular files, special devices, and unsupported inputs.
- [x] 5.4 Implement bounded content sniffing for supported image/PDF inputs; do not trust extension alone.
- [x] 5.5 Enforce byte, PDF-page, decoded-pixel, output, and timeout limits from controller/profile policy.
- [x] 5.6 Copy selected composer OCR files into an opaque staging directory and return only `StagedOcrSource` metadata.
- [x] 5.7 Implement atomic one-time staging claim/ownership transfer to OCR operations and short expiry for unclaimed inputs.
- [x] 5.8 Implement artifact admission for OnePiece without accepting arbitrary host paths in its tool contract.
- [x] 5.9 Implement cleanup guards on success/failure/cancel plus a bounded startup sweep for entries older than 24 hours.
- [x] 5.10 Add adversarial tests for oversized files, malformed PDFs/images, page/pixel limits, path escape, race/reuse, expiry, cleanup failure, and stale sweep.

## 6. Implement the worker bridge and supervisor

- [x] 6.1 Add the bundled `src-tauri/resources/local-media-worker` Python package/entry point.
- [x] 6.2 Implement protocol v1 hello/request/response/error/cancel/shutdown frames with line and frame-size limits.
- [x] 6.3 Keep stdout protocol-only and route redacted diagnostics to stderr.
- [x] 6.4 Implement Rust protocol parsing with strict version, engine, method, request-ID, frame-size, and path validation.
- [x] 6.5 Implement process launch without shell interpolation using the configured Python executable and Tauri resource resolver.
- [x] 6.6 Sanitize the worker environment, remove proxy variables by default, and set offline flags.
- [x] 6.7 Implement one independent worker slot and bounded queue for each engine.
- [x] 6.8 Implement lazy start, handshake timeout, one active request, cancellation grace, forced termination, restart/backoff, stale-profile restart, idle eviction, and app-shutdown cleanup.
- [x] 6.9 Implement probe use cases with safe package/version/device/model metadata.
- [x] 6.10 Add fake-worker Rust tests for success, malformed/oversized frames, stdout contamination, wrong IDs, crash, hang, cancellation, restart, queue bound, backoff, and redaction.
- [x] 6.11 Add Python tests using fake engine modules for protocol behavior and stable error mapping.
- [x] 6.12 Add a denied-socket/offline test and assert no worker feature silently downloads or contacts a provider.

## 7. Implement shared PaddleOCR

- [x] 7.1 Implement the PaddleOCR bridge adapter using explicit local PaddleX config or explicit local detection/recognition model paths.
- [x] 7.2 Disable optional orientation/unwarping/text-line models unless all required local paths are explicitly configured.
- [x] 7.3 Map package/import/version/model/device failures to stable local-media error codes.
- [x] 7.4 Implement bounded image/PDF OCR and structured page/line/provenance results.
- [x] 7.5 Derive deterministic plain text by preserving page and reading order and normalizing only line endings/outer whitespace.
- [x] 7.6 Treat no recognized text as `NO_TEXT_DETECTED`, not a worker crash.
- [x] 7.7 Implement composer OCR operation from a claimed staged input.
- [x] 7.8 Migrate or implement OnePiece OCR through `local_media::api`; remove/deprecate duplicate PaddleOCR worker/model/process code.
- [x] 7.9 Preserve OnePiece artifact-only input, structured result, provenance, privacy, and Web/mock behavior.
- [x] 7.10 Add fake-engine tests and an opt-in real local PaddleOCR smoke-test script that never downloads models.

## 8. Implement native microphone capture

- [x] 8.1 Add workspace/Rust dependencies for `cpal` and `hound` using repository dependency conventions.
- [x] 8.2 Implement microphone device enumeration with stable opaque device IDs and safe labels.
- [x] 8.3 Implement the singleton recording coordinator and one-active-recording policy.
- [x] 8.4 Open the configured/default input device and map OS permission/device failures to stable errors.
- [x] 8.5 Convert supported sample formats, downmix to mono, and transfer frames through a bounded channel.
- [x] 8.6 Implement a dedicated 16-bit PCM WAV writer; keep file I/O and allocation out of the real-time audio callback.
- [x] 8.7 Fail explicitly on channel overrun instead of silently dropping an unbounded sample range.
- [x] 8.8 Implement `300 ms` minimum, `120 s` hard maximum, automatic max-duration stop-and-transcribe, and safe finalization.
- [x] 8.9 Implement cancellation on explicit command, application shutdown, and owner disposal, with immediate temp cleanup.
- [x] 8.10 Add deterministic synthetic-audio tests for format conversion, downmix, duration, max auto-stop, overrun, writer failure, concurrent start, and cleanup.

## 9. Implement faster-whisper whole-utterance STT

- [x] 9.1 Implement faster-whisper probe/import/model validation with explicit local model directory.
- [x] 9.2 Construct `WhisperModel` with local-only behavior and configured `device`, `computeType`, `language`, `vadFilter`, and `beamSize`.
- [x] 9.3 Implement the complete-WAV `transcribe` method and exhaust segment generation before returning success.
- [x] 9.4 Return final text plus bounded detected-language/duration metadata; do not return/log word-level content in V1.
- [x] 9.5 Normalize only line endings and outer whitespace.
- [x] 9.6 Map empty speech to `NO_SPEECH_DETECTED` with an unchanged draft.
- [x] 9.7 Wire recording finalization to a stable STT operation and delete the WAV in all terminal paths.
- [x] 9.8 Add fake-engine tests and an opt-in real local faster-whisper smoke test that uses a checked-in/generated tiny audio fixture and performs no download.

## 10. Implement sherpa-onnx TTS and native playback

- [x] 10.1 Add the workspace/Rust dependency for `rodio` using repository conventions.
- [x] 10.2 Implement model-kind-specific sherpa-onnx profile validation for model, tokens, lexicon/data/dictionary/rule-FST paths.
- [x] 10.3 Implement sherpa-onnx probe and local offline TTS construction.
- [x] 10.4 Enforce non-empty input and the `4,000` Unicode-code-point limit without truncation.
- [x] 10.5 Generate only to a pre-authorized operation-owned WAV path and validate the worker's returned path/metadata.
- [x] 10.6 Implement the singleton native playback coordinator, output-device selection, completion detection, immediate stop, and one-active-playback policy.
- [x] 10.7 Keep the operation in `playing` until completion/stop/failure and expose only an opaque `playbackId` plus safe metadata.
- [x] 10.8 Delete generated speech in every terminal path and do not cache sensitive synthesis output.
- [x] 10.9 Add fake-engine/playback tests and an opt-in real local sherpa-onnx smoke test with no model download.

## 11. Implement the Local media settings page

- [x] 11.1 Add `local-media` to the capabilities group in `src/settings/settings-pages.ts` using the current tested order convention.
- [x] 11.2 Add the lazy page loader in `src/settings/settings-page-loaders.ts` and page component(s) under the existing settings structure.
- [x] 11.3 Add the master enable and three independent engine cards with explicit enabled/status/check controls.
- [x] 11.4 Add typed OCR fields for Python, PaddleX config or explicit local model directories, optional orientation model, language, device, and PDF page limit.
- [x] 11.5 Add typed STT fields for Python, local model directory, microphone, language, device, compute type, VAD, beam size, and recording limit.
- [x] 11.6 Add typed TTS fields for Python, model kind, model/tokens/auxiliary paths, speaker, speed, threads, and output device.
- [x] 11.7 Reuse current native path-picker/input patterns; do not add package/model install or download controls.
- [x] 11.8 Implement client validation, native authoritative validation, dirty/discard/save behavior, optimistic conflict handling, and saved-profile-only probes.
- [x] 11.9 Show independent readiness states and safe metadata; one failed engine must not disable the other ready engines.
- [x] 11.10 Add settings unit/component tests and update navigation-order tests.

## 12. Implement composer media orchestration

- [x] 12.1 Add pure `appendSpeechTranscript` and `appendOcrText` helpers with complete whitespace/Unicode tests.
- [x] 12.2 Add a per-composer `composerScopeId` and latest-draft getter; do not retain the recording-start draft for result insertion.
- [x] 12.3 Implement `useLocalMediaComposer`/controller with service calls, operation observation, result reads, state transitions, cancellation, scope checks, dialogs, and focus restoration.
- [x] 12.4 Ensure asynchronous results are discarded from draft mutation after session switch/unmount/cancel.
- [x] 12.5 Ensure all successful draft changes use the existing setter path so slash/file-reference suggestions remain synchronized.
- [x] 12.6 Add `mediaActions` (or an equivalent narrow slot) to `ButtonArea`; do not put engine/service logic into it.
- [x] 12.7 Add `ComposerMediaActions`, `RecordingIndicator`, `OcrReviewDialog`, and overflow/recoverable result UI.
- [x] 12.8 Wire the media controller from `api-session-composer.tsx` into `ChatInputBox` without changing existing send/stop/IME contracts.

## 13. Implement OCR composer UX

- [x] 13.1 Add the compact OCR action with native-only/unready/busy tooltips and fixed geometry.
- [x] 13.2 Open the existing native file picker through the Tauri service adapter, stage the selected source, and start OCR.
- [x] 13.3 Show progress without blocking the UI.
- [x] 13.4 Show an editable review dialog with source metadata, local/provenance badge, warnings, text, and character count.
- [x] 13.5 Add explicit Append, Copy (when supported), and Cancel actions.
- [x] 13.6 Append with blank-line semantics only after confirmation; never auto-send.
- [x] 13.7 Handle empty/oversized output without silent truncation or draft loss.
- [x] 13.8 Add component/controller tests for select-cancel, staging failure, OCR success, no text, edit, append, copy, cancel, overflow, and session switch.

## 14. Implement hold-to-talk composer UX

- [x] 14.1 Implement pointerdown start, pointer capture, pointerup finish/transcribe, and synthetic-click suppression.
- [x] 14.2 Implement pointercancel, lost capture, window blur, Escape, and disposal cancellation.
- [x] 14.3 Implement Space/Enter keydown start, repeat suppression, keyup finish, and Escape cancellation.
- [x] 14.4 Show opening, recording with elapsed duration/local badge, finalizing, transcribing, failure, and idle states without layout shift.
- [x] 14.5 Respect reduced motion and avoid announcing elapsed time every second to screen readers.
- [x] 14.6 Append final transcript to the latest draft with separator semantics; leave draft unchanged for empty/cancelled/stale-scope results.
- [x] 14.7 Focus the textarea/caret end after append only when doing so does not steal focus from another substantive control.
- [x] 14.8 Verify every path does not call send automatically.
- [x] 14.9 Add pointer, keyboard, race, latest-draft, max-duration, error, cancellation, and no-auto-send tests.

## 15. Implement TTS composer UX

- [x] 15.1 Add the compact speaker action with disabled/native-only/unready states.
- [x] 15.2 Read the current textarea selection at activation; use the entire latest draft only when selection is empty.
- [x] 15.3 Show generating and playing states in fixed geometry.
- [x] 15.4 Clicking during generation/playback cancels or stops immediately.
- [x] 15.5 Do not start on draft change and do not read assistant messages.
- [x] 15.6 Add selection/draft, empty, limit, generation, playback, stop, failure, and session-disposal tests.

## 16. Localization and accessibility

- [x] 16.1 Add all page, field, status, tooltip, dialog, error, privacy, permission, and operation strings to `zh-CN`, `en`, `zh-TW`, `ja`, and `ko`.
- [x] 16.2 Use semantic translation keys; do not hard-code user-visible strings in components or native DTOs.
- [x] 16.3 Add locale parity/missing-key checks using the repository's current localization tests.
- [x] 16.4 Add icon-button labels, `aria-pressed`, one polite live region, focus return, keyboard parity, and reduced-motion behavior.
- [x] 16.5 Run accessibility/component tests for all three actions and the settings page.

## 17. Privacy, logging, and security hardening

- [x] 17.1 Add explicit redaction/allowlist helpers for local-media logs and worker stderr.
- [x] 17.2 Verify logs exclude audio bytes, OCR/STT/TTS text, full paths, protocol frames, and raw tracebacks.
- [x] 17.3 Use process argument APIs only; reject shell command construction.
- [x] 17.4 Validate every worker input/output path is canonical and within an authorized root/model field.
- [x] 17.5 Remove proxy variables and set offline flags for workers; test denied networking.
- [x] 17.6 Ensure no telemetry/crash-report payload attaches temp media or recognized/generated text.
- [x] 17.7 Add automated log scanning/redaction tests for representative failures.
- [x] 17.8 Document the local-only guarantee precisely as feature code-path behavior, not a universal OS sandbox claim.

## 18. Packaging and platform integration

- [x] 18.1 Add `cpal`, `hound`, and `rodio` with reviewed feature flags and lockfile updates.
- [x] 18.2 Add the worker bridge to Tauri packaged resources and test packaged resource resolution.
- [x] 18.3 Add macOS `NSMicrophoneUsageDescription` and verify it is present in packaged metadata.
- [ ] 18.4 Verify Windows path quoting/non-ASCII paths, microphone privacy errors, default/selected devices, and worker launch without a shell.
- [ ] 18.5 Verify Linux target audio runtime dependencies and classify missing ALSA/PulseAudio/PipeWire devices/services accurately.
- [x] 18.6 Update release/runtime documentation with local Python/package/model prerequisites and no-download behavior.
- [x] 18.7 Verify no Python interpreter, inference package, CUDA runtime, or model was accidentally added to the bundle.

## 19. Automated and manual verification

- [x] 19.1 Run all TypeScript/unit/component tests added by this change.
- [x] 19.2 Run Rust domain/application/infrastructure tests with fake capture, playback, and workers.
- [x] 19.3 Run Python bridge tests with fake modules and denied sockets.
- [x] 19.4 Run Web Playwright tests for truthful native-only behavior and fake-service UI state coverage.
- [ ] 19.5 Run desktop E2E with deterministic audio/OCR/TTS fixtures and worker failure injection.
- [ ] 19.6 Run opt-in real local smoke tests for each configured engine on at least one supported developer machine; record package/model/device versions without recording content or paths.
- [ ] 19.7 Manually verify real press/hold/release microphone behavior, permission denial/recovery, OCR review, TTS playback/stop, and session-switch races on available desktop platforms.
- [x] 19.8 Record Windows/macOS/Linux results as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN` with concrete evidence/reason.

## 20. Final repository gates

- [x] 20.1 Run `npm run lint:ci`.
- [ ] 20.2 Run `npm run test`.
- [x] 20.3 Run `npm run build`.
- [x] 20.4 Run `npm run architecture:check`.
- [x] 20.5 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] 20.6 Run `cargo check --workspace`.
- [x] 20.7 Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 20.8 Run `npm run native:panic:check`.
- [ ] 20.9 Run `cargo test --workspace`.
- [x] 20.10 Run `openspec validate add-local-composer-media-tools --strict`.
- [x] 20.11 Run `openspec validate --specs --strict`.
- [ ] 20.12 Run the repository's current Playwright and native desktop commands.
- [x] 20.13 Review `git diff --check`, generated artifacts, lockfile, bundle resource list, and architecture/context-map synchronization.
- [x] 20.14 Confirm there is exactly one PaddleOCR runtime owner and no component-level Tauri invoke.
- [x] 20.15 Confirm every requested success criterion and scenario has direct automated or explicitly recorded manual evidence.
