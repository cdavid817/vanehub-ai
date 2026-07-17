## Context

VaneHub already has a service-backed settings center, runtime adapter selection, asynchronous operation tasks, SQLite storage, command auditing, unified native logging, semantic theme tokens, and zh-CN/en parity checks. It does not have a domain model for optional local inference frameworks or a native owner for their installation directories and processes.

The reference implementation in `D:\cdavid\Documents\code\clowder-ai` demonstrates a useful service manifest, compatibility recommendation, isolated environment, lifecycle lock, process ownership, health probe, and operation-log pattern. VaneHub must adapt those ideas to its Rust/Tauri and SQLite architecture rather than copying Clowder's Node routes, `services.json`, shell-driven trust boundary, hard-coded Chinese labels, or framework-specific service ids.

The first release targets Windows x64 and three built-in capability/framework pairs: OCR with PaddleOCR, ASR with faster-whisper, and TTS with sherpa-onnx. It establishes the management plane and local self-test boundary. Chat image extraction, microphone capture, and spoken response playback remain separate consumer integrations.

### First-version delivery progress

This table is intentionally maintained with the implementation so later work can distinguish shipped foundations from planned extensions.

| Area | Current state |
|---|---|
| Requirements and architecture | Implemented in this change and covered by strict change validation |
| Settings entry, dual-theme page, and i18n | Implemented with semantic theme tokens, synchronized zh-CN/en resources, component coverage, and Playwright coverage |
| Frontend service contract and Tauri/Web adapters | Implemented; Web/mock stays deterministic and blocks native mutations |
| Built-in OCR/ASR/TTS framework catalog | Implemented for PaddleOCR, faster-whisper, and sherpa-onnx with stable capability/framework ids |
| SQLite-backed installation configuration | Implemented with schema migration, seeded rows, lifecycle state, enablement, port, path, version, health, error, and timestamps |
| Async install/uninstall/self-test tasks and unified logs | Implemented through the shared task registry, per-framework mutation locks, page-visible output, command audit, and redacted native logging |
| Managed sidecar start/stop and loopback health | Implemented as an owned loopback management sidecar with foreign-port protection; inference endpoints are deferred |
| Portable Python bootstrap | Deferred; v1 uses a compatible discovered Python interpreter to create an application-owned virtual environment |
| Model recommendation/download matrix | Basic metadata only in v1; hardware-aware recommendations are deferred |
| Real inference consumer integration | Deferred to follow-up changes |

### Verified first-version behavior

- The settings entry is registered after SDK Dependencies and renders the same information hierarchy in both registered themes without checking theme names.
- Both locales contain matching extension navigation, capability, framework, status, action, confirmation, error, and environment text.
- The Web/mock adapter exposes the stable three-framework catalog, an install preview, and explicit desktop-only mutation failures without changing the host.
- Native state is initialized and read through SQLite, while executable package plans and managed paths remain backend-owned.
- Compatibility checks require Windows x64 and Python 3.10 or newer before install is allowed.
- Each framework receives its own application-managed virtual environment, and the installed marker plus SQLite installed state are committed only after allowlisted package-version and framework-import verification succeeds.
- Lifecycle mutations use the shared operation task model, reject concurrent mutations for the same framework, and preserve card-visible operation output.
- Sidecars bind only to `127.0.0.1`, are stopped only when owned by the current runtime, and do not terminate a foreign process occupying the configured port.
- Native diagnostic events use the unified logger's redaction path; no extension-specific log file is introduced.

### Known first-version limitations

- A compatible system Python must already exist. V1 does not download or repair a Python runtime.
- Package plans use constrained major-version ranges rather than a fully locked wheel manifest with hashes. Reproducible, signed runtime bundles remain follow-up work.
- The managed sidecar currently proves process ownership, loopback health, and lifecycle behavior; it is not an OCR/ASR/TTS inference server and the first-version UI must not claim that an inference API is available.
- Self-test verifies framework importability rather than running a model-backed sample inference.
- Model size/source metadata is informational. Model selection, download progress, mirrors, checksums, shared cache accounting, and cleanup are not implemented.
- Installation progress is exposed through the operation result/log boundary, but subprocess output is not yet streamed line-by-line while pip is still running.
- Automated tests do not download multi-gigabyte packages or models, so wheel availability, antivirus interaction, proxy behavior, and device-specific acceleration still require manual validation.

### Windows x64 manual smoke test

1. Start the Tauri desktop build on a Windows x64 machine with Python 3.10 or newer on `PATH` and open **Settings > Extension Capabilities**.
2. Refresh environment detection and confirm the detected Python version and supported state. Repeat once with Python unavailable to verify the localized unsupported reason and disabled install action.
3. Open the requirements preview for each framework and verify the managed destination, package plan, disk estimate, proxy note, and self-test plan before approving any download.
4. Install one framework at a time on a disposable test profile. Confirm the operation remains visible, the managed directory is below the VaneHub application-data extension root, and the persisted state becomes installed.
5. Run self-test, enable the framework, start and stop its sidecar, and verify running/healthy transitions. Bind its configured port with a separate process and confirm VaneHub reports the conflict without terminating that process.
6. Uninstall and confirm only the exact framework-managed directory is removed and the SQLite state returns to not installed. Inspect unified logs for redaction and confirm no feature-local log file was created.

The smoke test intentionally stops before downloading a model or exercising real inference. Record OS build, CPU/GPU, Python version, framework/version, proxy mode, elapsed install time, and failure logs for later compatibility-matrix work.

### Verification record (2026-07-17)

- `npm run test`: 22 files and 67 tests passed.
- `npm run build`: TypeScript and Vite production build passed; the existing bundle-size advisory remains non-blocking.
- Extension Rust tests: 10 passed, covering catalog ids, compatibility, SQLite state, managed paths, lifecycle transitions, mutation locking, foreign ports, unified log redaction, and verification-failure state safety.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 84 library tests passed; main and doc-test targets also passed.
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml`: passed with two pre-existing non-blocking warnings in `lib.rs` (`type_complexity` and `too_many_arguments`); the new extension module emitted no Clippy warning.
- Extension Playwright suite: 2 tests passed for localized catalog/search/preview behavior and both registered themes.
- `openspec validate "add-local-extension-capabilities" --strict`: passed.
- `openspec validate --specs --strict`: 20 main specifications passed.
- `npm run lint` could not be run because this repository currently defines no `lint` script; TypeScript compilation, Vitest, Playwright, Cargo check/test, and Clippy provide the executable verification for this change.

## Goals / Non-Goals

**Goals:**

- Add a clear Extension Capabilities settings surface that behaves consistently in both registered visual themes and both locales.
- Keep all React code behind a dedicated `ExtensionService` and provide interface-compatible Tauri and Web/mock adapters.
- Represent capability identity separately from framework identity so framework replacements do not change product-facing capability ids.
- Install each framework into an application-owned isolated environment, expose lifecycle state and logs, and persist configuration through SQLite.
- Reuse the existing asynchronous operation registry and unified log service for non-blocking, auditable operations.
- Restrict executable plans to backend-owned built-in definitions and validate every user-selectable argument.
- Establish extension points for additional platforms, models, frameworks, and capability consumers without enabling arbitrary third-party code in v1.

**Non-Goals:**

- Importing or executing user-authored extension manifests or scripts.
- Installing multiple active frameworks for one capability.
- Guaranteeing macOS, Linux, Windows ARM64, CUDA, ROCm, or Apple MLX support in v1.
- Providing upgrade, rollback, shared model-cache garbage collection, or background auto-update flows.
- Wiring OCR into chat image attachments, ASR into microphone input, or TTS into message playback.
- Downloading a portable Python distribution when no compatible Python is installed.
- Running model downloads or multi-gigabyte inference tests in automated CI.

## Decisions

### 1. Use a dedicated extension domain and runtime service boundary

The frontend adds `ExtensionService`, `runtime-extension-client`, `tauri-extension-client`, and `web-extension-client`. The settings page imports only the runtime-selected service. Tauri `invoke()` remains confined to the Tauri adapter, and native storage/process work remains in a Rust `extensions` module.

This is preferred over adding methods to `AgentService` because local inference frameworks have installation and process lifecycles unrelated to agent selection. It is preferred over `SdkService` because SDK dependencies are application libraries, while extensions are optional capability runtimes with health and activation state.

### 2. Separate capability, framework, and installation state

Stable capability ids are `ocr`, `asr`, and `tts`. Built-in framework ids are `paddleocr`, `faster-whisper`, and `sherpa-onnx`. A framework definition declares localized metadata keys, supported capability, platform support, runtime requirements, package/model estimates, default port, install plan id, and self-test plan id.

SQLite stores only mutable state such as selected framework, installed state, enabled state, port, install path, version, last health result, and timestamps. Backend-owned definitions remain code data, preventing stale or tampered command plans from becoming executable database content.

Alternative considered: persist full manifests in SQLite. Rejected because it turns mutable local data into an executable supply-chain boundary and complicates definition upgrades.

### 3. Start with an allowlisted Windows x64 installation strategy

V1 detects a compatible Python interpreter, creates one virtual environment per framework below the VaneHub application data directory, and runs backend-owned module/pip argument arrays without shell interpolation. Package installation uses the configured network proxy and emits progress to both the operation task and unified log. PaddleOCR, faster-whisper, and sherpa-onnx receive distinct allowlisted package and verification plans.

The managed directory is resolved natively and never accepted from the frontend. Uninstall resolves and verifies the exact framework directory before removal. Model assets remain below the framework directory or a future application-owned cache.

Alternative considered: bundle Python into the application installer. Deferred because it increases artifact size, platform packaging work, and security-update ownership. A portable runtime downloader is the preferred follow-up for machines without Python.

### 4. Use explicit lifecycle states and existing operation tasks

UI state uses `not-installed`, `installing`, `installed`, `starting`, `running`, `stopping`, `uninstalling`, `error`, and `unsupported`. Long-running mutations return an `OperationTask` immediately and update the existing task registry. Only one mutation may run for a framework at a time.

V1 start launches a backend-owned loopback sidecar wrapper for the installed framework and records the owned child process. Health probes verify loopback reachability; self-test verifies that the selected framework can load its runtime. Full OCR/transcription/synthesis request contracts are deferred until consumer integrations are designed.

Alternative considered: run inference inside the Tauri process. Rejected because Python/native ML dependencies can crash or block the desktop runtime and make resource ownership unclear.

### 5. Treat install, enablement, and running as separate concepts

`installed` means the managed environment passed verification. `enabled` means the user selected the framework as the active provider for its capability. `running` means the owned sidecar currently passes health checks. Installation does not implicitly enable or start a framework; the user can inspect and test it first. Exactly one framework can be enabled per capability.

### 6. Keep Web/mock deterministic and non-destructive

The Web/mock adapter exposes the same three definitions and stable mock states. Native mutations return clear unsupported failures or deterministic mock operations as appropriate for UI testing, and the page visibly labels desktop-only actions. It never downloads packages or claims that a framework is installed on the host.

### 7. Build the settings page from shared visual and localization primitives

The navigation entry follows SDK Dependencies. The page uses existing `PageHeader`, `SectionPanel`, `StatCard`, `StatusPill`, buttons, semantic color tokens, and internal scrolling behavior. It contains no theme-name branches. All visible copy, statuses, compatibility notes, confirmations, and errors use synchronized `zh-CN` and `en` translation keys.

### 8. Preserve two log audiences through one logging system

Operation output remains visible in each framework card for immediate feedback. The same native lifecycle emits redacted `info`, `warn`, `error`, and `debug` events through the unified logger. URLs, proxy credentials, tokens, local usernames, and sensitive path components are redacted before persistence. No extension-specific log file is created.

## Risks / Trade-offs

- [Python and ML wheels vary by machine] → Detect interpreter/platform compatibility before offering install, use pinned allowlisted package plans, and show actionable unsupported reasons.
- [Package and model downloads are large or fail behind regional networks] → Show estimates and source information before confirmation, inherit VaneHub proxy settings, retain operation logs, and make install retryable/idempotent.
- [A port may belong to another process] → Bind loopback only, probe before start, never terminate an unowned listener, and report the conflicting port.
- [A stale process may survive application failure] → Persist configuration but reconstruct running state from health plus process ownership; add orphan reconciliation as a follow-up.
- [Framework import success is weaker than real inference] → Label v1 tests as runtime self-tests and add capability-specific sample inference with consumer contracts later.
- [One framework per capability limits advanced users] → Preserve capability/framework separation and collection-shaped contracts so multi-install selection can be added without renaming ids.
- [Built-in manifests slow third-party innovation] → Prefer safety in v1, then add a signed, versioned manifest schema and explicit trust UI after the lifecycle boundary is proven.
- [Web/mock behavior can be mistaken for native support] → Display a localized desktop-only banner and never persist mock install state beyond the preview runtime.

## Migration Plan

1. Add the new OpenSpec requirements without changing existing settings or SDK behavior.
2. Add frontend types and adapters, then expose the new settings entry using mock data.
3. Add the Rust extension module, SQLite table initialization, Tauri commands, task integration, and logger integration.
4. Enable native mutations only after compatibility checks and directory guards pass.
5. Run unit/component/E2E tests with fake command runners; perform real package installation only as a documented manual Windows smoke test.
6. Rollback by removing the settings entry and commands. Existing managed extension directories and SQLite rows remain inert and can be removed by a later cleanup tool rather than being destructively deleted during application downgrade.

## Follow-up Optimization Roadmap

1. Add a signed and checksummed portable Python bootstrap with runtime security updates.
2. Add hardware detection and versioned recommendation matrices for CPU, CUDA, ROCm, Apple MLX, memory, disk, and model size.
3. Add macOS, Linux, Windows ARM64, and per-platform framework alternatives.
4. Add capability-specific inference protocols, sample fixtures, deep health probes, streaming, cancellation, and resource limits.
5. Integrate OCR with image/file flows, ASR with microphone input, and TTS with chat playback as separate consumer changes.
6. Allow multiple installed frameworks per capability with one active provider, controlled migrations, upgrades, and rollback.
7. Add shared model-cache accounting, offline bundles, mirrors, resumable downloads, integrity verification, and selective cleanup.
8. Define a signed third-party manifest/package format with permission declarations, provenance, sandboxing, and revocation.
9. Add GPU scheduling, idle shutdown, orphan reconciliation, crash backoff, and application-start policies.
10. Add real-device compatibility telemetry that respects local privacy and can be disabled.

## Open Questions

- Which portable Python distribution and update channel should be adopted after the v1 system-Python bootstrap?
- Which stable HTTP inference schemas should be shared between desktop sidecars and a future Web/HTTP runtime?
- What signature authority and review process should govern third-party extension manifests?
