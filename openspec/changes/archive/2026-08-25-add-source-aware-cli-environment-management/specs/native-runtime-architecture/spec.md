## MODIFIED Requirements

### Requirement: Asynchronous CLI detection operations

The native runtime SHALL perform bounded all-tool and targeted CLI installation discovery, source-specific version refresh, and optional read-only readiness probes as asynchronous backend-managed operations.

#### Scenario: Start first CLI detection

- **WHEN** the application starts and no persisted normalized CLI environment snapshot exists
- **THEN** the native runtime SHALL start at most one asynchronous all-tool refresh without blocking application startup

#### Scenario: Start targeted CLI detection

- **WHEN** the frontend requests refresh for a supported stable Agent id
- **THEN** the native runtime SHALL return an operation id before bounded path enumeration, version probes, source catalog queries, or readiness probes complete

#### Scenario: CLI refresh does not block

- **WHEN** local executable checks, provider probes, npm queries, WinGet queries, or other supported source lookups are running
- **THEN** they SHALL NOT block the Tauri main thread or frontend rendering

#### Scenario: Persist refresh results

- **WHEN** a CLI refresh operation completes or partially completes
- **THEN** the native runtime SHALL persist the normalized snapshot, active installation, bounded installation distribution, source confidence, source-specific catalogs, orthogonal status, conflicts, freshness, errors, and timestamps
- **AND** it SHALL not synthesize another source's latest version

### Requirement: Asynchronous CLI package operations

The native runtime SHALL prepare and execute CLI install, upgrade, downgrade, reinstall, uninstall, and repair only through asynchronous backend-managed action-plan operations when the selected source supports the action.

#### Scenario: Prepare CLI lifecycle operation

- **WHEN** the frontend requests a lifecycle action for a supported CLI, source, and optional target
- **THEN** the native runtime SHALL return a stable operation id before source preflight or catalog access completes
- **AND** the completed planning operation SHALL identify a persisted reviewable action plan

#### Scenario: Start CLI package operation

- **WHEN** the frontend confirms a valid plan id and revision
- **THEN** the native runtime SHALL return a stable operation id before the source process or download completes

#### Scenario: Capture CLI package operation logs

- **WHEN** planning, download, package execution, or verification emits output
- **THEN** the native runtime SHALL associate bounded redacted logs and phase with the operation

#### Scenario: Refresh after successful package operation

- **WHEN** a CLI lifecycle process reaches a terminal state
- **THEN** the native runtime SHALL perform best-effort affected-tool discovery
- **AND** it SHALL persist the observed post-operation environment or a stale partial-completion state

### Requirement: Guarded CLI package command construction

The native runtime SHALL construct source-specific CLI lifecycle plans and process arguments from backend-owned definitions, source capabilities, a freshly validated environment snapshot, and a persisted action plan rather than frontend-supplied command strings.

#### Scenario: Install selected CLI version

- **WHEN** a valid plan records source `S` and exact target version `X`
- **THEN** the runtime SHALL invoke only source adapter `S` with target `X`
- **AND** package identifiers and executable arguments SHALL come from backend-owned definitions

#### Scenario: Reject unsafe active source

- **WHEN** the active source is detect-only, unknown, broken, or no longer matches the plan fingerprint
- **THEN** the native runtime SHALL reject automatic mutation before launching an external command
- **AND** it SHALL return typed guidance or a stale-plan result

#### Scenario: Reject unknown CLI operation target

- **WHEN** the frontend submits an unknown Agent id, source id, action, or invalid target during planning
- **THEN** the native runtime SHALL reject the request without executing an external command

#### Scenario: Avoid shell interpolation

- **WHEN** the runtime performs discovery, source catalog lookup, download, probe, or lifecycle execution
- **THEN** it SHALL use explicit executable and argument values
- **AND** it SHALL not execute a frontend-supplied shell string, pipe-to-shell flow, `Invoke-Expression`, or `irm | iex`

#### Scenario: Do not fall back across sources

- **WHEN** a selected source fails preflight or execution
- **THEN** the operation SHALL fail for that source
- **AND** the runtime SHALL not silently start another source

### Requirement: Serialized CLI package mutations

The native runtime SHALL coordinate CLI mutations by stable tool id and source-declared mutation key, with a bounded global mutation concurrency.

#### Scenario: Package mutation already running

- **WHEN** a mutation for one tool is queued or running and another mutation targets the same tool
- **THEN** the runtime SHALL queue or reject the second deterministically without launching overlapping writes

#### Scenario: Same mutation key is busy

- **WHEN** two actions require the same global package-manager mutation key
- **THEN** they SHALL not execute concurrently

#### Scenario: Independent mutation capacity

- **WHEN** actions use independent mutation keys and global capacity is available
- **THEN** the runtime MAY execute them concurrently up to the documented bound of two

#### Scenario: Detection during package mutation

- **WHEN** a read-only targeted detection request occurs during a conflicting mutation
- **THEN** the runtime SHALL execute it only when the source declares the read safe or queue it after the mutation
- **AND** the Tauri command boundary SHALL remain nonblocking

### Requirement: Nonblocking CLI command boundaries

The native runtime SHALL keep CLI list, refresh, action planning, action execution, bulk planning/execution, and Doctor command boundaries responsive.

#### Scenario: Cached list returns directly

- **WHEN** the frontend requests persisted CLI environment snapshots
- **THEN** the bounded command MAY return them directly without starting a process or network request

#### Scenario: Refresh command returns before detection completes

- **WHEN** the frontend requests CLI environment refresh
- **THEN** the Tauri command SHALL return a stable operation id before path enumeration, version probes, or source catalog queries complete

#### Scenario: Package command returns before npm completes

- **WHEN** the frontend requests plan preparation, lifecycle execution, bulk preparation/execution, or Doctor work
- **THEN** the Tauri command SHALL return a stable operation id before package-manager queries, downloads, or child processes complete

#### Scenario: Background timeout is reported

- **WHEN** variable-duration CLI work times out
- **THEN** the timeout SHALL be recorded on the operation and in unified logs
- **AND** it SHALL not surface as a blocking Tauri command timeout

### Requirement: Managed CLI package operation parity

The native runtime SHALL use one source-aware CLI application service for every registered CLI tool while delegating source-specific catalog and lifecycle behavior to explicit distribution adapters.

#### Scenario: Resolve package metadata from catalog

- **WHEN** a CLI action is planned
- **THEN** the application SHALL resolve the stable Agent id, provider, executable names, distributions, source capability, package reference, probes, and trust policy from the backend registry

#### Scenario: Execute through selected adapter

- **WHEN** a valid action plan is confirmed
- **THEN** the application SHALL resolve the selected source adapter and execute the plan's exact source/action/target
- **AND** it SHALL not route all sources through npm

#### Scenario: Refresh affected CLI after package success

- **WHEN** a source process succeeds, fails, times out, or is cancelled after admission
- **THEN** the runtime SHALL attempt to refresh the affected environment
- **AND** the persisted snapshot SHALL include the operation id and normalized mutation outcome

## ADDED Requirements

### Requirement: Native CLI action-plan persistence

The native runtime SHALL persist versioned single-use CLI action plans and bulk plans through additive SQLite migrations owned by `tooling::cli`.

#### Scenario: Create plan atomically

- **WHEN** planning validates source capability, target, preconditions, and snapshot fingerprint
- **THEN** the repository SHALL persist the complete draft plan atomically before returning its id

#### Scenario: Consume plan atomically

- **WHEN** execution is admitted
- **THEN** the repository SHALL atomically validate revision/state/expiry and transition the plan to executing before the external effect begins

#### Scenario: Existing database is migrated

- **WHEN** an older VaneHub database is opened
- **THEN** new environment, catalog, and plan tables SHALL be added without deleting legacy CLI statuses or unrelated data

### Requirement: Native CLI source adapter boundary

The native CLI application layer SHALL depend on application-owned source, discovery, probe, repository, operation, and mutation-coordination ports, with concrete npm, WinGet, vendor, and detect-only adapters selected only in bootstrap.

#### Scenario: Source adapter is selected

- **WHEN** an action plan names a supported source
- **THEN** the application SHALL resolve the matching adapter through the assembled registry
- **AND** domain or application code SHALL not construct the concrete package-manager process directly

#### Scenario: Adapter output is unsafe or oversized

- **WHEN** a source adapter receives process output
- **THEN** it SHALL pass the output through bounded redacted operation/logging sinks before persistence or frontend delivery

### Requirement: Native CLI external-effect consistency

The native runtime SHALL model package-manager and installer execution as external effects that cannot be rolled back by SQLite.

#### Scenario: External effect may have occurred

- **WHEN** a process starts and later verification fails
- **THEN** the runtime SHALL not restore the pre-operation snapshot as a claimed rollback
- **AND** it SHALL preserve an observed or stale partial-completion result

#### Scenario: Database write fails after verified change

- **WHEN** post-discovery verifies a machine change but persistence fails
- **THEN** the operation SHALL report storage failure with a diagnostic id
- **AND** a later refresh SHALL be able to rediscover the actual machine state
