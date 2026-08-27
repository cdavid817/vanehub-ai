## ADDED Requirements

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
