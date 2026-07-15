# sdk-dependency-management Specification

## Purpose
Defines VaneHub-managed SDK dependency catalog, installation storage, status detection, version management, operation logs, uninstall safety, and frontend service adapter behavior.

## Requirements
### Requirement: Managed SDK dependency catalog
The system SHALL define a managed SDK dependency catalog with stable kebab-case SDK ids, display names, npm package names, required companion packages, fallback versions, descriptions, and related providers.

#### Scenario: List managed SDK definitions
- **WHEN** the SDK dependency service loads the managed catalog
- **THEN** the system SHALL include `claude-sdk` for `@anthropic-ai/claude-agent-sdk` and `codex-sdk` for `@openai/codex-sdk`

#### Scenario: Reject unknown SDK ids
- **WHEN** an SDK operation is requested for an id not present in the managed catalog
- **THEN** the system SHALL reject the operation without running npm or deleting files

### Requirement: VaneHub SDK installation storage
The system SHALL install managed SDK dependencies under the VaneHub-owned user directory `~/.vanehub/dependencies/<sdk-id>/`.

#### Scenario: Install into VaneHub dependency root
- **WHEN** a managed SDK is installed
- **THEN** the system SHALL create or use that SDK's directory under `~/.vanehub/dependencies/`
- **AND** the system SHALL NOT install the SDK into the project `node_modules` or `~/.codemoss/dependencies/`

#### Scenario: Record installed SDK metadata
- **WHEN** a managed SDK installation succeeds
- **THEN** the system SHALL record the installed version and install metadata in VaneHub dependency storage

### Requirement: SDK status detection
The system SHALL report each managed SDK's installation status, installed version, install path, latest known version, update availability, and error state through the SDK service boundary.

#### Scenario: Detect installed SDK
- **WHEN** the SDK package exists under the managed SDK's VaneHub dependency directory
- **THEN** the system SHALL report that SDK as installed with the version read from the package metadata

#### Scenario: Detect missing SDK
- **WHEN** the SDK package is absent from the managed SDK's VaneHub dependency directory
- **THEN** the system SHALL report that SDK as not installed without treating the absence as an error

### Requirement: Node and npm environment readiness
The system SHALL check Node.js and npm readiness before running install or update operations.

#### Scenario: Node environment available
- **WHEN** Node.js and npm can be detected and executed
- **THEN** the system SHALL report the environment as available with detected path and version details where available

#### Scenario: Node environment unavailable
- **WHEN** Node.js or npm cannot be detected or executed
- **THEN** the system SHALL prevent SDK install and update operations and return a user-displayable reason

### Requirement: SDK version discovery
The system SHALL provide selectable versions for each managed SDK using npm registry data when available and fallback versions when registry lookup fails.

#### Scenario: Load remote versions
- **WHEN** npm registry version lookup succeeds for a managed SDK
- **THEN** the system SHALL return normalized selectable versions sorted from newest to oldest with source `remote`

#### Scenario: Use fallback versions
- **WHEN** npm registry version lookup fails or times out
- **THEN** the system SHALL return the managed SDK's fallback versions with source `fallback`

### Requirement: SDK install update and rollback
The system SHALL install a selected SDK version, update an installed SDK to a newer selected version, and roll back an installed SDK to an older selected version.

#### Scenario: Install selected version
- **WHEN** a user installs a not-installed SDK with a selected valid version
- **THEN** the system SHALL install the SDK package spec for that selected version under the SDK's VaneHub dependency directory

#### Scenario: Update selected version
- **WHEN** a user selects a version newer than the installed version and starts the operation
- **THEN** the system SHALL install the selected version and report the operation as an update

#### Scenario: Roll back selected version
- **WHEN** a user selects a version older than the installed version and starts the operation
- **THEN** the system SHALL install the selected version and report the operation as a rollback

### Requirement: SDK operation logs
The system SHALL expose SDK install, update, rollback, and uninstall logs to the frontend SDK service.

#### Scenario: Show operation logs
- **WHEN** an SDK operation produces output or errors
- **THEN** the system SHALL make log lines or accumulated logs available to the SDK settings page with the related SDK id

#### Scenario: Complete operation with result
- **WHEN** an SDK operation finishes
- **THEN** the system SHALL return a success or failure result with the SDK id and user-displayable error information when applicable

### Requirement: SDK uninstall safety
The system SHALL uninstall a managed SDK only by deleting that SDK's normalized directory inside `~/.vanehub/dependencies/` and updating VaneHub dependency metadata.

#### Scenario: Uninstall managed SDK
- **WHEN** a user uninstalls a managed SDK
- **THEN** the system SHALL remove only that SDK's VaneHub dependency directory and update dependency metadata

#### Scenario: Block unsafe uninstall path
- **WHEN** the resolved SDK directory is outside the resolved VaneHub dependency root
- **THEN** the system SHALL abort uninstall without deleting files

### Requirement: SDK command safety
The system SHALL construct npm operations from backend-owned package definitions and validated version values without shell string interpolation.

#### Scenario: Reject invalid requested version
- **WHEN** a requested version is empty, a range, a tag, or contains characters outside the accepted semver-like format
- **THEN** the system SHALL reject or ignore that requested version before constructing npm package specifiers

#### Scenario: Disable npm lifecycle scripts
- **WHEN** the system runs npm install for a managed SDK
- **THEN** the system SHALL pass `--ignore-scripts` to the npm install command

### Requirement: SDK service adapter boundary
The frontend SHALL expose SDK dependency operations through a TypeScript service interface with runtime-specific adapters.

#### Scenario: Tauri runtime SDK operation
- **WHEN** the frontend runs in the Tauri desktop runtime and an SDK operation is requested
- **THEN** the SDK Tauri adapter SHALL call the matching Rust command through `invoke()`

#### Scenario: Web runtime SDK operation
- **WHEN** the frontend runs in browser Web runtime and an SDK operation is requested
- **THEN** the SDK Web adapter SHALL return mock SDK data and simulated operation behavior without requiring native commands

#### Scenario: React components use service interface
- **WHEN** SDK React components load state or run operations
- **THEN** they SHALL call the SDK service interface and SHALL NOT import or call Tauri `invoke()` directly

### Requirement: Observable SDK operations
SDK install, update, rollback, and uninstall operations SHALL run through the observable operation model when executed by the native runtime.

#### Scenario: SDK operation starts
- **WHEN** a user starts an SDK install, update, rollback, or uninstall
- **THEN** the SDK service SHALL expose an operation id and initial operation status

#### Scenario: SDK operation logs persist
- **WHEN** an SDK operation emits npm output, validation errors, completion, or failure
- **THEN** the system SHALL make logs available through the SDK service with the related SDK id and operation id

### Requirement: SDK storage uses native storage foundation
Managed SDK dependency metadata SHALL use the native runtime storage foundation for VaneHub-owned paths and migration-managed metadata.

#### Scenario: Read SDK status after migration
- **WHEN** the SDK service lists statuses after native storage migrations have run
- **THEN** it SHALL read SDK metadata from the VaneHub-owned storage path and return statuses through the SDK service boundary
