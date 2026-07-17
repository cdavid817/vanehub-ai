## ADDED Requirements

### Requirement: Built-in local capability catalog
The system SHALL expose stable `ocr`, `asr`, and `tts` capability ids with built-in PaddleOCR, faster-whisper, and sherpa-onnx framework definitions whose executable plans are owned by the native backend.

#### Scenario: List first-version capability definitions
- **WHEN** the Extension Capabilities page requests the catalog
- **THEN** the service SHALL return one built-in framework definition for each of OCR, ASR, and TTS with stable ids, localized metadata keys, requirements, platform compatibility, and install estimates

#### Scenario: Reject unknown framework mutation
- **WHEN** a client requests an operation for a framework id that is not in the native allowlist
- **THEN** the native service SHALL reject the request without executing a command or writing installation state

### Requirement: Extension frontend service boundary
The system SHALL expose extension queries and mutations through a dedicated frontend service interface with compatible Tauri and Web/mock adapters.

#### Scenario: Desktop extension request
- **WHEN** a React component requests extension state in the Tauri runtime
- **THEN** the runtime-selected extension service SHALL route the request through the Tauri adapter without the component importing or calling `invoke()`

#### Scenario: Web preview extension request
- **WHEN** the Extension Capabilities page runs in Web/mock mode
- **THEN** the Web adapter SHALL return deterministic catalog and status data and SHALL clearly report native mutation limitations without accessing the local filesystem or launching processes

### Requirement: Platform compatibility detection
The native service SHALL detect the current operating system, architecture, and compatible Python runtime separately from installation or launch.

#### Scenario: Supported Windows x64 host
- **WHEN** the host is Windows x64 and a compatible Python interpreter is available
- **THEN** the native service SHALL mark the first-version framework definitions installable and return the resolved prerequisite metadata

#### Scenario: Unsupported host
- **WHEN** the host platform or architecture is outside the verified first-version matrix
- **THEN** the service SHALL mark the affected framework unsupported with a user-displayable reason and SHALL NOT execute an install plan

### Requirement: Installation preview and explicit confirmation
The system SHALL show framework, package, model, storage, network, runtime, install-path, and offline-behavior information before starting a native installation.

#### Scenario: Preview installation
- **WHEN** a user selects install for a supported framework
- **THEN** the page SHALL present localized installation requirements and SHALL require explicit confirmation before invoking the install operation

#### Scenario: Cancel installation preview
- **WHEN** the user cancels the preview
- **THEN** the system SHALL leave framework state unchanged and SHALL NOT create an operation task

### Requirement: Asynchronous extension operations
Install, start, stop, uninstall, and self-test operations SHALL run as backend-managed tasks without blocking settings navigation.

#### Scenario: Start framework installation
- **WHEN** the user confirms installation
- **THEN** the native service SHALL return an operation task, transition the framework to `installing`, and stream or expose operation logs through the shared task interface

#### Scenario: Concurrent mutation for one framework
- **WHEN** a second mutation is requested while the same framework already has a running mutation
- **THEN** the native service SHALL reject or serialize the second mutation without corrupting persisted state

#### Scenario: Installation failure
- **WHEN** environment creation, package installation, or verification fails
- **THEN** the task SHALL finish as failed, the framework SHALL expose an actionable error, and the service SHALL NOT mark the framework installed

### Requirement: Application-owned extension storage
The native service SHALL resolve framework environments and model assets below an application-owned VaneHub data directory and persist mutable configuration through SQLite.

#### Scenario: Install managed framework files
- **WHEN** an extension installation creates runtime files
- **THEN** all managed files SHALL be created below the backend-resolved directory for that allowlisted framework rather than a frontend-provided path

#### Scenario: Uninstall framework
- **WHEN** the user confirms uninstall for a stopped framework
- **THEN** the native service SHALL validate the exact managed target, remove only that framework's owned files, update SQLite state, and preserve unrelated frameworks and application data

### Requirement: Local framework lifecycle
The system SHALL distinguish installed, enabled, and running state and SHALL allow at most one enabled framework per capability.

#### Scenario: Installed framework remains disabled
- **WHEN** a framework installation completes
- **THEN** the framework SHALL be installed but SHALL NOT be automatically enabled or started

#### Scenario: Enable framework for capability
- **WHEN** the user enables an installed framework
- **THEN** the service SHALL make it the sole active framework for its capability and persist the selection

#### Scenario: Start installed framework
- **WHEN** the user starts an installed and supported framework
- **THEN** the native service SHALL launch only the backend-owned loopback sidecar plan, record process ownership, and transition to running only after its health check succeeds

#### Scenario: Foreign port owner
- **WHEN** the selected loopback port is already owned by an unrelated process
- **THEN** the native service SHALL leave that process untouched and return a user-displayable conflict error

### Requirement: Health and runtime self-test
The system SHALL provide non-destructive health refresh and runtime self-test operations without requiring chat integration.

#### Scenario: Refresh health
- **WHEN** the page refreshes framework status
- **THEN** the service SHALL check current managed state and loopback health without installing packages or starting a missing process

#### Scenario: Run self-test
- **WHEN** a user requests a self-test for an installed framework
- **THEN** the native service SHALL execute the allowlisted verification plan, expose its task logs, and return a localized success or actionable failure result

### Requirement: Local management sidecar boundary
First-version extension management sidecars SHALL bind only to loopback interfaces, SHALL NOT send user inference content to a remote service, and SHALL NOT claim that an OCR, ASR, or TTS inference API is available before capability-specific inference protocols are implemented.

#### Scenario: Start installed framework management sidecar
- **WHEN** PaddleOCR, faster-whisper, or sherpa-onnx starts after its managed environment is installed
- **THEN** the backend SHALL launch an owned loopback-only health sidecar without exposing an OCR, ASR, or TTS inference endpoint or sending user inference content to a remote service

#### Scenario: Defer capability inference endpoint
- **WHEN** a first-version framework is installed, enabled, or running
- **THEN** the system SHALL report only management lifecycle and runtime self-test readiness and SHALL defer model-backed inference availability to a future capability-consumer change
