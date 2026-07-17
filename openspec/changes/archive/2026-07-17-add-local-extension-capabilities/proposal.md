## Why

VaneHub AI currently manages coding agents, CLIs, SDKs, MCP servers, and Skills, but it has no service boundary or settings surface for optional local inference capabilities. Adding a managed local-extension runtime now gives users a safe, consistent way to install and operate OCR, speech recognition, and speech synthesis while establishing a reusable foundation for future local capabilities.

## What Changes

- Add an Extension Capabilities settings page for OCR, ASR, and TTS with localized search, compatibility information, install previews, lifecycle controls, functional tests, and operation logs.
- Add a dedicated frontend `ExtensionService` contract with Tauri and Web/mock adapters; React components remain runtime-agnostic and never invoke Tauri commands directly.
- Add a Tauri-managed extension registry, Windows x64 environment detection, SQLite-backed configuration, asynchronous operations, allowlisted command execution, process ownership, loopback health checks, and unified logging.
- Provide built-in framework definitions for PaddleOCR, faster-whisper, and sherpa-onnx, with exactly one active framework per capability in the first version.
- Store managed framework environments below the VaneHub application data directory, show package/model download and storage estimates before installation, and keep the first-version management sidecar loopback-only. Model downloads and capability-specific inference endpoints remain follow-up work.
- Preserve the existing `futuristic` and `minimal` visual styles and maintain zh-CN/en translation parity.
- Keep the Web runtime usable through deterministic mock extension data while clearly identifying native-only operations.
- Record first-version progress, known limitations, and a versioned roadmap for multi-platform, multi-framework, third-party manifest, and chat-consumer integration work in the design artifact.

## Capabilities

### New Capabilities

- `local-extension-management`: Defines built-in local capability discovery, compatibility, installation, configuration, lifecycle, health, functional testing, persistence, and runtime-adapter behavior.

### Modified Capabilities

- `settings-center-ui`: Adds the Extension Capabilities navigation entry and service-backed page with dual-theme and localized behavior.
- `unified-log-management`: Extends operation-output and persisted native logging requirements to local-extension install and lifecycle operations.

## Impact

- Frontend: new extension types, service contract, Tauri/Web adapters, settings page modules, i18n resources, and tests under `src/`.
- Native runtime: new Rust extension domain, Tauri commands, SQLite storage, operation-task integration, process and health management, and unified log events under `src-tauri/`.
- Runtime behavior: desktop gains full local management; Web/mock remains non-destructive and previewable through the same interface.
- Dependencies and storage: after explicit user confirmation, the desktop uses a compatible discovered system Python to create an application-owned virtual environment and download allowlisted framework packages. Portable Python and model-asset download flows are deferred. No alternative frontend state library or database is introduced.
