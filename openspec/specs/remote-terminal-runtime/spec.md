# remote-terminal-runtime Specification

## Purpose
TBD - created by archiving change add-remote-terminal-management. Update Purpose after archive.
## Requirements
### Requirement: Authenticated remote Terminal
The desktop runtime SHALL open an SSH-backed PTY channel for a remote session only after resolving a valid SSH profile binding, verifying the server host key, and authenticating with native-owned credentials.

#### Scenario: Open bound remote Terminal
- **WHEN** a remote session has a current SSH profile binding and trusted host key
- **THEN** the runtime SHALL authenticate and open a PTY channel in the session remote path
- **AND** React SHALL receive the connected state through the existing service boundary

#### Scenario: Reject unbound remote Terminal
- **WHEN** a remote session has no SSH profile binding
- **THEN** the system SHALL keep its remote workspace snapshot readable and require an explicit profile bind before opening Terminal

#### Scenario: Reject stale binding
- **WHEN** the bound SSH profile revision or endpoint no longer matches the session binding and snapshot
- **THEN** the runtime SHALL block connection and require explicit rebind

### Requirement: SSH host identity verification
The desktop runtime MUST verify SSH server host identity before sending authentication credentials.

#### Scenario: Confirm first-seen host key
- **WHEN** a server presents an unknown host key
- **THEN** the service SHALL expose a bounded endpoint, algorithm, and fingerprint challenge
- **AND** it SHALL persist trust only after explicit user confirmation

#### Scenario: Reject changed host key
- **WHEN** a trusted endpoint presents a different host key
- **THEN** the runtime SHALL block authentication and identify the key change without automatically replacing trust

### Requirement: Reusable SSH connection pool
The desktop runtime SHALL reuse one authenticated SSH transport for compatible concurrent Terminal and quick-command channels.

#### Scenario: Reuse matching transport
- **WHEN** multiple operations use the same SSH connection id and profile revision while its transport is healthy
- **THEN** the pool SHALL establish at most one authenticated transport and open independent channels on it

#### Scenario: Do not share incompatible credentials
- **WHEN** operations use different profile ids or revisions even if host, port, and user match
- **THEN** the pool SHALL NOT reuse the same authenticated transport entry

#### Scenario: Drain edited profile
- **WHEN** an SSH profile revision changes or the profile is deleted
- **THEN** the pool SHALL reject new leases from the old entry and close it after active leases end or the drain timeout expires

#### Scenario: Evict idle connection
- **WHEN** a healthy pooled transport has no leases beyond the idle limit or the pool exceeds its capacity
- **THEN** the runtime SHALL close it without terminating channels owned by another pool entry

### Requirement: Independent remote channel lifecycle
Each remote Terminal SHALL own an independent PTY channel even when its SSH transport is shared.

#### Scenario: Resize remote PTY
- **WHEN** the visible remote Terminal dimensions change
- **THEN** the runtime SHALL resize only the matching PTY channel

#### Scenario: Disconnect one Terminal
- **WHEN** the user disconnects one remote Terminal
- **THEN** the runtime SHALL close that channel without closing a shared healthy transport still leased by other operations

#### Scenario: Shared transport fails
- **WHEN** a pooled transport becomes unavailable
- **THEN** every dependent channel SHALL transition to failed with a concise error
- **AND** the system SHALL NOT claim that their interactive remote processes were restored

### Requirement: Simulated Web remote Terminal
The Web/mock runtime SHALL provide deterministic remote Terminal semantics without opening a network connection or storing real credentials.

#### Scenario: Open Web remote Terminal
- **WHEN** a Web user opens a bound mock remote session
- **THEN** the adapter SHALL return a clearly labelled simulated channel and deterministic output

#### Scenario: Web host trust
- **WHEN** Web mode exercises a host-trust flow
- **THEN** it SHALL label the result simulated and SHALL NOT claim native SSH verification

### Requirement: Remote terminal performance evidence is bounded
The terminal benchmark SHALL cover a versioned long-output dataset, bounded UTF-8 chunks, retained buffer capacity, indexed search pages, cancellation, and dropped-content gap behavior without retaining raw terminal content in result records.

#### Scenario: Long terminal history is searched
- **WHEN** the versioned long-terminal dataset is captured and searched
- **THEN** chunk size, retained bytes, loaded rows, query count, and result page size SHALL remain within deterministic budgets
- **AND** P50/P95 latency SHALL be recorded only as dedicated evidence

#### Scenario: Terminal dataset exceeds safety limits
- **WHEN** fixture content or requested result size exceeds its declared bound
- **THEN** the harness SHALL reject or truncate it according to the existing terminal contract and record only bounded counts and reason codes

### Requirement: Pooled SSH transport supports Agent Runner channels
The existing authenticated SSH pool SHALL publish the bounded channel operations required by SSH Agent Runs while preserving profile revision, host trust, credential, capacity, lease, keepalive, drain, and shutdown invariants. Agent Runner channels MUST remain independent from Terminal and quick-command channels sharing the transport.

#### Scenario: Reuse a transport for Terminal and Agent execution
- **WHEN** a remote Terminal and SSH Agent Run use the same current compatible profile revision
- **THEN** both lease the same healthy authenticated transport and own independent channels

#### Scenario: Cancel one SSH Agent Run
- **WHEN** the user cancels one Run on a shared transport
- **THEN** only its remote process/channel is terminated and unrelated Terminal or Agent channels remain usable

### Requirement: SSH Agent disconnect and reconnect are bounded
An SSH Agent Run SHALL detect transport or channel loss, stop consuming stale events, and attempt reconnect only when the Runner declares recovery support, policy budget remains, and profile/host/credential/permission authority is current. Reconnect MUST NOT replay provider input or destructive work.

#### Scenario: Network loss is transient
- **WHEN** a recoverable remote reference survives a transport drop and authority remains current
- **THEN** bounded reconnect inspection either resumes event observation or records a safe terminal/attention outcome

#### Scenario: Network loss is not recoverable
- **WHEN** no verified remote reference exists or reconnect budget is exhausted
- **THEN** the Run stops reporting running and cleanup releases its channel lease

### Requirement: Typed Session Shell runtime descriptor

The Session Shell service SHALL return a discriminated runtime descriptor for `native`, `remote`, `simulated`, or `unavailable` behavior, including only capabilities actually supported by that runtime.

#### Scenario: Create a local Session Shell

- **WHEN** a local workspace Session Shell is created in the desktop runtime
- **THEN** the descriptor SHALL identify `native` and its resize, replay, and reconnect capabilities

#### Scenario: Create a remote Session Shell

- **WHEN** a remote workspace Session Shell is created through a current trusted SSH binding
- **THEN** the descriptor SHALL identify `remote`, the safe connection/profile revision identifiers, and supported resize/replay/reconnect capabilities
- **AND** the frontend transport schema SHALL accept that native value without an unsafe type assertion

#### Scenario: Render Web simulation

- **WHEN** Web/mock mode creates a Session Shell
- **THEN** the descriptor SHALL identify `simulated`
- **AND** it SHALL not claim a local process or SSH channel exists

#### Scenario: Shell runtime is unavailable

- **WHEN** the selected session cannot safely open a Shell
- **THEN** the descriptor SHALL identify `unavailable` with a stable reason code and safe remediation when known

### Requirement: Retained Session Shell lifecycle

The workspaces native runtime SHALL own Session Shell process/channel lifetime independently from the mounted React Shell view. React view cleanup SHALL detach, and only an explicit close operation, idle cleanup, terminal process exit, unrecoverable channel failure, or application shutdown SHALL close the native Shell.

#### Scenario: Switch away from Shell tab

- **WHEN** a user leaves the Shell tab while a Session Shell is live
- **THEN** the frontend SHALL detach its view from the Shell stream
- **AND** the native Shell process or remote channel SHALL remain live within configured capacity and idle policy

#### Scenario: Switch to another session

- **WHEN** a user switches sessions while the previous session has a retained Session Shell
- **THEN** the previous Shell SHALL remain associated with its original session and seat
- **AND** returning to that session SHALL attach to the retained Shell when it remains live

#### Scenario: Explicitly close Shell

- **WHEN** the user confirms Close for a Session Shell
- **THEN** the workspaces runtime SHALL terminate only that local process or remote channel and release its workers/resources
- **AND** closing one remote Shell SHALL not close unrelated channels on the shared SSH transport

#### Scenario: Application shuts down

- **WHEN** application shutdown begins
- **THEN** the native registry SHALL close all retained Session Shell processes/channels and join or cancel their owned workers within bounded shutdown policy

### Requirement: Multiple Session Shell instances

A session SHALL support a bounded set of independently named Session Shell instances, each with stable Shell id, optional seat id, runtime descriptor, state, creation time, and last-activity time.

#### Scenario: Create another Shell

- **WHEN** the user activates Add Shell within configured session capacity
- **THEN** the runtime SHALL create an independent Shell descriptor and channel/process
- **AND** the existing Shell SHALL remain live

#### Scenario: Rename a Shell

- **WHEN** the user renames a retained Shell with a bounded valid title
- **THEN** the registry and frontend SHALL show the new title without changing Shell id, process/channel, session, seat, or replay sequence

#### Scenario: Reach Shell capacity

- **WHEN** a session or application reaches the configured retained-Shell capacity
- **THEN** creation SHALL fail with a typed capacity result
- **AND** no existing Shell SHALL be terminated automatically to make room without declared eviction policy and user-visible evidence

#### Scenario: Multi-Agent Shell ownership

- **WHEN** a multi-Agent session creates or attaches a Shell
- **THEN** the descriptor SHALL identify one concrete owning seat
- **AND** changing the selected seat SHALL not silently reassign an existing Shell

### Requirement: Attachment ownership is explicit and stale-safe

Attaching a view to a retained Session Shell SHALL return an attachment identifier, and detach, write, and resize SHALL carry it. A Shell SHALL hold at most one current attachment, and an operation naming an attachment that is no longer current SHALL NOT affect the attachment that replaced it.

#### Scenario: Cleanup runs after a newer view has attached

- **WHEN** a view's cleanup detaches with an attachment identifier that is no longer current
- **THEN** the operation SHALL succeed as an idempotent no-op
- **AND** the current attachment SHALL remain attached and continue receiving frames

#### Scenario: Write from a replaced view

- **WHEN** a write or resize carries an attachment identifier that is no longer current
- **THEN** the service SHALL refuse it with a typed `shell_attachment_stale` result
- **AND** the input SHALL NOT reach the Shell

#### Scenario: Attach a Shell that does not exist

- **WHEN** a view attaches to a Shell id the registry does not hold
- **THEN** the service SHALL return a typed not-found result
- **AND** it SHALL NOT create a Shell

#### Scenario: Application restarts

- **WHEN** the application restarts
- **THEN** no attachment and no Session Shell SHALL be restored from before the restart
- **AND** the workspace SHALL show no retained Shell rather than replay for a process that no longer exists

### Requirement: Sequence-numbered Shell replay and attach

Every retained Session Shell SHALL emit UTF-8-safe sequence-numbered output frames, retain bounded replay, and attach a view from a declared sequence without duplicating replay and live output.

#### Scenario: Reattach to retained output

- **WHEN** a view attaches to a live retained Shell with a last-consumed sequence
- **THEN** the service SHALL return a bounded replay snapshot after that sequence plus the next sequence boundary
- **AND** subsequent live frames SHALL continue monotonically

#### Scenario: Frames arrive while the attach request is in flight

- **WHEN** a view registers its listener before requesting attach and frames arrive before the snapshot returns
- **THEN** those frames SHALL be buffered rather than dropped
- **AND** they SHALL be reconciled against the snapshot by Shell id and sequence so each frame is applied once

#### Scenario: Distinguish a dropped frame from a race

- **WHEN** a subscriber holds an attach snapshot and receives a live frame
- **THEN** the snapshot's next sequence SHALL be the exact sequence the next frame carries
- **AND** the subscriber SHALL treat a higher sequence as a gap rather than inferring one from timing

#### Scenario: Retained output exceeds capacity

- **WHEN** a Shell emits enough output to exceed the 1 MiB retained replay bound
- **THEN** the registry SHALL evict only the oldest chunks needed to restore the bound
- **AND** a later attach SHALL receive one gap marker before available newer frames

#### Scenario: Replay and live event overlap

- **WHEN** a frame is included in an attach snapshot and also arrives through a live subscription race
- **THEN** the frontend SHALL de-duplicate it by Shell id and sequence

#### Scenario: Output streams are not distinguishable

- **WHEN** a PTY provides merged output
- **THEN** frames SHALL identify `pty` rather than falsely classifying stdout and stderr

### Requirement: Observable Session Shell command boundaries

When Shell integration can observe command boundaries, the native runtime SHALL publish metadata-only command start/completion evidence with fidelity, timing, exit state, Shell/session/seat correlation, and bounded redacted display availability. It SHALL preserve `opaque` behavior when boundaries are unavailable.

#### Scenario: Observe a command boundary and exit code

- **WHEN** native Shell integration verifies a command start and terminal exit code
- **THEN** it SHALL publish one correlated command record with native or proxied fidelity
- **AND** it SHALL not publish raw terminal output or unrestricted command arguments into telemetry

#### Scenario: Shell integration is unavailable

- **WHEN** a shell does not expose reliable command markers
- **THEN** the runtime MAY expose only Shell/process lifecycle with `opaque` fidelity
- **AND** it SHALL not split terminal text heuristically into authoritative commands

### Requirement: Session Shell errors are visible and typed

Shell input, resize, attach, detach, reconnect, and close failures SHALL return or publish typed state/error information; frontend code SHALL NOT silently discard rejected asynchronous operations.

#### Scenario: Write after remote disconnect

- **WHEN** the frontend writes to a remote Shell whose channel has failed
- **THEN** the service SHALL return a typed channel/state error
- **AND** the Shell UI SHALL retain available replay and show the failed state

#### Scenario: Resize fails

- **WHEN** PTY resize fails for a live Shell
- **THEN** the failure SHALL be reported through bounded state/diagnostic channels
- **AND** the UI SHALL remain attached when the underlying Shell remains usable

#### Scenario: Reconnect is unsupported

- **WHEN** a failed Shell descriptor states reconnect is unsupported
- **THEN** the UI SHALL not offer or automatically attempt reconnect
- **AND** it MAY offer explicit Close and Create New Shell actions

### Requirement: Session Shell frontend service parity

The Session Shell lifecycle SHALL be exposed through one frontend service interface implemented by Tauri and Web/mock adapters with equivalent descriptor, state, attach/detach, replay, write, resize, rename, and close semantics.

#### Scenario: React attaches a desktop Shell

- **WHEN** the Shell tab becomes visible in the desktop runtime
- **THEN** React SHALL call the frontend Shell service
- **AND** the Tauri adapter SHALL own native commands and event listeners

#### Scenario: React hides a Web Shell

- **WHEN** a simulated Web Shell view becomes hidden or unmounts
- **THEN** the Web adapter SHALL detach the simulated view while retaining its bounded in-memory Shell until explicit close or simulated idle cleanup

