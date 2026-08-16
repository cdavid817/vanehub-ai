## ADDED Requirements

### Requirement: Trusted updater configuration
The desktop application MUST use the Tauri v2 updater with HTTPS endpoints and an embedded public verification key, and ordinary runtime settings MUST NOT replace the endpoint authority or verification key. TLS certificate errors MUST fail the update operation and MUST NOT be ignored.

#### Scenario: Runtime setting attempts to replace update source
- **WHEN** ordinary application configuration contains an alternate update URL or verification key
- **THEN** the desktop updater SHALL continue using its build-time trusted endpoint and public key

#### Scenario: TLS validation fails
- **WHEN** the configured update endpoint cannot pass platform TLS validation
- **THEN** the update check SHALL fail without downloading or installing an artifact

### Requirement: Signed update verification
Updater metadata and installable updater artifacts MUST be signed by the release updater key, and the desktop runtime MUST verify the signature before applying an update. Only the public verification key SHALL be included in source code or client bundles.

#### Scenario: Valid signed update is installed
- **WHEN** metadata and the selected platform artifact carry valid signatures from the configured updater key
- **THEN** the desktop runtime SHALL allow the verified artifact to proceed to installation

#### Scenario: Update payload is tampered
- **WHEN** a metadata document, signature, or downloaded artifact differs from the signed content
- **THEN** verification SHALL reject the update before installation and preserve the current application

### Requirement: Channel and downgrade policy
The updater SHALL support `stable` and `preview` channels using semantic-version precedence. Stable clients MUST NOT accept prerelease updates, and every ordinary client MUST reject an equal or lower version. A downgrade MAY occur only in an explicitly compiled development or desktop-test flow that cannot be enabled by ordinary runtime configuration.

#### Scenario: Stable client sees preview release
- **WHEN** a stable client checks a channel containing a newer prerelease
- **THEN** the prerelease SHALL NOT be offered

#### Scenario: Preview client sees eligible release
- **WHEN** a preview client checks its configured preview channel and a greater compatible version exists
- **THEN** that version SHALL be offered according to semantic-version precedence

#### Scenario: Metadata proposes a downgrade
- **WHEN** an ordinary client receives a validly signed update whose version is equal to or lower than the installed version
- **THEN** the update SHALL be rejected before download or installation

### Requirement: Observable asynchronous update lifecycle
Update checks and downloads MUST run asynchronously through the frontend service boundary and backend-managed operations. A started action SHALL return a stable operation id before variable-duration work completes and SHALL expose timestamps, progress, terminal result or command-safe error, and redacted unified-log association while retaining previously loaded page data.

#### Scenario: User starts an update check
- **WHEN** the user requests an update check
- **THEN** the service SHALL return a stable operation id before network access completes
- **AND** the UI SHALL remain responsive and retain the prior update snapshot

#### Scenario: Download progresses
- **WHEN** a verified update is downloading
- **THEN** the observable snapshot SHALL report bounded byte progress and remain available until a terminal state

#### Scenario: Operation fails
- **WHEN** checking, downloading, verification, or installation fails
- **THEN** the operation SHALL enter a failed terminal state with a safe recoverable error
- **AND** the currently installed application SHALL remain usable

### Requirement: Update preferences and automatic checks
The application SHALL persist an automatic-check preference and selected channel through the desktop settings boundary, default automatic checks to disabled, and derive the initial channel from the installed semantic version when no preference exists. Automatic checking MUST use the same signed asynchronous check path as manual checking.

#### Scenario: Existing installation has no update preference
- **WHEN** an existing settings store is opened without update keys
- **THEN** automatic checking SHALL default to disabled
- **AND** the channel SHALL default to `preview` for a prerelease build and `stable` otherwise

#### Scenario: Automatic check is enabled
- **WHEN** application startup observes an enabled automatic-check preference
- **THEN** it SHALL schedule the same non-blocking signed check used by the manual action

### Requirement: Runtime adapter parity
The Tauri and Web/mock adapters MUST implement the same update service contract. Tauri SHALL perform native signed update operations, while Web/mock SHALL simulate deterministic lifecycle states without native installation side effects.

#### Scenario: Browser user exercises update UI
- **WHEN** the About update surface runs with the Web adapter
- **THEN** check, available, progress, failure, ready, and restart states SHALL be representable without Tauri IPC

#### Scenario: Desktop user exercises update UI
- **WHEN** the same surface runs with the Tauri adapter
- **THEN** runtime-specific work SHALL occur only behind declared Tauri commands

### Requirement: Localized responsive update surface
Settings About SHALL display current version, selected channel, last checked time, release notes, manual check, download/install progress, failure recovery, ready-to-restart, automatic-check control, and restart action using all registered locales. The surface MUST remain usable in `futuristic` and `minimal` styles at desktop and narrow widths without clipping, overlap, unreadable contrast, layout shift, or blank content.

#### Scenario: Update is available
- **WHEN** a check finds an eligible signed update
- **THEN** the surface SHALL show the installed and new versions, release notes, and a download-and-install action

#### Scenario: Update is ready to restart
- **WHEN** verified installation staging completes
- **THEN** the surface SHALL show a restart action and preserve the release details

#### Scenario: Narrow localized layout
- **WHEN** any registered locale renders the update surface at narrow width in either supported style
- **THEN** controls and status content SHALL remain readable and operable without horizontal clipping

### Requirement: Failure recovery
The updater SHALL preserve the running installed version after a failed check, download, verification, or staged installation and SHALL allow a later explicit retry. Restart SHALL occur only after the updater reports a verified ready state and the user explicitly requests it.

#### Scenario: Download is interrupted
- **WHEN** an update download fails or is interrupted
- **THEN** the current installation SHALL remain runnable and the user SHALL be able to retry

#### Scenario: User has not approved restart
- **WHEN** a verified update is ready but the user has not selected restart
- **THEN** the application SHALL continue running without an automatic restart

### Requirement: Deterministic policy performance
Version/channel admission and updater manifest validation MUST have repeatable benchmark evidence using deterministic input batches and structural budgets suitable for shared CI runners.

#### Scenario: Policy benchmark runs
- **WHEN** the benchmark evaluates the declared batch of stable, preview, downgrade, and malformed candidates
- **THEN** it SHALL report processed cases and satisfy the declared complexity or throughput budget without relying on a fragile single wall-clock threshold

