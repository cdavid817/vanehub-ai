## ADDED Requirements

### Requirement: Shell startup reserves capacity and ownership before external execution

The session Shell application service SHALL atomically reserve every applicable capacity limit and SHALL register a generation-qualified Shell in `Opening` before opening a local process or remote channel. Startup SHALL transfer every acquired runtime resource through one launch guard that either commits ownership, confirms rollback, or hands cleanup to a retained Reaper.

#### Scenario: Concurrent opens compete for the last session slot

- **WHEN** two requests concurrently attempt to create different Shells and only one total or per-session capacity slot remains
- **THEN** exactly one request SHALL obtain the capacity reservation
- **AND** the other request SHALL fail with a typed capacity result before spawning a process or opening a remote channel

#### Scenario: Startup fails after the child is created

- **WHEN** a local child is spawned but reader, writer, worker, route, or retained-runtime setup fails before startup commit
- **THEN** the launch guard SHALL retain ownership of every acquired resource
- **AND** it SHALL either confirm cleanup and transition to `OpenFailed` or hand the same generation to the Reaper
- **AND** no child, PTY, channel, worker, or capacity lease SHALL become ownerless

#### Scenario: Shell exits during Opening

- **WHEN** a Shell emits output and exits before startup reaches `Running`
- **THEN** the pre-registered Shell SHALL retain the ordered output and exit evidence
- **AND** a later startup completion SHALL NOT overwrite the terminal state with `Running`

### Requirement: Shell close is bounded, typed, and confirmed before terminal publication

The session Shell close use case SHALL transition an addressable generation through `Closing`, `Reaping`, or `CloseFailed` while runtime cleanup is unconfirmed. It SHALL return within a finite command-path deadline and MUST NOT publish `ShellClosed`, remove lifecycle ownership, release capacity, or report terminal success until the owned process/channel is confirmed terminal.

#### Scenario: Child does not exit within the close deadline

- **WHEN** graceful and forceful close stages cannot confirm child termination within the command-path budget
- **THEN** close SHALL return a typed `Reaping` or `CloseFailed` disposition
- **AND** the same Shell generation, runtime handles, replay state, and capacity lease SHALL remain addressable for retry or Reaper cleanup
- **AND** the system SHALL NOT publish `ShellClosed`

#### Scenario: Reaper later confirms termination

- **WHEN** the retained Reaper confirms termination for the current Shell generation
- **THEN** the system SHALL finalize the terminal state exactly once
- **AND** it SHALL release runtime ownership, route, replay retention according to policy, and capacity exactly once
- **AND** it SHALL publish one terminal event after that finalization

#### Scenario: Close is requested repeatedly

- **WHEN** several callers close a Shell that is already Closing, Reaping, CloseFailed, or terminal
- **THEN** the service SHALL reconcile with the existing generation and close operation
- **AND** it SHALL NOT create competing termination attempts, double-release capacity, or publish duplicate terminal events

### Requirement: Session cleanup reports every Shell outcome

Archive, delete, idle sweep, and application shutdown SHALL consume typed Shell close results and SHALL retain an aggregate cleanup report. They MUST NOT silently discard a close failure or claim a Shell was closed merely because cleanup was requested.

#### Scenario: Session delete encounters a reaping Shell

- **WHEN** session deletion requests closure of all owned Shells and at least one remains `Reaping` or `CloseFailed`
- **THEN** strict deletion SHALL return `session_shell_cleanup_incomplete`
- **AND** the session and Shell identities required for retry and diagnosis SHALL remain available
- **AND** the UI SHALL NOT claim final deletion succeeded

#### Scenario: Idle sweep schedules cleanup

- **WHEN** idle sweep reaches a Shell that cannot close within its bounded attempt
- **THEN** the sweep SHALL record that Shell as reaping or failed rather than closed
- **AND** it SHALL expose bounded structural cleanup evidence without blocking the sweep indefinitely

### Requirement: Frontend and Web adapters expose retained lifecycle semantics

The frontend Shell service, Tauri adapter, and Web/mock adapter SHALL use one typed contract for `Opening`, `Running`, `Closing`, `Reaping`, `CloseFailed`, and terminal outcomes. React components SHALL reconcile events with service reads and SHALL present localized intermediate/failure states without claiming native cleanup that did not occur.

#### Scenario: Close returns Reaping

- **WHEN** the service returns a `Reaping` close disposition
- **THEN** the UI SHALL keep the Shell identifiable and disable or adapt operations that require Running
- **AND** it SHALL show a localized cleanup-in-progress state until pull or event reconciliation produces a terminal or failed result

#### Scenario: Web mode simulates a late old-generation completion

- **WHEN** Web/mock creates a newer generation after an older simulated Shell is closed and then emits a delayed completion for the old generation
- **THEN** the newer generation SHALL remain unchanged
- **AND** capacity and terminal events SHALL be applied only to the matching generation
