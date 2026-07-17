## MODIFIED Requirements

### Requirement: Asynchronous CLI detection operations
The native runtime SHALL perform bounded all-tool and targeted CLI installation discovery and version refresh as asynchronous backend-managed operations.

#### Scenario: Start first CLI detection
- **WHEN** the application starts and no persisted CLI detection result exists
- **THEN** the native runtime SHALL start at most one asynchronous all-tool CLI detection refresh operation without blocking application startup

#### Scenario: Start targeted CLI detection
- **WHEN** the frontend requests refresh for a supported stable agent id
- **THEN** the native runtime SHALL return an operation id before bounded path enumeration, version probes, or registry queries complete

#### Scenario: CLI refresh does not block
- **WHEN** local executable checks, CLI version commands, or npm registry queries are running
- **THEN** they SHALL NOT block the Tauri main thread or frontend rendering

#### Scenario: Persist refresh results
- **WHEN** a CLI refresh operation completes or partially completes
- **THEN** the native runtime SHALL persist per-CLI status, active path, bounded installation distribution, versions, conflict state, errors, and timestamps for later cached reads

### Requirement: Guarded CLI package command construction
The native runtime SHALL construct CLI package commands and lifecycle eligibility from backend-owned metadata and the freshly validated active installation rather than frontend-supplied command strings.

#### Scenario: Install selected CLI version
- **WHEN** the frontend submits a supported agent id and stable target version for an eligible missing or npm-managed CLI
- **THEN** the native runtime SHALL resolve the npm package from a backend whitelist and execute npm with explicit arguments equivalent to `npm install -g <package>@<targetVersion>`

#### Scenario: Reject unsafe active source
- **WHEN** the active executable is non-npm, unknown, broken, or no longer matches the confirmed lifecycle plan
- **THEN** the native runtime SHALL reject automatic npm mutation for that active installation and return concise manual or source-native guidance

#### Scenario: Reject unknown CLI operation target
- **WHEN** the frontend submits an unknown agent id for a CLI operation
- **THEN** the native runtime SHALL reject the operation without executing an external command

#### Scenario: Avoid shell interpolation
- **WHEN** the native runtime executes CLI detection, npm version checks, or npm package operations
- **THEN** it SHALL construct process invocations with explicit executable and argument values and SHALL NOT rely on shell string interpolation

## ADDED Requirements

### Requirement: Bounded CLI installation enumeration
The native runtime SHALL enumerate supported CLI installations from backend-owned bounded candidates and SHALL NOT recursively scan arbitrary user disks.

#### Scenario: Enumerate PATH and known locations
- **WHEN** the native runtime detects a supported CLI
- **THEN** it SHALL inspect all PATH results and a bounded platform-specific set of known locations, normalize candidates, and probe distinct targets with timeouts

#### Scenario: Preserve active PATH entry
- **WHEN** one or more PATH results exist
- **THEN** the native runtime SHALL identify the first valid PATH result as the active installation while retaining other distinct installations for diagnostics

#### Scenario: Executable is installed but broken
- **WHEN** a candidate executable exists but its bounded version probe exits unsuccessfully or times out
- **THEN** the native runtime SHALL preserve it as installed but non-runnable and record redacted diagnostics through unified logging

### Requirement: Serialized CLI package mutations
The native runtime SHALL prevent overlapping managed CLI package mutations.

#### Scenario: Package mutation already running
- **WHEN** an install, upgrade, or downgrade is requested while another managed CLI package mutation is queued or running
- **THEN** the native runtime SHALL reject or queue the new mutation deterministically without launching concurrent global package-manager writes

#### Scenario: Detection during package mutation
- **WHEN** a safe read-only detection request occurs while a package mutation is running
- **THEN** the runtime MAY execute or defer detection but SHALL keep the Tauri command boundary nonblocking and SHALL NOT corrupt the package mutation

