# app-settings Specification

## Purpose
TBD - created by archiving change complete-general-settings-i18n-font-fix. Update Purpose after archive.
## Requirements
### Requirement: Common settings model
The system SHALL manage common application settings for application language, font size, visual theme, and default folder path through a shared settings model.

#### Scenario: Load default settings
- **WHEN** no persisted common settings exist
- **THEN** the system SHALL provide valid defaults for language, font size, visual theme, and default folder path

#### Scenario: Accept a supported language setting
- **WHEN** the application-language value is `zh-CN`, `en`, `zh-TW`, `ja`, or `ko`
- **THEN** both desktop and Web/mock settings implementations SHALL accept and preserve the canonical locale id

#### Scenario: Reject invalid setting value
- **WHEN** a setting value is outside the supported values for its setting key
- **THEN** the system SHALL reject the value before applying it to the application UI

### Requirement: Settings side effects
The system SHALL apply common settings through centralized side effects owned by the settings provider and native settings layer.

#### Scenario: Apply language setting
- **WHEN** the application language changes to any supported locale
- **THEN** the settings provider SHALL load and synchronize the active i18next language with the selected value
- **AND** the desktop native settings layer SHALL refresh persistent framework-owned native copy when running under Tauri

#### Scenario: Apply font size setting
- **WHEN** the font size setting changes to 12px, 14px, 16px, or 18px
- **THEN** the system SHALL set the root `html` font size so rem-based Tailwind sizing scales with the selected value

#### Scenario: Apply visual theme setting
- **WHEN** the visual theme setting changes between futuristic and minimal styles
- **THEN** the system SHALL update the document theme attribute used by CSS variable groups

### Requirement: Settings persistence
The system SHALL persist common settings through the active runtime adapter and SHALL complete initial settings hydration before displaying the formal application surface.

#### Scenario: Persist desktop setting
- **WHEN** the application runs in the Tauri desktop runtime and a user saves a common setting
- **THEN** the system SHALL persist the setting through a Tauri command backed by SQLite storage

#### Scenario: Persist Web setting
- **WHEN** the application runs in the browser Web runtime and a user saves a common setting
- **THEN** the system SHALL persist the setting through the Web adapter without requiring a Tauri command

#### Scenario: Restore saved settings
- **WHEN** the application starts after common settings have been saved
- **THEN** the system SHALL restore and apply the saved setting values for the active runtime
- **AND** the formal application surface SHALL first become visible with the restored root font size, visual theme, and supported application language resource already applied

#### Scenario: Fall back when initial settings cannot be loaded
- **WHEN** the active runtime fails to load common settings or its persisted supported language resource during application startup
- **THEN** the system SHALL apply the shared default settings before displaying the formal application surface
- **AND** the settings provider SHALL retain a localized user-displayable error without preventing startup

### Requirement: Node.js environment display
The system SHALL expose read-only Node.js environment information for the Basic Configuration page.

#### Scenario: Node.js is available
- **WHEN** the runtime can resolve a Node.js executable and version
- **THEN** the settings page SHALL display the resolved path and version as read-only information

#### Scenario: Node.js is unavailable
- **WHEN** the runtime cannot resolve a Node.js executable or version
- **THEN** the settings page SHALL display an unavailable read-only state without blocking other settings controls

### Requirement: Logging settings model
The system SHALL include log directory and read-only logging policy values in the shared settings model.

#### Scenario: Load default logging settings
- **WHEN** no persisted logging settings exist
- **THEN** the system SHALL provide a valid default log directory and fixed first-version policies for 30-day retention, automatic archival, built-in redaction, and supported log levels

#### Scenario: Save log directory setting
- **WHEN** a user saves a log directory in the desktop runtime
- **THEN** the system SHALL persist the directory through the settings service and use it for newly written logs

#### Scenario: Reject invalid log directory
- **WHEN** the runtime cannot validate or create the requested log directory
- **THEN** the system SHALL reject the setting without changing the active log directory

#### Scenario: Restore log directory setting
- **WHEN** the application restarts after a custom log directory has been saved
- **THEN** the system SHALL restore that directory as the active log directory

### Requirement: Network proxy settings model
The system SHALL include a persisted network proxy URL and editable proxy bypass list in the shared application settings model.

#### Scenario: Load default network proxy setting
- **WHEN** no persisted network proxy setting exists
- **THEN** the system SHALL provide an empty proxy URL representing direct connection
- **AND** provide a default proxy bypass list for localhost and loopback traffic

#### Scenario: Save valid desktop network proxy setting
- **WHEN** a user saves a supported proxy URL in the Tauri desktop runtime
- **THEN** the system SHALL validate and persist the URL through the settings service
- **AND** apply it to new VaneHub-managed native outbound requests and newly launched child processes

#### Scenario: Save valid desktop network proxy bypass setting
- **WHEN** a user saves a proxy bypass list in the Tauri desktop runtime
- **THEN** the system SHALL validate, normalize, and persist the bypass value through the settings service
- **AND** apply it to new VaneHub-managed native outbound requests and newly launched child processes

#### Scenario: Clear desktop network proxy setting
- **WHEN** a user clears the network proxy URL in the Tauri desktop runtime
- **THEN** the system SHALL persist direct connection mode
- **AND** stop applying proxy environment variables to newly launched child processes

#### Scenario: Reject invalid desktop network proxy setting
- **WHEN** a user saves a malformed proxy URL or unsupported proxy scheme
- **THEN** the system SHALL reject the setting without changing the active proxy configuration

#### Scenario: Reject invalid desktop network proxy bypass setting
- **WHEN** a user saves a proxy bypass value containing unsafe control characters
- **THEN** the system SHALL reject the setting without changing the active proxy bypass configuration

#### Scenario: Restore saved network proxy setting
- **WHEN** the application starts after a valid network proxy setting has been saved
- **THEN** the system SHALL restore the proxy URL and bypass list and apply them before starting new VaneHub-managed network work

#### Scenario: Preserve Web mock proxy limitation
- **WHEN** the application runs with the Web/mock settings adapter
- **THEN** the system SHALL NOT claim browser or OS traffic is routed through the saved proxy setting

### Requirement: Network proxy runtime scope
The system SHALL define network proxy application scope as VaneHub-managed traffic in the first version.

#### Scenario: Apply proxy to child processes
- **WHEN** VaneHub launches a network-capable subprocess after a proxy has been configured
- **THEN** the subprocess environment SHALL include standard proxy variables for the configured proxy URL
- **AND** include `NO_PROXY` and `no_proxy` variables for the configured bypass list

#### Scenario: Do not mutate existing processes
- **WHEN** a proxy setting changes while subprocesses are already running
- **THEN** the system SHALL NOT forcibly restart or reconfigure those running subprocesses

#### Scenario: Do not promise system-wide proxying
- **WHEN** network proxy behavior is described in settings or documentation
- **THEN** the system SHALL describe the supported scope as VaneHub-managed native requests and VaneHub-launched subprocesses, not OS-wide interception

### Requirement: Automatic archival settings
The system SHALL expose settings for automatic inactive session archival.

#### Scenario: Default archival settings
- **WHEN** no automatic archival settings have been saved
- **THEN** the system SHALL treat automatic archival as enabled with an inactivity threshold of 10 days

#### Scenario: Save archival settings
- **WHEN** a user changes automatic archival enablement or inactivity threshold
- **THEN** the system SHALL persist the settings through the existing settings service boundary

#### Scenario: Apply disabled setting
- **WHEN** automatic archival is disabled
- **THEN** the Rust background scheduler SHALL skip archival mutations while leaving manual archive operations available

### Requirement: Launch-on-startup application setting
The system SHALL include launch-on-startup in common application settings and apply it through centralized settings side effects.

#### Scenario: Load startup setting
- **WHEN** application settings are loaded
- **THEN** the settings service SHALL return a boolean launch-on-startup value with a safe default of disabled

#### Scenario: Save startup setting
- **WHEN** the launch-on-startup setting is saved
- **THEN** the settings service SHALL validate and persist the boolean value
- **AND** desktop runtime side effects SHALL remain owned by the settings/native layer

#### Scenario: Preserve Web mock parity
- **WHEN** app settings are loaded or saved in the Web/mock runtime
- **THEN** the Web adapter SHALL preserve the launch-on-startup key shape without claiming native startup registration is active

### Requirement: Folder-opener application preferences
The shared application settings model SHALL expose atomically persisted folder-opener preferences containing one configured default stable id and a validated enabled stable-id list, while keeping runtime discovery data outside persisted user settings.

#### Scenario: Load first-use defaults
- **WHEN** no folder-opener preferences have been persisted
- **THEN** the desktop runtime SHALL provide a valid configured default and enabled list with File Explorer enabled
- **AND** SHALL prefer VS Code for the initial configured default when it is discovered according to the defined initialization policy

#### Scenario: Restore saved preferences
- **WHEN** the application starts after folder-opener preferences were saved
- **THEN** the active runtime adapter SHALL restore the configured default and enabled ids
- **AND** SHALL recompute availability and the effective default from the current environment

#### Scenario: Save preferences atomically
- **WHEN** a user submits valid default and enabled folder-opener preferences
- **THEN** the desktop runtime SHALL persist the aggregate in one transaction
- **AND** subscribers SHALL observe one coherent settings change

#### Scenario: Preserve Web settings parity
- **WHEN** preferences are saved through the Web/mock adapter
- **THEN** it SHALL preserve the same validated preference shape without claiming that native discovery or launch is active

### Requirement: Safe observability defaults
Desktop observability settings SHALL default to local metadata timelines enabled, OTLP export disabled, MCP relay disabled, metadata-only capture, and 30-day local trace retention.

#### Scenario: Existing installation upgrades
- **WHEN** an installation without saved observability settings starts after the migration
- **THEN** the runtime SHALL apply the safe defaults without enabling network export or content capture

### Requirement: Observability setting validation
The desktop runtime SHALL validate observability export, sampling, retention, and capture settings before persistence or use.

#### Scenario: Valid settings are saved
- **WHEN** a user saves a supported OTLP endpoint and protocol, sampling ratio from 0 through 1, retention from 1 through 90 days, and supported capture policy
- **THEN** the native settings service SHALL persist the non-secret settings and apply them to newly created execution runs

#### Scenario: Invalid endpoint is submitted
- **WHEN** a user submits a malformed or unsupported OTLP endpoint
- **THEN** the settings service SHALL reject it with a typed validation error
- **AND** it SHALL preserve the previously active configuration

### Requirement: Telemetry credential protection
Optional OTLP authentication material SHALL be stored through the native credential service and SHALL NOT be returned as plaintext through frontend settings contracts.

#### Scenario: Authentication material is saved
- **WHEN** a user configures supported OTLP authentication material
- **THEN** the native runtime SHALL store the secret in the credential store and persist only a safe reference or configured indicator
- **AND** logs and trace settings responses SHALL omit the plaintext value

### Requirement: Runtime-specific observability settings behavior
Observability settings SHALL remain behind the shared frontend settings service with Tauri and Web/mock adapter parity.

#### Scenario: Desktop changes export settings
- **WHEN** React saves observability settings in the desktop runtime
- **THEN** it SHALL call the settings service interface
- **AND** only the Tauri adapter SHALL invoke the native command that updates exporter state

#### Scenario: Web mock changes export settings
- **WHEN** the application runs through the Web/mock adapter
- **THEN** it SHALL return deterministic contract-compatible settings behavior
- **AND** it SHALL identify native OTLP export, credential storage, and SQLite retention as simulated or unavailable

### Requirement: Setting changes preserve active run context
Changes to observability settings SHALL apply prospectively and SHALL NOT rewrite the identity, sampling decision, capture policy, or relay state of an already active execution run.

#### Scenario: Settings change during generation
- **WHEN** a user changes telemetry settings while an Agent generation is running
- **THEN** the active run SHALL continue under its captured settings snapshot
- **AND** the new settings SHALL apply to later runs

### Requirement: Automatic context compaction application setting
The shared application settings model SHALL include a boolean automatic-context-compaction preference, default it to enabled, and persist it through the active settings adapter without a dedicated storage table.

#### Scenario: Existing installation has no saved preference
- **WHEN** settings are loaded without a saved automatic-context-compaction value
- **THEN** desktop and Web/mock runtimes SHALL return the preference as enabled

#### Scenario: Save desktop preference
- **WHEN** a user changes the preference in the desktop runtime
- **THEN** the settings service SHALL validate and persist the boolean value through the native settings layer

#### Scenario: Preserve Web mock parity
- **WHEN** the preference is loaded or saved through the Web/mock settings adapter
- **THEN** the adapter SHALL preserve the same boolean key and behavior without claiming SQLite access

### Requirement: Context quality history retention setting
Application settings SHALL persist a validated local context-quality history retention window, defaulting to 30 days and supporting only the documented bounded options consistently across desktop and Web/mock runtimes.

#### Scenario: Existing installation has no saved retention value
- **WHEN** settings are loaded without a stored context-quality retention value
- **THEN** the effective retention window SHALL be 30 days

#### Scenario: User selects a supported retention value
- **WHEN** the user saves a documented retention option
- **THEN** subsequent history pruning and settings loads SHALL use that value

#### Scenario: Stored retention value is invalid
- **WHEN** a persisted or incoming retention value is outside the supported options
- **THEN** the settings boundary SHALL reject the mutation or normalize corrupted stored data to the safe default

### Requirement: Direct connection does not inherit external proxy configuration
VaneHub-managed native outbound requests SHALL be routed only according to the application's own persisted network proxy setting. When no proxy URL is configured, those requests SHALL connect directly and MUST NOT adopt proxy configuration discovered from the operating system, the environment, or any other source outside that setting.

#### Scenario: Operating system proxy is configured but VaneHub is not
- **WHEN** the host operating system or environment declares a proxy and no VaneHub proxy URL is persisted
- **THEN** VaneHub-managed native outbound requests SHALL connect directly
- **AND** the system SHALL NOT route them through the externally declared proxy

#### Scenario: VaneHub proxy is configured
- **WHEN** a VaneHub proxy URL is persisted
- **THEN** VaneHub-managed native outbound requests SHALL use that proxy rather than any externally declared one

### Requirement: Proxy bypass applies in every routing mode
The configured proxy bypass list SHALL apply to VaneHub-managed native outbound requests regardless of whether a VaneHub proxy URL is configured, so that loopback and bypassed destinations are never routed through a proxy.

#### Scenario: Loopback request while a proxy is configured
- **WHEN** a VaneHub-managed native request targets a destination covered by the bypass list and a VaneHub proxy URL is persisted
- **THEN** the request SHALL connect directly to that destination

#### Scenario: Loopback request in direct connection mode
- **WHEN** a VaneHub-managed native request targets a destination covered by the bypass list and no VaneHub proxy URL is persisted
- **THEN** the request SHALL connect directly to that destination
- **AND** the outcome SHALL NOT depend on an operating system bypass list or its wildcard syntax

#### Scenario: Every native client constructor is covered
- **WHEN** any VaneHub-managed native HTTP client is constructed, whether asynchronous or blocking and whether or not it follows redirects
- **THEN** it SHALL apply the same routing and bypass decision

### Requirement: Minimal first-use visual style
The application SHALL use the `minimal` visual style when no valid persisted theme choice is available, while preserving any valid theme explicitly saved by the user.

#### Scenario: Start without a saved theme
- **WHEN** a new installation or cleared Web/mock profile starts without a persisted visual-theme value
- **THEN** the settings service SHALL return `minimal` as the effective theme
- **AND** the formal application surface SHALL first render with the minimal theme applied

#### Scenario: Restore a saved futuristic theme
- **WHEN** the persisted visual-theme value is `futuristic`
- **THEN** the application SHALL restore `futuristic` instead of replacing it with the new default

#### Scenario: Recover from an invalid theme value
- **WHEN** startup encounters a missing, invalid, or unreadable visual-theme value
- **THEN** the runtime SHALL fall back to `minimal` consistently in desktop and Web/mock modes

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

### Requirement: Personalization settings migration boundary
The system SHALL ensure that the generic shared application settings model is no longer the runtime source of truth for custom instructions or long-term memory policy after dedicated-personalization migration completes. Personalization policy SHALL be persisted and mutated through the revisioned personalization service. During the compatibility window, the settings layer MAY retain legacy custom-instruction and memory fields for deserialization and one-time migration, and SHALL persist a migration-generation marker without exposing whole-object personalization mutation to the new UI.

#### Scenario: Fresh installation loads personalization
- **WHEN** no legacy personalization settings or dedicated policy exist
- **THEN** the personalization service SHALL create or resolve its validated default global policy
- **AND** generic `AppSettings` SHALL not need to create a second personalization configuration

#### Scenario: Existing installation migrates personalization
- **WHEN** legacy about-you, response-style, custom-instruction enablement, memory enablement, or tool-assisted extraction fields exist and migration has not completed
- **THEN** the native runtime SHALL migrate them idempotently into dedicated policy records
- **AND** SHALL mark the migration generation only after the policy transaction succeeds

#### Scenario: Restore migrated personalization
- **WHEN** the application restarts after migration completes
- **THEN** runtime personalization SHALL load from the dedicated personalization service
- **AND** legacy `AppSettings` values SHALL not override newer policy revisions

#### Scenario: New UI saves personalization
- **WHEN** the user changes custom instructions or memory policy in the AI Personalization page
- **THEN** React SHALL call the dedicated personalization service with a typed scope patch and expected revision
- **AND** SHALL not submit or replace the entire `AppSettings` aggregate

#### Scenario: Legacy whole-settings save occurs during compatibility
- **WHEN** an older internal caller saves an `AppSettings` object containing legacy personalization fields after migration
- **THEN** the settings layer SHALL preserve ordinary non-personalization settings
- **AND** SHALL not overwrite the dedicated personalization policy from those deprecated fields

#### Scenario: Preserve Web/mock parity
- **WHEN** personalization is loaded or saved in Web/mock mode
- **THEN** the Web adapter SHALL implement the dedicated personalization contract and migration-shaped defaults deterministically
- **AND** generic mock `AppSettings` SHALL not be treated as the authoritative policy store

#### Scenario: Migration cannot establish a valid policy
- **WHEN** legacy migration or dedicated policy loading fails before any validated policy exists
- **THEN** the application SHALL retain a localized maintenance warning
- **AND** personalization runtime behavior SHALL use fail-closed instruction and memory defaults without blocking unrelated application startup

