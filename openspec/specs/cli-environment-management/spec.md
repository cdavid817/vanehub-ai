# cli-environment-management Specification

## Purpose
TBD - created by archiving change add-source-aware-cli-environment-management. Update Purpose after archive.
## Requirements
### Requirement: Source-aware CLI tool catalog

The system SHALL define each supported CLI tool as a stable Agent id with one or more platform-specific distribution sources, explicit source capabilities, executable names, read-only probes, and trust metadata.

#### Scenario: Tool supports multiple sources

- **WHEN** a supported tool can be distributed through npm, WinGet, or an audited vendor installer
- **THEN** the backend SHALL retain those sources as separate source definitions
- **AND** each source SHALL declare its own platforms, version-selection mode, lifecycle actions, channels, and trust policy

#### Scenario: Transport is not a source

- **WHEN** a vendor installer is downloaded by curl, wget, or a PowerShell download API
- **THEN** the system SHALL model the source as a vendor installer
- **AND** it SHALL NOT expose the download transport as lifecycle eligibility

### Requirement: Bounded installation discovery and active resolution

The system SHALL discover supported CLI installations from ordered PATH results and bounded platform-specific known locations without recursively scanning arbitrary disks.

#### Scenario: Multiple PATH entries exist

- **WHEN** multiple distinct executable candidates are found
- **THEN** the backend SHALL retain every bounded candidate with path priority, version, executable status, inferred source, and source confidence
- **AND** it SHALL select the first runnable PATH candidate as active

#### Scenario: First PATH entry is broken

- **WHEN** the first PATH candidate exists but cannot complete the bounded version probe
- **THEN** the backend SHALL preserve it as active-but-broken for diagnosis unless a documented active-resolution rule selects another PATH entry
- **AND** it SHALL retain later runnable candidates as shadowed installations

#### Scenario: Candidate paths alias the same executable

- **WHEN** two discovered paths resolve to the same canonical executable
- **THEN** the system SHALL store one installation identity while retaining safe alias information needed for diagnostics

### Requirement: PATH-selected and recommended installations are distinct

The system SHALL report which installation the host would run and which installation it recommends as two separate identities, and SHALL NOT present the recommended one as the PATH-selected one.

#### Scenario: A broken launcher precedes a healthy installation

- **WHEN** the first PATH-resolved launcher fails its bounded probe and a later discovered installation is healthy
- **THEN** the snapshot SHALL report the broken launcher as PATH-selected and the healthy installation as recommended
- **AND** it SHALL raise a conflict rather than silently reporting the healthy installation as what the host runs

#### Scenario: The PATH-selected installation is healthy

- **WHEN** the first PATH-resolved launcher completes its probe successfully
- **THEN** PATH-selected and recommended SHALL be the same installation
- **AND** no precedence conflict SHALL be raised

#### Scenario: Nothing is on PATH

- **WHEN** every discovered installation was found in a bounded known location and none is PATH-resolved
- **THEN** the snapshot SHALL report no PATH-selected installation
- **AND** it MAY report a recommended installation, marked as not what the host would run

### Requirement: Structured installation conflicts

The system SHALL report installation conflicts as typed values carrying severity, the installations involved, whether mutation is blocked, whether launch is blocked, and a stable reason code, rather than free text.

#### Scenario: One npm install produces several launcher aliases

- **WHEN** a single logical installation is exposed through several platform launcher aliases such as an extension-less shim, a `.cmd`, and a `.ps1`
- **THEN** the system SHALL treat them as one logical installation with alias information
- **AND** it SHALL NOT report them as several competing installations

#### Scenario: Two sources own separate installations

- **WHEN** two distinct installations of one tool come from different sources
- **THEN** the system SHALL raise a multiple-installation-sources conflict identifying both installations
- **AND** the conflict SHALL state whether it blocks mutation

#### Scenario: A conflict blocks a machine change

- **WHEN** a conflict makes the target of a mutation ambiguous
- **THEN** the conflict SHALL report that mutation is blocked
- **AND** action derivation SHALL withhold mutating actions for that tool

#### Scenario: A conflict is diagnostic only

- **WHEN** a conflict describes duplicate discovery that does not change which executable runs
- **THEN** the conflict SHALL report that neither mutation nor launch is blocked

### Requirement: Platform-specific installation discovery semantics

Discovery SHALL classify installations using platform-specific launcher, path, and architecture rules, and SHALL record what it could not determine rather than guessing.

#### Scenario: Windows launcher resolution

- **WHEN** a tool is discovered on Windows through PATH entries that differ only by launcher extension or by case
- **THEN** discovery SHALL resolve them to one logical installation using the platform's executable extension and case-insensitivity rules

#### Scenario: A launcher points at a missing target

- **WHEN** a discovered launcher, symlink, or junction resolves to a target that no longer exists
- **THEN** discovery SHALL record a stale-launcher-target conflict
- **AND** it SHALL NOT report the launcher as a healthy installation

#### Scenario: Architecture mismatch

- **WHEN** a discovered executable's architecture is known and does not match the host architecture
- **THEN** the system SHALL record an architecture-mismatch conflict
- **AND** compatibility SHALL report unsupported-architecture rather than unknown

#### Scenario: The desktop process PATH differs from the login shell PATH

- **WHEN** an installation is reachable from a login shell but is absent from the PATH this process inherited
- **THEN** the system SHALL record an environment-path-divergence conflict
- **AND** it SHALL NOT claim the installation is missing

### Requirement: Orthogonal CLI environment state

The system SHALL represent discovery, executable health, authentication, readiness, compatibility, update, freshness, source manageability, and conflicts as separate facts.

#### Scenario: Healthy manual installation

- **WHEN** a manually installed CLI is runnable and compatible but its source is detect-only
- **THEN** the system SHALL report the executable as healthy
- **AND** it SHALL report lifecycle actions as unavailable without presenting the CLI as broken

#### Scenario: Installed CLI requires login

- **WHEN** the active executable is healthy but a read-only authentication probe reports that login is required
- **THEN** the environment SHALL report authentication required and readiness needs-auth
- **AND** installation state SHALL remain installed

#### Scenario: Cached state is stale

- **WHEN** a stored environment snapshot has exceeded its freshness window or its environment fingerprint no longer matches
- **THEN** the system SHALL retain the snapshot as stale
- **AND** it SHALL not present stale update or action data as freshly verified

### Requirement: Source-specific version catalogs

The system SHALL obtain update candidates and selectable versions from the source selected for the action.

#### Scenario: npm-owned installation

- **WHEN** the active installation is npm-owned and the selected action source is npm
- **THEN** update comparison and selectable versions SHALL come from the npm source catalog

#### Scenario: WinGet-owned installation

- **WHEN** the active installation is WinGet-owned and the selected action source is WinGet
- **THEN** update comparison and selectable versions SHALL come from WinGet
- **AND** npm catalog data SHALL NOT determine WinGet update state

#### Scenario: Detect-only source

- **WHEN** the active source has no supported version catalog
- **THEN** update state SHALL be not-applicable, catalog-unavailable, or unknown
- **AND** the system SHALL NOT borrow a catalog from another source

#### Scenario: Version cannot be ordered

- **WHEN** the source returns an opaque version that cannot be safely ordered
- **THEN** the system MAY compare equality
- **AND** it SHALL NOT infer upgrade or downgrade from unequal opaque values

### Requirement: Backend-derived allowed actions

The backend SHALL derive allowed lifecycle actions from the current environment snapshot, selected source capability, version catalog, and platform preflight.

#### Scenario: Target is newer

- **WHEN** the selected exact target is newer than the active version and the source supports exact upgrade
- **THEN** the backend SHALL offer upgrade for that source and target

#### Scenario: Target is current

- **WHEN** the selected target equals the active version
- **THEN** the backend SHALL report current state
- **AND** it SHALL not offer or execute a redundant mutation

#### Scenario: Target is older

- **WHEN** the selected exact target is older than the active version
- **THEN** the backend SHALL offer downgrade only when the selected source explicitly supports it
- **AND** otherwise it SHALL return an unsupported-action reason

#### Scenario: Source is detect-only

- **WHEN** the active source is Homebrew, Bun, Volta, desktop, system, manual, or unknown in this change
- **THEN** the backend SHALL return safe source-native or manual guidance
- **AND** it SHALL not expose an automatic mutation action

### Requirement: Persisted CLI action plans

The system SHALL prepare every CLI machine mutation as a persisted, expiring, single-use action plan before execution.

#### Scenario: Prepare action plan

- **WHEN** a user requests a valid lifecycle action with a selected source and optional target
- **THEN** the backend SHALL persist a plan containing the exact source, action, target, channel, structured command preview, preconditions, warnings, network/elevation flags, snapshot fingerprint, revision, and expiry
- **AND** the planning command SHALL complete through an observable operation

#### Scenario: Execute reviewed plan

- **WHEN** the user confirms a draft plan with its id and expected revision before expiry
- **THEN** the backend SHALL atomically mark the plan executing before launching the external effect
- **AND** it SHALL execute only the source and target recorded in the plan

#### Scenario: Environment changed after review

- **WHEN** the current environment fingerprint differs from the plan fingerprint
- **THEN** execution SHALL reject the plan as stale before launching a process
- **AND** the user SHALL be required to prepare a new plan

#### Scenario: Plan is reused

- **WHEN** an execute request references an expired, executing, completed, cancelled, or otherwise consumed plan
- **THEN** the backend SHALL reject it without repeating the external effect

### Requirement: Explicit source execution with no silent fallback

A CLI action plan SHALL execute exactly one disclosed distribution source.

#### Scenario: Vendor installer fails

- **WHEN** an audited vendor installer exits unsuccessfully, times out, or cannot be downloaded
- **THEN** the operation SHALL fail for that source
- **AND** the system SHALL NOT start npm, WinGet, or another source as an automatic fallback

#### Scenario: Source unavailable

- **WHEN** the selected package manager or required runtime is unavailable during preflight
- **THEN** planning or execution SHALL return a typed source-unavailable or missing-dependency result
- **AND** no alternate source SHALL run silently

### Requirement: Platform-safe vendor installer execution

The system SHALL execute vendor installers only from audited platform-specific templates and bounded temporary files.

#### Scenario: Windows has only a Bash template

- **WHEN** a vendor source has no approved Windows-native execution template
- **THEN** the source SHALL be unavailable for automatic Windows lifecycle actions
- **AND** the backend SHALL NOT fall through to a Unix shell template

#### Scenario: Vendor script is downloaded

- **WHEN** an approved vendor installer is executed
- **THEN** the backend SHALL use HTTPS allowlisting, bounded download size and time, redirect policy, optional checksum or signature verification, temporary-file execution, and cleanup
- **AND** it SHALL NOT use pipe-to-shell, `Invoke-Expression`, or `irm | iex`

### Requirement: Source-native lifecycle execution

The system SHALL propagate the action plan's exact source and target to the selected source adapter.

#### Scenario: npm exact version

- **WHEN** an npm plan records target version `X`
- **THEN** the npm adapter SHALL execute the backend-whitelisted package reference at version `X`
- **AND** it SHALL not substitute a cached latest version

#### Scenario: WinGet exact version

- **WHEN** a WinGet plan records target version `X` and preflight confirms exact-version support
- **THEN** the WinGet adapter SHALL include version `X` in the explicit process arguments

#### Scenario: Unsupported exact target

- **WHEN** a source supports latest-only actions and the user requests an arbitrary exact target
- **THEN** planning SHALL reject the action before execution

### Requirement: Post-mutation verification and partial completion

The system SHALL perform best-effort post-mutation discovery and SHALL preserve the actual observed machine state rather than assuming transactional rollback.

#### Scenario: Mutation succeeds and verifies

- **WHEN** the source process succeeds and post-discovery verifies the target
- **THEN** the operation SHALL succeed with outcome verified
- **AND** the new environment snapshot SHALL become authoritative

#### Scenario: Mutation succeeds but verification fails

- **WHEN** the source process succeeds but the target cannot be verified
- **THEN** the operation SHALL complete with an applied-unverified warning outcome
- **AND** the environment SHALL be marked stale or verification-failed rather than restored to a false old state

#### Scenario: Mutation process fails but machine changed

- **WHEN** the source process fails or is cancelled and post-discovery observes a changed installation
- **THEN** the persisted environment SHALL reflect the observed change
- **AND** the terminal result SHALL identify changed-but-failed

#### Scenario: Post-discovery also fails

- **WHEN** a mutation may have occurred and post-discovery cannot complete
- **THEN** the system SHALL retain last-known fields only as stale
- **AND** it SHALL preserve the mutation warning and diagnostic correlation

### Requirement: Provider-specific read-only diagnostics

The system SHALL run only bounded, non-interactive, provider-specific version, Doctor, and authentication probes.

#### Scenario: Documented authentication probe exists

- **WHEN** a tool definition contains a documented non-interactive authentication probe
- **THEN** the backend SHALL execute it with timeout, cancellation, output bounds, and redaction
- **AND** it SHALL return a normalized authentication state rather than raw credential data

#### Scenario: No safe probe exists

- **WHEN** no stable safe non-interactive Doctor or authentication probe is defined
- **THEN** the system SHALL report unknown
- **AND** it SHALL not infer readiness by reading or exposing credentials

#### Scenario: Probe outputs sensitive data

- **WHEN** a probe emits secret-like content
- **THEN** the content SHALL be redacted before operation storage, frontend delivery, and disk persistence

### Requirement: Observable CLI operations

Refresh, planning, lifecycle execution, bulk execution, and Doctor work SHALL use backend-managed observable operations.

#### Scenario: Start variable-duration CLI work

- **WHEN** a request may enumerate paths, execute a process, access a package catalog, download an installer, or run diagnostics
- **THEN** the command SHALL return a stable operation id before that work completes

#### Scenario: Observe CLI operation

- **WHEN** the frontend polls or subscribes to a CLI operation
- **THEN** it SHALL receive lifecycle status, optional phase, optional bounded progress, cancellability, timestamps, redacted bounded logs, and terminal result or error

#### Scenario: Cancel CLI operation

- **WHEN** a user cancels a cancellable CLI operation
- **THEN** the backend SHALL stop not-yet-started work and attempt to terminate the active process
- **AND** it SHALL release operation and mutation reservations exactly once

### Requirement: Mutation coordination

The system SHALL serialize conflicting CLI mutations by tool and source mutation key while allowing unrelated read-only interaction.

#### Scenario: Same package manager is busy

- **WHEN** two operations require the same mutation key
- **THEN** the backend SHALL queue or reject the second deterministically
- **AND** it SHALL not launch overlapping writes for that key

#### Scenario: Unrelated tool is inspected

- **WHEN** one CLI mutation is running and the user opens another tool's details or reads cached state
- **THEN** the unrelated interaction SHALL remain available

#### Scenario: Global mutation bound is reached

- **WHEN** two independent mutations are already active
- **THEN** additional mutations SHALL remain queued until capacity is released

### Requirement: Bulk upgrade preview and item outcomes

The system SHALL prepare a persisted bulk plan that distinguishes eligible actions from skipped tools before execution.

#### Scenario: Prepare bulk upgrade

- **WHEN** the user requests a bulk upgrade preview
- **THEN** the backend SHALL return each eligible source/version transition
- **AND** it SHALL return every skipped tool with a stable reason

#### Scenario: Execute bulk plan

- **WHEN** the user confirms a valid bulk plan
- **THEN** each item SHALL run through the normal action-plan and mutation-coordinator rules
- **AND** the terminal result SHALL contain one outcome for every eligible and skipped item

#### Scenario: One item becomes stale

- **WHEN** one item fingerprint changes before it starts
- **THEN** that item SHALL be skipped as stale
- **AND** other still-valid items MAY continue

### Requirement: Fresh cached reads and background refresh

The system SHALL return cached environment snapshots without blocking the page and SHALL refresh stale data in the background.

#### Scenario: Fresh cache exists

- **WHEN** the CLI Management page opens with a fresh snapshot
- **THEN** the system SHALL render it immediately without an unnecessary blocking refresh

#### Scenario: Stale cache exists

- **WHEN** the page opens with a stale snapshot
- **THEN** the system SHALL render the stale data with a freshness indicator
- **AND** it SHALL start at most one matching background refresh

#### Scenario: Environment fingerprint changes

- **WHEN** PATH, platform identity, package-manager availability, or active executable metadata changes
- **THEN** affected snapshots and draft plans SHALL become stale

### Requirement: Additive CLI environment persistence

The system SHALL persist versioned environment snapshots, source catalogs, and action plans through additive SQLite migrations.

#### Scenario: Existing database is upgraded

- **WHEN** a database containing legacy CLI status rows is opened
- **THEN** the migration SHALL preserve those rows and all unrelated data
- **AND** it SHALL create the new tables without a destructive rewrite

#### Scenario: Legacy status is first read

- **WHEN** no new snapshot exists but a legacy row exists
- **THEN** the system SHALL map only trustworthy legacy fields into a stale snapshot
- **AND** it SHALL request a new refresh before enabling mutation actions

#### Scenario: Persisted document is invalid

- **WHEN** a snapshot, catalog, or plan document cannot be decoded
- **THEN** the system SHALL return a safe stale/unknown result and a redacted diagnostic
- **AND** it SHALL not panic or execute a mutation

### Requirement: Honest Web and mock parity

The Web/mock adapter SHALL implement the same CLI environment and operation contracts without claiming native host inspection or side effects.

#### Scenario: Web page lists CLI tools

- **WHEN** CLI Management is opened in Web/mock runtime
- **THEN** it SHALL show deterministic fixture snapshots and source capabilities suitable for UI testing
- **AND** it SHALL not invent real host paths, credentials, or package-manager state

#### Scenario: Web action is planned and executed

- **WHEN** a mock action is prepared, reviewed, executed, or cancelled
- **THEN** the adapter SHALL produce contract-compatible operation transitions and deterministic outcomes
- **AND** it SHALL execute no local process or filesystem mutation

### Requirement: Bounded and redacted CLI output

The system SHALL bound and redact CLI process output before retaining, displaying, or persisting it.

#### Scenario: Output exceeds the budget

- **WHEN** a version, Doctor, authentication, or lifecycle process emits more than its configured budget
- **THEN** the system SHALL retain one truncation marker and no additional retained output beyond the budget
- **AND** it SHALL continue safe process draining or termination without deadlock

#### Scenario: Sensitive value is emitted

- **WHEN** output contains a password, token, API key, bearer value, cookie, OAuth code, or provider credential pattern
- **THEN** the sensitive value SHALL be replaced before the operation log is visible to the frontend or written to disk

