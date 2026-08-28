## ADDED Requirements

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
